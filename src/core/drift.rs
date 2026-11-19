//! Semantic-drift guard for rewrites. Token facts miss most real drift: a
//! flipped negation, a hedge that appears, a request turned into a command, a
//! clause invented. This module compares a rewrite with the original on
//! logic words, mood and length, and falls back to the original sentence (or
//! paragraph) where they disagree. It also keeps the original's ASCII
//! punctuation habits and spots a model answering "nothing to translate".

use crate::core::facts::{missing_facts, FUNCTION_WORDS};
use std::collections::BTreeMap;

/// Words whose count must match between original and rewrite, per group.
/// Synonyms share a group so a rewrite may swap them ("every" for "each").
const GROUPS: &[(&str, &[&str])] = &[
    (
        "negation",
        &[
            "not", "no", "never", "none", "neither", "nor", "without", "nothing", "nobody",
        ],
    ),
    (
        "limit",
        &[
            "unless", "except", "only", "until", "before", "after", "within", "exactly", "least",
        ],
    ),
    ("all", &["all", "every", "each", "both", "always"]),
    ("any", &["any", "either", "anyone", "anything"]),
    (
        "approximation",
        &[
            "some",
            "several",
            "few",
            "many",
            "most",
            "roughly",
            "around",
            "approximately",
            "nearly",
            "almost",
        ],
    ),
    (
        "obligation",
        &["must", "shall", "required", "need", "needs", "mandatory"],
    ),
    (
        "hedge",
        &[
            "may",
            "might",
            "maybe",
            "perhaps",
            "possibly",
            "probably",
            "likely",
            "could",
            "seems",
            "apparently",
            "presumably",
            "guess",
            "i_think",
        ],
    ),
    ("future", &["will", "gonna"]),
    ("condition", &["if", "when", "provided", "otherwise"]),
];

/// Longest rewrite, as a multiple of the original word count (plus slack),
/// before it counts as invented content; and the shortest, as a fraction.
const MAX_GROWTH: f64 = 1.6;
const MIN_KEEP: f64 = 0.5;

/// Whether `text` can be checked as English: plain ASCII letters and, past a few
/// words, a normal share of English function words.
pub fn is_english(text: &str) -> bool {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    let ascii = text
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(|c| c.is_ascii());
    let function = words
        .iter()
        .filter(|w| FUNCTION_WORDS.contains(&w.as_str()))
        .count();
    // Short, telegraphic strings have few function words; accept them.
    ascii && words.len() >= 2 && (words.len() < 8 || function * 10 >= words.len())
}

/// Lowercase word tokens with contractions expanded (`don't` -> `do not`,
/// `I'll` -> `i will`) and the `i think` hedge as one token.
fn tokens(text: &str) -> Vec<String> {
    let mut s = text.to_lowercase().replace('’', "'");
    for (from, to) in [
        ("can't", "can not"),
        ("cannot", "can not"),
        ("won't", "will not"),
        ("n't", " not"),
        ("'ll", " will"),
        ("'d", " would"),
        ("'ve", " have"),
        ("'re", " are"),
        ("'m", " am"),
    ] {
        s = s.replace(from, to);
    }
    let words: Vec<String> = s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect();
    let mut out = Vec::with_capacity(words.len());
    for (k, w) in words.iter().enumerate() {
        if w == "think" && k > 0 && words[k - 1] == "i" {
            out.push("i_think".to_string());
        } else if matches!(w.as_str(), "need" | "needs")
            && words.get(k + 1).is_some_and(|n| {
                matches!(
                    n.as_str(),
                    "help" | "support" | "assistance" | "anything" | "more"
                )
            })
        {
            // "if you need help" offers, it does not oblige.
            out.push("need_help".to_string());
        } else {
            out.push(w.clone());
        }
    }
    out
}

fn group_counts(text: &str) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for t in tokens(text) {
        for (name, words) in GROUPS {
            if words.contains(&t.as_str()) {
                *counts.entry(*name).or_insert(0) += 1;
            }
        }
    }
    counts
}

fn first_person_commitments(text: &str) -> usize {
    let t = tokens(text);
    // "we are committed to X" promises as much as "we will X".
    t.windows(2)
        .filter(|w| matches!(w[0].as_str(), "i" | "we") && w[1] == "will")
        .count()
        + t.iter().filter(|w| *w == "committed").count()
}

