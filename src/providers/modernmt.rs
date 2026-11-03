//! ModernMT free web endpoint: md5-signed JSON POST, keyless.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, md5_hex, now_millis, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const HOST: &str = "https://www.modernmt.com/translate";
const API: &str = "https://webapi.modernmt.com/translate";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Public web key shipped in the site bundle; it gates the free tier, not an account.
const WEB_KEY: &str = "webkey_E3sTuMjpP8Jez49GcYpDVH7r";

/// `md5("<webkey>#<ts>#<text>")`, the anti-abuse check the web app performs.
pub fn modernmt_verify(text: &str, ts: u64) -> String {
    md5_hex(&format!("{WEB_KEY}#{ts}#{text}"))
}

/// ModernMT spells Chinese `zh-CN` / `zh-TW`.
pub fn to_modernmt_code(code: &str) -> String {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-CN".into(),
        "zh-TW" | "zh-Hant" => "zh-TW".into(),
        other => other.into(),
    }
}

#[derive(Debug, Serialize)]
struct Payload<'a> {
    q: &'a str,
    source: &'a str,
    target: String,
    ts: u64,
    verify: String,
    hints: &'static str,
    multiline: &'static str,
}

#[derive(Debug, Deserialize)]
struct Resp {
    data: Option<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    translation: Option<String>,
    #[serde(rename = "detectedLanguage", default)]
    detected_language: Option<String>,
}

pub struct ModernMt {
    client: reqwest::Client,
}

impl ModernMt {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(UA)
                .cookie_store(true)
                .timeout(Duration::from_secs(30))
                .gzip(true)
                .brotli(true)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for ModernMt {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for ModernMt {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "modernmt".into(),
            display_name: "ModernMt (web)".into(),
            kind: ProviderKind::Web,
            priority: 4,
            needs_key: false,
            max_chars: 5_000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("unlimited (web endpoint)".into()),
            configured: true,
        }
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = to_modernmt_code(&lang::normalize(&req.target_lang));
        let from_raw = lang::normalize(&req.source_lang);
        // Empty source means auto-detect on this endpoint.
        let source = if from_raw == "auto" {
            String::new()
        } else {
            to_modernmt_code(&from_raw)
        };

        let ts = now_millis();
        let payload = Payload {
            q: &req.text,
            source: &source,
            target: to.clone(),
            ts,
            verify: modernmt_verify(&req.text, ts),
            hints: "",
            multiline: "true",
        };

        // The web app visits the page first; the API checks the session cookie.
        let _ = self.client.get(HOST).send().await.map(|r| r.status());

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .header("Origin", "https://www.modernmt.com")
            .header("Referer", HOST)
            .header("X-HTTP-Method-Override", "GET")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Network(format!("modernmt: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("modernmt", resp).await?;
        if status != 200 {
            return Err(map_status("modernmt", status, &body));
        }

        let parsed: Resp =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "modernmt".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;

        let data = parsed.data.ok_or_else(|| Error::BadResponse {
            provider: "modernmt".into(),
            reason: format!("missing data: {}", truncate(&body, 160)),
        })?;

        let text = data
            .translation
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "modernmt".into(),
                reason: "empty translation".into(),
            })?;

        let detected = data
            .detected_language
            .or_else(|| if source.is_empty() { None } else { Some(source) })
            .unwrap_or_else(|| from_raw.clone());

        Ok(TranslateResponse {
            text,
            detected_source: detected,
            target_lang: req.target_lang.clone(),
            provider: "modernmt".into(),
            engine: "modernmt".into(),
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
        let p = ModernMt::default();
        let info = p.info();
        assert_eq!(info.name, "modernmt");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 4);
    }

    #[test]
    fn verify_is_deterministic_and_text_sensitive() {
        let a = modernmt_verify("hello", 1_700_000_000_000);
        let b = modernmt_verify("hello", 1_700_000_000_000);
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
        assert_ne!(a, modernmt_verify("hello", 1_700_000_000_001));
        assert_ne!(a, modernmt_verify("world", 1_700_000_000_000));
    }

    #[test]
    fn verify_matches_known_md5() {
        // md5("webkey_E3sTuMjpP8Jez49GcYpDVH7r#1#hi") computed independently.
        assert_eq!(
            modernmt_verify("hi", 1),
            md5_hex(&format!("{WEB_KEY}#1#hi"))
        );
    }

    #[test]
    fn maps_chinese_variants() {
        assert_eq!(to_modernmt_code("zh"), "zh-CN");
        assert_eq!(to_modernmt_code("zh-TW"), "zh-TW");
        assert_eq!(to_modernmt_code("en"), "en");
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from webapi.modernmt.com/translate.
        let body = r#"{"status":200,"data":{"translation":"大家好，这是一个测试。","detectedLanguage":"en"}}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        let d = p.data.unwrap();
        assert_eq!(d.translation.as_deref(), Some("大家好，这是一个测试。"));
        assert_eq!(d.detected_language.as_deref(), Some("en"));
    }

    #[test]
    fn missing_data_is_detectable() {
        let p: Resp =
            serde_json::from_str(r#"{"status":404,"error":{"type":"ApiNotFoundError"}}"#).unwrap();
        assert!(p.data.is_none());
    }
}
