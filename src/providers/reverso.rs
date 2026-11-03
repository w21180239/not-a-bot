//! Reverso Context free web endpoint: JSON POST, three-letter language codes.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const HOST: &str = "https://www.reverso.net/text-translation";
const API: &str = "https://api.reverso.net/translate/v1/translation";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Canonical code <-> Reverso's three-letter code, from the site's own map.
const CODES: &[(&str, &str)] = &[
    ("ar", "ara"),
    ("bg", "bul"),
    ("cs", "cze"),
    ("da", "dan"),
    ("de", "ger"),
    ("el", "gre"),
    ("en", "eng"),
    ("es", "spa"),
    ("fa", "per"),
    ("fi", "fin"),
    ("fr", "fra"),
    ("he", "heb"),
    ("hi", "hin"),
    ("hu", "hun"),
    ("id", "ind"),
    ("it", "ita"),
    ("ja", "jpn"),
    ("ko", "kor"),
    ("nl", "dut"),
    ("no", "nor"),
    ("pl", "pol"),
    ("pt", "por"),
    ("ro", "rum"),
    ("ru", "rus"),
    ("sk", "slo"),
    ("sr", "srp"),
    ("sv", "swe"),
    ("th", "tha"),
    ("tr", "tur"),
    ("uk", "ukr"),
    ("ur", "urd"),
    ("vi", "vie"),
    ("zh", "chi"),
    ("zh-TW", "chi"),
];

/// Canonical -> Reverso code; `None` means the engine has no such language.
pub fn to_reverso_code(code: &str) -> Option<&'static str> {
    CODES.iter().find(|(c, _)| *c == code).map(|(_, r)| *r)
}

/// Reverso code -> canonical code.
pub fn from_reverso_code(code: &str) -> Option<&'static str> {
    CODES.iter().find(|(_, r)| *r == code).map(|(c, _)| *c)
}

pub fn supports(code: &str) -> bool {
    to_reverso_code(code).is_some()
}

#[derive(Debug, Deserialize)]
struct Resp {
    translation: Option<Vec<String>>,
    #[serde(rename = "languageDetection", default)]
    language_detection: Option<serde_json::Value>,
}

/// Pull the detected source out of `languageDetection`, which the endpoint
/// fills in only when the request used an auto source.
fn detected(resp: &Resp) -> Option<String> {
    resp.language_detection
        .as_ref()
        .and_then(|v| {
            v.get("from")
                .or_else(|| v.get("input").and_then(|i| i.get("from")))
        })
        .and_then(|v| v.as_str())
        .and_then(from_reverso_code)
        .map(String::from)
}

pub struct Reverso {
    client: reqwest::Client,
}

impl Reverso {
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

impl Default for Reverso {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Reverso {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "reverso".into(),
            display_name: "Reverso Context (web)".into(),
            kind: ProviderKind::Web,
            priority: 9,
            needs_key: false,
            max_chars: 2_000,
            // The endpoint echoes the request language back rather than detecting,
            // so an explicit source is required for a trustworthy result.
            auto_detect: false,
            languages: CODES.iter().map(|(c, _)| c.to_string()).collect(),
            free_tier: Some("unlimited (web endpoint)".into()),
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
        let from_raw = lang::normalize(&req.source_lang);
        // The endpoint has no detection: fall back to Chinese, as the reference does.
        let from_raw = if from_raw == "auto" {
            "zh".to_string()
        } else {
            from_raw
        };

        let from = to_reverso_code(&from_raw).ok_or_else(|| Error::LanguagePairNotSupported {
            from_lang: from_raw.clone(),
            target: to.clone(),
            provider: "reverso".into(),
        })?;
        let target = to_reverso_code(&to).ok_or_else(|| Error::LanguagePairNotSupported {
            from_lang: from_raw.clone(),
            target: to.clone(),
            provider: "reverso".into(),
        })?;

        let payload = serde_json::json!({
            "format": "text",
            "from": from,
            "to": target,
            "input": req.text,
            "options": {
                "contextResults": "true",
                "languageDetection": "true",
                "sentenceSplitter": "true",
                "origin": "translation.web",
            }
        });

        // The API checks the session cookie set by the site page.
        let _ = self.client.get(HOST).send().await.map(|r| r.status());

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .header("Origin", "https://www.reverso.net")
            .header("Referer", HOST)
            .header("X-Reverso-Origin", "translation.web")
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Network(format!("reverso: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("reverso", resp).await?;
        if status != 200 {
            return Err(map_status("reverso", status, &body));
        }

        let parsed: Resp = serde_json::from_str(&body).map_err(|e| Error::BadResponse {
            provider: "reverso".into(),
            reason: format!("{e}: {}", truncate(&body, 160)),
        })?;

        // Read detection before moving `translation` out of the response.
        let source = detected(&parsed);

        // A 200 with a Cloudflare interstitial parses as None; treat as blocked.
        let text = parsed
            .translation
            .filter(|v| !v.is_empty())
            .map(|v| v.concat())
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "reverso".into(),
                reason: format!("missing translation: {}", truncate(&body, 160)),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: source.unwrap_or(from_raw),
            target_lang: req.target_lang.clone(),
            provider: "reverso".into(),
            engine: "reverso".into(),
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
        let p = Reverso::default();
        let info = p.info();
        assert_eq!(info.name, "reverso");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 9);
        assert!(!info.auto_detect);
    }

    #[test]
    fn maps_language_codes() {
        assert_eq!(to_reverso_code("en"), Some("eng"));
        assert_eq!(to_reverso_code("zh"), Some("chi"));
        assert_eq!(to_reverso_code("zh-TW"), Some("chi"));
        assert_eq!(to_reverso_code("ja"), Some("jpn"));
        assert_eq!(to_reverso_code("xx"), None);
        assert_eq!(from_reverso_code("eng"), Some("en"));
        assert_eq!(from_reverso_code("chi"), Some("zh"));
        assert_eq!(from_reverso_code("zzz"), None);
    }

    #[test]
    fn supports_only_mapped_codes() {
        assert!(supports("fr"));
        assert!(supports("ru"));
        assert!(!supports("xx"));
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from api.reverso.net.
        let body = r#"{"id":"cdb666a5","from":"eng","to":"chi","input":["Hello world, this is a test."],"correctedText":null,"translation":["你好,世界,这是一次测试。"],"engines":["Lingvanex"]}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(
            p.translation.map(|v| v.concat()).as_deref(),
            Some("你好,世界,这是一次测试。")
        );
    }

    #[test]
    fn multi_segment_translation_joins() {
        let p: Resp = serde_json::from_str(r#"{"translation":["你好","世界"]}"#).unwrap();
        assert_eq!(
            p.translation.map(|v| v.concat()).as_deref(),
            Some("你好世界")
        );
    }

    #[test]
    fn interstitial_shape_is_detectable() {
        // A blocked call still returns 200 JSON with no translation array.
        let p: Resp = serde_json::from_str(r#"{"error":"captcha required"}"#).unwrap();
        assert!(p.translation.is_none());
    }

    #[test]
    fn detection_maps_back_to_canonical() {
        let p: Resp =
            serde_json::from_str(r#"{"translation":["x"],"languageDetection":{"from":"fra"}}"#)
                .unwrap();
        assert_eq!(detected(&p).as_deref(), Some("fr"));
    }
}
