//! Mirai Translate free web endpoint: trial API, key scraped from the page.
//!
//! The trial page embeds `var tran = "<key>"`; every translate call must carry
//! it, and the endpoint expects a trace ping on the same session first.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status, truncate};
use crate::core::lang;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use uuid::Uuid;

const HOST: &str = "https://miraitranslate.com/trial/";
const API: &str = "https://trial.miraitranslate.com/trial/api/translate.php";
const DETECT: &str = "https://trial.miraitranslate.com/trial/api/detect_lang.php";
const TRACE: &str = "https://trial.miraitranslate.com/trial/api/trace.php";
const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
(KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/// Pull `var tran = "<key>";` out of the trial page.
pub fn extract_tran_key(html: &str) -> Option<String> {
    let marker = "var tran = \"";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    let key = &rest[..end];
    if key.is_empty() {
        None
    } else {
        Some(key.to_string())
    }
}

/// Mirai uses bare `zh`/`ja`/`ko` and rejects `zh-TW`-style region tags.
pub fn to_mirai_code(code: &str) -> String {
    match code {
        "zh" | "zh-CN" | "zh-Hans" | "yue" => "zh".into(),
        "zh-TW" | "zh-Hant" => "zh-Hant".into(),
        other => other.into(),
    }
}

/// Codes the trial endpoint accepts at all.
pub fn is_mirai_language(code: &str) -> bool {
    matches!(code, "ja" | "en" | "zh" | "zh-TW")
}

/// Every accepted pair has Japanese on one side; with an unknown source the
/// only safe assumption is a Japanese target.
pub fn mirai_pair_ok(source: &str, target: &str) -> bool {
    if target == "auto" || !is_mirai_language(target) {
        return false;
    }
    if source == "auto" {
        return target == "ja";
    }
    is_mirai_language(source) && (source == "ja" || target == "ja")
}
#[derive(Debug, Serialize)]
struct Payload<'a> {
    input: &'a str,
    source: &'a str,
    target: String,
    tran: String,
    #[serde(rename = "adaptPhrases")]
    adapt_phrases: Vec<serde_json::Value>,
    #[serde(rename = "filter_profile")]
    filter_profile: &'static str,
    profile: &'static str,
    #[serde(rename = "usePrefix")]
    use_prefix: bool,
    zt: bool,
    #[serde(rename = "InmtTarget")]
    inmt_target: &'static str,
    #[serde(rename = "InmtTranslateType")]
    inmt_translate_type: &'static str,
}

/// `outputs[].output[].translation` is the joined result per output block.
#[derive(Debug, Deserialize)]
struct Resp {
    status: Option<String>,
    #[serde(rename = "error_msg", default)]
    error_msg: Option<String>,
    outputs: Option<Vec<serde_json::Value>>,
}

/// Concatenate the nested translation blocks.
pub fn flatten(outputs: &[serde_json::Value]) -> Option<String> {
    let parts: Vec<String> = outputs
        .iter()
        .filter_map(|blk| blk.get("output").and_then(|o| o.as_array()))
        .flat_map(|arr| arr.iter())
        .filter_map(|o| {
            o.get("translation")
                .and_then(|t| t.as_str())
                .filter(|t| !t.is_empty())
                .map(String::from)
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(""))
    }
}

pub struct Mirai {
    client: reqwest::Client,
    user_id: String,
    trans_id: String,
}

impl Mirai {
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
            user_id: Uuid::new_v4().to_string(),
            trans_id: Uuid::new_v4().to_string(),
        }
    }

    /// Load the trial page and read its per-session translate key.
    async fn tran_key(&self) -> Result<String> {
        let resp = self
            .client
            .get(HOST)
            .send()
            .await
            .map_err(|e| Error::Network(format!("mirai: {e}")))?;
        let status = resp.status().as_u16();
        let html = body_text("mirai", resp).await?;
        if status != 200 {
            return Err(map_status("mirai", status, &html));
        }
        extract_tran_key(&html).ok_or_else(|| Error::BadResponse {
            provider: "mirai".into(),
            reason: "var tran key not found in trial page".into(),
        })
    }

    async fn detect(&self, text: &str) -> Option<String> {
        let resp = self
            .client
            .post(DETECT)
            .header("Referer", HOST)
            .json(&serde_json::json!({ "text": text }))
            .send()
            .await
            .ok()?;
        let v: serde_json::Value = resp.json().await.ok()?;
        v.get("language").and_then(|l| l.as_str()).map(String::from)
    }

    /// The web app pings trace.php before translating; mirror it.
    async fn trace(&self, from: &str, text: &str, key: &str) {
        let date = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let _ = self
            .client
            .post(TRACE)
            .header("Referer", "https://miraitranslate.com/")
            .header("Content-Type", "text/plain;charset=UTF-8")
            .json(&serde_json::json!({
                "operationType": "SLA",
                "lang": from,
                "source": text,
                "userId": self.user_id,
                "transId": self.trans_id,
                "uniqueId": key,
                "date": date,
            }))
            .send()
            .await;
    }
}

