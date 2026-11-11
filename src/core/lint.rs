//! Offline lint for AI tells and fact drift (port of the humanizer `lint.py`).
//!
//! Flags vocabulary, phrase, structure, spelling and punctuation tells in a
//! draft. With a source text it also lists the fact tokens (code spans, URLs,
//! paths, ticket IDs, mentions, versions, numbers, names) the draft dropped or added.
//! Hits are prompts to look, not errors: no network, no model.

use super::lint_data::{PHRASES, STAGED, VOCAB};
use once_cell::sync::Lazy;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const US_OK: &[&str] = &[
    "size", "sized", "sizes", "sizing", "seize", "seized", "prize", "capsize",
];

const NAME_STOP: &[&str] = &[
    "I",
    "The",
    "This",
    "That",
    "It",
    "We",
    "You",
    "They",
    "He",
    "She",
    "Hi",
    "Hey",
    "Thanks",
    "Cheers",
    "Yes",
    "No",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// Tell kind for a script the source lacks; reported but not counted as a hit.
const INTRODUCED_SCRIPT: &str = "introduced-script";

/// One flagged spot. `line` is 1-based; 0 means the whole text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tell {
    /// Category: `vocab`, a phrase family, `staged-contrast`, `us-spelling`,
    /// `bold-label-bullet`, `horizontal-rule`, `repeated-openings`,
    /// `punctuation` or `introduced-script`.
    pub kind: String,
    pub line: usize,
    #[serde(rename = "match")]
    pub matched: String,
    pub hint: String,
}

/// JSON output of `lint`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LintResponse {
    pub tells: Vec<Tell>,
    /// `kind:value` fact tokens present in the source but not in the draft.
    pub missing_from_draft: Vec<String>,
    /// `kind:value` fact tokens in the draft but not in the source.
    pub new_in_draft: Vec<String>,
}

/// Structured result plus the summary lines the text format prints.
#[derive(Debug, Clone)]
pub struct Report {
    pub response: LintResponse,
    /// `-- source ...` line, present when a source was given.
    pub source_summary: Option<String>,
    /// `-- N words, ...` line.
    pub footer: String,
}

impl Report {
    /// True when `--strict` should fail: any tell or any dropped fact.
    pub fn is_dirty(&self) -> bool {
        !self.response.tells.is_empty() || !self.response.missing_from_draft.is_empty()
    }

    /// Same line layout as `lint.py`.
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        for t in self
            .response
            .tells
            .iter()
            .filter(|t| t.kind != INTRODUCED_SCRIPT)
        {
            if t.kind == "vocab" {
                out.push_str(&format!(
                    "{}: {}: {} -> {}\n",
                    t.line, t.kind, t.matched, t.hint
                ));
            } else {
                out.push_str(&format!("{}: {}: {}\n", t.line, t.kind, t.matched));
            }
        }
        if let Some(summary) = &self.source_summary {
            for m in &self.response.missing_from_draft {
                out.push_str(&format!("missing-from-draft: {m}\n"));
            }
            for a in &self.response.new_in_draft {
                out.push_str(&format!("new-in-draft: {a}\n"));
            }
            for t in self
                .response
                .tells
                .iter()
                .filter(|t| t.kind == INTRODUCED_SCRIPT)
            {
                out.push_str(&format!("{}: {}\n", t.kind, t.matched));
            }
            out.push_str(summary);
            out.push('\n');
        }
        out.push_str(&self.footer);
        out.push('\n');
        out
    }
}

fn rx(p: &str) -> Regex {
    Regex::new(p).expect("static lint regex")
}

