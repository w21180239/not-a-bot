//! Lingvanex (B1 / Backenster) free web endpoint: bearer token scraped from the page.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const HOST: &str = "https://lingvanex.com/en/translate/";
const API: &str = "https://api-b2b.backenster.com/b1/api/v3/translate/";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Lingvanex uses `xx_YY` region forms and `zh-Hans_CN` / `zh-Hant_TW` for Chinese.
pub fn to_lingvanex_code(code: &str) -> String {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-Hans_CN".into(),
        "zh-TW" | "zh-Hant" => "zh-Hant_TW".into(),
        "en" => "en_GB".into(),
        "pt" => "pt_PT".into(),
        "he" => "he_IL".into(),
        "ar" => "ar_AE".into(),
        other => other.into(),
    }
}

/// Pull the `const API_BEARER_TOKEN = "..."` literal out of the page HTML.
pub fn extract_token(html: &str) -> Option<String> {
    let marker = "const API_BEARER_TOKEN = \"";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    let token = &rest[..end];
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct Resp {
    result: Option<String>,
    err: Option<serde_json::Value>,
}

pub struct Lingvanex {
    client: reqwest::Client,
}

impl Lingvanex {
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

    async fn token(&self) -> Result<String> {
        let resp = self
            .client
            .get(HOST)
            .send()
            .await
            .map_err(|e| Error::Network(format!("lingvanex: {e}")))?;
        let status = resp.status().as_u16();
        let html = body_text("lingvanex", resp).await?;
        if status != 200 {
            return Err(map_status("lingvanex", status, &html));
        }
        extract_token(&html).ok_or_else(|| Error::BadResponse {
            provider: "lingvanex".into(),
            reason: "API_BEARER_TOKEN not found in page".into(),
        })
    }
}

impl Default for Lingvanex {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Lingvanex {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "lingvanex".into(),
            display_name: "Lingvanex (web)".into(),
            kind: ProviderKind::Web,
            priority: 6,
            needs_key: false,
            max_chars: 1_000,
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
        let to = to_lingvanex_code(&lang::normalize(&req.target_lang));
        let from_raw = lang::normalize(&req.source_lang);
        // No detection endpoint: the reference substitutes the provider default.
        let from = if from_raw == "auto" {
            to.clone()
        } else {
            to_lingvanex_code(&from_raw)
        };

        let token = self.token().await?;

        let form = [
            ("from", from.as_str()),
            ("to", to.as_str()),
            ("text", req.text.as_str()),
            ("platform", "dp"),
        ];

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .query(&[
                ("client", "site"),
                ("feature", "seo_text"),
                ("lang_pair", "en_te"),
            ])
            .header("authorization", &token)
            .header("referer", "https://lingvanex.com")
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("lingvanex: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("lingvanex", resp).await?;
        if status != 200 {
            return Err(map_status("lingvanex", status, &body));
        }

        let parsed: Resp = serde_json::from_str(&body).map_err(|e| Error::BadResponse {
            provider: "lingvanex".into(),
            reason: format!("{e}: {}", truncate(&body, 160)),
        })?;

        if matches!(&parsed.err, Some(v) if !v.is_null()) {
            return Err(Error::Upstream {
                provider: "lingvanex".into(),
                status,
                body: truncate(&body, 200),
            });
        }

        let text = parsed
            .result
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "lingvanex".into(),
                reason: format!("missing result: {}", truncate(&body, 160)),
            })?;

        Ok(TranslateResponse {
            text: text.replace("\n ", "\n"),
            detected_source: from,
            target_lang: req.target_lang.clone(),
            provider: "lingvanex".into(),
            engine: "lingvanex".into(),
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
        let p = Lingvanex::default();
        let info = p.info();
        assert_eq!(info.name, "lingvanex");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 6);
    }

    #[test]
    fn extracts_bearer_token() {
        let html = r#"<script>const API_BEARER_TOKEN = "Bearer abc_123";</script>"#;
        assert_eq!(extract_token(html).as_deref(), Some("Bearer abc_123"));
    }

    #[test]
    fn missing_token_is_none() {
        assert_eq!(extract_token("<html>nope</html>"), None);
        assert_eq!(
            extract_token(r#"const API_BEARER_TOKEN = """#),
            None,
            "empty token must not pass"
        );
    }

    #[test]
    fn maps_language_codes() {
        assert_eq!(to_lingvanex_code("zh"), "zh-Hans_CN");
        assert_eq!(to_lingvanex_code("zh-TW"), "zh-Hant_TW");
        assert_eq!(to_lingvanex_code("en"), "en_GB");
        assert_eq!(to_lingvanex_code("ja"), "ja");
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from api-b2b.backenster.com. `err: null` deserialises to None.
        let body = r#"{"err":null,"result":"你好,世界"}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.result.as_deref(), Some("你好,世界"));
        assert!(p.err.is_none());
    }

    #[test]
    fn error_field_is_detectable() {
        let p: Resp = serde_json::from_str(r#"{"err":"Bad authorization code"}"#).unwrap();
        assert!(p.result.is_none());
        assert!(!p.err.as_ref().unwrap().is_null());
    }
}
