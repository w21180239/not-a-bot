//! Google Translate free web endpoint: no key, fallback only.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{client, json, map_status};
use crate::core::lang;
use crate::providers::{Provider, ProviderKind};
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

/// The free web endpoint returns a nested JSON array: `[[["translated","original",...]],...,"detectedLang"]`
/// We parse only the first element (list of translation segments) and the last (language).
#[derive(Debug, Deserialize)]
struct WebResp(Vec<serde_json::Value>);

pub struct GoogleFree {
    client: reqwest::Client,
}

impl GoogleFree {
    pub fn new() -> Self {
        Self {
            client: client(Duration::from_secs(20)).unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

/// Extract translated text and detected language from the nested Google response array.
fn parse_google_web(body: &str) -> Result<(String, String)> {
    let parsed: WebResp = serde_json::from_str(body).map_err(|e| Error::BadResponse {
        provider: "google".into(),
        reason: format!("{e}"),
    })?;

    // First element: array of [translated, original, ...] segments
    let segments = parsed
        .0
        .first()
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::BadResponse {
            provider: "google".into(),
            reason: "no segments".into(),
        })?;

    let text = segments
        .iter()
        .filter_map(|seg| seg.as_array().and_then(|a| a.first()).and_then(|v| v.as_str()))
        .collect::<String>();

    if text.is_empty() {
        return Err(Error::BadResponse {
            provider: "google".into(),
            reason: "empty translation".into(),
        });
    }

    // Detected language: sometimes at index 2 as a string
    let detected = parsed
        .0
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("auto")
        .to_string();

    Ok((text, detected))
}

#[async_trait]
impl Provider for GoogleFree {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "google".into(),
            display_name: "Google Translate (free)".into(),
            kind: ProviderKind::Free,
            needs_key: false,
            max_chars: 5000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("unlimited (best effort)".into()),
            configured: true,
        }
    }

    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = lang::normalize(&req.target_lang);
        let from = lang::normalize(&req.source_lang);

        let mut form: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
        form.insert("client".into(), "gtx".into());
        form.insert("sl".into(), from.clone());
        form.insert("tl".into(), to.clone());
        form.insert("dt".into(), "t".into());
        form.insert("q".into(), req.text.clone());

        let resp = self
            .client
            .post("https://translate.googleapis.com/translate_a/single")
            .form(&form)
            .send()
            .await
            .map_err(|e| Error::Network(format!("google: {e}")))?;

        let status = resp.status().as_u16();
        let body = crate::core::http::body_text("google", resp).await?;
        if status != 200 {
            return Err(map_status("google", status, &body));
        }

        let (text, detected) = parse_google_web(&body)?;

        Ok(TranslateResponse {
            text,
            detected_source: detected,
            target_lang: to,
            provider: "google".into(),
            engine: "google".into(),
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
    use crate::core::creds::Credential;
    use crate::core::types::TranslateRequest;
    use std::collections::BTreeMap;

    fn req() -> TranslateRequest {
        TranslateRequest {
            text: "hello".into(),
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
        let p = GoogleFree::new();
        let info = p.info();
        assert_eq!(info.name, "google");
        assert!(!info.needs_key);
        assert_eq!(info.max_chars, 5000);
        assert!(p.supports("zh"));
    }

    #[test]
    fn parses_real_shaped_response() {
        // Google returns: [[["你好","hello",null,null,10]],null,"en",...]
        let body = r#"[[["你好","hello",null,null,10]],null,"en"]"#;
        let (text, detected) = parse_google_web(body).unwrap();
        assert_eq!(text, "你好");
        assert_eq!(detected, "en");
    }

    #[test]
    fn joins_multiple_segments() {
        let body = r#"[[["你好","hello",null,null,10],["世界","world",null,null,5]],null,"en"]"#;
        let (text, _) = parse_google_web(body).unwrap();
        assert_eq!(text, "你好世界");
    }

    #[test]
    fn empty_segments_is_bad_response() {
        let body = r#"[[],null,"en"]"#;
        let err = parse_google_web(body).unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
    }

    #[test]
    fn missing_segments_is_bad_response() {
        let body = r#"[null,null,"en"]"#;
        let err = parse_google_web(body).unwrap_err();
        assert_eq!(err.code(), "BAD_RESPONSE");
    }

    #[tokio::test]
    async fn no_key_required() {
        let p = GoogleFree::new();
        let cred = Credential {
            provider: "google".into(),
            label: "default".into(),
            fields: BTreeMap::new(),
        };
        // Will fail with network error in CI (no internet), but proves no auth check
        let result = p.translate_once(&req(), &cred).await;
        // Network may or may not be available; just assert no AuthFailed
        if let Err(e) = &result {
            assert_ne!(e.code(), "AUTH_FAILED", "google free must not require auth");
        }
    }
}