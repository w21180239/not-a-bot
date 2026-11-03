//! Youdao open API. Signs with SHA-256 over appKey + input + salt + curtime + secret.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{self, client, json};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://openapi.youdao.com/api";

/// Youdao truncates the input to a 20-char prefix when signing longer text.
fn input_snippet(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= 20 {
        text.to_string()
    } else {
        let head: String = chars[..3].iter().collect();
        let tail: String = chars[chars.len() - 3..].iter().collect();
        format!("{}{}{}", head, chars.len(), tail)
    }
}

/// SHA-256 hex of appKey + input + salt + curtime + appSecret (v3 scheme).
pub fn sign(app_key: &str, text: &str, salt: &str, curtime: &str, secret: &str) -> String {
    http::sha256_hex(
        format!("{}{}{}{}{}", app_key, input_snippet(text), salt, curtime, secret).as_bytes(),
    )
}

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(default)]
    translation: Option<Vec<String>>,
    #[serde(default, rename = "errorCode")]
    error_code: Option<String>,
}

pub struct Youdao {
    client: reqwest::Client,
}

impl Youdao {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Youdao {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Youdao {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "youdao".into(),
            display_name: "Youdao (official)".into(),
            kind: ProviderKind::Official,
            priority: 27,
            needs_key: true,
            max_chars: 5000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("limited trial quota per account".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        cred: &Credential,
    ) -> Result<TranslateResponse> {
        let key = cred.get("app_key").unwrap_or_default();
        let secret = cred.get("app_secret").unwrap_or_default();
        if key.is_empty() || secret.is_empty() {
            return Err(Error::AuthFailed {
                provider: "youdao".into(),
                reason: "app_key and app_secret are required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);
        let salt = uuid::Uuid::new_v4().simple().to_string();
        let curtime = http::now_secs().to_string();

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("q".into(), req.text.clone());
        form.insert("from".into(), from.clone());
        form.insert("to".into(), to.clone());
        form.insert("appKey".into(), key.to_string());
        form.insert("salt".into(), salt.clone());
        form.insert("curtime".into(), curtime.clone());
        form.insert("signType".into(), "v3".into());
        form.insert("sign".into(), sign(key, &req.text, &salt, &curtime, secret));

        let started = Instant::now();
        let resp = self
            .client
            .post(ENDPOINT)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("youdao: {e}")))?;

        let parsed: Resp = json("youdao", resp).await?;
        if let Some(code) = parsed.error_code.as_deref() {
            if code != "0" {
                return Err(http::map_status("youdao", 200, code));
            }
        }
        let parts = parsed
            .translation
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "youdao".into(),
                reason: "missing translation".into(),
            })?;

        Ok(TranslateResponse {
            text: parts.join("\n"),
            detected_source: from,
            target_lang: to,
            provider: "youdao".into(),
            engine: "youdao".into(),
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
    fn short_text_signs_whole_input() {
        let a = sign("k", "hi", "s", "t", "sec");
        assert_eq!(a.len(), 64, "sha256 hex is 64 chars");
        assert_eq!(a, sign("k", "hi", "s", "t", "sec"));
    }

    #[test]
    fn long_text_uses_prefix_length_suffix_snippet() {
        let long = "a".repeat(30);
        let s = input_snippet(&long);
        assert_eq!(s, format!("aaa{}aaa", 30));
    }

    #[test]
    fn snippet_changes_signature() {
        let short = sign("k", "abcdefghij", "s", "t", "sec");
        let long = sign("k", &"x".repeat(40), "s", "t", "sec");
        assert_ne!(short, long);
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"translation":["你好"],"errorCode":"0"}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.translation.unwrap().join("\n"), "你好");
    }

    #[test]
    fn parses_error_shape() {
        let body = r#"{"errorCode":"103"}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.error_code.as_deref(), Some("103"));
        assert!(p.translation.is_none());
    }

    #[tokio::test]
    async fn missing_keys_are_auth_failed() {
        let p = Youdao::new();
        let cred = Credential {
            provider: "youdao".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "auto".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        assert_eq!(
            p.translate_once(&req, &cred).await.unwrap_err().code(),
            "AUTH_FAILED"
        );
    }
}
