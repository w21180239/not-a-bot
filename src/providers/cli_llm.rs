//! LLM translation through a local agent CLI: Claude Code (`claude -p`) on the
//! user's subscription, or `opencode run` on their OpenCode quota.
//!
//! Slow and quota-spending, so these never join the `auto` chain; see
//! [`crate::providers::DEFAULT_EXCLUDED`]. They run only when named.

use crate::core::config::ProviderSettings;
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const CLAUDE_CLI: &str = "claude-cli";
pub const OPENCODE_CLI: &str = "opencode-cli";

/// Default per-call budget: an agent CLI boots, loads a model and streams.
pub const DEFAULT_TIMEOUT_MS: u64 = 180_000;

/// Metadata keys read by [`build_prompt`].
pub const STYLE_KEY: &str = "translatex.style";
pub const HISTORY_KEY: &str = "translatex.history";
/// Source text of the whole round trip, and its language, for drift repair.
pub const ORIGINAL_KEY: &str = "translatex.original";
pub const ORIGINAL_LANG_KEY: &str = "translatex.original_lang";
/// JSON array of spans the output must carry verbatim (`--preserve` matches,
/// sent unmasked because LLMs mangle opaque placeholders).
pub const PRESERVE_KEY: &str = "translatex.preserve";

/// Prompt clause listing [`PRESERVE_KEY`] spans; `None` when there are none.
pub fn preserve_clause(req: &TranslateRequest) -> Option<String> {
    let spans: Vec<&str> = req
        .metadata
        .get(PRESERVE_KEY)?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    if spans.is_empty() {
        return None;
    }
    Some(format!(
        "These spans must appear in your output exactly as written, character for character:\n{}",
        spans.join("\n")
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cli {
    Claude,
    OpenCode,
}

pub struct CliLlm {
    cli: Cli,
    bin: Option<String>,
    model: String,
}

impl CliLlm {
    pub fn claude(cfg: &ProviderSettings) -> Self {
        Self {
            cli: Cli::Claude,
            bin: cfg.bin.clone(),
            model: cfg.model.clone().unwrap_or_else(|| "haiku".into()),
        }
    }

    pub fn opencode(cfg: &ProviderSettings) -> Self {
        Self {
            cli: Cli::OpenCode,
            bin: cfg.bin.clone(),
            model: cfg
                .model
                .clone()
                .unwrap_or_else(|| "opencode-go/deepseek-v4.1-flash".into()),
        }
    }

    fn name(&self) -> &'static str {
        match self.cli {
            Cli::Claude => CLAUDE_CLI,
            Cli::OpenCode => OPENCODE_CLI,
        }
    }

    fn program(&self) -> &'static str {
        match self.cli {
            Cli::Claude => "claude",
            Cli::OpenCode => "opencode",
        }
    }

    fn resolve(&self) -> Option<PathBuf> {
        let wanted = self.bin.as_deref().unwrap_or(self.program());
        crate::process::resolve_on_path(wanted)
    }

    fn command(&self, bin: &Path) -> tokio::process::Command {
        let mut cmd = crate::process::command(bin);
        match self.cli {
            Cli::Claude => {
                cmd.args([
                    "-p",
                    "--setting-sources",
                    "",
                    "--model",
                    &self.model,
                    "--tools",
                    "",
                ])
                .env("CLAUDE_CODE_DISABLE_CLAUDE_MDS", "1")
                .env("CLAUDE_CODE_DISABLE_AUTO_MEMORY", "1")
                .env_remove("CLAUDECODE");
            }
            // The message comes from stdin: Windows caps a .cmd command line near 8 KB.
            Cli::OpenCode => {
                cmd.args(["run", "--archived", "-m", &self.model, "--format", "json"]);
            }
        }
        cmd.current_dir(std::env::temp_dir());
        cmd
    }

    async fn run(&self, bin: &Path, prompt: String, timeout_ms: u64) -> Result<String> {
        use crate::process::{run_with_input, RunError};
        let output = match run_with_input(self.command(bin), prompt, timeout_ms).await {
            Ok(out) => out,
            Err(RunError::Spawn(e)) => {
                tracing::warn!("{}: cannot start {}: {e}", self.name(), bin.display());
                return Err(Error::ProviderNotConfigured {
                    name: self.name().into(),
                });
            }
            Err(RunError::Wait(e)) => {
                return Err(Error::BadResponse {
                    provider: self.name().into(),
                    reason: format!("waiting for {}: {e}", bin.display()),
                })
            }
            Err(RunError::Timeout) => return Err(Error::Timeout { timeout_ms }),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let text = match self.cli {
            Cli::Claude => {
                if !output.status.success() {
                    return Err(classify_failure(
                        self.name(),
                        &format!("{stderr}\n{stdout}"),
                    ));
                }
                stdout.trim().to_string()
            }
            Cli::OpenCode => match parse_opencode_events(&stdout) {
                Ok(text) if output.status.success() || !text.is_empty() => text,
                Ok(_) => {
                    return Err(classify_failure(
                        self.name(),
                        &format!("{stderr}\n{stdout}"),
                    ))
                }
                Err(message) => return Err(classify_failure(self.name(), &message)),
            },
        };
        let text = strip_meta_line(&text);
        if text.is_empty() {
            return Err(Error::BadResponse {
                provider: self.name().into(),
                reason: format!("empty output{}", excerpt_suffix(&stderr)),
            });
        }
        Ok(text)
    }
}

#[async_trait]
impl Provider for CliLlm {
    fn info(&self) -> ProviderInfo {
        let (display, priority, free_tier) = match self.cli {
            Cli::Claude => ("Claude Code CLI", 60, "uses your Claude subscription"),
            Cli::OpenCode => ("OpenCode CLI", 61, "uses your OpenCode quota"),
        };
        ProviderInfo {
            name: self.name().into(),
            display_name: display.into(),
            kind: ProviderKind::Llm,
            priority,
            needs_key: false,
            max_chars: 20_000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some(free_tier.into()),
            configured: self.resolve().is_some(),
        }
    }

    fn default_timeout_ms(&self) -> Option<u64> {
        Some(DEFAULT_TIMEOUT_MS)
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        let bin = self.resolve().ok_or_else(|| Error::ProviderNotConfigured {
            name: self.name().into(),
        })?;
        let prompt = build_prompt(req)?;
        let started = Instant::now();
        let text = self
            .run(&bin, prompt, req.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS))
            .await?;
        Ok(TranslateResponse {
            text,
            detected_source: req.source_lang.clone(),
            target_lang: req.target_lang.clone(),
            provider: self.name().into(),
            engine: self.model.clone(),
            latency_ms: started.elapsed().as_millis() as u64,
            request_id: String::new(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }

    /// Never spends quota: a resolvable binary is the whole check.
    async fn probe(&self, _cred: &Credential) -> Result<()> {
        self.resolve()
            .map(|_| ())
            .ok_or_else(|| Error::ProviderNotConfigured {
                name: self.name().into(),
            })
    }
}

/// The full prompt sent on stdin. Pure, so tests need no process.
pub fn build_prompt(req: &TranslateRequest) -> Result<String> {
    let source = match req.source_lang.as_str() {
        "auto" => "its original language".to_string(),
        code => language_label(code),
    };
    let target = language_label(&req.target_lang);
    let style = req
        .metadata
        .get(STYLE_KEY)
        .and_then(|v| v.as_str())
        .unwrap_or("translate");
    let original = req.metadata.get(ORIGINAL_KEY).and_then(|v| v.as_str());
    let orig_lang = match req.metadata.get(ORIGINAL_LANG_KEY).and_then(|v| v.as_str()) {
        Some(code) if code != "auto" => language_label(code),
        _ => "its original language".to_string(),
    };

    if style == "fix" {
        let original = original
            .ok_or_else(|| Error::InvalidInput(format!("{STYLE_KEY} fix needs {ORIGINAL_KEY}")))?;
        return Ok(format!(
            "Below are an ORIGINAL text and a REWRITE of it in {target}. The rewrite may have lost, changed or added meaning. \
             List the repairs that restore the original meaning, as ONLY a JSON array with no prose and no code fence:\n\
             [{{\"from\": \"<exact substring of REWRITE>\", \"to\": \"<replacement>\", \"why\": \"<few words>\"}}]\n\
             Output [] when the meaning already matches.\n\
             Rules:\n\
             - Each \"from\" is the shortest span that contains the error, usually one word or number; a whole clause only when the clause itself is missing, invented or inverted.\n\
             - Never use an edit to change wording, style or structure that is already correct in meaning.\n\
             - Names must match the original letter for letter; check numbers, dates, currencies, IDs, code spans, negations and qualifiers.\n\
             - When a whole clause is missing, set \"from\" to the word right before where it belongs and \"to\" to that word plus the inserted clause, written in the REWRITE's style.\n\
             ORIGINAL ({orig_lang}):\n<<<\n{original}\n>>>\nREWRITE ({target}):\n<<<\n{}\n>>>\n",
            req.text
        ));
    }

    let mut prompt = String::new();
    if let Some(history) = req.metadata.get(HISTORY_KEY) {
        let field = |key: &str| {
            history
                .get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::InvalidInput(format!("{HISTORY_KEY}.{key} must be a string")))
        };
        prompt.push_str(&format!(
            "The previous step turned this input:\n<<<\n{}\n>>>\ninto:\n<<<\n{}\n>>>\nKeep its choices; do not undo them.\n\n",
            field("input")?,
            field("output")?
        ));
    }
    if let Some(original) = original {
        prompt.push_str(&format!(
            "The text you receive may have passed through machine translation and drifted from the original. Original (in {orig_lang}):
<<<
{original}
>>>
First compare the text with the original. Wherever meaning was lost, changed or added (names spelled differently, numbers, dates, currencies, negations, qualifiers, who did what to whom, a dropped clause), fix it with the smallest possible change. Do not copy the original's phrasing or structure back in; only repair meaning. Then do the task below on the repaired text.

"
        ));
    }
    let instructions = match style {
        "translate" => format!(
            "Translate the text below from {source} into {target}. Keep the meaning, tone and \
             formatting; keep every name, number, URL, code span, placeholder and markup exactly \
             as written. Output only the {target} translation, with no preamble or notes."
        ),
        "humanize" => format!(
            "Translate the text below from {source} into {target}, and while translating rewrite it the way a capable person writes to one reader, not the way an AI writes. Fix the shape, not just the words: state claims directly instead of 'not just X, it's Y' or 'X, not Y' foils nobody raised; drop filler openers ('In today's...', 'When it comes to', 'It's worth noting'), signpost closers and summary lines, run-ups ('Here's the thing'), deep-sounding sayings ('At its core'), inflated significance and promotional adjectives, and reflexive lists of three. Use plain words and varied sentence length. Delete sentences that only express feelings or pledges (thanks for patience or hard work, excitement, pride, 'we remain committed', 'we look forward') and closing pleasantries. Before deleting one, move any name, place, number, date or amount in it into a sentence you keep. Hard rules: keep every fact, name, number, date, currency, ID, path, URL and code span exactly as written (copy names letter for letter, do not translate or respell them); keep placeholders and markup untouched; add nothing that is not in the text; keep the meaning and certainty. Output only the rewritten {target} text, with no preamble or notes."
        ),
        other => {
            return Err(Error::InvalidInput(format!(
                "{STYLE_KEY} must be translate, humanize or fix, got {other}"
            )))
        }
    };
    prompt.push_str(&instructions);
    if let Some(clause) = preserve_clause(req) {
        prompt.push_str("\n\n");
        prompt.push_str(&clause);
    }
    prompt.push_str("\n\nText:\n<<<\n");
    prompt.push_str(&req.text);
    prompt.push_str("\n>>>\n");
    Ok(prompt)
}

fn language_label(code: &str) -> String {
    match crate::core::lang::name_of(code) {
        Some(name) => format!("{name} ({code})"),
        None => code.to_string(),
    }
}

/// Drop a leading "Here is the translation:" style line the model adds anyway.
pub fn strip_meta_line(text: &str) -> String {
    let text = text.trim();
    let (first, rest) = match text.split_once('\n') {
        Some((first, rest)) => (first.trim(), rest),
        None => return text.to_string(),
    };
    let lower = first.to_lowercase();
    let english = [
        "here is",
        "here's",
        "sure",
        "certainly",
        "translation:",
        "rewritten",
    ];
    let meta = (first.ends_with(':') || first.ends_with('：'))
        && (english.iter().any(|p| lower.starts_with(p))
            || first.starts_with("以下是")
            || first.starts_with("以下は")
            || lower.contains("translation")
            || first.contains("翻译")
            || first.contains("翻訳"));
    if meta {
        rest.trim().to_string()
    } else {
        text.to_string()
    }
}

/// Assistant text from `opencode run --format json` (one JSON event per line).
/// `Err` carries the message of an `error` event.
pub fn parse_opencode_events(stdout: &str) -> std::result::Result<String, String> {
    let mut text = String::new();
    for line in stdout.lines() {
        let Ok(event) = serde_json::from_str::<serde_json::Value>(line.trim()) else {
            continue;
        };
        match event.get("type").and_then(|t| t.as_str()) {
            Some("text") => {
                if let Some(part) = event.pointer("/part/text").and_then(|t| t.as_str()) {
                    text.push_str(part);
                }
            }
            Some("error") => {
                let message = event
                    .pointer("/error/message")
                    .or_else(|| event.pointer("/error/data/message"))
                    .and_then(|m| m.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| event["error"].to_string());
                return Err(message);
            }
            _ => {}
        }
    }
    Ok(text.trim().to_string())
}

fn classify_failure(provider: &str, output: &str) -> Error {
    let lower = output.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| lower.contains(w));
    if has(&[
        "quota",
        "usage limit",
        "credit balance",
        "out of credits",
        "limit reached",
    ]) {
        Error::QuotaExhausted {
            provider: provider.into(),
            retry_after_secs: None,
        }
    } else if has(&[
        "rate limit",
        "rate-limit",
        "too many requests",
        "429",
        "overloaded",
    ]) {
        Error::RateLimited {
            provider: provider.into(),
            retry_after_secs: None,
        }
    } else {
        Error::BadResponse {
            provider: provider.into(),
            reason: format!("non-zero exit{}", excerpt_suffix(output)),
        }
    }
}

fn excerpt_suffix(output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let excerpt: String = trimmed.chars().take(300).collect();
    format!(": {excerpt}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn req(style: Option<&str>, history: Option<serde_json::Value>) -> TranslateRequest {
        let mut metadata = BTreeMap::new();
        if let Some(style) = style {
            metadata.insert(STYLE_KEY.to_string(), serde_json::json!(style));
        }
        if let Some(history) = history {
            metadata.insert(HISTORY_KEY.to_string(), history);
        }
        TranslateRequest {
            text: "Aroha owes NZD 18,900.".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata,
        }
    }

    #[test]
    fn translate_style_is_the_default() {
        let p = build_prompt(&req(None, None)).unwrap();
        assert!(p.starts_with("Translate the text below from English (en) into Chinese"));
        assert!(p.contains("Output only the"));
        assert!(!p.contains("capable person"));
        assert!(p.ends_with("<<<\nAroha owes NZD 18,900.\n>>>\n"));
    }

    #[test]
    fn humanize_style_uses_the_rewrite_brief() {
        let p = build_prompt(&req(Some("humanize"), None)).unwrap();
        assert!(p.contains("the way a capable person writes to one reader"));
        assert!(p.contains("copy names letter for letter"));
        assert!(p.contains("only express feelings or pledges"));
        assert!(p.contains("move any name, place, number, date or amount"));
        assert!(!p.contains("{source}") && !p.contains("{target}"));
        assert!(!p.contains("previous step"));
    }

    #[test]
    fn preserve_spans_are_listed_before_the_text() {
        let mut r = req(Some("humanize"), None);
        r.text = "Run `make test` now.".into();
        r.metadata.insert(
            PRESERVE_KEY.into(),
            serde_json::json!(["`make test`", "https://x.io"]),
        );
        let p = build_prompt(&r).unwrap();
        assert!(p.contains(
            "These spans must appear in your output exactly as written, character for character:\n`make test`\nhttps://x.io\n\nText:\n<<<\nRun `make test` now.\n>>>\n"
        ));
        assert!(!build_prompt(&req(Some("humanize"), None))
            .unwrap()
            .contains("These spans"));
    }

    #[test]
    fn history_is_prepended() {
        let history = serde_json::json!({"input": "in-text", "output": "out-text"});
        let p = build_prompt(&req(Some("humanize"), Some(history))).unwrap();
        assert!(p.starts_with(
            "The previous step turned this input:\n<<<\nin-text\n>>>\ninto:\n<<<\nout-text\n>>>\nKeep its choices; do not undo them."
        ));
        assert!(p.contains("capable person"));
    }

    #[test]
    fn bad_style_or_history_is_invalid_input() {
        assert_eq!(
            build_prompt(&req(Some("poem"), None)).unwrap_err().code(),
            "INVALID_INPUT"
        );
        let history = serde_json::json!({"input": 1});
        assert_eq!(
            build_prompt(&req(None, Some(history))).unwrap_err().code(),
            "INVALID_INPUT"
        );
    }

    fn with(mut r: TranslateRequest, key: &str, value: &str) -> TranslateRequest {
        r.metadata.insert(key.to_string(), serde_json::json!(value));
        r
    }

    #[test]
    fn original_is_added_before_the_instructions() {
        let r = with(req(Some("humanize"), None), ORIGINAL_KEY, "Ask Aroha.");
        let r = with(r, ORIGINAL_LANG_KEY, "en");
        let p = build_prompt(&r).unwrap();
        let orig = p
            .find(
                "drifted from the original. Original (in English (en)):
<<<
Ask Aroha.
>>>",
            )
            .expect("original block");
        assert!(orig < p.find("capable person").unwrap());
        assert!(p.contains("Then do the task below on the repaired text."));
        assert!(!build_prompt(&req(Some("humanize"), None))
            .unwrap()
            .contains("drifted"));
    }

    #[test]
    fn fix_style_compares_rewrite_with_original() {
        let r = with(req(Some("fix"), None), ORIGINAL_KEY, "Ask Aroha.");
        let mut r = with(r, ORIGINAL_LANG_KEY, "en");
        r.source_lang = "auto".into();
        r.target_lang = "en".into();
        let p = build_prompt(&r).unwrap();
        assert!(p.starts_with("Below are an ORIGINAL text and a REWRITE of it in English (en)."));
        assert!(p.contains("as ONLY a JSON array") && p.contains("Output [] when"));
        assert!(p.contains(r#"[{"from": "<exact substring of REWRITE>""#));
        assert!(p.contains(
            "ORIGINAL (English (en)):
<<<
Ask Aroha.
>>>
REWRITE (English (en)):
<<<
Aroha owes NZD 18,900.
>>>"
        ));
        assert_eq!(
            build_prompt(&req(Some("fix"), None)).unwrap_err().code(),
            "INVALID_INPUT",
            "fix without the original"
        );
    }

    #[test]
    fn auto_source_is_described() {
        let mut r = req(None, None);
        r.source_lang = "auto".into();
        assert!(build_prompt(&r)
            .unwrap()
            .contains("from its original language"));
    }

    #[test]
    fn meta_line_is_stripped() {
        assert_eq!(strip_meta_line("Here is the translation:\n你好"), "你好");
        assert_eq!(strip_meta_line("以下是翻译：\n你好"), "你好");
        assert_eq!(strip_meta_line("你好"), "你好");
        assert_eq!(
            strip_meta_line("Steps:\n1. one"),
            "Steps:\n1. one",
            "only meta openers go"
        );
    }

    #[test]
    fn opencode_events_yield_text_or_error() {
        let ok = r#"{"type":"step_start","part":{"type":"step-start"}}
{"type":"text","part":{"type":"text","text":"你好"}}
{"type":"step_finish","part":{"type":"step-finish","reason":"stop"}}"#;
        assert_eq!(parse_opencode_events(ok).unwrap(), "你好");
        let err = r#"{"type":"error","error":{"type":"provider.no-route","message":"Model unavailable: x"}}"#;
        assert_eq!(
            parse_opencode_events(err).unwrap_err(),
            "Model unavailable: x"
        );
    }

    #[test]
    fn failures_are_classified() {
        assert_eq!(
            classify_failure("c", "Usage limit reached").code(),
            "QUOTA_EXHAUSTED"
        );
        assert_eq!(
            classify_failure("c", "429 Too Many Requests").code(),
            "RATE_LIMITED"
        );
        let other = classify_failure("c", "boom");
        assert_eq!(other.code(), "BAD_RESPONSE");
        assert!(other.to_string().contains("boom"));
    }

    #[test]
    fn info_is_keyless_llm() {
        let p = CliLlm::claude(&Default::default());
        let info = p.info();
        assert_eq!(info.name, CLAUDE_CLI);
        assert_eq!(info.kind, ProviderKind::Llm);
        assert!(!info.needs_key);
        assert_eq!(
            CliLlm::opencode(&Default::default()).info().name,
            OPENCODE_CLI
        );
    }
}
