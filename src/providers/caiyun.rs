//! Caiyun (LingoCloud) small translation API. Bearer token, JSON body.

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

const ENDPOINT: &str = "https://api.intertrans.com/api/web/v2/translate/text";

#[derive(Debug, Deserialize)]
struct Wrapper {
    data: Option<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[serde(default)]
    target: Vec<String>,
}

pub struct Caiyun {
    client: reqwest::Client,
    endpoint: String,
}

impl Caiyun {
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

impl Default for Caiyun {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for Caiyun {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "caiyun".into(),
            display_name: "Caiyun".into(),
            kind: ProviderKind::Keyed,
            needs_key: true,
            // The web API caps a single request at 2000 characters.
            max_chars: 2000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("1M chars for new accounts".into()),
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
        let token = cred.get("token").unwrap_or_default();
        if token.is_empty() {
            return Err(Error::AuthFailed {
                provider: "caiyun".into(),
                reason: "token is required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);
        let body = serde_json::json!({
            "source_type": from,
            "target_type": to,
            "text": [req.text],
        });

        let started = Instant::now();
        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("caiyun: {e}")))?;

        let parsed: Wrapper = json("caiyun", resp).await?;
        let target = parsed
            .data
            .and_then(|d| d.content)
            .filter(|c| !c.target.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "caiyun".into(),
                reason: "missing data.content.target".into(),
            })?;

        Ok(TranslateResponse {
            text: target.target.join(""),
            detected_source: from,
            target_lang: to,
            provider: "caiyun".into(),
            engine: "caiyun".into(),
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
        let body = r#"{"data":{"content":{"target":["你好"]}}}"#;
        let w: Wrapper = serde_json::from_str(body).unwrap();
        assert_eq!(w.data.unwrap().content.unwrap().target, vec!["你好"]);
    }

    #[test]
    fn empty_target_parses_but_is_empty() {
        let body = r#"{"data":{"content":{"target":[]}}}"#;
        let w: Wrapper = serde_json::from_str(body).unwrap();
        assert!(w.data.unwrap().content.unwrap().target.is_empty());
    }

    #[test]
    fn joins_segments() {
        let body = r#"{"data":{"content":{"target":["甲","乙"]}}}"#;
        let w: Wrapper = serde_json::from_str(body).unwrap();
        assert_eq!(w.data.unwrap().content.unwrap().target.join(""), "甲乙");
    }

    #[tokio::test]
    async fn missing_token_is_auth_failed() {
        let p = Caiyun::default();
        let cred = Credential {
            provider: "caiyun".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
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
            "AUTH_FAILED"
        );
    }
}