/// Logic-word disagreements between `original` and `text`, English only.
pub fn logic_issues(original: &str, text: &str) -> Vec<String> {
    if !is_english(original) || !is_english(text) {
        return Vec::new();
    }
    let (a, b) = (group_counts(original), group_counts(text));
    let mut out = Vec::new();
    for (name, _) in GROUPS {
        let (x, y) = (
            a.get(name).copied().unwrap_or(0),
            b.get(name).copied().unwrap_or(0),
        );
        if x != y {
            out.push(format!("{name}: {x} in the original, {y} in the rewrite"));
        }
    }
    let (x, y) = (
        first_person_commitments(original),
        first_person_commitments(text),
    );
    if y > x {
        out.push("a first-person commitment the original does not make".into());
    }
    out
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Everything that differs in meaning-carrying surface between the two texts:
/// token facts, logic words, question marks and length.
pub fn issues(original: &str, text: &str) -> Vec<String> {
    let mut out: Vec<String> = missing_facts(original, text)
        .into_iter()
        .map(|f| format!("fact {f} is missing"))
        .collect();
    out.extend(logic_issues(original, text));
    let (x, y) = (original.matches('?').count(), text.matches('?').count());
    if x != y {
        out.push(format!(
            "question marks: {x} in the original, {y} in the rewrite"
        ));
    }
    let (n, m) = (word_count(original) as f64, word_count(text) as f64);
    if m > n * MAX_GROWTH + 3.0 {
        out.push("much longer than the original (invented content)".into());
    } else if n >= 6.0 && m < n * MIN_KEEP {
        out.push("much shorter than the original (dropped content)".into());
    }
    out
}

/// Number of logic and fact disagreements, used by the repair to decide
/// whether an edit moves the text back toward the original's meaning.
pub fn drift_count(original: &str, text: &str) -> usize {
    let doc = original;
    let original = content(original);
    let tc = strip_filler(text);
    let tc = if tc.trim().is_empty() {
        text.to_string()
    } else {
        tc
    };
    fix_facts(doc, text).len() + logic_issues(&original, &tc).len()
}

/// Facts of `original` (minus courtesy sentences) missing from `text`, as the fix
/// hop sees them. NAB_EXP_FACTPOS: a capitalised word the original only uses at
/// a sentence, line or heading start is not a fact, as in the guard.
pub fn fix_facts(original: &str, text: &str) -> Vec<String> {
    let facts: Vec<String> = missing_facts(&content(original), text)
        .into_iter()
        .filter(|f| !heading_cased(f, original, text))
        .collect();
    if std::env::var("NAB_EXP_FACTPOS").is_err() {
        return facts;
    }
    facts
        .into_iter()
        .filter(|f| !only_at_starts(f, original))
        .collect()
}

/// Most terms listed for the repair hop.
const MAX_LOST_TERMS: usize = 10;

fn stems(text: &str) -> std::collections::HashSet<String> {
    text.split(|c: char| !c.is_alphabetic() && c != '\'' && c != '-')
        .filter(|w| !w.is_empty())
        .map(|w| stem(&w.to_lowercase()))
        .collect()
}

fn stem(word: &str) -> String {
    for suffix in ["ing", "ed", "es", "s", "ly"] {
        if let Some(s) = word.strip_suffix(suffix) {
            if s.chars().count() >= 4 {
                return s.to_string();
            }
        }
    }
    word.to_string()
}

/// Words of `original` (six letters or more, not function words) that `text`
/// no longer has in any inflection: candidates for a swapped domain term.
/// The repair hop decides which are real swaps.
pub fn lost_terms(original: &str, text: &str) -> Vec<String> {
    let kept = stems(text);
    let mut out: Vec<String> = Vec::new();
    for w in original.split(|c: char| !c.is_alphabetic() && c != '\'' && c != '-') {
        let lower = w.to_lowercase();
        if lower.chars().count() < 6
            || FUNCTION_WORDS.contains(&lower.as_str())
            || is_ai_word(&lower)
            || kept.contains(&stem(&lower))
            || out.contains(&lower)
        {
            continue;
        }
        out.push(lower);
        if out.len() == MAX_LOST_TERMS {
            break;
        }
    }
    out
}

/// First word of `original` when `text` neither starts with it nor has it in
/// its first three words: the opening verb or form may have changed.
pub fn lost_opening(original: &str, text: &str) -> Option<String> {
    let first = |t: &str| -> Vec<String> {
        t.split(|c: char| !c.is_alphabetic() && c != '\'')
            .filter(|w| !w.is_empty())
            .take(3)
            .map(|w| stem(&w.to_lowercase()))
            .collect()
    };
    let word = original
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .find(|w| !w.is_empty())?;
    (word.chars().count() >= 3 && !first(text).contains(&stem(&word.to_lowercase())))
        .then(|| word.to_string())
}

/// Spans of `text` split after `.`, `!`, `?` or a newline, trailing
/// whitespace kept, so the spans concatenate back to `text`.
/// Courtesy and filler markers. A sentence of the original with one of these
/// and no number, code, link or handle is one the rewrite may cut, names in it
/// included, so fact checks ignore it.
const FILLER_MARKERS: &[&str] = &[
    "hesitate",
    "reach out",
    "reaching out",
    "happy to help",
    "glad to help",
    "appreciate",
    "patience",
    "thank you",
    "thanks",
    "hope this",
    "hope you",
    "look forward",
    "looking forward",
    "feel free",
    "excited",
    "thrilled",
    "proud",
    "committed",
    "valued",
    "inconvenience",
    "any questions",
    "any further questions",
    "let me know",
    "warm regards",
    "best regards",
    "delighted",
    "pleased to",
    "we value",
    "trusted partner",
    "partnership",
    "welcoming you",
    "completely understand",
    "understand how",
    "we understand",
];

/// Logic groups whose count change triggers a fallback. Quantifier and hedge
/// counts ("any questions", "may have caused") ride on cut filler too often.
const GUARD_GROUPS: &[&str] = &["negation", "limit", "obligation"];

fn is_filler(sentence: &str) -> bool {
    let l = sentence.to_lowercase();
    FILLER_MARKERS.iter().any(|m| l.contains(m))
        && !sentence
            .chars()
            .any(|c| c.is_ascii_digit() || matches!(c, '`' | '@' | '/' | '#'))
        && !carries_content(sentence)
}

/// A courtesy-shaped sentence that routes the reader to a named contact
/// ("reach out to Priya") or asks a real question ("let me know how ...")
/// carries content the rewrite must keep. Names in thanks or cheer do not.
fn carries_content(sentence: &str) -> bool {
    let named = sentence
        .split_whitespace()
        .skip(1)
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .any(|w| w.len() > 1 && w.chars().next().is_some_and(char::is_uppercase));
    let l = sentence.to_lowercase();
    let routes = [
        "reach out to",
        "contact ",
        "talk to",
        "speak to",
        "speak with",
        "get in touch",
        "ask ",
        "email ",
        "call ",
        "ping ",
    ]
    .iter()
    .any(|m| l.contains(m));
    let ask = ["let me know", "let us know", "tell me", "tell us"]
        .iter()
        .any(|m| {
            l.find(m).is_some_and(|i| {
                let rest = &l[i + m.len()..];
                [
                    " how ",
                    " what ",
                    " whether ",
                    " when ",
                    " which ",
                    " who ",
                    " where ",
                ]
                .iter()
                .any(|q| rest.starts_with(q) || rest.starts_with(&q[..q.len() - 1]))
            })
        });
    // "excited to share that X" wraps a real claim in cheer.
    let announces = [
        "to share",
        "to announce",
        "to report",
        "to confirm",
        "to let you know",
        "to inform",
    ]
    .iter()
    .any(|m| l.contains(m));
    (named && routes) || ask || announces
}

/// The original without its courtesy sentences ([`is_filler`]); the original
/// itself when nothing else is left. Facts are checked against this.
pub fn content(original: &str) -> String {
    let c = strip_filler(original);
    if c.trim().is_empty() {
        original.to_string()
    } else {
        c
    }
}

/// The original without its courtesy sentences.
fn strip_filler(original: &str) -> String {
    original
        .split("\n\n")
        .map(|p| {
            sentences(p)
                .into_iter()
                .filter(|s| !is_filler(s))
                .collect::<String>()
        })
        .filter(|p| !p.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Disagreements that make the guard fall back: facts, negation, limits,
/// obligation, first-person commitments, question marks and length, measured
/// against the original minus its fact-free filler.
/// Courtesy sentences are set aside on both sides, so a logic word kept or cut
/// with them ("need support", "must be disappointing") does not count. Growth
/// is measured against the whole original: a rewrite may keep a courtesy
/// sentence's facts without its cheer.
fn guard_issues(original: &str, text: &str) -> Vec<String> {
    let (n, m) = (word_count(original) as f64, word_count(text) as f64);
    let longer = m > n * MAX_GROWTH + 3.0;
    let mut out = Vec::new();
    let oc = strip_filler(original);
    if !oc.trim().is_empty() {
        let tc = strip_filler(text);
        let tc = if tc.trim().is_empty() {
            text.to_string()
        } else {
            tc
        };
        out.extend(
            missing_facts(&oc, text)
                .into_iter()
                .filter(|f| keeps_fact_missing(f))
                .filter(|f| !GUARD_DOC.with(|d| heading_cased(f, &d.borrow(), text)))
                .map(|f| format!("fact {f} is missing")),
        );
        out.extend(logic_issues(&oc, &tc).into_iter().filter(|i| {
            let group = i.split(':').next().unwrap_or("");
            (!GROUPS.iter().any(|(g, _)| *g == group) || GUARD_GROUPS.contains(&group))
                && !only_gained(i)
        }));
        // A lost question can drop an ask; a gained one recasts a claim
        // ("Why does it matter? Because ..."), so only losses fall back.
        let (x, y) = (oc.matches('?').count(), tc.matches('?').count());
        if y < x {
            out.push(format!(
                "question marks: {x} in the original, {y} in the rewrite"
            ));
        }
        let (k, l) = (word_count(&oc) as f64, word_count(&tc) as f64);
        if !longer && k >= 6.0 && l < k * MIN_KEEP {
            out.push("much shorter than the original (dropped content)".into());
        }
    }
    if longer {
        out.push("much longer than the original (invented content)".into());
    }
    out
}

/// The kind of a guard issue: "fact", "longer", "shorter", or the group name before ':'.
fn issue_kind(issue: &str) -> &str {
    if issue.starts_with("fact ") {
        "fact"
    } else if issue.starts_with("much longer") {
        "longer"
    } else if issue.starts_with("much shorter") {
        "shorter"
    } else {
        issue.split(':').next().unwrap_or(issue)
    }
}

/// A negation, limit or obligation the rewrite gains is how idiomatic English
/// restates a claim ("difficult" -> "not easy", "subject to approval" -> "a
/// manager has to approve"); a lost one can drop a condition, so only losses
/// fall back.
fn only_gained(issue: &str) -> bool {
    let Some((group, rest)) = issue.split_once(": ") else {
        return false;
    };
    if !matches!(group, "negation" | "limit" | "obligation") {
        return false;
    }
    let n: Vec<usize> = rest
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|s| s.parse().ok())
        .collect();
    matches!(n.as_slice(), [x, y] if y > x)
}

/// A fact missing from a paragraph still counts unless it is a word-only fact
/// that is an AI-flavoured word. Facts with digits always count: a number moved
/// elsewhere can change meaning.
fn keeps_fact_missing(fact: &str) -> bool {
    (fact.chars().any(|c| c.is_ascii_digit()) || !is_ai_word(&fact.to_lowercase()))
        && !GUARD_DOC.with(|d| only_at_starts(fact, &d.borrow()))
}

thread_local! {
    /// The whole original during [`guard`], so paragraph and sentence checks see every use of a word.
    static GUARD_DOC: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// NAB_EXP_TITLE: a word `doc` capitalises only by position or in Title Case
/// lines ("Plan Your Menu First") that `text` still has in lower case.
fn heading_cased(word: &str, doc: &str, text: &str) -> bool {
    if std::env::var("NAB_EXP_TITLE").is_err() || doc.is_empty() {
        return false;
    }
    let lower = word.to_lowercase();
    let has = |s: &str, w: &str| s.split(|c: char| !c.is_alphanumeric()).any(|x| x == w);
    if lower == word || !word.chars().all(char::is_alphabetic) || !has(text, &lower) || !has(doc, word) {
        return false;
    }
    let title = |line: &str| {
        let long: Vec<&str> = line
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| w.chars().count() >= 4)
            .collect();
        long.len() >= 2
            && long.iter().filter(|w| w.starts_with(char::is_uppercase)).count() * 4
                >= long.len() * 3
    };
    let rest: String = doc
        .lines()
        .filter(|l| !title(l))
        .collect::<Vec<_>>()
        .join("\n");
    let in_rest = rest
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w == word);
    !in_rest || only_at_starts(word, &rest)
}

/// A capitalised word that `doc` only ever uses to open a sentence, line,
/// heading or list item ("Use", "Plan"): capitalised by position, not a name.
fn only_at_starts(word: &str, doc: &str) -> bool {
    if doc.is_empty()
        || !word.chars().all(char::is_alphabetic)
        || !word.chars().next().is_some_and(char::is_uppercase)
    {
        return false;
    }
    let mut seen = false;
    for (pos, _) in doc.match_indices(word) {
        let after = doc[pos + word.len()..].chars().next();
        if after.is_some_and(|c| c.is_alphanumeric()) {
            continue;
        }
        let before = &doc[..pos];
        if before
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric())
        {
            continue;
        }
        let lead = before.trim_end_matches(|c: char| {
            matches!(c, ' ' | '\t' | '#' | '*' | '-' | '>' | '"' | '(' | '_')
        });
        if !(lead.is_empty() || lead.ends_with(['\n', '.', '!', '?', ':'])) {
            return false;
        }
        seen = true;
    }
    seen
}

