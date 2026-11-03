//! iFlytek iFlyrec free web endpoint. Uses numeric language indices.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

const API: &str = "https://fanyi.iflyrec.com/TranslationService/v1/textAutoTranslation";
const DETECT: &str = "https://fanyi.iflyrec.com/TranslationService/v1/languageDetection";

/// Endpoint-internal language indices from the site's bundle.
const LANG_INDEX: &[(&str, u32)] = &[
    ("zh", 1),
    ("en", 2),
    ("ja", 3),
    ("ko", 4),
    ("ru", 5),
    ("fr", 6),
    ("es", 7),
    ("vi", 8),
    ("yue", 9),
    ("ar", 12),
    ("de", 13),
    ("it", 14),
];

pub fn to_iflyrec_id(code: &str) -> Option<u32> {
    LANG_INDEX.iter().find(|(c, _)| *c == code).map(|(_, i)| *i)
}

pub fn from_iflyrec_id(id: u32) -> Option<&'static str> {
    LANG_INDEX.iter().find(|(_, i)| *i == id).map(|(c, _)| *c)
}

/// True when the endpoint serves this canonical code.
pub fn supports(code: &str) -> bool {
    to_iflyrec_id(code).is_some()
}

#[derive(Debug, Serialize)]
struct DetectReq<'a> {
    #[serde(rename = "originalText")]
    original_text: &'a str,
}

#[derive(Debug, Deserialize)]
struct DetectResp {
    #[serde(rename = "code", default)]
    _code: serde_json::Value,
    #[serde(rename = "desc", default)]
    _desc: Option<String>,
    biz: Option<Vec<DetectItem>>,
}

#[derive(Debug, Deserialize)]
struct DetectItem {
    #[serde(rename = "detectionLanguage")]
    detection_language: Option<u32>,
}

#[derive(Debug, Serialize)]
struct TranslateReq {
    from: u32,
    to: u32,
    #[serde(rename = "openTerminology")]
    open_terminology: String,
    contents: Vec<Content>,
}

#[derive(Debug, Serialize)]
struct Content {
    text: String,
    #[serde(rename = "frontBlankLine")]
    front_blank_line: u8,
}

#[derive(Debug, Deserialize)]
struct TranslateResp {
    #[serde(rename = "code", default)]
    _code: serde_json::Value,
    #[serde(rename = "desc", default)]
    _desc: Option<String>,
    biz: Option<Vec<BizItem>>,
}

#[derive(Debug, Deserialize)]
struct BizItem {
    #[serde(rename = "translateResult", default)]
    translate_result: Option<String>,
}

/// Split input into the endpoint's line-per-content shape.
fn build_lines(text: &str) -> Vec<Content> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| Content {
            text: l.to_string(),
            front_blank_line: 0,
        })
        .collect()
}

pub struct Iflyrec {
    client: reqwest::Client,
}