static VOCAB_RES: Lazy<Vec<Regex>> = Lazy::new(|| {
    VOCAB
        .iter()
        .map(|(w, _)| rx(&format!(r"\b{}\b", regex::escape(w))))
        .collect()
});
static STAGED_RES: Lazy<Vec<Regex>> = Lazy::new(|| STAGED.iter().map(|p| rx(p)).collect());
static US_IZE: Lazy<Regex> = Lazy::new(|| rx(r"\b\w{3,}iz(?:e|es|ed|ing|ation)\b"));
static US_OR: Lazy<Regex> = Lazy::new(|| rx(r"\b(?:colou?r|behaviou?r|favou?r|honou?r)\b"));
static US_CENTER: Lazy<Regex> = Lazy::new(|| rx(r"\bcenter(?:s|ed)?\b"));
static US_DEFENSE: Lazy<Regex> = Lazy::new(|| rx(r"\bdefense\b"));
static BOLD_BULLET: Lazy<Regex> = Lazy::new(|| rx(r"^\s*(?:[-*]|\d+\.)\s+\*\*[^*]+\*\*:?"));
static HRULE: Lazy<Regex> = Lazy::new(|| rx(r"^\s*(?:-{3,}|\*{3,}|_{3,})\s*$"));
static BULLET: Lazy<Regex> = Lazy::new(|| rx(r"^\s*(?:[-*]|\d+\.)\s"));
static EN_DASH: Lazy<Regex> = Lazy::new(|| rx("\\s\u{2013}\\s"));
static DOUBLE_HYPHEN: Lazy<Regex> = Lazy::new(|| rx(r"\s--\s"));
static CURLY: Lazy<Regex> = Lazy::new(|| rx("[\u{201c}\u{201d}]"));
static BOLD: Lazy<Regex> = Lazy::new(|| rx(r"\*\*[^*]+\*\*"));
static WORD: Lazy<Regex> = Lazy::new(|| rx(r"\w+"));
static CJK: Lazy<Regex> = Lazy::new(|| rx("[\u{3040}-\u{30ff}\u{3400}-\u{4dbf}\u{4e00}-\u{9fff}]"));

// Fact token patterns. Path and name need lookbehind, so they are scanned by hand.
static FACT_CODE: Lazy<Regex> = Lazy::new(|| rx(r"`[^`\n]+`"));
static FACT_URL: Lazy<Regex> = Lazy::new(|| rx(r"https?://\S+[^\s.,;:)]"));
static FACT_TICKET: Lazy<Regex> = Lazy::new(|| rx(r"\b[A-Z][A-Z0-9]{1,9}-\d+\b"));
static FACT_MENTION: Lazy<Regex> = Lazy::new(|| rx(r"@[\w.-]+"));
static FACT_VERSION: Lazy<Regex> =
    Lazy::new(|| rx(r"(?i)\b(?:v\d+(?:\.\d+)*|\d+(?:\.\d+){2,})(?:-[a-z0-9]+(?:\.[a-z0-9]+)*)?\b"));
static FACT_NUMBER: Lazy<Regex> =
    Lazy::new(|| rx(r"(?i)\b\d[\d,.:]*(?:%|am|pm|ms|s|k|m|gb|mb)?\b"));
static PATH_DIR: Lazy<Regex> = Lazy::new(|| rx(r"(?:[\w.-]+/)+[\w.-]+"));
static PATH_FILE: Lazy<Regex> = Lazy::new(|| {
    rx(r"\b[\w-]+\.(?:php|py|js|ts|go|rs|sql|md|json|ya?ml|toml|sh|ps1|txt|csv|log)\b")
});
static NAME_CORE: Lazy<Regex> = Lazy::new(|| rx(r"\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+)*\b"));

