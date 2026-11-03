//! Volcengine machine translation, browser-extension endpoint (no key).
//! Mirrors the `volcEngine` engine of the original translators project: the
//! public web endpoint needs no credential, so it acts as a free fallback.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const HOST: &str = "https://translate.volcengine.com";
const ENDPOINT: &str = "https://translate.volcengine.com/crx/translate/v1/";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Codes the endpoint accepts; Chinese arrives as `zh` or `zh-Hant`.
const CODES: &[&str] = &["zh", "zh-Hant", "en", "ja", "ko", "fr", "de", "es", "ru"];

/// Map a canonical code onto Volcengine's; `None` means unsupported.
pub fn to_volc_code(code: &str) -> Option<&'static str> {
    match code {
        "zh" | "zh-CN" | "zh-Hans" | "yue" => Some("zh"),
        "zh-TW" | "zh-Hant" => Some("zh-Hant"),
        "en" => Some("en"),
        "ja" => Some("ja"),
        "ko" => Some("ko"),
        "fr" => Some("fr"),
        "de" => Some("de"),
        "es" => Some("es"),
        "ru" => Some("ru"),
        _ => None,
    }
}

pub fn supports(code: &str) -> bool {
    to_volc_code(code).is_some()
}

#[derive(Debug, Deserialize)]
struct Resp {
    translation: Option<String>,
    #[serde(rename = "detected_language", default)]
    detected_language: Option<String>,
    #[serde(rename = "base_resp", default)]
    base_resp: Option<BaseResp>,
}

/// Volcengine reports failures inside a 200 body via `base_resp.status_code`.
#[derive(Debug, Deserialize)]
struct BaseResp {
    #[serde(rename = "status_code", default)]
    status_code: Option<i64>,
    #[serde(rename = "status_message", default)]
    status_message: Option<String>,
}

pub struct Volcengine {
    client: reqwest::Client,
}

impl Volcengine {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(UA)
                .cookie_store(true)
                .timeout(Duration::from_secs(20))
                .gzip(true)
                .brotli(true)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Volcengine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Volcengine {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "volcengine".into(),
            display_name: "Volcengine (web)".into(),
            kind: ProviderKind::Web,
            priority: 5,
            needs_key: false,
            max_chars: 5_000,
            auto_detect: true,
            languages: CODES
                .iter()
                .map(|c| crate::core::lang::normalize(c))
                .collect(),
            free_tier: Some("public web endpoint, best effort".into()),
            configured: true,
        }
    }

    fn supports(&self, target: &str) -> bool {
        supports(target)
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let target = to_volc_code(&to).ok_or_else(|| Error::LanguagePairNotSupported {
            from_lang: req.source_lang.clone(),
            target: to.clone(),
            provider: "volcengine".into(),
        })?;
        // The endpoint accepts `auto` directly.
        let from_raw = lang::normalize(&req.source_lang);
        let source = match to_volc_code(&from_raw) {
            Some(c) => c,
            None => "auto",
        };

        let body = serde_json::json!({
            "text": req.text,
            "source_language": source,
            "target_language": target,
        });

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .header("Origin", HOST)
            .header("Referer", format!("{HOST}/"))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("volcengine: {e}")))?;

        let status = resp.status().as_u16();
        let text_body = body_text("volcengine", resp).await?;
        if status != 200 {
            return Err(map_status("volcengine", status, &text_body));
        }

        let parsed: Resp =
            serde_json::from_str(&text_body).map_err(|e| Error::BadResponse {
                provider: "volcengine".into(),
                reason: format!("{e}: {}", truncate(&text_body, 160)),
            })?;

        if let Some(br) = &parsed.base_resp {
            if br.status_code.unwrap_or(0) != 0 {
                return Err(Error::Upstream {
                    provider: "volcengine".into(),
                    status,
                    body: format!(
                        "base_resp {}: {}",
                        br.status_code.unwrap_or(0),
                        br.status_message.clone().unwrap_or_default()
                    ),
                });
            }
        }

        let text = parsed
            .translation
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "volcengine".into(),
                reason: format!("missing translation: {}", truncate(&text_body, 160)),
            })?;

        let detected = match source {
            "auto" => parsed
                .detected_language
                .clone()
                .unwrap_or_else(|| "auto".into()),
            other => other.to_string(),
        };

        Ok(TranslateResponse {
            text,
            detected_source: detected,
            target_lang: req.target_lang.clone(),
            provider: "volcengine".into(),
            engine: "volcengine".into(),
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
    fn keyless_provider_reports_web() {
        let p = Volcengine::new();
        let info = p.info();
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 5);
        assert!(p.supports("ja"));
        assert!(!p.supports("xx"));
    }

    #[test]
    fn maps_chinese_variants() {
        assert_eq!(to_volc_code("zh"), Some("zh"));
        assert_eq!(to_volc_code("zh-CN"), Some("zh"));
        assert_eq!(to_volc_code("zh-TW"), Some("zh-Hant"));
        assert_eq!(to_volc_code("xx"), None);
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from translate.volcengine.com/crx/translate/v1/.
        let body = r#"{"translation":"你好，世界，这是一个测试。","detected_language":"en","probability":1,"base_resp":{"status_code":0,"status_message":""}}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(
            p.translation.as_deref(),
            Some("你好，世界，这是一个测试。")
        );
        assert_eq!(p.detected_language.as_deref(), Some("en"));
        assert_eq!(p.base_resp.unwrap().status_code, Some(0));
    }

    #[test]
    fn empty_translation_parses_as_unusable() {
        let body = r#"{"translation":"","base_resp":{"status_code":0}}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert!(p.translation.filter(|t| !t.trim().is_empty()).is_none());
    }

    #[test]
    fn nonzero_base_resp_is_detectable() {
        let body = r#"{"translation":"","base_resp":{"status_code":500,"status_message":"internal"}}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_ne!(p.base_resp.unwrap().status_code.unwrap_or(0), 0);
    }

    #[tokio::test]
    async fn unsupported_target_is_rejected_before_the_request() {
        let p = Volcengine::new();
        let cred = Credential {
            provider: "volcengine".into(),
            label: "default".into(),
            fields: Default::default(),
        };
        let mut req = TranslateRequest {
            text: "hi".into(),
            source_lang: "auto".into(),
            target_lang: "xx".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: Default::default(),
        };
        req.target_lang = "xx".into();
        assert_eq!(
            p.translate_once(&req, &cred).await.unwrap_err().code(),
            "LANGUAGE_PAIR_NOT_SUPPORTED"
        );
    }
}
