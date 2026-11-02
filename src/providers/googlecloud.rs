//! Google Cloud Translation v2 (Basic). Broad language coverage, 500k chars free.

use crate::core::config::ProviderSettings;
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

const ENDPOINT: &str = "https://translation.googleapis.com/language/translate/v2";

#[derive(Debug, Deserialize)]
struct Resp {
    data: Option<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    translations: Option<Vec<Translation>>,
}

#[derive(Debug, Deserialize)]
struct Translation {
    #[serde(rename = "translatedText")]
    translated_text: String,
    #[serde(rename = "detectedSourceLanguage", default)]
    detected_source_language: Option<String>,
}

pub struct GoogleCloud {
    client: reqwest::Client,
    endpoint: String,
}

impl GoogleCloud {
    pub fn new(settings: &ProviderSettings) -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
            endpoint: settings
                .base_url
                .clone()
                .unwrap_or_else(|| ENDPOINT.to_string()),
        }
    }
}

impl Default for GoogleCloud {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for GoogleCloud {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "googlecloud".into(),
            display_name: "Google Cloud Translation".into(),
            kind: ProviderKind::Keyed,
            needs_key: true,
            max_chars: 30_000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("500k chars/month".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        cred: &Credential,
    ) -> Result<TranslateResponse> {
        let key = cred.get("api_key").unwrap_or_default();
        if key.is_empty() {
            return Err(Error::AuthFailed {
                provider: "googlecloud".into(),
                reason: "api_key is required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("format".into(), "text".into());
        form.insert("target".into(), to.clone());
        form.insert("q".into(), req.text.clone());
        if from != "auto" {
            form.insert("source".into(), from.clone());
        }

        let url = format!("{}?key={}", self.endpoint, crate::core::http::percent_encode(key));
        let started = Instant::now();
        let resp = self
            .client
            .post(&url)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("googlecloud: {e}")))?;

        let parsed: Resp = json("googlecloud", resp).await?;
        let first = parsed
            .data
            .and_then(|d| d.translations)
            .and_then(|mut v| v.pop())
            .ok_or_else(|| Error::BadResponse {
                provider: "googlecloud".into(),
                reason: "missing data.translations".into(),
            })?;

        Ok(TranslateResponse {
            text: first.translated_text,
            detected_source: first
                .detected_source_language
                .unwrap_or_else(|| from.clone()),
            target_lang: to,
            provider: "googlecloud".into(),
            engine: "googlecloud".into(),
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

    fn cred(key: &str) -> Credential {
        Credential {
            provider: "googlecloud".into(),
            label: "default".into(),
            fields: [("api_key".to_string(), key.to_string())]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn parses_api_shape() {
        let body = r#"{"data":{"translations":[{"translatedText":"Bonjour","detectedSourceLanguage":"en"}]}}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        let t = &parsed.data.unwrap().translations.unwrap()[0];
        assert_eq!(t.translated_text, "Bonjour");
        assert_eq!(t.detected_source_language.as_deref(), Some("en"));
    }

    #[test]
    fn missing_data_field_parses_as_none() {
        let body = r#"{"error":{"code":403,"message":"quota exceeded"}}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert!(parsed.data.is_none());
    }

    #[tokio::test]
    async fn empty_key_is_rejected_before_the_request() {
        let p = GoogleCloud::default();
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "auto".into(),
            target_lang: "fr".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        let err = p.translate_once(&req, &cred("")).await.unwrap_err();
        assert_eq!(err.code(), "AUTH_FAILED");
    }

    #[test]
    fn info_reports_v2_limits() {
        let info = GoogleCloud::default().info();
        assert_eq!(info.max_chars, 30_000);
        assert!(info.auto_detect);
    }
}