impl Iflyrec {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                     (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36",
                )
                .cookie_store(true)
                .timeout(Duration::from_secs(30))
                .gzip(true)
                .brotli(true)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    async fn detect(&self, text: &str) -> Option<String> {
        let resp = self
            .client
            .post(DETECT)
            .query(&[("t", crate::core::http::now_millis().to_string())])
            .json(&DetectReq {
                original_text: text,
            })
            .send()
            .await
            .ok()?;
        let parsed: DetectResp = resp.json().await.ok()?;
        parsed
            .biz?
            .first()?
            .detection_language
            .and_then(from_iflyrec_id)
            .map(String::from)
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
            display_name: "iFlytek iFlyrec (web)".into(),
            kind: ProviderKind::Web,
            priority: 4,
            needs_key: false,
            max_chars: 2_000,
            auto_detect: true,
            languages: LANG_INDEX.iter().map(|(c, _)| c.to_string()).collect(),
            free_tier: Some("unlimited (web endpoint)".into()),
            configured: true,
        }
    }

    fn supports(&self, target: &str) -> bool {
        supports(target)
    }

    /// The endpoint only serves zh-anchored pairs: either side must be zh.
    fn supports_pair(&self, source: &str, target: &str) -> bool {
        supports(source) && supports(target) && (source == "zh" || source == "auto" || target == "zh")
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let mut from = lang::normalize(&req.source_lang);

        let to_id = to_iflyrec_id(&to).ok_or_else(|| Error::LanguagePairNotSupported {
            from_lang: from.clone(),
            target: to.clone(),
            provider: "iflyrec".into(),
        })?;

        if from == "auto" {
            from = self.detect(&req.text).await.unwrap_or_else(|| "zh".into());
        }
        let from_id = to_iflyrec_id(&from).ok_or_else(|| Error::LanguagePairNotSupported {
            from_lang: from.clone(),
            target: to.clone(),
            provider: "iflyrec".into(),
        })?;
        // A non-zh pair is rejected by the endpoint with code 200004; skip it.
        if from != "zh" && to != "zh" {
            return Err(Error::LanguagePairNotSupported {
                from_lang: from,
                target: to,
                provider: "iflyrec".into(),
            });
        }

        let body = TranslateReq {
            from: from_id,
            to: to_id,
            open_terminology: "false".into(),
            contents: build_lines(&req.text),
        };

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .query(&[("t", crate::core::http::now_millis().to_string())])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(format!("iflyrec: {e}")))?;

        let status = resp.status().as_u16();
        let text_body = body_text("iflyrec", resp).await?;
        if status != 200 {
            return Err(map_status("iflyrec", status, &text_body));
        }

        let parsed: TranslateResp =
            serde_json::from_str(&text_body).map_err(|e| Error::BadResponse {
                provider: "iflyrec".into(),
                reason: format!("{e}: {}", truncate(&text_body, 160)),
            })?;

        let lines: Vec<String> = parsed
            .biz
            .ok_or_else(|| Error::BadResponse {
                provider: "iflyrec".into(),
                reason: format!("missing biz: {}", truncate(&text_body, 160)),
            })?
            .into_iter()
            .filter_map(|b| b.translate_result.filter(|s| !s.is_empty()))
            .collect();

        if lines.is_empty() {
            return Err(Error::BadResponse {
                provider: "iflyrec".into(),
                reason: "empty translateResult".into(),
            });
        }

        Ok(TranslateResponse {
            text: lines.join("\n"),
            detected_source: from,
            target_lang: req.target_lang.clone(),
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
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 4);
    }

    #[test]
    fn language_id_mapping() {
        assert_eq!(to_iflyrec_id("zh"), Some(1));
        assert_eq!(to_iflyrec_id("en"), Some(2));
        assert_eq!(to_iflyrec_id("it"), Some(14));
        assert_eq!(to_iflyrec_id("xx"), None);
        assert_eq!(from_iflyrec_id(2), Some("en"));
        assert_eq!(from_iflyrec_id(999), None);
    }

    #[test]
    fn supports_only_listed_codes() {
        let p = Iflyrec::default();
        assert!(p.supports("zh"));
        assert!(p.supports("ja"));
        assert!(!p.supports("xx"));
    }

    #[test]
    fn build_lines_skips_blank() {
        let lines = build_lines("one\n\n  \ntwo\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "one");
        assert_eq!(lines[1].text, "two");
        assert_eq!(lines[0].front_blank_line, 0);
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"biz":[{"translateResult":"你好","detectedLanguage":2},{"translateResult":"世界"}]}"#;
        let p: TranslateResp = serde_json::from_str(body).unwrap();
        let items = p.biz.unwrap();
        assert_eq!(items[0].translate_result.as_deref(), Some("你好"));
        assert_eq!(items[1].translate_result.as_deref(), Some("世界"));
    }

    #[test]
    fn missing_biz_is_detectable() {
        let p: TranslateResp = serde_json::from_str(r#"{"code":500}"#).unwrap();
        assert!(p.biz.is_none());
    }
}
