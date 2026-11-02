//! Iflyrec (讯飞语音) free web endpoint.

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

const ENDPOINT: &str = "https://fanyi.iflyrec.com/api/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    data: Option<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    result: Option<String>,
}

pub struct Iflyrec {
    client: reqwest::Client,
}

impl Iflyrec {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Iflyrec {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Iflyrec {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "iflyrec".into(),
            display_name: "Iflyrec".into(),
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
        form.insert("from".into(), from.clone());
        form.insert("to".into(), to.clone());

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("iflyrec: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("iflyrec", resp).await?;
            return Err(map_status("iflyrec", status, &body));
        }

        let parsed: Resp = json("iflyrec", resp).await?;

        let text = parsed
            .data
            .and_then(|d| d.result)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "iflyrec".into(),
                reason: "missing data.result".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "iflyrec".into(),
            engine: "iflyrec".into(),
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
        let p = Iflyrec::default();
        let info = p.info();
        assert_eq!(info.name, "iflyrec");
        assert!(!info.needs_key);
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"data":{"result":"你好"}}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.data.unwrap().result.as_deref(), Some("你好"));
    }
}