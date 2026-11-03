//! Tencent Cloud Text Translation (TC3-HMAC-SHA256 signature).

use crate::core::config::ProviderSettings;
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{self, client};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::{Duration, Instant};

const HOST: &str = "tmt.tencentcloudapi.com";
const SERVICE: &str = "tmt";
const ACTION: &str = "TextTranslate";
const VERSION: &str = "2018-03-21";

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct Wrapper {
    Response: Response,
}

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename = "TargetText", default)]
    target_text: Option<String>,
    #[serde(rename = "Error", default)]
    error: Option<ApiError>,
    #[serde(rename = "RequestId", default)]
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    #[serde(rename = "Code", default)]
    code: Option<String>,
    #[serde(rename = "Message", default)]
    message: Option<String>,
}

/// Derive the TC3 signing key for a date: HMAC chain secret -> date -> service.
pub fn tc3_signing_key(secret_key: &str, date: &str) -> Vec<u8> {
    let secret_date = http::hmac_sha256_bytes(format!("TC3{secret_key}").as_bytes(), date.as_bytes());
    let secret_service = http::hmac_sha256_bytes(&secret_date, SERVICE.as_bytes());
    http::hmac_sha256_bytes(&secret_service, b"tc3_request")
}

/// Canonical request + string to sign for the fixed POST / JSON shape.
pub fn tc3_string_to_sign(payload: &str, ts: &str, date: &str) -> String {
    let content_hash = http::sha256_hex(payload.as_bytes());
    let canonical = format!(
        "POST\n/\n\ncontent-type:application/json; charset=utf-8\nhost:{HOST}\n\ncontent-type;host\n{content_hash}"
    );
    format!(
        "TC3-HMAC-SHA256\n{ts}\n{date}/{SERVICE}/tc3_request\n{}",
        http::sha256_hex(canonical.as_bytes())
    )
}

pub struct Tencent {
    client: reqwest::Client,
    region: String,
}

impl Tencent {
    pub fn new(settings: &ProviderSettings) -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
            region: settings
                .region
                .clone()
                .unwrap_or_else(|| "ap-guangzhou".to_string()),
        }
    }
}

impl Default for Tencent {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for Tencent {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "tencent".into(),
            display_name: "Tencent Cloud MT (official)".into(),
            kind: ProviderKind::Official,
            priority: 26,
            needs_key: true,
            max_chars: 5000,
            auto_detect: false,
            languages: vec!["*".into()],
            free_tier: Some("1M chars/month for the first 3 months".into()),
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
        let id = cred.get("secret_id").unwrap_or_default();
        let secret = cred.get("secret_key").unwrap_or_default();
        if id.is_empty() || secret.is_empty() {
            return Err(Error::AuthFailed {
                provider: "tencent".into(),
                reason: "secret_id and secret_key are required".into(),
            });
        }
        let from = lang::normalize(&req.source_lang);
        if from == "auto" {
            return Err(Error::InvalidInput(
                "tencent requires an explicit source language".into(),
            ));
        }

        let to = lang::normalize(&req.target_lang);
        let payload = serde_json::json!({
            "SourceText": req.text,
            "Source": from,
            "Target": to,
            "ProjectId": 0,
            "Region": self.region,
        })
        .to_string();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let ts = now.as_secs().to_string();
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();

        let content_hash = http::sha256_hex(payload.as_bytes());
        let _ = content_hash; // folded into the string to sign below
        let scope = format!("{date}/{SERVICE}/tc3_request");
        let string_to_sign = tc3_string_to_sign(&payload, &ts, &date);
        let signing = tc3_signing_key(secret, &date);
        let signature = hex::encode(http::hmac_sha256_bytes(&signing, string_to_sign.as_bytes()));

        let started = Instant::now();
        let resp = self
            .client
            .post(format!("https://{HOST}"))
            .header("Authorization", format!(
                "TC3-HMAC-SHA256 Credential={id}/{scope}, SignedHeaders=content-type;host, Signature={signature}"
            ))
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Host", HOST)
            .header("X-TC-Action", ACTION)
            .header("X-TC-Version", VERSION)
            .header("X-TC-Timestamp", &ts)
            .header("X-TC-Region", &self.region)
            .body(payload)
            .send()
            .await
            .map_err(|e| Error::Network(format!("tencent: {e}")))?;

        let wrapper: Wrapper = http::json("tencent", resp).await?;
        let inner = wrapper.Response;
        if let Some(err) = inner.error {
            let msg = format!(
                "{} {}",
                err.code.unwrap_or_default(),
                err.message.unwrap_or_default()
            );
            return Err(http::map_status("tencent", 200, &msg));
        }

        let text = inner.target_text.ok_or_else(|| Error::BadResponse {
            provider: "tencent".into(),
            reason: "missing TargetText".into(),
        })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "tencent".into(),
            engine: "tencent".into(),
            latency_ms: started.elapsed().as_millis() as u64,
            request_id: inner.request_id.unwrap_or_default(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn string_to_sign_has_the_tc3_shape() {
        let s = tc3_string_to_sign("{\"a\":1}", "1700000000", "2024-01-01");
        assert!(s.starts_with("TC3-HMAC-SHA256\n1700000000\n2024-01-01/tmt/tc3_request\n"));
        // 4 lines: algorithm, ts, scope, hashed canonical request
        assert_eq!(s.lines().count(), 4);
    }

    #[test]
    fn signing_key_depends_on_the_secret() {
        let a = tc3_signing_key("s1", "2024-01-01");
        let b = tc3_signing_key("s2", "2024-01-01");
        assert_ne!(a, b);
        assert_eq!(a.len(), 32, "SHA256 output is 32 bytes");
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"Response":{"TargetText":"你好","RequestId":"abc"}}"#;
        let w: Wrapper = serde_json::from_str(body).unwrap();
        assert_eq!(w.Response.target_text.as_deref(), Some("你好"));
        assert_eq!(w.Response.request_id.as_deref(), Some("abc"));
    }

    #[test]
    fn parses_error_shape() {
        let body = r#"{"Response":{"Error":{"Code":"FailedOperation","Message":"bad key"}}}"#;
        let w: Wrapper = serde_json::from_str(body).unwrap();
        assert_eq!(
            w.Response.error.unwrap().code.as_deref(),
            Some("FailedOperation")
        );
    }

    #[tokio::test]
    async fn missing_credentials_are_auth_failed() {
        let p = Tencent::default();
        let cred = Credential {
            provider: "tencent".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        let req = TranslateRequest {
            text: "hi".into(),
            source_lang: "en".into(),
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

    #[tokio::test]
    async fn auto_source_is_rejected() {
        let p = Tencent::default();
        let cred = Credential {
            provider: "tencent".into(),
            label: "default".into(),
            fields: [
                ("secret_id".to_string(), "id".into()),
                ("secret_key".to_string(), "s".into()),
            ]
            .into_iter()
            .collect(),
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
            "INVALID_INPUT"
        );
    }
}
