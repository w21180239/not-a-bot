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
pub const STYLE_KEY: &str = "not-a-bot.style";
pub const HISTORY_KEY: &str = "not-a-bot.history";
/// Source text of the whole round trip, and its language, for drift repair.
pub const ORIGINAL_KEY: &str = "not-a-bot.original";
pub const ORIGINAL_LANG_KEY: &str = "not-a-bot.original_lang";
/// JSON array of fact tokens of the original that the text to repair lacks (fix hops).
pub const MISSING_KEY: &str = "not-a-bot.missing";
/// JSON array of original words the text to repair no longer has (fix hops): swapped-term candidates.
pub const TERMS_KEY: &str = "not-a-bot.terms";
/// First word of the original when the text to repair no longer opens with it (fix hops).
pub const OPENING_KEY: &str = "not-a-bot.opening";
/// JSON array of spans the output must carry verbatim (`--preserve` matches,
/// sent unmasked because LLMs mangle opaque placeholders).
pub const PRESERVE_KEY: &str = "not-a-bot.preserve";

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

/// Detected register of the original, as `{"register": "casual", "markers": [..]}`
/// (see [`crate::core::register`]).
pub const REGISTER_KEY: &str = "not-a-bot.register";

/// Register class and marker list from [`REGISTER_KEY`]; `neutral` and no
/// markers when absent.
fn register_of(req: &TranslateRequest) -> (&str, Vec<&str>) {
    let Some(v) = req.metadata.get(REGISTER_KEY) else {
        return ("neutral", Vec::new());
    };
    let class = v
        .get("register")
        .and_then(|r| r.as_str())
        .unwrap_or("neutral");
    let markers = v
        .get("markers")
        .and_then(|m| m.as_array())
        .map(|a| a.iter().filter_map(|m| m.as_str()).collect())
        .unwrap_or_default();
    (class, markers)
}

/// Brief for the rewrite hops: keep the source's register and voice.
pub fn register_clause(req: &TranslateRequest) -> String {
    let (class, markers) = register_of(req);
    let seen = markers.join("; ");
    match class {
        "casual" => format!(
            "Register, which outranks the shape rules above (they apply to machine-flavoured phrasing, not to the source's own voice): the source is casual, spoken or dictated text. Markers seen: {seen}. Keep every one of them. Do not capitalise a sentence start the source leaves lowercase, add a full stop the source leaves off, expand contractions, merge fragments, smooth the text into tidy prose, or turn a question or request into an imperative ('can you go through' stays a question). Keep fillers, hedges, self-corrections, false starts, trailing-off pauses and laughter ('hey', 'oh actually no', 'like ... I mean', 'honestly', 'haha') where they are, rendered naturally in {}. Casing and punctuation habits carry over only where the script has them. Add no marker the source lacks.",
            language_label(&req.target_lang)
        ),
        "formal" => format!(
            "Register: the source is formal ({seen}). Keep it formal and precise: complete sentences, its courtesy and terminology, its salutation and sign-off. Add no contractions, slang, fillers or chatty asides."
        ),
        _ => "Register: match the source's register and voice, whatever it is; a technical text stays technical, a formal one formal, a casual one casual. Remove only machine-flavoured phrasing and never tidy, polish, formalise or casualise the rest.".to_string(),
    }
}

/// Hints of the repair brief from the term check and the opening check.
fn fix_hints(req: &TranslateRequest) -> String {
    let mut out = String::new();
    let terms: Vec<&str> = req
        .metadata
        .get(TERMS_KEY)
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    if !terms.is_empty() {
        out.push_str(&format!(
            "- A term check found these words of the ORIGINAL absent from the REWRITE: {}. Where the REWRITE swapped one for a synonym or changed its form (a technical or domain term, a verb, a noun), put the ORIGINAL's word back with a \"kind\": \"term\" edit; skip any that is an ordinary word the REWRITE rephrased with the same meaning.\n",
            terms.join(", ")
        ));
    }
    if let Some(word) = req.metadata.get(OPENING_KEY).and_then(|v| v.as_str()) {
        out.push_str(&format!(
            "- The ORIGINAL opens with \"{word}\" and the REWRITE does not. If that changed the sentence form (a leading verb, a request, a question), restore the opening with a \"kind\": \"term\" edit.\n"
        ));
    }
    out
}

