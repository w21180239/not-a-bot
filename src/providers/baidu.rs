//! Baidu Fanyi API. Chinese-first, signs requests with MD5.
//! Baidu reports most failures in the body with HTTP 200, so `error_code`
//! drives the typed error rather than the status line.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, json, now_secs};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Duration;

const ENDPOINT: &str = "https://fanyi-api.baidu.com/api/trans/vip/translate";

#[derive(Debug, Deserialize)]
struct Resp {
    #[serde(default)]
    trans_result: Option<Vec<TransItem>>,
    #[serde(default)]
    error_code: Option<String>,
    #[serde(default)]
    error_msg: Option<String>,
    #[serde(default)]
    from: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransItem {
    dst: String,
}

/// Baidu signs md5(appid + query + salt + secret).
fn sign(appid: &str, query: &str, salt: &str, secret: &str) -> String {
    crate::core::http::md5_hex(&format!("{appid}{query}{salt}{secret}"))
}

/// Map Baidu's in-body error codes onto the shared taxonomy.
fn map_baidu_error(code: &str, msg: &str) -> Error {
    let provider = "baidu";
    let reason = format!("{code} {msg}");
    match code {
        // Signature / credential problems.
        "54000" | "54001" | "54003" | "54004" | "58000" | "58001" => Error::AuthFailed {
            provider: provider.into(),
            reason,
        },
        // QPS and daily quota.
        "54005" | "54002" => Error::RateLimited {
            provider: provider.into(),
            retry_after_secs: Some(1),
        },
        "54008" | "54009" | "58002" => Error::QuotaExhausted {
            provider: provider.into(),
            retry_after_secs: Some(3600),
        },
        // Unsupported language pair is terminal for this provider.
        "54006" | "54007" | "54010" | "54011" => Error::LanguagePairNotSupported {
            from_lang: String::new(),
            target: String::new(),
            provider: provider.into(),
        },
        _ => Error::Upstream {
            provider: provider.into(),
            status: 200,
            body: reason,
        },
    }
}

pub struct Baidu {
    client: reqwest::Client,
}

impl Baidu {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for Baidu {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Baidu {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "baidu".into(),
            display_name: "Baidu Fanyi (official)".into(),
            kind: ProviderKind::Official,
            priority: 20,
            needs_key: true,
            max_chars: 6000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("50k chars/month standard tier".into()),
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
        let appid = cred.get("appid").unwrap_or_default();
        let secret = cred.get("secret_key").unwrap_or_default();
        if appid.is_empty() || secret.is_empty() {
            return Err(Error::AuthFailed {
                provider: "baidu".into(),
                reason: "appid and secret_key are required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let from = lang::baidu_source(&req.source_lang);
        let salt = format!("{}{}", now_secs(), std::process::id());

        let mut form: BTreeMap<String, String> = BTreeMap::new();
        form.insert("q".into(), req.text.clone());
        form.insert("from".into(), from.clone());
        form.insert("to".into(), to.clone());
        form.insert("appid".into(), appid.to_string());
        form.insert("salt".into(), salt.clone());
        form.insert("sign".into(), sign(appid, &req.text, &salt, secret));

        let resp = self
            .client
            .post(ENDPOINT)
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("baidu: {e}")))?;

        let parsed: Resp = json("baidu", resp).await?;

        if let Some(code) = parsed.error_code.as_deref() {
            let mut err = map_baidu_error(code, parsed.error_msg.as_deref().unwrap_or(""));
            if let Error::LanguagePairNotSupported {
                from_lang: source,
                target,
                ..
            } = &mut err
            {
                *source = from.clone();
                *target = to.clone();
            }
            return Err(err);
        }

        let items = parsed
            .trans_result
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Error::BadResponse {
                provider: "baidu".into(),
                reason: "missing trans_result".into(),
            })?;

        let text = items
            .iter()
            .map(|i| i.dst.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        Ok(TranslateResponse {
            text,
            detected_source: parsed.from.unwrap_or(from),
            target_lang: to,
            provider: "baidu".into(),
            engine: "baidu".into(),
            latency_ms: 0,
            request_id: String::new(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(text: &str) -> TranslateRequest {
        TranslateRequest {
            text: text.into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        }
    }

    #[test]
    fn info_contract() {
        let p = Baidu::new();
        let info = p.info();
        assert_eq!(info.name, "baidu");
        assert!(info.needs_key);
        assert!(info.auto_detect);
        assert!(p.supports("zh"));
    }

    #[test]
    fn signature_is_md5_of_appid_query_salt_secret() {
        // Independently computed with .NET MD5 over the UTF-8 concatenation.
        let got = sign("20201021000561732", "hello", "1598346210", "12345678");
        assert_eq!(got, "8b5aad7a2a02891cb7e58ffc1e717468");
        assert_eq!(got.len(), 32);
    }

    #[test]
    fn signature_changes_with_every_input() {
        let base = sign("a", "hello", "1", "s");
        assert_eq!(base, sign("a", "hello", "1", "s"));
        assert_ne!(base, sign("b", "hello", "1", "s"));
        assert_ne!(base, sign("a", "world", "1", "s"));
        assert_ne!(base, sign("a", "hello", "2", "s"));
        assert_ne!(base, sign("a", "hello", "1", "t"));
    }

    #[test]
    fn error_codes_map_to_right_variants() {
        assert!(matches!(
            map_baidu_error("54003", "sign error"),
            Error::AuthFailed { .. }
        ));
        assert!(matches!(
            map_baidu_error("54005", "qps"),
            Error::RateLimited { .. }
        ));
        assert_eq!(
            map_baidu_error("54008", "daily quota").code(),
            "QUOTA_EXHAUSTED"
        );
        assert_eq!(
            map_baidu_error("54006", "no such language").code(),
            "LANGUAGE_PAIR_NOT_SUPPORTED"
        );
        assert!(map_baidu_error("54005", "qps").retryable());
        assert!(!map_baidu_error("54003", "bad sign").retryable());
    }

    #[test]
    fn joins_segments_with_newline() {
        let body = r#"{"from":"en","to":"zh","trans_result":[{"src":"a","dst":"甲"},{"src":"b","dst":"乙"}]}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        let items = parsed.trans_result.unwrap();
        assert_eq!(
            items
                .iter()
                .map(|i| i.dst.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            "甲\n乙"
        );
    }

    #[tokio::test]
    async fn in_body_error_becomes_typed_error() {
        let body = r#"{"error_code":"54003","error_msg":"Invalid sign value"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert!(parsed.trans_result.is_none());
        let err = map_baidu_error(parsed.error_code.as_deref().unwrap(), "Invalid sign value");
        assert_eq!(err.code(), "AUTH_FAILED");
    }

    #[tokio::test]
    async fn missing_credentials_are_rejected_before_request() {
        let p = Baidu::new();
        let empty = Credential {
            provider: "baidu".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        let err = p.translate_once(&req("hi"), &empty).await.unwrap_err();
        assert_eq!(err.code(), "AUTH_FAILED");
    }
}
