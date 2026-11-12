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
const MAX_GROWTH: f64 = 1.35;
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
    t.windows(2)
        .filter(|w| matches!(w[0].as_str(), "i" | "we") && w[1] == "will")
        .count()
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
    missing_facts(original, text).len() + logic_issues(original, text).len()
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
/// clean; else, sentence by sentence when both have as many sentences, the
/// original sentences that drifted; else the whole original paragraph.
fn guard_paragraph(original: &str, text: &str, warnings: &mut Vec<String>) -> String {
    let found = issues(original, text);
    if found.is_empty() {
        return text.to_string();
    }
    let (os, ts) = (sentences(original), sentences(text));
    if os.len() == ts.len() && os.len() > 1 {
        let mut out = String::new();
        for (n, (o, t)) in os.iter().zip(&ts).enumerate() {
            let bad = issues(o.trim(), t.trim());
            if bad.is_empty() {
                out.push_str(t);
            } else {
                warnings.push(format!(
                    "drift: kept original sentence {} ({})",
                    n + 1,
                    bad.join("; ")
                ));
                out.push_str(o.trim_end());
                out.push_str(trailing_space(t));
            }
        }
        // Sentences that are fine alone can still disagree together.
        if issues(original, &out).is_empty() {
            return out;
        }
    }
    warnings.push(format!(
        "drift: kept original paragraph ({})",
        found.join("; ")
    ));
    format!("{}{}", original.trim_end(), trailing_space(text))
}

/// Drift guard for a finished rewrite: paragraphs (and sentences) whose
/// meaning-carrying surface disagrees with the original fall back to the
/// original text. Returns the text and one warning per fallback.
pub fn guard(original: &str, text: &str) -> (String, Vec<String>) {
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
    } else {
        guard_paragraph(original, text, &mut warnings)
    };
    (out, warnings)
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
}
