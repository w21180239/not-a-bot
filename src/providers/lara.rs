//! Lara Translate free web endpoint: JSON POST with a client marker header.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const HOST: &str = "https://laratranslate.com/translate";
const API: &str = "https://webapi.laratranslate.com/translate/segmented";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

#[derive(Debug, Serialize)]
struct Payload<'a> {
    q: &'a str,
    source: &'a str,
    target: String,
    source_hint: &'static str,
    style: &'static str,
    content_type: &'static str,
    adapt_to: Vec<serde_json::Value>,
    glossaries: Vec<serde_json::Value>,
    instructions: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Resp {
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    translations: Option<Vec<Segment>>,
    #[serde(rename = "source_language", default)]
    source_language: Option<String>,
    limit: Option<Limit>,
}

#[derive(Debug, Deserialize)]
struct Segment {
    translation: Option<String>,
}

/// Lara meters the free tier per IP; surface exhaustion instead of returning blanks.
#[derive(Debug, Deserialize)]
struct Limit {
    overquota: Option<bool>,
}

pub struct Lara {
    client: reqwest::Client,
}

impl Lara {
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

impl Default for Lara {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Lara {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "lara".into(),
            display_name: "Lara Translate (web)".into(),
            kind: ProviderKind::Web,
            priority: 5,
            needs_key: false,
            max_chars: 500,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("daily per-IP credit quota".into()),
            configured: true,
        }
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let from_raw = lang::normalize(&req.source_lang);
        // Empty source means auto-detect.
        let source = if from_raw == "auto" { "" } else { &from_raw };

        let payload = Payload {
            q: &req.text,
            source,
            target: to.clone(),
            source_hint: "",
            style: "faithful",
            content_type: "text/plain",
            adapt_to: Vec::new(),
            glossaries: Vec::new(),
            instructions: Vec::new(),
        };

        let _ = self.client.get(HOST).send().await.map(|r| r.status());

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .header("Origin", "https://laratranslate.com")
            .header("Referer", HOST)
            .header("X-Lara-Client", "Webapp")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Network(format!("lara: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("lara", resp).await?;
        if status != 200 {
            return Err(map_status("lara", status, &body));
        }

        let parsed: Resp =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "lara".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;

        let content = parsed.content.ok_or_else(|| Error::BadResponse {
            provider: "lara".into(),
            reason: format!("missing content: {}", truncate(&body, 160)),
        })?;

        if content.limit.as_ref().and_then(|l| l.overquota) == Some(true) {
            return Err(Error::QuotaExhausted {
                provider: "lara".into(),
                retry_after_secs: Some(3_600),
            });
        }

        let segments: Vec<String> = content
            .translations
            .ok_or_else(|| Error::BadResponse {
                provider: "lara".into(),
                reason: "missing translations".into(),
            })?
            .into_iter()
            .filter_map(|s| s.translation.filter(|t| !t.is_empty()))
            .collect();

        if segments.is_empty() {
            return Err(Error::BadResponse {
                provider: "lara".into(),
                reason: "empty translations array".into(),
            });
        }

        Ok(TranslateResponse {
            text: segments.join(""),
            detected_source: content.source_language.unwrap_or(from_raw),
            target_lang: req.target_lang.clone(),
            provider: "lara".into(),
            engine: "lara".into(),
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
        let p = Lara::default();
        let info = p.info();
        assert_eq!(info.name, "lara");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 5);
        assert_eq!(info.max_chars, 500);
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from webapi.laratranslate.com/translate/segmented.
        let body = r#"{"status":200,"content":{"source_language":"en","translations":[{"sentence":"Hello world, this is a test.","translation":"大家好，这是一条测试消息。"}],"limit":{"plan":"credit_web","overquota":false}}}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        let c = p.content.unwrap();
        assert_eq!(
            c.translations.unwrap()[0]
                .translation
                .as_deref()
                .unwrap(),
            "大家好，这是一条测试消息。"
        );
        assert_eq!(c.source_language.as_deref(), Some("en"));
        assert_eq!(c.limit.unwrap().overquota, Some(false));
    }

    #[test]
    fn overquota_flag_is_detectable() {
        let body = r#"{"content":{"translations":[{"translation":"x"}],"limit":{"overquota":true}}}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.content.unwrap().limit.unwrap().overquota, Some(true));
    }

    #[test]
    fn missing_content_is_detectable() {
        let p: Resp = serde_json::from_str(r#"{"status":429,"message":"rate limited"}"#).unwrap();
        assert!(p.content.is_none());
    }
}