/// Register pass of the repair brief: restore voice markers the rewrite lost.
fn fix_register_clause(req: &TranslateRequest) -> String {
    let (class, markers) = register_of(req);
    let seen = markers.join("; ");
    let drift = match class {
        "casual" => format!("The ORIGINAL is casual, spoken or dictated text (markers: {seen}). The REWRITE may have tidied it: a capitalised sentence start the ORIGINAL keeps lowercase, a full stop the ORIGINAL leaves off, a dropped filler, hedge, self-correction, false start or laugh ('hey', 'oh actually no', 'like ... I mean', 'haha'), expanded contractions, a question or request turned into an imperative."),
        "formal" => format!("The ORIGINAL is formal (markers: {seen}). The REWRITE may have turned chatty: contractions, slang, fillers, fragments, a lost salutation or sign-off."),
        _ => "The REWRITE may have changed the ORIGINAL's register: casual text made polished, formal text made chatty, technical text made loose.".to_string(),
    };
    format!(
        "- Register pass, which is the one exception to \"meaning, not style\": {drift} Where it did, add an edit with \"kind\": \"register\" that puts the voice marker back, taking its wording from the ORIGINAL and leaving the REWRITE's other words alone. A register edit never changes a fact and never brings back a cut feeling or pleasantry sentence. Leave a REWRITE whose voice already matches untouched.\n"
    )
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
        let missing: Vec<&str> = req
            .metadata
            .get(MISSING_KEY)
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let missing = if missing.is_empty() {
            String::new()
        } else {
            format!(
                "- A fact check found these tokens of the ORIGINAL absent from the REWRITE: {}. Restore each exactly as written (numbers as digits, not words); skip one only when it is an ordinary word such as a heading or sentence opener.\n",
                missing.join(", ")
            )
        };
        let register = fix_register_clause(req);
        let hints = fix_hints(req);
        return Ok(format!(
            "Below are an ORIGINAL text and a REWRITE of it in {target}. The rewrite deliberately changed wording and structure and cut sentences that only express feelings, pledges or pleasantries; that is correct, do not undo it. \
             Find every place where the REWRITE's meaning differs from the ORIGINAL: a missing or altered name, place, product, number, date, amount, currency, ID, version, path or code span; a flipped negation; a lost or added qualifier or certainty; a changed actor, cause or order (who did what to whom, and why); a dropped or invented claim; a domain term swapped for a synonym ('attempts' for 'retries', 'acquisition point' for 'collection point', 'commit' for 'commitment'); a changed sentence form (a lost leading verb, a request turned into a command or the reverse, a question turned into a statement); a changed condition, quantifier or modality ('not before', 'within one minute', 'any of the three', 'is used as' turned into 'will be used, maybe'); an invented clause, detail or commitment; a dropped short phrase such as a greeting, thanks or an aside. \
             List the smallest repairs that restore that meaning, as ONLY a JSON array with no prose and no code fence:\n\
             [{{\"from\": \"<exact substring of REWRITE>\", \"to\": \"<replacement>\", \"why\": \"<few words>\"}}]\n\
             Output [] when the meaning already matches.\n\
             Rules:\n\
             - Each \"from\" is the shortest span that contains the error, usually one word or number; a clause only when a whole claim is missing, invented or inverted.\n\
             - Repair meaning, not style: never restore a cut feeling, pledge or pleasantry sentence (a short greeting, thanks or acknowledgement of the original is a phrase to restore, not such a sentence), never move wording or structure back toward the original beyond the term, form or condition that changed, and leave anything already correct in meaning untouched. Delete an invented clause with an empty \"to\". Fewer edits are better.\n\
             - Add \"kind\": \"term\" to an edit of at most 3 words that puts back a swapped term, a changed sentence form, condition, quantifier or modality, or deletes an invented word or clause; put the ORIGINAL's own words in \"to\".\n\
             - Names must match the original letter for letter.\n\
             - When a fact is missing, set \"from\" to the word right before where it belongs and \"to\" to that word plus the fact, written in the REWRITE's style; a fact from a cut sentence goes into the nearest kept sentence.\n\
             {missing}{hints}{register}\
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
            "Translate the text below from {source} into {target}, and while translating rewrite it the way a capable person writes to one reader, not the way an AI writes. Fix the shape, not just the words: state claims directly instead of 'not just X, it's Y' or 'X, not Y' foils nobody raised; drop filler openers ('In today's...', 'When it comes to', 'It's worth noting'), signpost closers and summary lines, run-ups ('Here's the thing'), deep-sounding sayings ('At its core'), inflated significance and promotional adjectives, and reflexive lists of three. Use plain words and varied sentence length. Delete boilerplate sentences that only express feelings or pledges (thanks for patience or hard work, excitement, pride, 'we remain committed', 'we look forward') and closing pleasantries, but keep the source's own short greeting, thanks, apology or acknowledgement ('hey', 'thanks for flagging this early', 'sorry', 'ack'). Before deleting one, move any name, place, number, date or amount in it into a sentence you keep. Hard rules: keep every fact, name, number, date, currency, ID, path, URL and code span exactly as written (copy names letter for letter, do not translate or respell them); keep placeholders and markup untouched; add nothing that is not in the text (no clause, detail, example, reason or commitment of your own); keep the meaning and certainty. Keep every domain term as written and never swap it for a synonym ('attempts' stays 'attempts', 'rollback line' stays 'rollback line', 'page' stays 'page'). Keep conditions, quantifiers, negations, order and modality exactly: 'not before', 'within', 'at least', 'any of', 'will' versus 'may', and add no hedge. Keep the sentence form and mood: a leading verb stays ('Gives the times' is not 'Times are shown'), a request or question stays one, an imperative stays one. Keep the source's punctuation, plain ASCII hyphens and quotes included. Output only the rewritten {target} text, with no preamble or notes."
        ),
        other => {
            return Err(Error::InvalidInput(format!(
                "{STYLE_KEY} must be translate, humanize or fix, got {other}"
            )))
        }
    };
    prompt.push_str(&instructions);
    if style == "humanize" {
        prompt.push_str("\n\n");
        prompt.push_str(&register_clause(req));
    }
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
    fn fix_style_lists_missing_facts_only_when_given() {
        let r = with(req(Some("fix"), None), ORIGINAL_KEY, "Pay $24.95 by 2 May.");
        assert!(!build_prompt(&r).unwrap().contains("A fact check found"));
        let mut r = r;
        r.metadata
            .insert(MISSING_KEY.into(), serde_json::json!(["$24.95", "2"]));
        let p = build_prompt(&r).unwrap();
        assert!(p.contains("absent from the REWRITE: $24.95, 2. Restore each exactly as written"));
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
        assert!(p.contains("who did what to whom"));
        assert!(p.contains("Repair meaning, not style: never restore a cut feeling"));
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

    fn with_register(r: TranslateRequest, class: &str, markers: &[&str]) -> TranslateRequest {
        let mut r = r;
        r.metadata.insert(
            REGISTER_KEY.into(),
            serde_json::json!({"register": class, "markers": markers}),
        );
        r
    }

    #[test]
    fn humanize_carries_the_casual_register_directive_after_the_brief() {
        let r = with_register(
            req(Some("humanize"), None),
            "casual",
            &["lowercase sentence starts", "no final full stops"],
        );
        let p = build_prompt(&r).unwrap();
        let brief = p.find("capable person").unwrap();
        let reg = p
            .find("the source is casual, spoken or dictated text")
            .unwrap();
        assert!(brief < reg && reg < p.find("\n\nText:\n<<<").unwrap());
        assert!(p.contains("Markers seen: lowercase sentence starts; no final full stops."));
        assert!(p.contains("'can you go through' stays a question"));
        assert!(p.contains("Do not capitalise a sentence start the source leaves lowercase"));
    }

    #[test]
    fn humanize_register_directive_follows_the_class() {
        let formal = build_prompt(&with_register(
            req(Some("humanize"), None),
            "formal",
            &["capitalised complete sentences"],
        ))
        .unwrap();
        assert!(formal.contains("the source is formal (capitalised complete sentences)"));
        assert!(!formal.contains("casual, spoken"));
        // No detection result: still told to mirror the source, whatever it is.
        let none = build_prompt(&req(Some("humanize"), None)).unwrap();
        assert!(none.contains("match the source's register and voice, whatever it is"));
        // Plain translation keeps its own brief.
        assert!(!build_prompt(&req(None, None)).unwrap().contains("Register"));
    }

    #[test]
    fn fix_prompt_asks_for_register_edits_against_the_original() {
        let r = with_register(
            with(req(Some("fix"), None), ORIGINAL_KEY, "hey, ask Aroha"),
            "casual",
            &["lowercase sentence starts"],
        );
        let p = build_prompt(&r).unwrap();
        assert!(p.contains("Register pass, which is the one exception to \"meaning, not style\""));
        assert!(p.contains(
            "The ORIGINAL is casual, spoken or dictated text (markers: lowercase sentence starts)"
        ));
        assert!(p.contains("\"kind\": \"register\""));
        assert!(p.find("Register pass").unwrap() < p.find("ORIGINAL (").unwrap());
        // Without a detection result the pass is the generic drift check.
        let p = build_prompt(&with(req(Some("fix"), None), ORIGINAL_KEY, "Ask Aroha.")).unwrap();
        assert!(p.contains("changed the ORIGINAL's register"));
    }

    #[test]
    fn fix_prompt_lists_term_and_opening_hints_only_when_given() {
        let r = with(
            req(Some("fix"), None),
            ORIGINAL_KEY,
            "Gives the times after 6 attempts.",
        );
        let p = build_prompt(&r).unwrap();
        assert!(!p.contains("A term check found") && !p.contains("opens with"));
        let mut r = r;
        r.metadata
            .insert(TERMS_KEY.into(), serde_json::json!(["attempts", "times"]));
        r.metadata
            .insert(OPENING_KEY.into(), serde_json::json!("Gives"));
        let p = build_prompt(&r).unwrap();
        assert!(
            p.contains("absent from the REWRITE: attempts, times. Where the REWRITE swapped one")
        );
        assert!(p.contains("The ORIGINAL opens with \"Gives\" and the REWRITE does not"));
        assert!(p.contains("Add \"kind\": \"term\" to an edit of at most 3 words"));
    }

    #[test]
    fn humanize_brief_forbids_synonym_swaps_and_inventions() {
        let p = build_prompt(&req(Some("humanize"), None)).unwrap();
        assert!(p.contains("never swap it for a synonym"));
        assert!(p.contains("no clause, detail, example, reason or commitment of your own"));
        assert!(p.contains("a leading verb stays"));
        assert!(
            p.contains("keep the source's own short greeting, thanks, apology or acknowledgement")
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
