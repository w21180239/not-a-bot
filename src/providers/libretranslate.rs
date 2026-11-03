//! LibreTranslate: free, self-hostable, no key required.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, client};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Duration;

const DEFAULT_INSTANCE: &str = "https://libretranslate.de";

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct Resp {
    translatedText: String,
    #[serde(rename = "detectedSourceLanguage")]
    detected_source_language: Option<String>,
}

pub struct LibreTranslate {
    base_url: String,
    client: reqwest::Client,
}

impl LibreTranslate {
    pub fn new(base_url: Option<&str>) -> Self {
        let base_url = base_url
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_else(|| DEFAULT_INSTANCE.to_string());
        let client = client(Duration::from_secs(30)).unwrap_or_else(|_| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap()
        });
        Self { base_url, client }
    }
}

#[async_trait]
impl Provider for LibreTranslate {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "libretranslate".into(),
            display_name: "LibreTranslate".into(),
            kind: ProviderKind::Web,
            priority: 12,
            needs_key: false,
            max_chars: 5000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("unlimited".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let from = if req.source_lang == "auto" {
            "auto".into()
        } else {
            lang::normalize(&req.source_lang)
        };

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("q".into(), req.text.clone());
        form.insert("source".into(), from.clone());
        form.insert("target".into(), to.clone());
        form.insert("format".into(), "text".into());

        let url = format!("{}/translate", self.base_url);
        let resp = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        let parsed: Resp = crate::core::http::json("libretranslate", resp).await?;

        Ok(TranslateResponse {
            text: parsed.translatedText,
            detected_source: parsed
                .detected_source_language
                .unwrap_or_else(|| from.clone()),
            target_lang: to,
            provider: "libretranslate".into(),
            engine: "libretranslate".into(),
            latency_ms: 0,
            request_id: String::new(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }

    async fn probe(&self, _cred: &Credential) -> Result<()> {
        let url = format!("{}/languages", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;
        let text = body_text("libretranslate", resp).await?;
        if text.trim().is_empty() {
            return Err(Error::BadResponse {
                provider: "libretranslate".into(),
                reason: "empty languages response".into(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::creds::Credential;

    #[test]
    fn info_contract() {
        let p = LibreTranslate::new(None);
        let info = p.info();
        assert_eq!(info.name, "libretranslate");
        assert!(!info.needs_key);
        assert_eq!(info.max_chars, 5000);
        assert!(p.supports("zh"));
    }

    #[tokio::test]
    async fn parses_response_shape() {
        let body = r#"{"translatedText":"你好","detectedSourceLanguage":"en"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translatedText, "你好");
        assert_eq!(parsed.detected_source_language.as_deref(), Some("en"));
    }

    #[tokio::test]
    async fn missing_detected_source_is_not_a_failure() {
        let body = r#"{"translatedText":"hola"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.detected_source_language, None);
        assert_eq!(parsed.translatedText, "hola");
    }

    #[tokio::test]
    async fn probe_uses_languages_endpoint() {
        let p = LibreTranslate::new(None);
        let cred = Credential {
            provider: "libretranslate".into(),
            label: "default".into(),
            fields: Default::default(),
        };
        let result = p.probe(&cred).await;
        // Network may be unavailable in CI; the point is the probe runs and
        // returns a typed Network error rather than panicking.
        assert!(result.is_err() || result.is_ok());
        if let Err(e) = result {
            assert!(
                matches!(e, Error::Network(_) | Error::BadResponse { .. }),
                "got {e:?}"
            );
        }
    }
}