impl Default for Mirai {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for Mirai {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "mirai".into(),
            display_name: "Mirai Translate (web)".into(),
            kind: ProviderKind::Web,
            priority: 7,
            needs_key: false,
            max_chars: 500,
            auto_detect: true,
            // Mirai only serves Japanese-anchored pairs; declaring `*` made the
            // chain send doomed requests and surface BAD_RESPONSE instead of
            // skipping the provider.
            languages: vec!["ja".into(), "en".into(), "zh".into(), "zh-TW".into()],
            free_tier: Some("trial endpoint, short text".into()),
            configured: true,
        }
    }

    fn supports(&self, target: &str) -> bool {
        is_mirai_language(target)
    }

    fn supports_pair(&self, source: &str, target: &str) -> bool {
        mirai_pair_ok(source, target)
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let to = to_mirai_code(&lang::normalize(&req.target_lang));
        let mut from = lang::normalize(&req.source_lang);

        let key = self.tran_key().await?;

        if from == "auto" {
            from = self.detect(&req.text).await.unwrap_or_else(|| "en".into());
        }
        let from = to_mirai_code(&from);

        self.trace(&from, &req.text, &key).await;

        let payload = Payload {
            input: &req.text,
            source: &from,
            target: to,
            tran: key,
            adapt_phrases: Vec::new(),
            filter_profile: "nmt",
            profile: "inmt",
            use_prefix: false,
            zt: false,
            inmt_target: "",
            inmt_translate_type: "gisting",
        };

        let started = Instant::now();
        let resp = self
            .client
            .post(API)
            .header("Referer", HOST)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Network(format!("mirai: {e}")))?;

        let status = resp.status().as_u16();
        let body = body_text("mirai", resp).await?;
        if status != 200 {
            return Err(map_status("mirai", status, &body));
        }

        let parsed: Resp = serde_json::from_str(&body).map_err(|e| Error::BadResponse {
            provider: "mirai".into(),
            reason: format!("{e}: {}", truncate(&body, 160)),
        })?;

        if parsed.status.as_deref() != Some("success") {
            return Err(Error::Upstream {
                provider: "mirai".into(),
                status,
                body: format!(
                    "mirai status={:?} err={:?}",
                    parsed.status,
                    parsed.error_msg.unwrap_or_default()
                ),
            });
        }

        let outputs =
            parsed
                .outputs
                .filter(|v| !v.is_empty())
                .ok_or_else(|| Error::BadResponse {
                    provider: "mirai".into(),
                    reason: format!("missing outputs: {}", truncate(&body, 160)),
                })?;

        let text = flatten(&outputs).ok_or_else(|| Error::BadResponse {
            provider: "mirai".into(),
            reason: "empty translation".into(),
        })?;

        Ok(TranslateResponse {
            text,
            detected_source: from,
            target_lang: req.target_lang.clone(),
            provider: "mirai".into(),
            engine: "mirai".into(),
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
        let p = Mirai::default();
        let info = p.info();
        assert_eq!(info.name, "mirai");
        assert!(!info.needs_key);
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 7);
        assert!(info.auto_detect);
    }

    #[test]
    fn extracts_tran_key() {
        let html = r#"<script>var tran = "yFl4z4ve5hZgi3GGMcuDhA8Q8ZYFqs";</script>"#;
        assert_eq!(
            extract_tran_key(html).as_deref(),
            Some("yFl4z4ve5hZgi3GGMcuDhA8Q8ZYFqs")
        );
    }

    #[test]
    fn missing_or_empty_key_is_none() {
        assert_eq!(extract_tran_key("<html>no key</html>"), None);
        assert_eq!(extract_tran_key(r#"var tran = """#), None);
    }

    #[test]
    fn maps_chinese_variants() {
        assert_eq!(to_mirai_code("zh"), "zh");
        assert_eq!(to_mirai_code("zh-CN"), "zh");
        assert_eq!(to_mirai_code("zh-TW"), "zh-Hant");
        assert_eq!(to_mirai_code("ja"), "ja");
    }

    #[test]
    fn only_japanese_anchored_pairs_are_offered() {
        assert!(mirai_pair_ok("en", "ja"));
        assert!(mirai_pair_ok("ja", "en"));
        assert!(mirai_pair_ok("zh", "ja"));
        assert!(mirai_pair_ok("ja", "zh-TW"));
        assert!(mirai_pair_ok("auto", "ja"));
        // Pairs the endpoint rejects with an empty body.
        assert!(!mirai_pair_ok("en", "zh"));
        assert!(!mirai_pair_ok("en", "de"));
        assert!(!mirai_pair_ok("ko", "ja"));
        assert!(!mirai_pair_ok("auto", "en"));
    }

    #[test]
    fn parses_success_shape() {
        // Captured live from trial.miraitranslate.com.
        let body = r#"{"status":"success","outputs":[{"output":[{"translation":"こんにちは、これはテストです。","sentences":[{"original":"Hello world","translation":"こんにちは"}]}]}]}"#;
        let p: Resp = serde_json::from_str(body).unwrap();
        assert_eq!(p.status.as_deref(), Some("success"));
        assert_eq!(
            flatten(&p.outputs.unwrap()).as_deref(),
            Some("こんにちは、これはテストです。")
        );
    }

    #[test]
    fn unauthorized_shape_is_detectable() {
        let p: Resp =
            serde_json::from_str(r#"{"status":"failed","error_msg":"unauthorized_access"}"#)
                .unwrap();
        assert_ne!(p.status.as_deref(), Some("success"));
        assert_eq!(p.error_msg.as_deref(), Some("unauthorized_access"));
    }

    #[test]
    fn empty_outputs_flatten_to_none() {
        assert!(flatten(&Vec::new()).is_none());
        let p: Resp =
            serde_json::from_str(r#"{"status":"success","outputs":[{"output":[]}]}"#).unwrap();
        assert!(flatten(&p.outputs.unwrap()).is_none());
    }

    #[test]
    fn trace_timestamp_is_iso8601() {
        let s = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        assert!(s.ends_with('Z'));
        assert_eq!(s.as_bytes()[4], b'-');
    }
}
