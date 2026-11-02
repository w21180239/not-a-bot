//! Mirai Translate (Japanese specialist).

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

const ENDPOINT: &str = "https://miraitranslate.com/api/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(rename = "translated_text")]
    translated_text: Option<String>,
    #[serde(rename = "source_lang")]
    source_lang: Option<String>,
}

pub struct Mirai {
    client: reqwest::Client,
}

impl Mirai {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Mirai {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Mirai {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "mirai".into(),
            display_name: "Mirai Translate".into(),
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

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("text".into(), req.text.clone());
        form.insert("src".into(), from.clone());
        form.insert("dst".into(), to.clone());

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("mirai: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("mirai", resp).await?;
            return Err(map_status("mirai", status, &body));
        }

        let parsed: Resp = json("mirai", resp).await?;

        let text = parsed
            .translated_text
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "mirai".into(),
                reason: "missing translated_text".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: parsed.source_lang.unwrap_or(from),
            target_lang: to,
            provider: "mirai".into(),
            engine: "mirai".into(),
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
        let p = Mirai::default();
        let info = p.info();
        assert_eq!(info.name, "mirai");
        assert!(!info.needs_key);
        assert!(p.supports("ja"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"translated_text":"こんにちは","source_lang":"en"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translated_text.as_deref(), Some("こんにちは"));
    }
}