//! DeepL API. Highest quality for European language pairs.
//! A `:fx` key is the free tier and must hit api-free.deepl.com, so the host
//! is derived from the key rather than configured.

use crate::core::config::ProviderSettings;
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, json};
use crate::core::lang;
use crate::providers::{Provider, ProviderKind};
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
struct Resp {
    translations: Vec<Translation>,
}

#[derive(Debug, Deserialize)]
struct Translation {
    text: String,
    #[serde(rename = "detected_source_language", default)]
    detected_source_language: Option<String>,
}

/// Free-tier keys end in `:fx` and are served by a different host.
pub fn host_for_key(key: &str) -> &'static str {
    if key.trim().ends_with(":fx") {
        "https://api-free.deepl.com"
    } else {
        "https://api.deepl.com"
    }
}

/// DeepL's own language codes.
static CODES: std::sync::LazyLock<BTreeMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        BTreeMap::from([
            ("en", "EN-US"),
            ("zh", "ZH"),
            ("zh-TW", "ZH"),
            ("ja", "JA"),
            ("ko", "KO"),
            ("fr", "FR"),
            ("de", "DE"),
            ("es", "ES"),
            ("pt", "PT-PT"),
            ("it", "IT"),
            ("ru", "RU"),
            ("ar", "AR"),
            ("nl", "NL"),
            ("pl", "PL"),
            ("tr", "TR"),
            ("th", "TH"),
            ("vi", "VI"),
            ("id", "ID"),
            ("ms", "MS"),
            ("hi", "HI"),
            ("bg", "BG"),
            ("cs", "CS"),
            ("da", "DA"),
            ("el", "EL"),
            ("et", "ET"),
            ("fi", "FI"),
            ("hu", "HU"),
            ("lv", "LV"),
            ("lt", "LT"),
            ("ro", "RO"),
            ("sk", "SK"),
            ("sl", "SL"),
            ("sv", "SV"),
            ("uk", "UK"),
            ("no", "NB"),
            ("he", "HE"),
        ])
    });

pub struct DeepL {
    client: reqwest::Client,
    base_url: Option<String>,
}

impl DeepL {
    pub fn new(settings: &ProviderSettings) -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
            base_url: settings
                .base_url
                .clone()
                .map(|s| s.trim_end_matches('/').to_string()),
        }
    }
}

impl Default for DeepL {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for DeepL {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "deepl".into(),
            display_name: "DeepL (official)".into(),
            kind: ProviderKind::Official,
            priority: 20,
            needs_key: true,
            max_chars: 5000,
            auto_detect: true,
            languages: CODES.keys().map(|k| k.to_string()).collect(),
            free_tier: Some("1M chars total on the Developer plan".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    /// DeepL needs a key: every supported canonical target must map to a code.
    fn supports(&self, target: &str) -> bool {
        CODES.contains_key(target)
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        cred: &Credential,
    ) -> Result<TranslateResponse> {
        let key = cred.get("api_key").unwrap_or_default();
        if key.is_empty() {
            return Err(Error::AuthFailed {
                provider: "deepl".into(),
                reason: "api_key is required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let target = CODES
            .get(to.as_str())
            .ok_or_else(|| Error::LanguagePairNotSupported {
                from_lang: req.source_lang.clone(),
                target: to.clone(),
                provider: "deepl".into(),
            })?;

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("text".into(), req.text.clone());
        form.insert("target_lang".into(), (*target).to_string());
        if req.source_lang != "auto" {
            let from = lang::normalize(&req.source_lang);
            if let Some(src) = CODES.get(from.as_str()) {
                form.insert("source_lang".into(), (*src).to_string());
            }
        }

        let host = self
            .base_url
            .clone()
            .unwrap_or_else(|| host_for_key(key).to_string());
        let url = format!("{host}/v2/translate");

        let started = Instant::now();
        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("DeepL-Auth-Key {key}"))
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("deepl: {e}")))?;

        let parsed: Resp = json("deepl", resp).await?;
        let first =
            parsed
                .translations
                .first()
                .ok_or_else(|| Error::BadResponse {
                    provider: "deepl".into(),
                    reason: "empty translations array".into(),
                })?;

        Ok(TranslateResponse {
            text: first.text.clone(),
            detected_source: first
                .detected_source_language
                .clone()
                .unwrap_or_else(|| req.source_lang.clone()),
            target_lang: to,
            provider: "deepl".into(),
            engine: "deepl".into(),
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

    fn cred_with(key: &str) -> Credential {
        Credential {
            provider: "deepl".into(),
            label: "default".into(),
            fields: [("api_key".to_string(), key.to_string())]
                .into_iter()
                .collect(),
        }
    }

    fn req(text: &str, to: &str) -> TranslateRequest {
        TranslateRequest {
            text: text.into(),
            source_lang: "auto".into(),
            target_lang: to.into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn free_key_routes_to_free_host() {
        assert_eq!(host_for_key("abc123:fx"), "https://api-free.deepl.com");
        assert_eq!(host_for_key("abc123"), "https://api.deepl.com");
        assert_eq!(host_for_key("  abc:fx  "), "https://api-free.deepl.com");
    }

    #[test]
    fn info_and_support_follow_the_code_table() {
        let p = DeepL::default();
        let info = p.info();
        assert_eq!(info.name, "deepl");
        assert!(info.needs_key);
        assert_eq!(info.max_chars, 5000);
        assert!(p.supports("zh"));
        assert!(p.supports("de"));
        assert!(!p.supports("xx"));
        assert_eq!(info.languages.len(), CODES.len());
    }

    #[test]
    fn parses_api_shape() {
        let body = r#"{"translations":[{"text":"Hallo","detected_source_language":"EN"}]}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translations[0].text, "Hallo");
        assert_eq!(
            parsed.translations[0].detected_source_language.as_deref(),
            Some("EN")
        );
    }

    #[test]
    fn missing_detected_source_is_not_fatal() {
        let body = r#"{"translations":[{"text":"Hallo"}]}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translations[0].detected_source_language, None);
    }

    #[tokio::test]
    async fn empty_key_is_rejected_before_the_request() {
        let p = DeepL::default();
        let err = p
            .translate_once(&req("hi", "de"), &cred_with(""))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "AUTH_FAILED");
    }

    #[tokio::test]
    async fn unsupported_target_is_rejected_before_the_request() {
        let p = DeepL::default();
        let err = p
            .translate_once(&req("hi", "xx"), &cred_with("k"))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "LANGUAGE_PAIR_NOT_SUPPORTED");
    }
}
