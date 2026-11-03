//! Iciba free web endpoint: AES-ECB signed request, AES-ECB encrypted reply.

use crate::core::crypto::{aes_ecb_decrypt_pkcs7, aes_ecb_encrypt_pkcs7};
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, md5_hex, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use std::time::{Duration, Instant};

const HOST: &str = "https://www.iciba.com/fy";
const API: &str = "https://ifanyi.iciba.com/index.php";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Keys hard-coded in the site's own bundle.
const AUTH_USER: &str = "key_web_new_fanyi";
const SIGN_SALT: &str = "6key_web_new_fanyi";
const SIGN_SECRET: &str = "6dVjYLFyzfkFkk";
const ENCRYPT_KEY: &str = "L4fBtD5fLC9FQw22";
const DECRYPT_KEY: &str = "aahc3TfyfCEmER33";

/// Envelope: `content` is base64 of the AES-ECB ciphertext.
#[derive(Debug, Deserialize)]
struct Envelope {
    content: Option<String>,
}

/// Plaintext body after decryption.
#[derive(Debug, Deserialize)]
pub struct Payload {
    out: Option<String>,
    #[serde(default)]
    err_no: Option<i64>,
    #[serde(rename = "from", default)]
    from_lang: Option<String>,
}

/// `base64(AES-ECB-PKCS7(md5("6key_web_new_fanyi" + secret + text)[:16]))`.
pub fn iciba_sign(text: &str) -> std::result::Result<String, String> {
    let digest = md5_hex(&format!("{SIGN_SALT}{SIGN_SECRET}{text}"));
    let head = digest.get(..16).ok_or("md5 shorter than 16 chars")?;
    let ct = aes_ecb_encrypt_pkcs7(head, ENCRYPT_KEY)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(ct))
}

/// base64 -> AES-ECB decrypt -> JSON body.
pub fn iciba_unwrap(content_b64: &str) -> std::result::Result<Payload, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(content_b64.trim())
        .map_err(|e| e.to_string())?;
    let plain = aes_ecb_decrypt_pkcs7(&raw, DECRYPT_KEY)?;
    serde_json::from_str(&plain).map_err(|e| format!("{e}: {}", truncate(&plain, 120)))
}

pub struct Iciba {
    client: reqwest::Client,
}

impl Iciba {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent(UA)
                .timeout(Duration::from_secs(30))
                .cookie_store(true)
                .gzip(true)
                .brotli(true)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// The endpoint expects cookies from the site page; the reference primes them the same way.
    async fn prime_cookies(&self) {
        let _ = self.client.get(HOST).send().await.map(|r| r.status());
    }
}

impl Default for Iciba {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Iciba {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "iciba".into(),
            display_name: "Iciba (web)".into(),
            kind: ProviderKind::Web,
            priority: 3,
            needs_key: false,
            max_chars: 3_000,
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
        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);
        let sign = iciba_sign(&req.text).map_err(|e| Error::Internal(format!("iciba sign: {e}")))?;

        let query: [(&str, String); 5] = [
            ("c", "trans".into()),
            ("m", "fyV2".into()),
            ("client", "6".into()),
            ("auth_user", AUTH_USER.into()),
            ("sign", sign),
        ];
        // The endpoint only accepts `from=auto&to=auto`; it detects and returns both codes.
        let form: [(&str, &str); 3] = [
            ("from", if from == "auto" { "auto" } else { &from }),
            ("to", if from == "auto" { "auto" } else { &to }),
            ("q", req.text.as_str()),
        ];

        self.prime_cookies().await;

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .query(&query)
            .form(&form)
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .map_err(|e| Error::Network(format!("iciba: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("iciba", resp).await?;
        if status != 200 {
            return Err(map_status("iciba", status, &body));
        }

        let envelope: Envelope =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse {
                provider: "iciba".into(),
                reason: format!("{e}: {}", truncate(&body, 160)),
            })?;

        let content = envelope.content.ok_or_else(|| Error::BadResponse {
            provider: "iciba".into(),
            reason: format!("missing content: {}", truncate(&body, 160)),
        })?;

        let payload = iciba_unwrap(&content).map_err(|e| Error::BadResponse {
            provider: "iciba".into(),
            reason: format!("unwrap: {e}"),
        })?;

        if payload.err_no.unwrap_or(0) != 0 {
            return Err(Error::Upstream {
                provider: "iciba".into(),
                status,
                body: format!("iciba err_no={}", payload.err_no.unwrap_or(0)),
            });
        }

        let text = payload.out.filter(|s| !s.trim().is_empty()).ok_or_else(|| {
            Error::BadResponse {
                provider: "iciba".into(),
                reason: "empty out field".into(),
            }
        })?;

        let detected = payload
            .from_lang
            .filter(|s| s != "auto")
            .or(if from == "auto" { None } else { Some(from.clone()) })
            .unwrap_or_else(|| from.clone());

        Ok(TranslateResponse {
            text,
            detected_source: detected,
            target_lang: req.target_lang.clone(),
            provider: "iciba".into(),
            engine: "iciba".into(),
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
        let p = Iciba::default();
        let info = p.info();
        assert_eq!(info.name, "iciba");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 3);
    }

    #[test]
    fn sign_is_deterministic_and_matches_reference() {
        // Captured from the Python reference for "Hello world".
        let a = iciba_sign("Hello world").unwrap();
        let b = iciba_sign("Hello world").unwrap();
        assert_eq!(a, b);
        assert_eq!(a, "YCWr7txviG/ChW/yHJEBH9RriqVIAJSQ+xmfU0q7dIE=");
    }

    #[test]
    fn sign_varies_with_text() {
        assert_ne!(iciba_sign("a").unwrap(), iciba_sign("b").unwrap());
    }

    #[test]
    fn unwrap_round_trips_captured_envelope() {
        // Ciphertext captured live from ifanyi.iciba.com for "Hello world".
        let b64 = "X2NheRsV7GVaBbfK/jxZ6h6rWRz0J268vfthunwKmlJIHB687XwU1lxRMBgI+YF5\
                   Y8AP+ft3k8fJzHLLUaosfnvsP4MNSM74/BzsC05MtvnJTCV5bhVIW9p9pL3DT4Ks\
                   notKqYBMlQ9Fcu55E2aqDB/CuvHtL+Xr7QaSqshoo0lqBvQYYZSQr2f6ifHKZVBK\
                   EtVSOUoazmuGGCpbXRqq7HMzGGHeY6XMGAa59stAfInPz/xptWwWop4gQiTqYUBd\
                   HV00AljV1fAJVAgIV0b14Lm93k25t24lqQbivk5wMXVR44kGC6S8pJDN0SoIenp3\
                   cnEZgCDNAKeiW5EymEoPNLHeldCMKaIOcQ7cMaH+HAA=";
        let payload = iciba_unwrap(b64).unwrap();
        assert_eq!(payload.out.as_deref(), Some("你好，世界"));
        assert_eq!(payload.from_lang.as_deref(), Some("en"));
        assert_eq!(payload.err_no, Some(0));
    }

    #[test]
    fn bad_envelope_is_an_error() {
        assert!(iciba_unwrap("not-base64!!!").is_err());
        assert!(serde_json::from_str::<Envelope>(r#"{"status":1}"#)
            .unwrap()
            .content
            .is_none());
    }
}
