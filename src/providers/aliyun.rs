//! Alibaba Cloud Machine Translation (RPC API, HMAC-SHA1 signature).

use crate::core::config::ProviderSettings;
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{self, client, json, percent_encode};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const ENDPOINT: &str = "https://mt.cn-hangzhou.aliyuncs.com";

#[derive(Debug, Deserialize)]
struct Resp {
    data: Option<Data>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Data {
    #[serde(rename = "translatedText", default)]
    translated_text: Option<String>,
}

/// Alibaba RPC signature: HMAC-SHA1(secret + "&", "GET&%2F&" + encoded query).
pub fn sign_query(params: &BTreeMap<String, String>, secret: &str) -> String {
    let canonical = http::canonical_query(params);
    let string_to_sign = format!("GET&{}&{}", percent_encode("/"), percent_encode(&canonical));
    http::hmac_sha1(&format!("{secret}&"), &string_to_sign)
}

pub struct Aliyun {
    client: reqwest::Client,
    endpoint: String,
}

impl Aliyun {
    pub fn new(settings: &ProviderSettings) -> Self {
        Self {
            client: client(Duration::from_secs(30)).unwrap_or_else(|_| reqwest::Client::new()),
            endpoint: settings
                .base_url
                .clone()
                .unwrap_or_else(|| ENDPOINT.to_string())
                .trim_end_matches('/')
                .to_string(),
        }
    }
}

impl Default for Aliyun {
    fn default() -> Self {
        Self::new(&ProviderSettings::default())
    }
}

#[async_trait]
impl Provider for Aliyun {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "aliyun".into(),
            display_name: "Alibaba Cloud MT".into(),
            kind: ProviderKind::Keyed,
            needs_key: true,
            max_chars: 5000,
            auto_detect: false,
            languages: vec!["*".into()],
            free_tier: Some("1M chars/month".into()),
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
        let id = cred.get("access_key_id").unwrap_or_default();
        let secret = cred.get("access_key_secret").unwrap_or_default();
        if id.is_empty() || secret.is_empty() {
            return Err(Error::AuthFailed {
                provider: "aliyun".into(),
                reason: "access_key_id and access_key_secret are required".into(),
            });
        }

        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);
        if from == "auto" {
            return Err(Error::InvalidInput(
                "aliyun requires an explicit source language".into(),
            ));
        }

        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("Action".into(), "TranslateGeneral".into());
        params.insert("Version".into(), "2018-10-12".into());
        params.insert("FormatType".into(), "text".into());
        params.insert("SourceLanguage".into(), from.clone());
        params.insert("TargetLanguage".into(), to.clone());
        params.insert("SourceText".into(), req.text.clone());
        params.insert("AccessKeyId".into(), id.to_string());
        params.insert("SignatureMethod".into(), "HMAC-SHA1".into());
        params.insert("SignatureVersion".into(), "1.0".into());
        params.insert("SignatureNonce".into(), uuid::Uuid::new_v4().simple().to_string());
        params.insert("Timestamp".into(), chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        params.insert("Signature".into(), sign_query(&params, secret));

        let started = Instant::now();
        let resp = self
            .client
            .get(&self.endpoint)
            .query(&params)
            .send()
            .await
            .map_err(|e| Error::Network(format!("aliyun: {e}")))?;

        let parsed: Resp = json("aliyun", resp).await?;
        if let (Some(code), Some(msg)) = (parsed.code.as_deref(), parsed.message.as_deref()) {
            if !code.is_empty() && code != "200" && code != "OK" {
                return Err(http::map_status("aliyun", 200, &format!("{code} {msg}")));
            }
        }

        let text = parsed
            .data
            .and_then(|d| d.translated_text)
            .ok_or_else(|| Error::BadResponse {
                provider: "aliyun".into(),
                reason: "missing data.translatedText".into(),
            })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: to,
            provider: "aliyun".into(),
            engine: "aliyun".into(),
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
    fn signature_is_stable_and_secret_dependent() {
        let mut p = BTreeMap::new();
        p.insert("Action".into(), "TranslateGeneral".into());
        p.insert("AccessKeyId".into(), "testid".into());
        let a = sign_query(&p, "secret");
        let b = sign_query(&p, "secret");
        let c = sign_query(&p, "other");
        assert_eq!(a, b);
        assert_ne!(a, c, "secret must change the signature");
    }

    #[test]
    fn signature_is_base64_of_20_bytes() {
        let mut p = BTreeMap::new();
        p.insert("A".into(), "1".into());
        let sig = sign_query(&p, "k");
        let raw = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &sig).unwrap();
        assert_eq!(raw.len(), 20, "SHA1 output is 20 bytes");
    }

    #[test]
    fn parses_success_shape() {
        let body = r#"{"code":"200","data":{"translatedText":"你好"}}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.data.unwrap().translated_text.as_deref(), Some("你好"));
    }

    #[test]
    fn parses_error_shape() {
        let body = r#"{"code":"10011","message":"invalid language"}"#;
        let parsed: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(parsed.code.as_deref(), Some("10011"));
    }

    #[tokio::test]
    async fn auto_source_is_rejected() {
        let p = Aliyun::default();
        let cred = Credential {
            provider: "aliyun".into(),
            label: "default".into(),
            fields: [
                ("access_key_id".to_string(), "id".into()),
                ("access_key_secret".to_string(), "s".into()),
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
        let err = p.translate_once(&req, &cred).await.unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[tokio::test]
    async fn missing_keys_are_auth_failed() {
        let p = Aliyun::default();
        let cred = Credential {
            provider: "aliyun".into(),
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
        let err = p.translate_once(&req, &cred).await.unwrap_err();
        assert_eq!(err.code(), "AUTH_FAILED");
    }
}
