//! Reverso free web endpoint.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, json, map_status};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://api.reverso.net/translate/v1/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    translation: Option<String>,
}

pub struct Reverso {
    client: reqwest::Client,
}

impl Reverso {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Reverso {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Reverso {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "reverso".into(),
            display_name: "Reverso".into(),
            kind: ProviderKind::Free,
            needs_key: false,
            max_chars: 5000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("unlimited (web endpoint)".into()),
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
        let from = lang::normalize(&req.source_lang);

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("text".into(), req.text.clone());
        params.insert("from".into(), from.clone());
        params.insert("to".into(), to.clone());
        params.insert("format".into(), "text".into());
        params.insert("options".into(), "0".into());

        let started = Instant::now();
        let resp = self
            .client
            .get(ENDPOINT)
            .query(&params)
            .send()
            .await
            .map_err(|e| Error::Network(format!("reverso: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("reverso", resp).await?;
            return Err(map_status("reverso", status, &body));
        }

        let parsed: Resp = json("reverso", resp).await?;

        let text = parsed
            .translation
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "reverso".into(),
                reason: "missing translation".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "reverso".into(),
            engine: "reverso".into(),
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
        let p = Reverso::default();
        let info = p.info();
        assert_eq!(info.name, "reverso");
        assert!(!info.needs_key);
        assert!(p.supports("fr"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"translation":"Bonjour"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translation.as_deref(), Some("Bonjour"));
    }
}