//! Volcengine machine translation, browser-extension endpoint (no key).
//! Mirrors the `volcEngine` engine of the original translators project: the
//! public web endpoint needs no credential, so it acts as a free fallback.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, json};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://translate.volcengine.com/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    translation: Option<String>,
    #[serde(rename = "detected_source_language", default)]
    detected_source_language: Option<String>,
}

/// Volcengine's own codes for the languages the web endpoint accepts.
static CODES: std::sync::LazyLock<BTreeMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        BTreeMap::from([
            ("zh", "zh"),
            ("zh-TW", "zh"),
            ("en", "en"),
            ("ja", "ja"),
            ("ko", "ko"),
            ("fr", "fr"),
            ("de", "de"),
            ("es", "es"),
            ("ru", "ru"),
            ("it", "it"),
            ("pt", "pt"),
            ("th", "th"),
            ("vi", "vi"),
            ("id", "id"),
            ("ar", "ar"),
        ])
    });

pub struct Volcengine {
    client: reqwest::Client,
}

impl Volcengine {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(20)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Volcengine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Volcengine {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "volcengine".into(),
            display_name: "Volcengine (free endpoint)".into(),
            kind: ProviderKind::Free,
            needs_key: false,
            max_chars: 5000,
            auto_detect: true,
            languages: CODES.keys().map(|k| k.to_string()).collect(),
            free_tier: Some("public web endpoint, best effort".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    fn supports(&self, target: &str) -> bool {
        CODES.contains_key(target)
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let target =
            CODES
                .get(to.as_str())
                .ok_or_else(|| Error::LanguagePairNotSupported {
                    from_lang: req.source_lang.clone(),
                    target: to.clone(),
                    provider: "volcengine".into(),
                })?;

        let body = serde_json::json!({
            "SourceLanguage": req.source_lang,
            "TargetLanguage": *target,
            "Text": req.text,
        });

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("volcengine: {e}")))?;

        let parsed: Resp = json("volcengine", resp).await?;
        let text =
            parsed
                .translation
                .filter(|t| !t.is_empty())
                .ok_or_else(|| Error::BadResponse {
                    provider: "volcengine".into(),
                    reason: "missing translation".into(),
                })?;

        Ok(TranslateResponse {
            text,
            detected_source: parsed
                .detected_source_language
                .unwrap_or_else(|| req.source_lang.clone()),
            target_lang: to,
            provider: "volcengine".into(),
            engine: "volcengine".into(),
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
    fn keyless_provider_reports_free() {
        let p = Volcengine::new();
        let info = p.info();
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Free);
        assert!(p.supports("ja"));
        assert!(!p.supports("xx"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"translation":"你好","detected_source_language":"en"}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.translation.as_deref(), Some("你好"));
        assert_eq!(p.detected_source_language.as_deref(), Some("en"));
    }

    #[test]
    fn empty_translation_parses_as_unusable() {
        let body = r#"{"translation":""}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert!(p.translation.filter(|t| !t.is_empty()).is_none());
    }

    #[tokio::test]
    async fn unsupported_target_is_rejected_before_the_request() {
        let p = Volcengine::new();
        let cred = Credential {
            provider: "volcengine".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "auto".into(),
            target_lang: "xx".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(
            p.translate_once(&req, &cred).await.unwrap_err().code(),
            "LANGUAGE_PAIR_NOT_SUPPORTED"
        );
    }
}
