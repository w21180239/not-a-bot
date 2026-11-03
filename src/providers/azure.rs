//! Azure AI Translator. Regional endpoints; the region is a credential field
//! because a key is bound to the resource that issued it.

use crate::core::config::ProviderSettings;
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, json};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://api.cognitive.microsofttranslator.com";

#[derive(Debug, Deserialize)]
struct RespItem {
    translations: Vec<Translation>,
}

#[derive(Debug, Deserialize)]
struct Translation {
    text: String,
    #[serde(rename = "detectedLanguage", default)]
    detected_language: Option<String>,
}

pub struct Azure {
    client: reqwest::Client,
    endpoint: String,
}

impl Azure {
    pub fn new(settings: &ProviderSettings) -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
            endpoint: settings
                .base_url
                .clone()
                .unwrap_or_else(|| ENDPOINT.to_string())
                .trim_end_matches('/')
                .to_string(),
        }
    }
}

impl Default for Azure {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for Azure {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "azure".into(),
            display_name: "Azure AI Translator (official)".into(),
            kind: ProviderKind::Official,
            priority: 22,
            needs_key: true,
            max_chars: 50_000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("2M chars/month on the F0 tier".into()),
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
                provider: "azure".into(),
                reason: "api_key is required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);

        let mut query: Vec<(String, String)> = vec![("to".into(), to.clone())];
        if from != "auto" {
            query.push(("from".into(), from.clone()));
        }

        // Azure takes an array of documents; one element is enough here.
        let body = serde_json::json!([{ "Text": req.text }]);

        let started = Instant::now();
        let mut call = self
            .client
            .post(format!("{}/translate", self.endpoint))
            .query(&[("api-version", "3.0")])
            .query(&query)
            .header("Ocp-Apim-Subscription-Key", key);
        // Global resources take no region header.
        if let Some(region) = cred.get("region").filter(|r| !r.is_empty() && *r != "global") {
            call = call.header("Ocp-Apim-Subscription-Region", region);
        }

        let resp = call
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("azure: {e}")))?;

        let parsed: Vec<RespItem> = json("azure", resp).await?;
        let first = parsed
            .first()
            .and_then(|i| i.translations.first())
            .ok_or_else(|| Error::BadResponse {
                provider: "azure".into(),
                reason: "empty translations".into(),
            })?;

        Ok(TranslateResponse {
            text: first.text.clone(),
            detected_source: first
                .detected_language
                .clone()
                .unwrap_or_else(|| from.clone()),
            target_lang: to,
            provider: "azure".into(),
            engine: "azure".into(),
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
    use std::collections::BTreeMap;

    fn cred(fields: &[(&str, &str)]) -> Credential {
        Credential {
            provider: "azure".into(),
            label: "default".into(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn req() -> TranslateRequest {
        TranslateRequest {
            text: "hi".into(),
            source_lang: "auto".into(),
            target_lang: "de".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn parses_api_shape() {
        let body = r#"[{"detectedLanguage":{"language":"en","score":0.99},"translations":[{"text":"Hallo","to":"de"}]}]"#;
        let parsed: Vec<RespItem> = serde_json::from_str(body).unwrap();
        assert_eq!(parsed[0].translations[0].text, "Hallo");
    }

    #[test]
    fn detected_language_is_optional_in_the_translation_object() {
        let body = r#"[{"translations":[{"text":"Hallo","to":"de"}]}]"#;
        let parsed: Vec<RespItem> = serde_json::from_str(body).unwrap();
        assert_eq!(parsed[0].translations[0].detected_language, None);
    }

    #[tokio::test]
    async fn missing_key_is_auth_failed() {
        let p = Azure::default();
        let err = p
            .translate_once(&req(), &cred(&[("api_key", "")]))
            .await
            .unwrap_err();
        assert_eq!(err.code(), "AUTH_FAILED");
    }

    #[test]
    fn info_reports_f0_free_tier() {
        let info = Azure::default().info();
        assert_eq!(info.name, "azure");
        assert!(info.free_tier.unwrap().contains("F0"));
    }
}
