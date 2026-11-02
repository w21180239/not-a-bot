//! NiuTrans. Broad language coverage, form POST with an apikey field.

use crate::core::config::ProviderSettings;
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{self, client, json};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://api.niutrans.com/NiuTransServer/translation";

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(default)]
    target_text: Option<String>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error_msg: Option<String>,
}

pub struct Niutrans {
    client: reqwest::Client,
    endpoint: String,
}

impl Niutrans {
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

impl Default for Niutrans {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for Niutrans {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "niutrans".into(),
            display_name: "NiuTrans".into(),
            kind: ProviderKind::Keyed,
            needs_key: true,
            // NiuTrans rejects requests above 5000 bytes.
            max_chars: 5000,
            auto_detect: false,
            languages: vec!["*".into()],
            free_tier: Some("500k chars on signup".into()),
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
                provider: "niutrans".into(),
                reason: "api_key is required".into(),
            });
        }
        let from = lang::normalize(&req.source_lang);
        if from == "auto" {
            return Err(Error::InvalidInput(
                "niutrans requires an explicit source language".into(),
            ));
        }
        let to = lang::normalize(&req.target_lang);

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("from".into(), from.clone());
        form.insert("to".into(), to.clone());
        form.insert("apikey".into(), key.to_string());
        form.insert("source_text".into(), req.text.clone());

        let started = Instant::now();
        let resp = self
            .client
            .post(&self.endpoint)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("niutrans: {e}")))?;

        let parsed: Resp = json("niutrans", resp).await?;
        if let Some(code) = parsed.error_code.as_deref() {
            if !code.is_empty() && code != "0" && code != "200" {
                let msg = parsed.error_msg.unwrap_or_default();
                return Err(http::map_status("niutrans", 200, &format!("{code} {msg}")));
            }
        }
        let text = parsed.target_text.ok_or_else(|| Error::BadResponse {
            provider: "niutrans".into(),
            reason: "missing target_text".into(),
        })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "niutrans".into(),
            engine: "niutrans".into(),
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
    fn parses_success_shape() {
        let body = r#"{"target_text":"你好"}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.target_text.as_deref(), Some("你好"));
        assert!(p.error_code.is_none());
    }

    #[test]
    fn parses_error_shape() {
        let body = r#"{"error_code":"5100R","error_msg":"invalid apikey"}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.error_code.as_deref(), Some("5100R"));
        assert!(p.target_text.is_none());
    }

    #[tokio::test]
    async fn missing_key_is_auth_failed() {
        let p = Niutrans::default();
        let cred = Credential {
            provider: "niutrans".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(
            p.translate_once(&req, &cred).await.unwrap_err().code(),
            "AUTH_FAILED"
        );
    }

    #[tokio::test]
    async fn auto_source_is_rejected() {
        let p = Niutrans::default();
        let cred = Credential {
            provider: "niutrans".into(),
            label: "default".into(),
            fields: [("api_key".to_string(), "k".into())]
                .into_iter()
                .collect(),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "auto".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(
            p.translate_once(&req, &cred).await.unwrap_err().code(),
            "INVALID_INPUT"
        );
    }
}
