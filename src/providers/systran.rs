//! SysTran free web endpoint: OAuth client credentials lifted from the site bundle.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const HOST: &str = "https://www.systran.net/translate/";
const API: &str = "https://api-translate.systran.net/translation/text/translate";
const TOKEN_URL: &str = "https://translate.systran.net/oidc/token";
const BUNDLE: &str = "https://www.systransoft.com/wp-content/themes/systran/dist/translatebox/translateBox.bundle.js";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// `client_id` and `client_secret` live next to the oidc URL in the bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientData {
    pub client_id: String,
    pub client_secret: String,
}

/// Pull the two credentials out of translateBox.bundle.js.
///
/// The site minifies, so the shape is
/// `"https://translate.systran.net/oidc",XXXX="id",YYYY="secret";`.
pub fn extract_client_data(js: &str) -> Option<ClientData> {
    let marker = "\"https://translate.systran.net/oidc\",";
    let start = js.find(marker)? + marker.len();
    let tail = &js[start..js.len().min(start + 400)];
    let first = quote_value(tail)?;
    let after_first = &tail[first.consumed..];
    let second = quote_value(after_first)?;
    Some(ClientData {
        client_id: first.value,
        client_secret: second.value,
    })
}

struct Found {
    value: String,
    consumed: usize,
}

/// Find the next `name="value"` pair and report how much input it ate.
fn quote_value(s: &str) -> Option<Found> {
    let eq = s.find("=\"")?;
    let rest = &s[eq + 2..];
    let end = rest.find('"')?;
    Some(Found {
        value: rest[..end].to_string(),
        consumed: eq + 2 + end + 1,
    })
}

#[derive(Debug, Deserialize)]
struct TokenResp {
    access_token: Option<String>,
    token_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct Payload<'a> {
    target: &'a str,
    source: Option<&'a str>,
    inputs: Vec<&'a str>,
    format: &'static str,
    #[serde(rename = "autodetectionMode")]
    autodetection_mode: &'static str,
    #[serde(rename = "withInfo")]
    with_info: &'static str,
    #[serde(rename = "withAnnotations")]
    with_annotations: &'static str,
    #[serde(rename = "profileId")]
    profile_id: Option<String>,
    domain: Option<String>,
    owner: Option<String>,
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Resp {
    outputs: Option<Vec<Output>>,
}

#[derive(Debug, Deserialize)]
struct Output {
    output: DocWrapper,
}

#[derive(Debug, Deserialize)]
struct DocWrapper {
    documents: Vec<Document>,
}

#[derive(Debug, Deserialize)]
struct Document {
    #[serde(rename = "trans_units")]
    trans_units: Vec<TransUnit>,
}

#[derive(Debug, Deserialize)]
struct TransUnit {
    sentences: Vec<Sentence>,
}

#[derive(Debug, Deserialize)]
struct Sentence {
    #[serde(rename = "alt_transes")]
    alt_transes: Vec<AltTrans>,
}

#[derive(Debug, Deserialize)]
struct AltTrans {
    target: Target,
}

#[derive(Debug, Deserialize)]
struct Target {
    text: Option<String>,
}

/// Join the nested SysTran reply back into paragraphs.
fn flatten(outputs: &[Output]) -> Option<String> {
    let paragraphs: Vec<String> = outputs
        .iter()
        .filter_map(|o| {
            let joined = o
                .output
                .documents
                .iter()
                .flat_map(|d| d.trans_units.iter())
                .flat_map(|u| u.sentences.iter())
                .filter_map(|s| {
                    s.alt_transes
                        .first()
                        .and_then(|a| a.target.text.clone())
                        .filter(|t| !t.is_empty())
                })
                .collect::<Vec<_>>()
                .join(" ");
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        })
        .collect();
    if paragraphs.is_empty() {
        None
    } else {
        Some(paragraphs.join("\n"))
    }
}

pub struct SysTran {
    client: reqwest::Client,
}

impl SysTran {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(UA)
                .cookie_store(true)
                .timeout(Duration::from_secs(30))
                .gzip(true)
                .brotli(true)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// Fetch the bundle and exchange its client credentials for a bearer token.
    async fn token(&self) -> Result<String> {
        let js = {
            let resp = self
                .client
                .get(BUNDLE)
                .header("Referer", "https://www.systransoft.com/")
                .send()
                .await
                .map_err(|e| Error::Network(format!("systran bundle: {e}")))?;
            body_text("systran", resp).await?
        };
        let creds = extract_client_data(&js).ok_or_else(|| Error::BadResponse {
            provider: "systran".into(),
            reason: "oidc client credentials not found in bundle".into(),
        })?;

        let form = [
            ("grant_type", "client_credentials"),
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
        ];
        let resp = self
            .client
            .post(TOKEN_URL)
            .header("Origin", "https://www.systran.net")
            .header("Referer", HOST)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("systran token: {e}")))?;
        let status = resp.status().as_u16();
        let body = body_text("systran", resp).await?;
        if status != 200 {
            return Err(map_status("systran", status, &body));
        }
        let parsed: TokenResp =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "systran".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;
        let access = parsed.access_token.ok_or_else(|| Error::BadResponse {
            provider: "systran".into(),
            reason: "no access_token".into(),
        })?;
        Ok(format!(
            "{} {}",
            parsed.token_type.unwrap_or_else(|| "Bearer".into()),
            access
        ))
    }
}

