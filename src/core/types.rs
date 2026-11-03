//! Wire types shared by CLI, MCP and providers. Serde shapes are the public contract.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TranslateRequest {
    pub text: String,
    #[serde(default = "default_auto")]
    pub source_lang: String,
    pub target_lang: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_chain: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preserve: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

fn default_auto() -> String {
    "auto".into()
}

/// Round trip through a pivot language: source -> pivot -> source. The second
/// hop is what strips machine-flavoured phrasing, because the pivot forces the
/// text through a different syntax instead of paraphrasing in place.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoundtripRequest {
    pub text: String,
    #[serde(default = "default_auto")]
    pub source_lang: String,
    /// Bridge language. `auto` uses the recommended pair for the source
    /// (English for Chinese, Chinese for everything else).
    #[serde(default = "default_auto")]
    pub pivot: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_chain: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preserve: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoundtripResponse {
    /// Final text, back in the detected source language.
    pub text: String,
    pub detected_source: String,
    pub pivot_lang: String,
    /// The intermediate translation, kept so a caller can audit the hop.
    pub pivot_text: String,
    /// Provider that served the final hop.
    pub provider: String,
    /// One entry per hop, in order.
    pub providers: Vec<String>,
    pub latency_ms: u64,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chars: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    pub text: String,
    pub detected_source: String,
    pub target_lang: String,
    pub provider: String,
    pub engine: String,
    pub latency_ms: u64,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chars: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Per-hop record emitted with `--verbose` and in MCP logs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttemptRecord {
    pub provider: String,
    pub key_label: Option<String>,
    pub ok: bool,
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub name: String,
    pub display_name: String,
    pub kind: ProviderKind,
    pub priority: u8,  // Lower = higher priority (web scraping first)
    pub needs_key: bool,
    pub max_chars: usize,
    pub auto_detect: bool,
    pub languages: Vec<String>,
    pub free_tier: Option<String>,
    pub configured: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Web,      // Web scraping / reverse-engineered endpoints (PRIMARY)
    Official, // Official APIs (FALLBACK)
    Llm,      // LLM-based translation
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthRecord {
    pub provider: String,
    pub configured: bool,
    pub healthy: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub error: crate::core::error::WireError,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attempts: Vec<AttemptRecord>,
    pub request_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_camel_case_and_defaults() {
        let req: TranslateRequest =
            serde_json::from_str(r#"{"text":"hi","targetLang":"zh"}"#).unwrap();
        assert_eq!(req.source_lang, "auto");
        assert_eq!(req.preserve, Vec::<String>::new());
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("targetLang").is_some());
        assert!(json.get("fallbackChain").is_none());
    }

    #[test]
    fn unknown_field_rejected() {
        let r = serde_json::from_str::<TranslateRequest>(
            r#"{"text":"hi","targetLang":"zh","bogus":1}"#,
        );
        assert!(r.is_err());
    }
}
