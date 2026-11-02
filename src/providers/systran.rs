//! SysTran (French specialist).

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

const ENDPOINT: &str = "https://www.systran.net/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    outputs: Option<Vec<Output>>,
}

#[derive(Debug, Deserialize)]
struct Output {
    output: String,
}

pub struct SysTran {
    client: reqwest::Client,
}

impl SysTran {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for SysTran {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for SysTran {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "systran".into(),
            display_name: "SysTran".into(),
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
        params.insert("input".into(), req.text.clone());
        params.insert("srcLang".into(), from.clone());
        params.insert("tgtLang".into(), to.clone());

        let started = Instant::now();
        let resp = self
            .client
            .get(ENDPOINT)
            .query(&params)
            .send()
            .await
            .map_err(|e| Error::Network(format!("systran: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("systran", resp).await?;
            return Err(map_status("systran", status, &body));
        }

        let parsed: Resp = json("systran", resp).await?;

        let text = parsed
            .outputs
            .and_then(|v| v.first().map(|o| o.output.clone()))
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "systran".into(),
                reason: "missing outputs".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "systran".into(),
            engine: "systran".into(),
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
        let p = SysTran::default();
        let info = p.info();
        assert_eq!(info.name, "systran");
        assert!(!info.needs_key);
        assert!(p.supports("fr"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"outputs":[{"output":"Bonjour"}]}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.outputs.unwrap()[0].output, "Bonjour");
    }
}