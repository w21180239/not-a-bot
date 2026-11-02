//! Iciba (爱词霸) free web endpoint.

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

const ENDPOINT: &str = "https://fy.iciba.com/ajax.php";

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(rename = "content")]
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    out: String,
}

pub struct Iciba {
    client: reqwest::Client,
}

impl Iciba {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Iciba {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Iciba {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "iciba".into(),
            display_name: "Iciba".into(),
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

        // Iciba uses different param names: a=fy, f=from, t=to
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("a".into(), "fy".into());
        params.insert("f".into(), from.clone());
        params.insert("t".into(), to.clone());
        params.insert("w".into(), req.text.clone());

        let started = Instant::now();
        let resp = self
            .client
            .get(ENDPOINT)
            .query(&params)
            .send()
            .await
            .map_err(|e| Error::Network(format!("iciba: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("iciba", resp).await?;
            return Err(map_status("iciba", status, &body));
        }

        let parsed: Resp = json("iciba", resp).await?;

        let text = parsed
            .content
            .and_then(|c| Some(c.out))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "iciba".into(),
                reason: "missing content.out".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "iciba".into(),
            engine: "iciba".into(),
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
        let p = Iciba::default();
        let info = p.info();
        assert_eq!(info.name, "iciba");
        assert!(!info.needs_key);
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"content":{"out":"你好"}}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.content.unwrap().out, "你好");
    }
}