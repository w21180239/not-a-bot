//! Microsoft Edge Translator web endpoint: keyless, same models as Bing.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const API_URL: &str = "https://edge.microsoft.com/translate/translatetext";

/// The endpoint rejects user agents without a supported `Edg/` token.
const EDGE_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36 Edg/141.0.0.0";

#[derive(Debug, Deserialize)]
struct Item {
    translations: Option<Vec<Translation>>,
    #[serde(rename = "detectedLanguage")]
    detected_language: Option<DetectedLanguage>,
}

#[derive(Debug, Deserialize)]
struct Translation {
    text: String,
}

#[derive(Debug, Deserialize)]
struct DetectedLanguage {
    language: String,
}

/// Microsoft spells the Chinese variants `zh-Hans` / `zh-Hant`.
pub fn to_ms_code(code: &str) -> String {
    match code {
        "zh-TW" => "zh-Hant".into(),
        "zh" => "zh-Hans".into(),
        other => other.into(),
    }
}

pub struct Edge {
    client: reqwest::Client,
}

impl Edge {
    pub fn new() -> Self {
        Self {
            client: ClientBuilder::build(),
        }
    }
}

/// Client with the Edge UA; the endpoint answers 403 to the default one.
struct ClientBuilder;
impl ClientBuilder {
    fn build() -> reqwest::Client {
        reqwest::Client::builder()
            .user_agent(EDGE_UA)
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .brotli(true)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }
}

impl Default for Edge {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Edge {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "edge".into(),
            display_name: "Microsoft Edge Translator (web)".into(),
            kind: ProviderKind::Web,
            priority: 2,
            needs_key: false,
            max_chars: 50_000,
            auto_detect: true,
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
        let to = to_ms_code(&lang::normalize(&req.target_lang));
        let from = to_ms_code(&lang::normalize(&req.source_lang));

        let mut query: Vec<(&str, String)> =
            vec![("to", to.clone()), ("isEnterpriseClient", "false".into())];
        // `from=auto-detect` is rejected with 400; omitting it auto-detects.
        if from != "auto" {
            query.push(("from", from.clone()));
        }

        let started = Instant::now();
        let resp = self
            .client
            .post(API_URL)
            .query(&query)
            .json(&vec![req.text.clone()])
            .send()
            .await
            .map_err(|e| Error::Network(format!("edge: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("edge", resp).await?;
        if status != 200 {
            return Err(crate::core::error::map_status("edge", status, &body));
        }

        let parsed: Vec<Item> = serde_json::from_str(&body).map_err(|e| Error::BadResponse {
            provider: "edge".into(),
            reason: format!("{}: {}", e, truncate(&body, 160)),
        })?;

        let item = parsed.first().ok_or_else(|| Error::BadResponse {
            provider: "edge".into(),
            reason: "empty response array".into(),
        })?;

        let text = item
            .translations
            .as_ref()
            .and_then(|v| v.first())
            .map(|t| t.text.clone())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "edge".into(),
                reason: "missing translations".into(),
            })?;

        let detected = item
            .detected_language
            .as_ref()
            .map(|d| d.language.clone())
            .unwrap_or_else(|| from.clone());

        Ok(TranslateResponse {
            text,
            detected_source: detected,
            target_lang: req.target_lang.clone(),
            provider: "edge".into(),
            engine: "edge".into(),
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
        let p = Edge::default();
        let info = p.info();
        assert_eq!(info.name, "edge");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 2);
    }

    #[test]
    fn maps_chinese_variants() {
        assert_eq!(to_ms_code("zh"), "zh-Hans");
        assert_eq!(to_ms_code("zh-TW"), "zh-Hant");
        assert_eq!(to_ms_code("en"), "en");
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"[{"detectedLanguage":{"language":"en","score":1.0},"translations":[{"text":"你好，世界","to":"zh-Hans"}]}]"#;
        let parsed: Vec<Item> = serde_json::from_str(body).unwrap();
        assert_eq!(
            parsed[0].translations.as_ref().unwrap()[0].text,
            "你好，世界"
        );
        assert_eq!(parsed[0].detected_language.as_ref().unwrap().language, "en");
    }

    #[test]
    fn missing_translations_is_detectable() {
        let body = r#"[{}]"#;
        let parsed: Vec<Item> = serde_json::from_str(body).unwrap();
        assert!(parsed[0].translations.is_none());
    }
}
