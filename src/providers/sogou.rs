//! Sogou free web endpoint.

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

const ENDPOINT: &str = "https://fanyi.sogou.com/reventondc/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(rename = "translate")]
    translate: Option<Translate>,
}

#[derive(Debug, Deserialize)]
struct Translate {
    #[serde(rename = "dit")]
    dit: Option<String>,
}

pub struct Sogou {
    client: reqwest::Client,
}

impl Sogou {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Sogou {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Sogou {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "sogou".into(),
            display_name: "Sogou".into(),
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
        form.insert("from".into(), from.clone());
        form.insert("to".into(), to.clone());
        form.insert("text".into(), req.text.clone());
        form.insert("client".into(), "pc".into());
        form.insert("fr".into(), "browser_web".into());

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("sogou: {e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let body = crate::core::http::body_text("sogou", resp).await?;
            return Err(map_status("sogou", status, &body));
        }

        let parsed: Resp = json("sogou", resp).await?;

        let text = parsed
            .translate
            .and_then(|t| t.dit)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "sogou".into(),
                reason: "missing translate.dit".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "sogou".into(),
            engine: "sogou".into(),
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
        let p = Sogou::default();
        let info = p.info();
        assert_eq!(info.name, "sogou");
        assert!(!info.needs_key);
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"translate":{"dit":"你好"}}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.translate.unwrap().dit.as_deref(), Some("你好"));
    }
}