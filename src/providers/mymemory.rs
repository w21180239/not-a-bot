//! MyMemory free translation API. No key required, generous free tier.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::client;
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://api.mymemory.translated.net/get";

/// MyMemory wants RFC3066 region tags and does NOT support "auto" source.
/// If source is "auto", use the default target language (zh-CN).
pub fn to_mymemory_code(code: &str) -> String {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-CN".into(),
        "zh-TW" | "zh-Hant" => "zh-TW".into(),
        "en" => "en-GB".into(),
        other => other.into(),
    }
}

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(rename = "responseData")]
    response_data: Option<ResponseData>,
    #[serde(rename = "responseStatus")]
    response_status: i32,
    #[serde(rename = "responseDetails")]
    response_details: String,
}

#[derive(Debug, Deserialize)]
struct ResponseData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

pub struct MyMemory {
    client: reqwest::Client,
    email: Option<String>,
}

impl MyMemory {
    pub fn new(email: Option<String>) -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
            email,
        }
    }
}

impl Default for MyMemory {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Provider for MyMemory {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "mymemory".into(),
            display_name: "MyMemory".into(),
            kind: ProviderKind::Web,
            priority: 11,
            needs_key: false,
            max_chars: 5000,
            auto_detect: false,
            languages: vec!["*".into()],
            free_tier: Some("10000 chars/day with email, 1000 chars/day anonymous".into()),
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
        let to = to_mymemory_code(&lang::normalize(&req.target_lang));
        let from_raw = lang::normalize(&req.source_lang);
        // The endpoint rejects "auto"; mirror the reference by using target as source fallback.
        let from = if from_raw == "auto" {
            to.clone()
        } else {
            to_mymemory_code(&from_raw)
        };

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("q".into(), req.text.clone());
        params.insert("langpair".into(), format!("{from}|{to}"));
        if let Some(email) = &self.email {
            params.insert("de".into(), email.clone());
        }

        let started = Instant::now();
        let resp = self
            .client
            .get(ENDPOINT)
            .query(&params)
            .send()
            .await
            .map_err(|e| Error::Network(format!("mymemory: {e}")))?;

        let status = resp.status().as_u16();
        let text = crate::core::http::body_text("mymemory", resp).await?;
        if status != 200 {
            return Err(crate::core::error::map_status("mymemory", status, &text));
        }
        let parsed: Resp = serde_json::from_str(&text).map_err(|e| Error::BadResponse {
            provider: "mymemory".into(),
            reason: format!("{}: {}", e, crate::core::http::truncate(&text, 160)),
        })?;

        if parsed.response_status != 200 {
            return Err(Error::Upstream {
                provider: "mymemory".into(),
                status: parsed.response_status as u16,
                body: parsed.response_details,
            });
        }

        let data = parsed.response_data.ok_or_else(|| Error::BadResponse {
            provider: "mymemory".into(),
            reason: "missing responseData".into(),
        })?;

        Ok(TranslateResponse {
            text: data.translated_text,
            detected_source: from,
            target_lang: to,
            provider: "mymemory".into(),
            engine: "mymemory".into(),
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
        let p = MyMemory::default();
        let info = p.info();
        assert_eq!(info.name, "mymemory");
        assert!(!info.needs_key);
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"responseData":{"translatedText":"你好","match":1.0},"responseStatus":200,"responseDetails":""}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.response_data.unwrap().translated_text, "你好");
    }

    #[test]
    fn error_status_is_mapped() {
        let body =
            r#"{"responseData":null,"responseStatus":403,"responseDetails":"quota exceeded"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.response_status, 403);
    }
}
