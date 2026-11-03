//! Papago (Naver) free web endpoint. Keyless, cookie-primed, form-encoded.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const HOST: &str = "https://papago.naver.com";
const API: &str = "https://papago.naver.com/api/text/translation";
const DETECT: &str = "https://papago.naver.com/api/langs/dect";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Codes the endpoint accepts; Chinese needs the CN/TW suffix.
const LANGS: &[&str] = &[
    "ar", "de", "en", "es", "fr", "hi", "id", "it", "ja", "ko", "pt", "ru", "th", "vi", "zh-CN",
    "zh-TW",
];

pub fn to_papago_code(code: &str) -> String {
    match code {
        "zh" | "zh-CN" => "zh-CN".into(),
        "zh-TW" | "zh-Hant" => "zh-TW".into(),
        other => other.into(),
    }
}

pub fn supports(code: &str) -> bool {
    LANGS.contains(&code)
}

#[derive(Debug, Deserialize)]
struct DetectResp {
    #[serde(rename = "langCode")]
    lang_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranslateResp {
    #[serde(rename = "translatedText")]
    translated_text: Option<String>,
    #[serde(rename = "srcLangType")]
    src_lang_type: Option<String>,
}

pub struct Papago {
    client: reqwest::Client,
}

impl Papago {
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

    fn api_headers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("Origin", "https://papago.naver.com"),
            ("Referer", "https://papago.naver.com/"),
            ("X-Requested-With", "XMLHttpRequest"),
        ]
    }

    async fn detect(&self, text: &str) -> Option<String> {
        let _ = self.client.get(HOST).send().await.ok();
        let mut req = self.client.post(DETECT).form(&[("query", text)]);
        for (k, v) in Self::api_headers() {
            req = req.header(k, v);
        }
        let resp = req.send().await.ok()?;
        let parsed: DetectResp = resp.json().await.ok()?;
        parsed.lang_code.filter(|s| !s.is_empty())
    }
}

impl Default for Papago {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Papago {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "papago".into(),
            display_name: "Papago (Naver, web)".into(),
            kind: ProviderKind::Web,
            priority: 8,
            needs_key: false,
            max_chars: 1_000,
            auto_detect: true,
            languages: LANGS
                .iter()
                .map(|c| crate::core::lang::normalize(c))
                .collect(),
            free_tier: Some("unlimited (web endpoint)".into()),
            configured: true,
        }
    }

    fn supports(&self, target: &str) -> bool {
        supports(&to_papago_code(target))
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = to_papago_code(&lang::normalize(&req.target_lang));
        if !supports(&to) {
            return Err(Error::LanguagePairNotSupported {
                from_lang: req.source_lang.clone(),
                target: to,
                provider: "papago".into(),
            });
        }
        let mut from = to_papago_code(&lang::normalize(&req.source_lang));

        // Prime the cookies the API checks.
        let _ = self.client.get(HOST).send().await.map(|r| r.status());

        if from == "auto" {
            from = self
                .detect(&req.text)
                .await
                .ok_or_else(|| Error::BadResponse {
                    provider: "papago".into(),
                    reason: "language detection failed".into(),
                })?;
        }

        let form = [
            ("source", from.as_str()),
            ("target", to.as_str()),
            ("text", req.text.as_str()),
            ("dict", "true"),
            ("useGlossary", "false"),
            ("honorific", "false"),
            ("dictDisplay", "30"),
        ];

        let started = Instant::now();
        let mut call = self.client.post(API).form(&form);
        for (k, v) in Self::api_headers() {
            call = call.header(k, v);
        }
        let resp = call
            .send()
            .await
            .map_err(|e| Error::Network(format!("papago: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("papago", resp).await?;
        if status != 200 {
            return Err(map_status("papago", status, &body));
        }

        let parsed: TranslateResp =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "papago".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;

        let text = parsed
            .translated_text
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "papago".into(),
                reason: format!("missing translatedText: {}", truncate(&body, 160)),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: parsed.src_lang_type.unwrap_or(from),
            target_lang: req.target_lang.clone(),
            provider: "papago".into(),
            engine: "papago".into(),
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
        let p = Papago::default();
        let info = p.info();
        assert_eq!(info.name, "papago");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 8);
    }

    #[test]
    fn maps_chinese_variants() {
        assert_eq!(to_papago_code("zh"), "zh-CN");
        assert_eq!(to_papago_code("zh-TW"), "zh-TW");
        assert_eq!(to_papago_code("en"), "en");
    }

    #[test]
    fn supports_only_listed_codes() {
        assert!(supports("zh-CN"));
        assert!(supports("ja"));
        assert!(!supports("xx"));
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from papago.naver.com.
        let body = r#"{"delay":400,"srcLangType":"en","tarLangType":"zh-CN","translatedText":"你好，世界，这是一个测试。","engineType":"N2MT"}"#;
        let p: TranslateResp = serde_json::from_str(body).unwrap();
        assert_eq!(
            p.translated_text.as_deref(),
            Some("你好，世界，这是一个测试。")
        );
        assert_eq!(p.src_lang_type.as_deref(), Some("en"));
    }

    #[test]
    fn detect_shape_parses() {
        let d: DetectResp = serde_json::from_str(r#"{"langCode":"en"}"#).unwrap();
        assert_eq!(d.lang_code.as_deref(), Some("en"));
    }

    #[test]
    fn missing_translation_is_detectable() {
        let p: TranslateResp = serde_json::from_str(r#"{"delay":400}"#).unwrap();
        assert!(p.translated_text.is_none());
    }
}
