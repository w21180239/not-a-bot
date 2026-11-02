//! Lingvanex free web endpoint.

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

const ENDPOINT: &str = "https://translate.lingvanex.com/api/v1/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    result: Option<String>,
    #[serde(rename = "source_lang")]
    source_lang: Option<String>,
    #[serde(rename = "target_lang")]
    target_lang: Option<String>,
}

pub struct Lingvanex {
    client: reqwest::Client,
}

impl Lingvanex {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Lingvanex {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Lingvanex {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "lingvanex".into(),
            display_name: "Lingvanex".into(),
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

        // Lingvanex uses B2C mode
        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("text".into(), req.text.clone());
        form.insert("src".into(), from.clone());
        form.insert("dst".into(), to.clone());
        form.insert("mode".into(), "B2C".into());

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("lingvanex: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("lingvanex", resp).await?;
            return Err(map_status("lingvanex", status, &body));
        }

        let parsed: Resp = json("lingvanex", resp).await?;

        let text = parsed
            .result
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "lingvanex".into(),
                reason: "missing result".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: parsed.source_lang.unwrap_or(from),
            target_lang: to,
            provider: "lingvanex".into(),
            engine: "lingvanex".into(),
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
        let p = Lingvanex::default();
        let info = p.info();
        assert_eq!(info.name, "lingvanex");
        assert!(!info.needs_key);
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"result":"你好","source_lang":"en","target_lang":"zh"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.result.as_deref(), Some("你好"));
    }
}