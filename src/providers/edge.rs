//! Microsoft Edge/Bing Translator free web endpoint.

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

const ENDPOINT: &str = "https://www.bing.com/translate";

/// Bing/Edge returns a JSON object with a nested array structure.
#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(rename = "translations")]
    translations: Option<Vec<Translation>>,
}

#[derive(Debug, Deserialize)]
struct Translation {
    #[serde(rename = "text")]
    text: String,
    #[serde(rename = "to")]
    target_lang: String,
}

pub struct Edge {
    client: reqwest::Client,
}

impl Edge {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
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
            display_name: "Microsoft Edge/Bing Translator".into(),
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

        let started = Instant::now();
        let resp = self
            .client
            .get(ENDPOINT)
            .query(&params)
            .send()
            .await
            .map_err(|e| Error::Network(format!("edge: {e}")))?;

        let status = resp.status().as_u16();
        let text = crate::core::http::body_text("edge", resp).await?;
        if status != 200 {
            return Err(map_status("edge", status, &text));
        }
        let parsed: Resp = serde_json::from_str(&text).map_err(|e| Error::BadResponse {
            provider: "edge".into(),
            reason: format!("{}: {}", e, crate::core::http::truncate(&text, 160)),
        })?;

        let translations = parsed
            .translations
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "edge".into(),
                reason: "missing translations".into(),
            })?;

        let text = translations.first().map(|t| t.text.clone()).ok_or_else(|| Error::BadResponse {
            provider: "edge".into(),
            reason: "empty translations".into(),
        })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
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
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"translations":[{"text":"你好","to":"zh"}]}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translations.unwrap()[0].text, "你好");
    }
}