type Span = (usize, usize);

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Python `str.splitlines()` line boundaries.
fn splitlines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if matches!(
            c,
            '\n' | '\r' | '\x0b' | '\x0c' | '\x1c'..='\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}'
        ) {
            out.push(&text[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' && matches!(it.peek(), Some(&(_, '\n'))) {
                it.next();
                end += 1;
            }
            start = end;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

fn words(text: &str) -> Vec<String> {
    WORD.find_iter(&text.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect()
}

fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// Split at whitespace runs that follow sentence-final punctuation.
fn split_sentences(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<char> = None;
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        let after_stop = matches!(
            prev,
            Some('.' | '!' | '?' | '\u{3002}' | '\u{ff01}' | '\u{ff1f}')
        );
        if py_space(c) && after_stop {
            let mut end = i + c.len_utf8();
            while let Some(&(j, d)) = it.peek() {
                if !py_space(d) {
                    break;
                }
                end = j + d.len_utf8();
                it.next();
            }
            out.push(&text[start..i]);
            start = end;
            prev = text[..end].chars().next_back();
        } else {
            prev = Some(c);
        }
    }
    out.push(&text[start..]);
    out
}

fn sentences(text: &str) -> Vec<&str> {
    split_sentences(text)
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect()
}

fn repeated_openings(text: &str) -> Vec<String> {
    let mut hits = Vec::new();
    let sents: Vec<&str> = sentences(text).into_iter().map(str::trim).collect();
    let mut run = 1;
    for i in 1..sents.len() {
        let a = words(sents[i - 1]).into_iter().next();
        let b = words(sents[i]).into_iter().next();
        run = if a.is_some() && a == b { run + 1 } else { 1 };
        if run == 3 {
            hits.push(format!(
                "3+ sentences in a row start with '{}'",
                b.unwrap_or_default()
            ));
        }
    }
    hits
}

fn tell(kind: &str, line: usize, matched: impl Into<String>, hint: impl Into<String>) -> Tell {
    Tell {
        kind: kind.into(),
        line,
        matched: matched.into(),
        hint: hint.into(),
    }
}

fn line_tells(n: usize, line: &str, out: &mut Vec<Tell>) {
    let low = line.to_lowercase().replace('\u{2019}', "'");
    for (re, (w, rep)) in VOCAB_RES.iter().zip(VOCAB) {
        if re.is_match(&low) {
            let hint = if rep.is_empty() {
                "cut, state the point"
            } else {
                rep
            };
            out.push(tell("vocab", n, *w, hint));
        }
    }
    for (p, fam) in PHRASES {
        if low.contains(p) {
            out.push(tell(
                fam,
                n,
                *p,
                format!("pattern family in patterns.md: {fam}"),
            ));
        }
    }
    for re in STAGED_RES.iter() {
        if let Some(m) = re.find(&low) {
            out.push(tell(
                "staged-contrast",
                n,
                m.as_str(),
                "staged contrast: state the point directly",
            ));
        }
    }
    let us_hint = "US spelling: use NZ/British";
    for m in US_IZE.find_iter(&low) {
        if !US_OK.contains(&m.as_str()) {
            out.push(tell("us-spelling", n, m.as_str(), us_hint));
        }
    }
    for m in US_OR.find_iter(&low) {
        if !m.as_str().contains("ou") {
            out.push(tell("us-spelling", n, m.as_str(), us_hint));
        }
    }
    for re in [&*US_CENTER, &*US_DEFENSE] {
        if let Some(m) = re.find(&low) {
            out.push(tell("us-spelling", n, m.as_str(), us_hint));
        }
    }
    if BOLD_BULLET.is_match(line) {
        let head: String = line.trim().chars().take(60).collect();
        out.push(tell(
            "bold-label-bullet",
            n,
            head,
            "bold-label bullet: write a sentence",
        ));
    }
    if HRULE.is_match(line) {
        out.push(tell(
            "horizontal-rule",
            n,
            line.trim(),
            "horizontal rule: drop it",
        ));
    }
    if line.contains('\u{2014}') {
        out.push(tell(
            "punctuation",
            n,
            "\u{2014}",
            "em dash: use a comma, colon or full stop",
        ));
    }
    if EN_DASH.is_match(line) {
        out.push(tell(
            "punctuation",
            n,
            "\u{2013}",
            "spaced en dash: use a comma, colon or full stop",
        ));
    }
    if DOUBLE_HYPHEN.is_match(line) {
        out.push(tell(
            "punctuation",
            n,
            "--",
            "spaced double hyphen: use a comma, colon or full stop",
        ));
    }
    if let Some(m) = CURLY.find(line) {
        out.push(tell(
            "punctuation",
            n,
            m.as_str(),
            "curly double quote: use straight quotes",
        ));
    }
}

/// Next match of `re` at or after `pos` whose start passes `ok(text, start)`.
fn next_valid(
    re: &Regex,
    text: &str,
    mut pos: usize,
    ok: impl Fn(&str, usize) -> bool,
) -> Option<Span> {
    while pos <= text.len() {
        let m = re.find_at(text, pos)?;
        if ok(text, m.start()) {
            return Some((m.start(), m.end()));
        }
        pos = m.start() + text[m.start()..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

fn path_start_ok(text: &str, start: usize) -> bool {
    !matches!(text[..start].chars().next_back(), Some(c) if is_word(c) || c == '@')
}

/// Not at a line start and not right after sentence-final punctuation.
fn name_start_ok(text: &str, start: usize) -> bool {
    let mut back = text[..start].chars().rev();
    match back.next() {
        None | Some('\n') => false,
        Some(c) if c.is_whitespace() => !matches!(back.next(), Some('.' | '!' | '?')),
        Some(_) => true,
    }
}

/// Paths and file names: `a/b` runs not glued to a word or `@`, or `name.ext`.
fn path_spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut pos = 0;
    while pos <= text.len() {
        let dir = next_valid(&PATH_DIR, text, pos, path_start_ok);
        let file = PATH_FILE.find_at(text, pos).map(|m| (m.start(), m.end()));
        let pick = match (dir, file) {
            (Some(d), Some(f)) => Some(if f.0 < d.0 { f } else { d }),
            (d, f) => d.or(f),
        };
        match pick {
            Some(s) => {
                out.push(s);
                pos = s.1;
            }
            None => break,
        }
    }
    out
}

fn name_spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(s) = next_valid(&NAME_CORE, text, pos, name_start_ok) {
        out.push(s);
        pos = s.1;
    }
    out
}

fn regex_spans(re: &Regex, text: &str) -> Vec<Span> {
    re.find_iter(text).map(|m| (m.start(), m.end())).collect()
}

fn blank_spans(text: &str, spans: &[Span]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for &(s, e) in spans {
        out.push_str(&text[last..s]);
        out.push(' ');
        last = e;
    }
    out.push_str(&text[last..]);
    out
}

fn normalise(m: &str) -> String {
    m.trim_matches('`')
        .trim_end_matches(['.', ',', ';', ':'])
        .to_string()
}

/// Tokens a rewrite must carry over exactly, as ordered unique `(kind, value)`.
/// Each kind is blanked out before the next so a code span's digits do not
/// count again as numbers.
fn fact_tokens(text: &str) -> Vec<(&'static str, String)> {
    let mut found: Vec<(&'static str, String)> = Vec::new();
    let mut seen: HashSet<(&'static str, String)> = HashSet::new();
    let mut rest = text.to_string();
    for kind in [
        "code", "url", "path", "ticket", "mention", "version", "number",
    ] {
        let spans = match kind {
            "code" => regex_spans(&FACT_CODE, &rest),
            "url" => regex_spans(&FACT_URL, &rest),
            "path" => path_spans(&rest),
            "ticket" => regex_spans(&FACT_TICKET, &rest),
            "mention" => regex_spans(&FACT_MENTION, &rest),
            "version" => regex_spans(&FACT_VERSION, &rest),
            _ => regex_spans(&FACT_NUMBER, &rest),
        };
        for &(s, e) in &spans {
            let v = normalise(&rest[s..e]);
            if seen.insert((kind, v.clone())) {
                found.push((kind, v));
            }
        }
        rest = blank_spans(&rest, &spans);
    }
    for (s, e) in name_spans(&rest) {
        let m = &rest[s..e];
        if !NAME_STOP.contains(&m.split_whitespace().next().unwrap_or("")) {
            let v = m.to_string();
            if seen.insert(("name", v.clone())) {
                found.push(("name", v));
            }
        }
    }
    found
}

struct Diff {
    missing: Vec<String>,
    added: Vec<String>,
    introduced_script: bool,
    summary: String,
}

fn counts(ws: &[String]) -> HashMap<&str, usize> {
    let mut m: HashMap<&str, usize> = HashMap::new();
    for w in ws {
        *m.entry(w).or_default() += 1;
    }
    m
}

fn source_diff(src: &str, draft: &str) -> Diff {
    let d_low = draft.to_lowercase();
    let s_low = src.to_lowercase();
    let missing = fact_tokens(src)
        .iter()
        .filter(|(_, v)| !d_low.contains(&v.to_lowercase()))
        .map(|(k, v)| format!("{k}:{v}"))
        .collect();
    let added = fact_tokens(draft)
        .iter()
        .filter(|(k, v)| *k != "name" && !s_low.contains(&v.to_lowercase()))
        .map(|(k, v)| format!("{k}:{v}"))
        .collect();
    let sw = words(src);
    let dw = words(draft);
    let dc = counts(&dw);
    let shared: usize = counts(&sw)
        .iter()
        .map(|(t, c)| (*c).min(dc.get(t).copied().unwrap_or(0)))
        .sum();
    let denom = sw.len().max(1) as f64;
    let summary = format!(
        "-- source {} words / {} sentences -> draft {} words / {} sentences; length ratio {:.2}; word recall {:.2}",
        sw.len(),
        sentences(src.trim()).len(),
        dw.len(),
        sentences(draft.trim()).len(),
        dw.len() as f64 / denom,
        shared as f64 / denom
    );
    Diff {
        missing,
        added,
        introduced_script: CJK.is_match(draft) && !CJK.is_match(src),
        summary,
    }
}

/// Lint `draft`; with `source`, also diff the fact tokens.
pub fn lint(draft: &str, source: Option<&str>) -> Report {
    let lines = splitlines(draft);
    let mut tells = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        line_tells(i + 1, line, &mut tells);
    }
    for h in repeated_openings(draft) {
        tells.push(tell("repeated-openings", 0, h, "vary how sentences open"));
    }
    let count = |re: &Regex| re.find_iter(draft).count();
    let footer = format!(
        "-- {} words, {} em / {} en / {} double-hyphen dashes, {} curly double quotes, {} bold spans, {} bullet lines, {} hits",
        count(&WORD),
        draft.matches('\u{2014}').count(),
        count(&EN_DASH),
        count(&DOUBLE_HYPHEN),
        count(&CURLY),
        count(&BOLD),
        lines.iter().filter(|l| BULLET.is_match(l)).count(),
        tells.len()
    );
    let mut response = LintResponse {
        tells,
        ..Default::default()
    };
    let mut source_summary = None;
    if let Some(src) = source {
        let d = source_diff(src, draft);
        response.missing_from_draft = d.missing;
        response.new_in_draft = d.added;
        if d.introduced_script {
            response.tells.push(tell(
                INTRODUCED_SCRIPT,
                0,
                "CJK characters in the draft but not the source",
                "the draft switched script",
            ));
        }
        source_summary = Some(d.summary);
    }
    Report {
        response,
        source_summary,
        footer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(r: &Report) -> Vec<(&str, &str)> {
        r.response
            .tells
            .iter()
            .map(|t| (t.kind.as_str(), t.matched.as_str()))
            .collect()
    }

    fn has(r: &Report, kind: &str, matched: &str) -> bool {
        kinds(r).contains(&(kind, matched))
    }

    #[test]
    fn vocab_flags_words_with_plain_replacement() {
        let r = lint("We will delve into it and leverage the data.", None);
        let t = &r.response.tells;
        assert_eq!(t.len(), 2);
        assert_eq!((t[0].line, t[0].kind.as_str()), (1, "vocab"));
        assert_eq!(t[0].matched, "delve");
        assert_eq!(t[0].hint, "look at / dig into");
        assert_eq!(t[1].hint, "use");
    }

    #[test]
    fn vocab_respects_word_boundaries_and_empty_replacement() {
        let r = lint("A robustness check.\nA vibrant market.", None);
        assert!(!has(&r, "vocab", "robust"));
        let t = &r.response.tells[0];
        assert_eq!((t.line, t.matched.as_str()), (2, "vibrant"));
        assert_eq!(t.hint, "cut, state the point");
    }

    #[test]
    fn vocab_normalises_curly_apostrophe_and_case() {
        let r = lint("IT\u{2019}S WORTH NOTING that it works.", None);
        assert!(has(&r, "vocab", "it's worth noting"));
    }

    #[test]
    fn phrases_report_their_family() {
        let r = lint("Let's dive in. I hope this helps.", None);
        assert!(has(&r, "run-up", "let's dive in"));
        assert!(has(&r, "chatbot residue", "i hope this helps"));
    }

    #[test]
    fn staged_contrast_is_flagged() {
        let r = lint("It's not just fast, but also cheap.", None);
        assert!(r
            .response
            .tells
            .iter()
            .any(|t| t.kind == "staged-contrast" && t.matched.starts_with("not just fast")));
        let r = lint("It's not about speed, it's about cost.", None);
        assert!(r.response.tells.iter().any(|t| t.kind == "staged-contrast"));
    }

    #[test]
    fn us_spelling_flags_us_forms_only() {
        let r = lint(
            "We organize the color center. Size matters and so does colour.",
            None,
        );
        assert!(has(&r, "us-spelling", "organize"));
        assert!(has(&r, "us-spelling", "color"));
        assert!(has(&r, "us-spelling", "center"));
        assert!(!has(&r, "us-spelling", "size"));
        assert!(!has(&r, "us-spelling", "colour"));
    }

    #[test]
    fn structure_tells_bold_bullet_rule_and_repeated_openings() {
        let text = "- **Speed**: fast\n---\nDone. We ship. We test. We fix.";
        let r = lint(text, None);
        assert!(has(&r, "bold-label-bullet", "- **Speed**: fast"));
        assert!(r
            .response
            .tells
            .iter()
            .any(|t| t.kind == "horizontal-rule" && t.line == 2));
        let rep = r
            .response
            .tells
            .iter()
            .find(|t| t.kind == "repeated-openings")
            .expect("repeated opening");
        assert_eq!(rep.line, 0);
        assert_eq!(rep.matched, "3+ sentences in a row start with 'we'");
    }

    #[test]
    fn punctuation_tells_and_footer_counts() {
        let text = "One \u{2014} two.\nThree \u{2013} four.\nFive -- six.\n\u{201c}Seven\u{201d}.";
        let r = lint(text, None);
        let p: Vec<(usize, &str)> = r
            .response
            .tells
            .iter()
            .filter(|t| t.kind == "punctuation")
            .map(|t| (t.line, t.matched.as_str()))
            .collect();
        assert_eq!(
            p,
            [(1, "\u{2014}"), (2, "\u{2013}"), (3, "--"), (4, "\u{201c}")]
        );
        assert_eq!(
            r.footer,
            "-- 7 words, 1 em / 1 en / 1 double-hyphen dashes, 2 curly double quotes, 0 bold spans, 0 bullet lines, 4 hits"
        );
    }

    #[test]
    fn clean_text_has_no_tells() {
        let r = lint(
            "The build failed on Tuesday. I reran it and it passed.",
            None,
        );
        assert!(r.response.tells.is_empty());
        assert!(!r.is_dirty());
    }

    #[test]
    fn fact_diff_reports_missing_tokens() {
        let src = "Fix `parse_args` in src/cli.rs for PROJ-123 by 5pm, said Alice Smith, 42 files.";
        let r = lint("I fixed it.", Some(src));
        let m = &r.response.missing_from_draft;
        for want in [
            "code:parse_args",
            "path:src/cli.rs",
            "ticket:PROJ-123",
            "number:5pm",
            "number:42",
        ] {
            assert!(m.contains(&want.to_string()), "missing {want} in {m:?}");
        }
        assert!(r.is_dirty());
    }

    #[test]
    fn fact_diff_reports_new_tokens_but_not_new_names() {
        let src = "The job ran for 5 minutes.";
        let draft = "The job ran for 5 minutes, see https://example.com/log and @bob, 99 times. Carol agreed.";
        let r = lint(draft, Some(src));
        let n = &r.response.new_in_draft;
        assert!(
            n.contains(&"url:https://example.com/log".to_string()),
            "{n:?}"
        );
        assert!(n.contains(&"mention:@bob".to_string()));
        assert!(n.contains(&"number:99".to_string()));
        assert!(!n.iter().any(|t| t.starts_with("name:")));
        assert!(r.response.missing_from_draft.is_empty());
    }

    #[test]
    fn version_is_one_token_and_not_split_into_numbers() {
        let toks = fact_tokens("deploy v2.4.0 on 12 may, then 1.2.3-rc.1 and v10.");
        assert_eq!(
            toks,
            [
                ("version", "v2.4.0".to_string()),
                ("version", "1.2.3-rc.1".to_string()),
                ("version", "v10".to_string()),
                ("number", "12".to_string()),
            ]
        );
    }

    #[test]
    fn version_missing_and_kept() {
        let src = "Deploy v2.4.0 on 12 March to prod.";
        let r = lint("Deploy on March 12 to prod.", Some(src));
        assert_eq!(r.response.missing_from_draft, ["version:v2.4.0"]);
        let r = lint("Ship v2.4.0 on 12 March.", Some(src));
        assert!(r.response.missing_from_draft.is_empty());
    }

    #[test]
    fn plain_numbers_are_not_versions() {
        let toks = fact_tokens("12 items, 1,284.50 total, 98%, $4,800, 3.14.");
        assert!(toks.iter().all(|(k, _)| *k == "number"), "{toks:?}");
        assert_eq!(toks.len(), 5);
    }

    #[test]
    fn number_written_as_word_shows_as_missing() {
        let r = lint("It retried three times.", Some("It retried 3 times."));
        assert_eq!(r.response.missing_from_draft, ["number:3"]);
    }

    #[test]
    fn code_span_digits_do_not_count_as_numbers() {
        let toks = fact_tokens("Run `make -j8` now");
        assert_eq!(toks, [("code", "make -j8".to_string())]);
    }

    #[test]
    fn names_skip_sentence_and_line_starts() {
        let toks = fact_tokens("Thanks for that. Ask Alice Smith.\nBob left. Then Carol came.");
        let names: Vec<&str> = toks
            .iter()
            .filter(|(k, _)| *k == "name")
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(names, ["Alice Smith", "Carol"]);
    }

    #[test]
    fn paths_not_glued_to_words_or_mentions() {
        let toks = fact_tokens("See docs/a.md and notes.txt, not @user/x.");
        let paths: Vec<&str> = toks
            .iter()
            .filter(|(k, _)| *k == "path")
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(paths, ["docs/a.md", "notes.txt"]);
    }

    #[test]
    fn introduced_script_is_reported_but_not_a_hit() {
        let r = lint("\u{4f60}\u{597d} world", Some("hello world"));
        assert!(has(
            &r,
            "introduced-script",
            "CJK characters in the draft but not the source"
        ));
        assert!(r.footer.ends_with(", 0 hits"));
        assert!(r
            .render_text()
            .contains("introduced-script: CJK characters"));
    }

    #[test]
    fn text_layout_matches_lint_py() {
        let r = lint("We delve in.", Some("We look at 3 things."));
        let text = r.render_text();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "1: vocab: delve -> look at / dig into");
        assert_eq!(lines[1], "missing-from-draft: number:3");
        assert!(lines[2].starts_with("-- source 5 words / 1 sentences -> draft 3 words / 1 sentences; length ratio 0.60; word recall"));
        assert!(lines[3].starts_with("-- 3 words, 0 em / 0 en / 0 double-hyphen dashes"));
        assert!(lines[3].ends_with("1 hits"));
    }

    #[test]
    fn json_shape_is_camel_case_with_match_key() {
        let r = lint("We delve in.", Some("We delve in 3 times."));
        let v = serde_json::to_value(&r.response).unwrap();
        assert_eq!(v["tells"][0]["match"], "delve");
        assert_eq!(v["tells"][0]["line"], 1);
        assert!(v["missingFromDraft"].is_array());
        assert!(v["newInDraft"].is_array());
    }

    #[test]
    fn splitlines_matches_python() {
        assert_eq!(splitlines("a\r\nb\rc\n"), ["a", "b", "c"]);
        assert_eq!(splitlines(""), Vec::<&str>::new());
        assert_eq!(splitlines("a\n\nb"), ["a", "", "b"]);
    }
}
