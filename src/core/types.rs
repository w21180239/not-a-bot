//! Wire types shared by the CLI and providers. Serde shapes are the public contract.

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

/// Round trip through one or more hops and back to the source language. Each
/// hop forces the text through a different syntax; LLM hops also rewrite the
/// sentence shapes that machine translation keeps.
///
/// Hop selection, first match wins: `chain`, `preset`, `depth`, a non-auto
/// `pivot`, then the configured `roundtrip_preset`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoundtripRequest {
    pub text: String,
    #[serde(default = "default_auto")]
    pub source_lang: String,
    /// Single bridge language. `auto` uses the recommended pair for the source
    /// (English for Chinese, Chinese for everything else).
    #[serde(default = "default_auto")]
    pub pivot: String,
    /// Hop specs `[engine:]lang`, e.g. `["claude-cli:zh", "google:fi", "edge:en"]`.
    /// No engine (or `auto`) uses the fallback race. A hop back to the source
    /// is appended when the last hop is not the source language.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chain: Vec<String>,
    /// Machine-translation hops before returning: 1-3 (zh, ja, fi; en, ja, fi for Chinese).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depth: Option<u8>,
    /// Built-in chain: `mt`, `mt-deep`, `llm`, `lynote`, `lynote-fix`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// Provider for hops without an engine.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallback_chain: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preserve: Vec<String>,
    /// Per-hop timeout; also bounds each scorer call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, serde_json::Value>,
    /// Detector command: reads the candidate on stdin and prints a score in
    /// [0,1] (higher = more AI-like) as its last line. Split into program and
    /// args without a shell. Overrides the configured `roundtrip_scorer`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scorer: Option<String>,
    /// Independent runs of the planned chain (1-5); above 1 needs a scorer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_of: Option<u8>,
    /// Climb the preset ladder until the selected score is at or below this
    /// (0-1). Needs a scorer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_score: Option<f64>,
}

/// One finished run of a round trip, as scored for selection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    /// Preset name or comma-joined chain that produced it.
    pub preset_or_chain: String,
    /// `null` when no scorer ran or the scorer failed.
    pub score: Option<f64>,
    /// Number of facts the candidate lost.
    pub missing_facts: usize,
}

/// One translation step of a round trip.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Hop {
    pub from: String,
    pub to: String,
    pub provider: String,
    /// Output of this hop, kept so a caller can audit it.
    pub text: String,
    pub latency_ms: u64,
    /// Repairs proposed by a `fix` hop and whether each was applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edits: Option<Vec<HopEdit>>,
}

/// What a `fix` hop edit repairs. Fact edits skip the copy-back budget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EditKind {
    Fact,
    #[default]
    Meaning,
}

/// One meaning repair from a `fix` hop: replace `from` (a span of the hop
/// input) with `to`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HopEdit {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub why: String,
    /// `fact` when applying it restored a fact the rewrite had lost.
    #[serde(default)]
    pub kind: EditKind,
    #[serde(default)]
    pub applied: bool,
    /// Why the edit was skipped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RoundtripResponse {
    /// Final text, back in the detected source language.
    pub text: String,
    pub detected_source: String,
    /// Languages visited after the source, in order; ends with the source.
    pub chain: Vec<String>,
    pub hops: Vec<Hop>,
    /// Provider that served the final hop.
    pub provider: String,
    /// Names, numbers, IDs, URLs and code spans from the input that the
    /// output lost. Empty when every checked token survived.
    pub missing_facts: Vec<String>,
    pub latency_ms: u64,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chars: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Detector score of the returned text; `null` without a usable scorer.
    #[serde(default)]
    pub score: Option<f64>,
    /// Scorer program name (no arguments) when one was configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scorer: Option<String>,
    /// Every run, in order, when more than one ran. Texts are not included.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<Candidate>,
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

/// Per-hop record emitted with `--verbose` and in batch logs.
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
    pub priority: u8, // Lower = higher priority (web scraping first)
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
