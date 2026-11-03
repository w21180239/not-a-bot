//! Sogou free web endpoint: md5-signed form POST, no key required.

use crate::core::creds::Credential;
use crate::core::http::md5_hex;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

const HOST: &str = "https://fanyi.sogou.com/text";
const API: &str = "https://fanyi.sogou.com/api/transpc/text/result";
const SECRET_CODE: &str = "109984457";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Sogou wants `zh-CHS` as the canonical simplified code.
pub fn to_sogou_code(code: &str) -> String {
    match code {
        "zh" | "zh-CN" | "zh-Hans" => "zh-CHS".into(),
        "zh-TW" | "zh-Hant" => "zh-CHT".into(),
        other => other.into(),
    }
}

/// MD5 sign over `from + to + text + secretCode` (raw string, not URL-encoded).
pub fn sogou_sign(from: &str, to: &str, text: &str) -> String {
    md5_hex(&format!("{from}{to}{text}{SECRET_CODE}"))
}

#[derive(Debug, Deserialize)]
struct Resp {
    data: Option<Data>,
}

#[derive(Debug, Deserialize)]
struct Data {
    translate: Option<TranslateItem>,
    detect: Option<DetectItem>,
}

#[derive(Debug, Deserialize)]
struct TranslateItem {
    #[serde(rename = "errorCode")]
    error_code: Option<String>,
    #[serde(rename = "dit", default)]
    dit: Option<String>,
    #[serde(rename = "from")]
    from_lang: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DetectItem {
    #[serde(rename = "detect", default)]
    detect: Option<String>,
}

pub struct Sogou {
    client: reqwest::Client,
    /// One UUID per process lifetime; the reference reuses it.
    uuid: String,
}

impl Sogou {
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
            uuid: Uuid::new_v4().to_string(),
        }
    }

    /// The endpoint expects cookies from the page load.
    async fn prime(&self) {
        let _ = self.client.get(HOST).send().await.map(|r| r.status());
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
            display_name: "Sogou (web)".into(),
            kind: ProviderKind::Web,
            priority: 10,
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
        let to = to_sogou_code(&lang::normalize(&req.target_lang));
        let from = to_sogou_code(&lang::normalize(&req.source_lang));

        self.prime().await;

        let sign = sogou_sign(&from, &to, &req.text);

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("from".into(), from.clone());
        form.insert("to".into(), to.clone());
        form.insert("text".into(), req.text.clone());
        form.insert("uuid".into(), self.uuid.clone());
        form.insert("s".into(), sign);
        form.insert("client".into(), "pc".into());
        form.insert("fr".into(), "browser_pc".into());
        form.insert("needQc".into(), "1".into());

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .header("Origin", "https://fanyi.sogou.com")
            .header("Referer", "https://fanyi.sogou.com/text")
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("sogou: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("sogou", resp).await?;
        if status != 200 {
            return Err(map_status("sogou", status, &body));
        }

        let parsed: Resp =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "sogou".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;

        let data = parsed.data.ok_or_else(|| Error::BadResponse {
            provider: "sogou".into(),
            reason: format!("missing data: {}", truncate(&body, 160)),
        })?;

        let item = data.translate.ok_or_else(|| Error::BadResponse {
            provider: "sogou".into(),
            reason: "missing translate".into(),
        })?;

        // errorCode 20 is the endpoint's per-IP throttle; it is the same class of
        // failure the reference surfaces, and it clears on its own.
        match item.error_code.as_deref() {
            Some("0") | None => {}
            Some("20") => {
                return Err(Error::RateLimited {
                    provider: "sogou".into(),
                    retry_after_secs: Some(300),
                })
            }
            Some(code) => {
                return Err(Error::BadResponse {
                    provider: "sogou".into(),
                    reason: format!("error code {code}"),
                })
            }
        }

        let text = item.dit.filter(|s| !s.trim().is_empty()).ok_or_else(|| {
            Error::BadResponse {
                provider: "sogou".into(),
                reason: "empty dit".into(),
            }
        })?;

        let detected = item
            .from_lang
            .filter(|s| !s.is_empty())
            .or(data.detect.and_then(|d| d.detect))
            .unwrap_or(from);

        Ok(TranslateResponse {
            text,
            detected_source: detected,
            target_lang: req.target_lang.clone(),
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
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 10);
    }

    #[test]
    fn sign_is_deterministic() {
        let a = sogou_sign("en", "zh-CHS", "Hello world");
        let b = sogou_sign("en", "zh-CHS", "Hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn chinese_variants_map_to_chs() {
        assert_eq!(to_sogou_code("zh"), "zh-CHS");
        assert_eq!(to_sogou_code("zh-CN"), "zh-CHS");
        assert_eq!(to_sogou_code("zh-Hans"), "zh-CHS");
        assert_eq!(to_sogou_code("zh-TW"), "zh-CHT");
        assert_eq!(to_sogou_code("zh-Hant"), "zh-CHT");
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"status":0,"data":{"translate":{"errorCode":"0","dit":"你好，世界","from":"en","to":"zh-CHS"}}}"#;
        let r: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(r.data.unwrap().translate.unwrap().dit.as_deref(), Some("你好，世界"));
    }

    #[test]
    fn detects_shape() {
        let body = r#"{"status":0,"data":{"detect":{"detect":"en","errorCode":"0"}}}"#;
        let r: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(r.data.unwrap().detect.unwrap().detect.as_deref(), Some("en"));
    }
}