impl Default for SysTran {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for SysTran {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "systran".into(),
            display_name: "SysTran (web)".into(),
            kind: ProviderKind::Web,
            priority: 12,
            needs_key: false,
            max_chars: 5_000,
            auto_detect: false,
            languages: vec!["*".into()],
            free_tier: Some("unlimited (web endpoint)".into()),
            configured: true,
        }
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let from_raw = lang::normalize(&req.source_lang);
        // No detection with a real engine: fall back to Chinese, as the reference does.
        let from = if from_raw == "auto" { "zh" } else { &from_raw };

        let token = self.token().await?;

        let inputs: Vec<&str> = req
            .text
            .split('\n')
            .filter(|p| !p.trim().is_empty())
            .collect();

        let payload = Payload {
            target: &to,
            source: Some(from),
            inputs,
            format: "text/plain",
            autodetection_mode: "single",
            with_info: "true",
            with_annotations: "true",
            profile_id: None,
            domain: Some("Generic".into()),
            owner: Some("Systran".into()),
            size: Some("L".into()),
        };

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .header("authorization", &token)
            .header("x-user-agent", "File Translate Box Portable")
            .header("Origin", "https://www.systran.net")
            .header("Referer", HOST)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Network(format!("systran: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("systran", resp).await?;
        if status != 200 {
            return Err(map_status("systran", status, &body));
        }

        let parsed: Resp =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "systran".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;

        let outputs = parsed.outputs.filter(|v| !v.is_empty()).ok_or_else(|| {
            Error::BadResponse {
                provider: "systran".into(),
                reason: format!("missing outputs: {}", truncate(&body, 160)),
            }
        })?;

        let text = flatten(&outputs).ok_or_else(|| Error::BadResponse {
            provider: "systran".into(),
            reason: "empty translation".into(),
        })?;

        Ok(TranslateResponse {
            text,
            detected_source: from.to_string(),
            target_lang: req.target_lang.clone(),
            provider: "systran".into(),
            engine: "systran".into(),
            latency_ms: started.elapsed().as_millis() as u64,
            request_id: String::new(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_contract() {
        let p = SysTran::default();
        let info = p.info();
        assert_eq!(info.name, "systran");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 12);
    }

    #[test]
    fn extracts_oidc_client_pair() {
        // Shape taken from the real (minified) bundle.
        let js = r#"...x("https://translate.systran.net/oidc",ab="CLIENT123",cd="SECRET456");rest"#;
        let got = extract_client_data(js).unwrap();
        assert_eq!(got.client_id, "CLIENT123");
        assert_eq!(got.client_secret, "SECRET456");
    }

    #[test]
    fn missing_bundle_marker_is_none() {
        assert!(extract_client_data("no oidc here").is_none());
        assert!(extract_client_data("\"https://translate.systran.net/oidc\",")
            .is_none());
    }

    #[test]
    fn parses_success_shape() {
        // Trimmed from a live api-translate.systran.net response.
        let body = r#"{"outputs":[{"output":{"documents":[{"trans_units":[{"sentences":[{"alt_transes":[{"id":"","target":{"lang":"zh","text":"大家好，这是一个考验。"}}],"id":"Sp1.s2_o","source":{"lang":"en","text":"Hello world"}}]}],"src_lang":"en","tgt_lang":"zh"}],"version":"1.3"}}]}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        let outputs = p.outputs.unwrap();
        assert_eq!(flatten(&outputs).as_deref(), Some("大家好，这是一个考验。"));
    }

    #[test]
    fn multi_sentence_input_joins_with_spaces() {
        let body = r#"{"outputs":[
            {"output":{"documents":[{"trans_units":[{"sentences":[
                {"alt_transes":[{"target":{"text":"你好"}}]}]}]}]}},
            {"output":{"documents":[{"trans_units":[{"sentences":[
                {"alt_transes":[{"target":{"text":"世界"}}]}]}]}]}}]}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(flatten(&p.outputs.unwrap()).as_deref(), Some("你好\n世界"));
    }

    #[test]
    fn empty_transes_flatten_to_none() {
        let body = r#"{"outputs":[{"output":{"documents":[{"trans_units":[{"sentences":[{"alt_transes":[]}]}]}]}}]}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert!(flatten(&p.outputs.unwrap()).is_none());
    }
}