/// A word on the lint list of AI-flavoured vocabulary or a filler marker: the
/// rewrite is meant to drop it, so it is never a lost term to restore.
pub(crate) fn is_ai_word(lower: &str) -> bool {
    super::lint_data::VOCAB.iter().any(|(w, _)| *w == lower)
        || FILLER_MARKERS.contains(&lower)
        || matches!(
            lower,
            "seamless"
                | "seamlessly"
                | "transform"
                | "transformative"
                | "discover"
                | "journey"
                | "empower"
                | "empowers"
                | "unlock"
                | "elevate"
                | "streamline"
                | "streamlined"
                | "effortless"
                | "effortlessly"
                | "experience"
                | "exciting"
                | "incredible"
                | "amazing"
                | "sincerely"
                | "truly"
                | "genuinely"
                | "dedicated"
                | "delighted"
                | "pleased"
                | "thrilled"
                | "excited"
                | "encouraging"
                | "hesitate"
                | "committed"
                | "trusted"
                | "valued"
                | "exceptional"
                | "fantastic"
                | "huge"
                | "shoutout"
                | "smooth"
                | "collaborative"
                | "partnership"
        )
}

fn sentences(text: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut k = 0;
    while k < chars.len() {
        let (i, c) = chars[k];
        let ends = c == '\n' || matches!(c, '.' | '!' | '?' | '…');
        if ends {
            let mut j = k + 1;
            while j < chars.len() && matches!(chars[j].1, '.' | '!' | '?' | '"' | '\'' | ')') {
                j += 1;
            }
            let next_is_space = chars.get(j).is_none_or(|&(_, n)| n.is_whitespace());
            if c == '\n' || next_is_space {
                while j < chars.len() && chars[j].1.is_whitespace() {
                    j += 1;
                }
                let end = chars.get(j).map_or(text.len(), |&(p, _)| p);
                out.push(&text[start..end]);
                start = end;
                k = j;
                let _ = i;
                continue;
            }
        }
        k += 1;
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

fn trailing_space(s: &str) -> &str {
    &s[s.trim_end().len()..]
}

/// A paragraph of the rewrite checked against its original: the rewrite when
/// clean; else the original sentences (or aligned sentence groups) that
/// drifted; else the whole original paragraph.
fn guard_paragraph(original: &str, text: &str, warnings: &mut Vec<String>) -> String {
    let found = guard_issues(original, text);
    if found.is_empty() {
        return text.to_string();
    }
    // NAB_EXP_KIND: a sentence or group falls back only for a kind of issue its paragraph has.
    let kinds: Vec<&str> = found.iter().map(|i| issue_kind(i)).collect();
    let narrow = std::env::var("NAB_EXP_KIND").is_ok();
    let unit_issues = |o: &str, t: &str| -> Vec<String> {
        let bad = guard_issues(o, t);
        if narrow {
            bad.into_iter()
                .filter(|i| kinds.contains(&issue_kind(i)))
                .collect()
        } else {
            bad
        }
    };
    let (os, ts) = (sentences(original), sentences(text));
    if os.len() == ts.len() && os.len() > 1 {
        let mut out = String::new();
        for (n, (o, t)) in os.iter().zip(&ts).enumerate() {
            let bad = unit_issues(o.trim(), t.trim());
            if bad.is_empty() {
                out.push_str(t);
            } else {
                warnings.push(format!(
                    "drift: kept original sentence {} ({})",
                    n + 1,
                    bad.join("; ")
                ));
                // Cutting a courtesy sentence is allowed; a fallback must not bring it back.
                if !is_filler(o) {
                    out.push_str(o.trim_end());
                    out.push_str(trailing_space(t));
                }
            }
        }
        // Sentences that are fine alone can still disagree together.
        if guard_issues(original, &out).is_empty() {
            return out;
        }
    } else if os.len() > 1 && ts.len() > 1 {
        // Counts differ (a merge or split): align sentence groups and put back only the groups that drifted.
        let (mut out, mut kept) = (String::new(), Vec::new());
        let mut at = 1;
        for (o, t) in align_paragraphs(&os, &ts) {
            let (oj, tj) = (o.concat(), t.concat());
            let first = at;
            at += o.len();
            if o.is_empty() {
                out.push_str(&tj);
                continue;
            }
            let bad = if t.is_empty() {
                if o.iter().all(|s| is_filler(s)) {
                    continue;
                }
                vec!["dropped from the rewrite".to_string()]
            } else {
                unit_issues(oj.trim(), tj.trim())
            };
            if bad.is_empty() {
                out.push_str(&tj);
                continue;
            }
            kept.push(format!(
                "drift: kept original sentence {first} ({})",
                bad.join("; ")
            ));
            let oc = content(&oj);
            if !oc.trim().is_empty() {
                out.push_str(oc.trim_end());
                out.push_str(if t.is_empty() {
                    " "
                } else {
                    trailing_space(&tj)
                });
            }
        }
        let out = format!("{}{}", out.trim_end(), trailing_space(text));
        if guard_issues(original, &out).is_empty() {
            warnings.extend(kept);
            return out;
        }
    }
    warnings.push(format!(
        "drift: kept original paragraph ({})",
        found.join("; ")
    ));
    format!("{}{}", content(original).trim_end(), trailing_space(text))
}

/// Drift guard for a finished rewrite: paragraphs (and sentences) whose
/// meaning-carrying surface disagrees with the original fall back to the
/// original text. Returns the text and one warning per fallback.
pub fn guard(original: &str, text: &str) -> (String, Vec<String>) {
    GUARD_DOC.with(|d| *d.borrow_mut() = original.to_string());
    let (out, warnings) = guard_inner(original, text);
    GUARD_DOC.with(|d| d.borrow_mut().clear());
    (drop_repeats(original, &out), warnings)
}

/// Drops a sentence the output repeats more often than the original from a
/// rewrite paragraph, when an original paragraph put back whole also holds it:
/// the put-back restores a lead-in the rewrite had moved into its neighbour.
fn drop_repeats(original: &str, out: &str) -> String {
    let originals: std::collections::HashSet<&str> =
        original.split("\n\n").map(str::trim).collect();
    let mut paras: Vec<String> = out.split("\n\n").map(String::from).collect();
    for i in 0..paras.len() {
        if originals.contains(paras[i].trim()) {
            continue;
        }
        let mut kept = String::new();
        for s in sentences(&paras[i].clone()) {
            let key = s.trim();
            let doc = paras.join("\n\n");
            let repeated = word_count(key) >= 4
                && doc.matches(key).count() > original.matches(key).count()
                && paras
                    .iter()
                    .enumerate()
                    .any(|(j, p)| j != i && originals.contains(p.trim()) && p.contains(key));
            if !repeated {
                kept.push_str(s);
            }
        }
        paras[i] = kept.trim_end().to_string();
    }
    paras.retain(|p| !p.trim().is_empty());
    format!("{}{}", paras.join("\n\n").trim_end(), trailing_space(out))
}

fn guard_inner(original: &str, text: &str) -> (String, Vec<String>) {
    let mut warnings = Vec::new();
    let (op, tp): (Vec<&str>, Vec<&str>) = (
        original.split("\n\n").collect(),
        text.split("\n\n").collect(),
    );
    let out = if op.len() == tp.len() && op.len() > 1 {
        op.iter()
            .zip(&tp)
            .map(|(o, t)| guard_paragraph(o, t, &mut warnings))
            .collect::<Vec<_>>()
            .join("\n\n")
    } else if op.len() > 1 && tp.len() > 1 {
        let mut parts = Vec::new();
        let mut greeted = false;
        for (mut o, t) in align_paragraphs(&op, &tp) {
            // The rewrite's own salutation stays; the original's must not return beside it.
            if greeted && o.first().is_some_and(|p| is_salutation(p)) {
                o.remove(0);
            }
            greeted = o.is_empty() && t.len() == 1 && is_salutation(t[0]);
            match (o.is_empty(), t.is_empty()) {
                (_, true) => {}
                (true, false) => parts.push(t.join("\n\n")),
                _ => parts.push(guard_paragraph(
                    &o.join("\n\n"),
                    &t.join("\n\n"),
                    &mut warnings,
                )),
            }
        }
        parts.join("\n\n")
    } else {
        guard_paragraph(original, text, &mut warnings)
    };
    (out, warnings)
}

/// A paragraph the rewrite may drop or add freely: blank, all courtesy
/// sentences, or a short greeting or sign-off line ending in a comma.
fn is_courtesy_paragraph(p: &str) -> bool {
    let t = p.trim();
    t.is_empty()
        || sentences(t)
            .iter()
            .all(|s| s.trim().is_empty() || is_filler(s))
        || is_salutation(t)
}

/// A short greeting or sign-off line ending in a comma.
fn is_salutation(p: &str) -> bool {
    let t = p.trim();
    t.ends_with(',') && t.split_whitespace().count() <= 4 && !t.chars().any(|c| c.is_ascii_digit())
}

/// Monotone alignment of original and rewrite paragraphs when their counts
/// differ: 1:1, k:1 and 1:k (k <= MAX_MERGE) matches scored by content-word overlap; a
/// courtesy paragraph may go unmatched cheaply, any other one at full cost.
/// Returns (original group, rewrite group) pairs in order; one side may be empty.
const MAX_MERGE: usize = 4;

fn align_paragraphs<'a>(op: &[&'a str], tp: &[&'a str]) -> Vec<(Vec<&'a str>, Vec<&'a str>)> {
    use crate::core::repair::overlap;
    let (n, m) = (op.len(), tp.len());
    let skip = |p: &str| if is_courtesy_paragraph(p) { 0.3 } else { 1.0 };
    let mut cost = vec![vec![f64::INFINITY; m + 1]; n + 1];
    let mut back = vec![vec![(0usize, 0usize); m + 1]; n + 1];
    cost[0][0] = 0.0;
    for i in 0..=n {
        for j in 0..=m {
            let here = cost[i][j];
            if here.is_infinite() {
                continue;
            }
            let mut relax = |di: usize, dj: usize, c: f64| {
                if i + di <= n && j + dj <= m && here + c < cost[i + di][j + dj] {
                    cost[i + di][j + dj] = here + c;
                    back[i + di][j + dj] = (di, dj);
                }
            };
            if i < n {
                relax(1, 0, skip(op[i]));
            }
            if j < m {
                relax(0, 1, skip(tp[j]));
            }
            if i < n && j < m {
                relax(1, 1, 1.0 - overlap(op[i], tp[j]));
            }
            // k:1 and 1:k merges, so a split or joined paragraph is checked whole.
            for k in 2..=MAX_MERGE {
                let extra = 0.1 * k as f64 + 1.0;
                if i + k <= n && j < m {
                    relax(k, 1, extra - overlap(&op[i..i + k].join("\n\n"), tp[j]));
                }
                if i < n && j + k <= m {
                    relax(1, k, extra - overlap(op[i], &tp[j..j + k].join("\n\n")));
                }
            }
        }
    }
    let (mut i, mut j) = (n, m);
    let mut out = Vec::new();
    while i > 0 || j > 0 {
        let (di, dj) = back[i][j];
        out.push((op[i - di..i].to_vec(), tp[j - dj..j].to_vec()));
        i -= di;
        j -= dj;
    }
    out.reverse();
    out
}

/// Replace typographic punctuation the original does not use: a source typed
/// with a keyboard keeps hyphens, straight quotes and three dots.
pub fn ascii_habits(original: &str, text: &str) -> String {
    let mut out = text.to_string();
    for (ch, plain) in [
        ("–", "-"),
        ("—", " - "),
        ("“", "\""),
        ("”", "\""),
        ("‘", "'"),
        ("’", "'"),
        ("…", "..."),
    ] {
        if !original.contains(ch) && out.contains(ch) {
            out = out.replace(ch, plain);
        }
    }
    out.replace("  - ", " - ").replace(" -  ", " - ")
}

/// Phrases of a model answering that it received no text.
const REFUSALS: &[&str] = &[
    "nothing came through",
    "nothing to translate",
    "no text to",
    "text is empty",
    "is empty, so",
    "paste the",
    "please provide the text",
    "please share the text",
    "there's no text",
    "there is no text",
    "no content to",
];

/// Whether `output` is a "nothing to translate" style answer rather than the
/// rewrite: it carries a refusal phrase that the `input` does not.
pub fn looks_like_refusal(input: &str, output: &str) -> bool {
    let (i, o) = (input.to_lowercase(), output.to_lowercase());
    REFUSALS.iter().any(|p| o.contains(p) && !i.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courtesy_sentences_are_set_aside_on_both_sides() {
        // "need" moves from a courtesy sentence in the original to the rewrite.
        let o = "We plan to finish testing by 21 November. Please reach out if you have questions or need support!";
        let t = "We plan to finish testing by 21 November. If you have questions or need support, contact us.";
        assert!(guard_issues(o, t).is_empty(), "{:?}", guard_issues(o, t));
        // Epistemic "must" inside sympathy is not an obligation.
        let o = "Your trip on 14 November was cancelled. We completely understand how disappointing this must be.";
        assert!(guard_issues(o, "Your trip on 14 November was cancelled.").is_empty());
        // Keeping a courtesy sentence's facts without its cheer is not invented content.
        let o = "We're thrilled to announce a milestone at Acme, a Hamilton-based company. On 3 November we launched v2 to 14 clinics.";
        let t = "Acme is a Hamilton-based company. On 3 November we launched v2 to 14 clinics.";
        assert!(guard_issues(o, t).is_empty(), "{:?}", guard_issues(o, t));
        // An all-courtesy paragraph may shrink to one line.
        let o = "If you have any questions, our support team is always happy to help. We look forward to a successful partnership!";
        assert!(guard_issues(o, "Questions go to our support team.").is_empty());
        // A real obligation outside courtesy still counts.
        let o = "You must restart the gateway. Thanks for your patience.";
        assert!(!guard_issues(o, "Restart the gateway.").is_empty());
    }

    #[test]
    fn courtesy_sentences_with_a_name_or_an_ask_are_content() {
        assert!(!carries_content("Thanks so much for your patience, Sarah."));
        assert!(carries_content(
            "If you are still stuck, reach out to Priya Nair."
        ));
        assert!(carries_content(
            "Please let me know how you want to handle the copy."
        ));
        assert!(!carries_content("Let me know if you have any questions."));
        assert!(carries_content(
            "We're excited to share that the new billing dashboard is now live in staging."
        ));
        assert!(!carries_content("We're excited about this one."));
    }

    #[test]
    fn a_claim_wrapped_in_cheer_is_not_cut() {
        let o = "Quick update on 2.8.0. We're excited to share that the new billing dashboard is now live in staging. We closed 14 of 17 tickets.";
        let t = "Quick update on 2.8.0. We closed 14 of 17 tickets.";
        let (out, w) = guard(o, t);
        assert!(!w.is_empty(), "{out}");
        assert!(out.contains("billing dashboard"), "{out}");
    }

    #[test]
    fn a_split_single_paragraph_is_checked_whole() {
        let o = "You are invited on Monday, October 8, at the Four Seasons.\nCocktails are at 6:45 p.m.\nPlease RSVP to Kelly at extension 36485 by Thursday.";
        let t = "You're invited on Monday, October 8, at the Four Seasons.\n\nCocktails start at 6:45 p.m.\n\nRSVP to Kelly by Thursday.";
        let (out, w) = guard(o, t);
        assert!(!w.is_empty());
        // One fallback for the whole text, never the original pasted beside rewrite paragraphs.
        for fact in ["Four Seasons", "6:45", "Thursday"] {
            assert_eq!(out.matches(fact).count(), 1, "{fact}: {out}");
        }
        assert!(out.contains("36485"));
        // Multi-paragraph splits align 1:3 instead of leaving rewrite paragraphs unmatched.
        let o2 = format!("Intro line about the 2.8.0 release.\n\n{o}");
        let t2 = format!("Intro line about the 2.8.0 release.\n\n{t}");
        let (out2, _) = guard(&o2, &t2);
        for fact in ["Four Seasons", "6:45", "Thursday"] {
            assert_eq!(out2.matches(fact).count(), 1, "{fact}: {out2}");
        }
    }

    #[test]
    fn fallbacks_leave_courtesy_sentences_out() {
        // Paragraph fallback: the flagged rewrite reverts, its thanks does not come back.
        let o = "Thank you all for the great collaboration this sprint. You must restart the gateway before 5 pm.";
        let mut w = Vec::new();
        let out = guard_paragraph(o, "Restart the gateway when you can.", &mut w);
        assert_eq!(out, "You must restart the gateway before 5 pm.");
        assert!(!w.is_empty());
    }

    #[test]
    fn dropped_courtesy_paragraphs_do_not_revert_the_rest() {
        let original = "Hi Sarah,\n\nThank you for reaching out, we appreciate your patience.\n\n\
            The refund for INV-20481 was approved on 11 November. It will not be reversed.\n\n\
            If you have any further questions, please do not hesitate to reply.\n\nWarm regards,\n\nJames";
        let rewrite = "Hi Sarah,\n\nWe approved the refund for INV-20481 on 11 November. \
            It won't be reversed.\n\nJames";
        let (text, warnings) = guard(original, rewrite);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(text, rewrite);
    }

    #[test]
    fn names_inside_courtesy_sentences_are_not_facts() {
        let original =
            "The order ships Monday. We value you as part of the Waikato farming community.";
        assert_eq!(drift_count(original, "The order ships Monday."), 0);
        assert_eq!(drift_count(original, "The order ships soon."), 1);
    }

    #[test]
    fn a_merged_paragraph_puts_back_only_the_sentence_that_drifted() {
        let o = "The museum opens at 9 am. Entry costs NZD 40 for adults. Tours leave before noon.";
        let t = "The museum opens at 9 am and adult entry costs NZD 40. Tours leave around lunch.";
        let mut w = Vec::new();
        let out = guard_paragraph(o, t, &mut w);
        assert_eq!(
            out,
            "The museum opens at 9 am and adult entry costs NZD 40. Tours leave before noon."
        );
        assert!(w.iter().all(|x| x.contains("sentence")), "{w:?}");
    }

    #[test]
    fn misaligned_paragraph_with_lost_fact_still_falls_back() {
        let original = "Intro line here.\n\nPay NZD 18,900 by Friday.\n\nThanks for your patience.";
        let rewrite = "Pay soon please.\n\nIntro.";
        let (text, warnings) = guard(original, rewrite);
        assert!(!warnings.is_empty());
        assert!(text.contains("18,900"), "{text}");
    }

    #[test]
    fn flipped_condition_and_added_hedge_are_drift() {
        let o = "The value is used as the fallback, not before it.";
        assert!(logic_issues(o, "The value is used as the fallback, not before it.").is_empty());
        let dropped = logic_issues(o, "The value is used as the fallback.");
        assert!(
            dropped.iter().any(|i| i.starts_with("negation")),
            "{dropped:?}"
        );
        assert!(
            dropped.iter().any(|i| i.starts_with("limit")),
            "{dropped:?}"
        );
        let hedged = logic_issues(
            o,
            "The value will be used as the fallback, maybe, not before it.",
        );
        assert!(hedged.iter().any(|i| i.starts_with("hedge")), "{hedged:?}");
        assert!(hedged.iter().any(|i| i.starts_with("future")), "{hedged:?}");
    }

    #[test]
    fn synonyms_in_one_group_and_contractions_are_not_drift() {
        assert!(logic_issues(
            "Each pod must pass. It doesn't retry.",
            "Every pod needs to pass, and it does not retry."
        )
        .is_empty());
        assert!(logic_issues("We can't ship it.", "We cannot ship it.").is_empty());
        assert!(logic_issues("I think it works.", "I think it works fine.").is_empty());
        assert!(logic_issues("it works.", "It works, I think.")
            .iter()
            .any(|i| i.starts_with("hedge")));
    }

    #[test]
    fn non_english_text_skips_the_logic_check() {
        assert!(logic_issues("不要在周五上线。", "Do not ship on Friday.").is_empty());
        assert!(!is_english("不要在周五上线"));
        assert!(is_english("The review is on Thursday and we are ready."));
    }

    #[test]
    fn mood_length_and_facts_are_checked() {
        let o = "can you go through INC-2291 for me";
        assert!(issues(o, "Go through INC-2291 for me.").is_empty());
        assert!(issues("can you go through it?", "Go through it.")
            .iter()
            .any(|i| i.starts_with("question marks")));
        assert!(issues(o, "Go through INC-2292.")
            .iter()
            .any(|i| i.contains("fact")));
        let long = "Check it. ".repeat(12);
        assert!(issues("Check it.", &long)
            .iter()
            .any(|i| i.contains("invented")));
        assert!(issues("one two three four five six seven eight", "one two")
            .iter()
            .any(|i| i.contains("shorter")));
    }

    #[test]
    fn guard_keeps_clean_sentences_and_restores_drifted_ones() {
        let original = "The window is exactly 25 minutes. Do not push before 14:00. Sam signs off.";
        let rewrite =
            "The window lasts about 25 minutes. Push after 14:00 if it suits. Sam signs off.";
        let (text, warnings) = guard(original, rewrite);
        assert_eq!(text, original);
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].starts_with("drift: kept original sentence 1"));
        assert!(warnings[1].starts_with("drift: kept original sentence 2"));
    }

    #[test]
    fn guard_falls_back_to_the_paragraph_when_sentences_do_not_line_up() {
        let original = "Do not ship before the freeze lifts. Sam checks.";
        let rewrite = "Ship once the freeze lifts, and Sam checks.";
        let (text, warnings) = guard(original, rewrite);
        assert_eq!(text, original);
        assert!(warnings[0].starts_with("drift: kept original paragraph"));
    }

    #[test]
    fn guard_works_per_paragraph_and_passes_clean_text() {
        let original = "Pay NZD 18,900 by Friday.\n\nSam never signs late.";
        let rewrite = "Pay NZD 18,900 by Friday.\n\nSam signs late.";
        let (text, warnings) = guard(original, rewrite);
        assert_eq!(text, "Pay NZD 18,900 by Friday.\n\nSam never signs late.");
        assert_eq!(warnings.len(), 1);
        let (same, none) = guard(original, original);
        assert_eq!(same, original);
        assert!(none.is_empty());
    }

    #[test]
    fn ascii_habits_follow_the_original() {
        assert_eq!(
            ascii_habits("14:00-16:00 NZ, don't", "14:00–16:00 NZ, don’t…"),
            "14:00-16:00 NZ, don't..."
        );
        assert_eq!(ascii_habits("a - b", "a — b"), "a - b");
        // Typographic marks in the original stay.
        assert_eq!(
            ascii_habits("it’s 14:00–16:00", "it’s 14:00–16:00"),
            "it’s 14:00–16:00"
        );
    }

    #[test]
    fn refusals_are_spotted_only_when_the_input_lacks_them() {
        let out = "Nothing came through to translate. The space between <<< and >>> is empty.";
        assert!(looks_like_refusal(
            "Steps 5 and 6 run after the freeze.",
            out
        ));
        assert!(!looks_like_refusal(
            "Nothing came through to translate, he said.",
            out
        ));
        assert!(!looks_like_refusal(
            "Steps 5 and 6 run after the freeze.",
            "Steps 5 and 6 run later."
        ));
    }

    #[test]
    fn lost_terms_list_swapped_words_but_not_inflections() {
        let o = "Stop after 6 attempts and cite the absence of a rollback line.";
        let t = "Stop after 6 retries and cite the lack of a rollback plan.";
        assert_eq!(lost_terms(o, t), vec!["attempts", "absence"]);
        assert!(lost_terms("Attempts are counted.", "An attempt is counted.").is_empty());
        assert!(lost_terms(o, o).is_empty());
    }

    #[test]
    fn lost_opening_flags_a_changed_leading_word_only() {
        assert_eq!(
            lost_opening(
                "Gives the incident times in UTC.",
                "Event times are shown in UTC."
            ),
            Some("Gives".to_string())
        );
        assert_eq!(
            lost_opening("Gives the times.", "It gives the times."),
            None
        );
        assert_eq!(lost_opening("Go through it.", "Go through it now."), None);
    }

    #[test]
    fn sentence_spans_concatenate_back() {
        let t = "Hey, it is 4.19.0 now. Really?  Yes!\nnext line";
        let s = sentences(t);
        assert_eq!(s.concat(), t);
        assert_eq!(s.len(), 4, "{s:?}");
    }

    #[test]
    fn guard_keeps_a_gained_question_but_not_a_lost_one() {
        let o = "Repair cafes matter because they keep phones out of landfill.";
        let t = "Why do repair cafes matter? They keep phones out of landfill.";
        assert!(guard_issues(o, t).is_empty(), "{:?}", guard_issues(o, t));
        assert!(guard_issues(t, o)
            .iter()
            .any(|i| i.starts_with("question marks")));
    }

    #[test]
    fn guard_keeps_a_gained_obligation_but_not_a_lost_one() {
        let o = "Approval is subject to a manager's sign-off and the team's workload.";
        let t = "A manager has to sign off, and the team's workload still sets a limit, so approval needs both.";
        assert!(!guard_issues(o, t)
            .iter()
            .any(|i| i.starts_with("obligation")));
        assert!(guard_issues(t, o)
            .iter()
            .any(|i| i.starts_with("obligation")));
    }

    #[test]
    fn restored_original_does_not_sit_beside_the_rewrite_greeting() {
        let original = "Dear Dana,\n\nThank you so much for reaching out, and I sincerely apologize for any inconvenience this may have caused! I completely understand how frustrating this must be. Rest assured that we are committed to delivering a seamless experience.\n\nOur team has identified that the issue is not a data loss \u{2014} rather, it is a display filter problem. Your invoices from 15 October are safe. The Finance tab was defaulting to \"Today\" after our update on 16 October. To resolve this, simply change the date filter to \"Last 7 days\".\n\nWe have also logged this as ACME-1301 and a fix is expected in the next release (v12.7). Please don't hesitate to reach out if you require further assistance!\n\nWarm regards,\nAcme Clinic Cloud Support";
        let rewrite = "Hi Dana,\n\nSorry for the hassle. Nothing's been lost. It's a display filter problem, and your invoices from 15 October are all still there. After our update on 16 October, the date filter on the \"Finance\" tab started defaulting to \"Today\". Switch it to \"Last 7 days\" and you'll see them.\n\nWe've logged it as ACME-1301, and it should be fixed in the next version (v12.7).\n\nAcme Clinic Cloud Support";
        let (out, _) = guard(original, rewrite);
        let salutations = out
            .split("\n\n")
            .filter(|p| p.trim_end().ends_with(',') && p.contains("Dana"))
            .count();
        assert_eq!(salutations, 1, "{out}");
    }

    #[test]
    fn a_word_capitalised_only_by_position_is_not_a_name() {
        let doc = "## Plan\n\nPlan the week on Sunday. Use the Sonata app.\n\n- Use a timer.";
        assert!(only_at_starts("Plan", doc));
        assert!(only_at_starts("Use", doc));
        assert!(!only_at_starts("Sonata", doc));
        assert!(!only_at_starts("Sunday", doc));
        let o = "Plan meals on Sunday so the week runs on autopilot.";
        let (out, w) = guard(o, "Sort out meals on Sunday and the week runs itself.");
        assert!(w.is_empty(), "{w:?} {out}");
        let (_, w) = guard("Book the Sonata room.", "Book the big room.");
        assert!(!w.is_empty());
    }

    #[test]
    fn put_back_lead_in_is_not_repeated_in_the_rewrite_paragraph() {
        let original =
            "Intro line here.\n\nTo move forward, we would need:\n- **A:** one\n- **B:** two";
        let out = "Intro line here. To move forward, we would need:\n\nTo move forward, we would need:\n- **A:** one\n- **B:** two";
        assert_eq!(
            drop_repeats(original, out),
            "Intro line here.\n\nTo move forward, we would need:\n- **A:** one\n- **B:** two"
        );
        // A repeat the original also has stays.
        let original = "Check the log again now.\n\nCheck the log again now.";
        assert_eq!(drop_repeats(original, original), original);
    }
}
