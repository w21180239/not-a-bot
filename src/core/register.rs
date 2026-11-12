//! Register of a source text (casual, formal or neither), detected from cheap
//! surface signals, so a rewrite can keep the source's voice. The LLM hops and
//! the repair hop get the result as a prompt directive; [`restore_surface`]
//! then puts back the casing and full-stop habits a round trip tidied away.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Register {
    /// Spoken, dictated or chat-like: lowercase starts, fillers, fragments.
    Casual,
    /// Complete capitalised sentences with no contractions or fillers.
    Formal,
    /// Anything else, including text in scripts without case.
    Neutral,
}

impl Register {
    pub fn as_str(self) -> &'static str {
        match self {
            Register::Casual => "casual",
            Register::Formal => "formal",
            Register::Neutral => "neutral",
        }
    }
}

/// Detected register and the voice markers that were seen, in prompt wording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub register: Register,
    pub markers: Vec<&'static str>,
}

const CASUAL_WORDS: &[&str] = &[
    "hey",
    "hi",
    "oh",
    "um",
    "uh",
    "haha",
    "hehe",
    "lol",
    "kinda",
    "sorta",
    "gonna",
    "wanna",
    "gotta",
    "tbh",
    "btw",
    "yeah",
    "yep",
    "nope",
    "ok",
    "okay",
    "stuff",
    "honestly",
    "basically",
];
const CASUAL_PHRASES: &[&str] = &[
    "i mean",
    "you know",
    "i guess",
    "i think?",
    "or something",
    "sort of",
    "kind of",
    "like a ",
];
const SELF_CORRECTIONS: &[&str] = &[
    "actually no",
    "oh actually",
    "no wait",
    "scratch that",
    "or rather",
    "sorry,",
    "i mean",
    "hang on",
];
const FORMAL_PHRASES: &[&str] = &[
    "dear ",
    "sincerely",
    "kind regards",
    "yours faithfully",
    "pursuant",
    "hereby",
    "please find",
    "we are writing",
    "i am writing",
    "kindly",
    "shall ",
    "hereinafter",
    "notwithstanding",
];

/// Sentence-like segments of `text`: split at newlines and after `.`, `!`,
/// `?` or `…` followed by whitespace or the end (so `4.19.0` stays whole).
fn segments(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (k, &(i, c)) in chars.iter().enumerate() {
        let boundary = c == '\n'
            || (matches!(c, '.' | '!' | '?' | '…')
                && chars.get(k + 1).is_none_or(|&(_, n)| n.is_whitespace()));
        if boundary {
            out.push(&text[start..i]);
            start = i + c.len_utf8();
        }
    }
    out.push(&text[start..]);
    out.into_iter()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// List items, headings, tables and fences are structure, not sentences.
fn is_structural(line: &str) -> bool {
    let line = line.trim_start();
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    ["- ", "* ", "• ", "#", "|", "```"]
        .iter()
        .any(|p| line.starts_with(p))
        || (digits > 0 && line[digits..].starts_with(['.', ')']))
}

/// First alphanumeric char of a segment, when it is a letter.
fn lead_letter(segment: &str) -> Option<char> {
    if is_structural(segment) {
        return None;
    }
    segment
        .chars()
        .find(|c| c.is_alphanumeric())
        .filter(|c| c.is_alphabetic())
}

/// Count of contractions such as `don't`, `we're`, `I'll`.
fn contractions(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut n = 0;
    for k in 1..chars.len() {
        if !matches!(chars[k], '\'' | '’') || !chars[k - 1].is_alphabetic() {
            continue;
        }
        let tail: String = chars[k + 1..]
            .iter()
            .take_while(|c| c.is_alphabetic())
            .collect::<String>()
            .to_lowercase();
        if matches!(tail.as_str(), "t" | "re" | "ve" | "ll" | "d" | "m") {
            n += 1;
        }
    }
    n
}

/// Detect the register of `text`. Latin-script text only; anything else is
/// [`Register::Neutral`].
pub fn detect(text: &str) -> Profile {
    let neutral = Profile {
        register: Register::Neutral,
        markers: Vec::new(),
    };
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    let latin = letters.iter().filter(|c| c.is_ascii_alphabetic()).count();
    if latin < 12 || latin * 10 < letters.len() * 8 {
        return neutral;
    }

    let segs = segments(text);
    let (mut lower, mut upper) = (0usize, 0usize);
    for s in &segs {
        match lead_letter(s) {
            Some(c) if c.is_lowercase() => lower += 1,
            Some(c) if c.is_uppercase() => upper += 1,
            _ => {}
        }
    }
    let cased = lower + upper;
    let lower_starts = cased >= 2 && lower * 2 >= cased;
    let capital_starts = cased >= 2 && lower * 10 <= cased;

    let (mut ended, mut unended) = (0usize, 0usize);
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !is_structural(l))
    {
        match line.chars().last() {
            Some('.' | '!' | '?') => ended += 1,
            Some(c) if c.is_alphanumeric() => unended += 1,
            _ => {}
        }
    }
    let no_final_stops = unended > 0 && unended >= ended;

    let lowered = format!(" {} ", text.to_lowercase().replace('\n', " "));
    let words: HashSet<&str> = lowered
        .split(|c: char| !c.is_alphanumeric() && c != '\'')
        .filter(|w| !w.is_empty())
        .collect();
    let fillers = CASUAL_WORDS.iter().any(|w| words.contains(w))
        || CASUAL_PHRASES.iter().any(|p| lowered.contains(p));
    let corrections = SELF_CORRECTIONS.iter().any(|p| lowered.contains(p));
    let ellipses = text.contains("...") || text.contains('…');
    let contracted = contractions(text) > 0;

    let mut markers = Vec::new();
    if lower_starts {
        markers.push("lowercase sentence starts");
    }
    if no_final_stops {
        markers.push("no final full stops");
    }
    if fillers {
        markers.push("fillers and interjections (hey, oh, haha, like, I mean)");
    }
    if corrections {
        markers.push("self-corrections and false starts");
    }
    if ellipses {
        markers.push("trailing-off ellipses");
    }
    if contracted {
        markers.push("contractions");
    }
    let score = 2 * usize::from(lower_starts)
        + usize::from(no_final_stops)
        + usize::from(fillers)
        + usize::from(corrections)
        + usize::from(ellipses);
    if score >= 3 {
        return Profile {
            register: Register::Casual,
            markers,
        };
    }

    let words_per_segment = words.len() as f64 / segs.len().max(1) as f64;
    let formal_phrase = FORMAL_PHRASES.iter().any(|p| lowered.contains(p));
    if capital_starts
        && !fillers
        && !corrections
        && !ellipses
        && !contracted
        && (formal_phrase || words_per_segment >= 14.0)
    {
        return Profile {
            register: Register::Formal,
            markers: vec![
                "capitalised complete sentences",
                "no contractions, slang or fillers",
            ],
        };
    }
    neutral
}

/// Expand contractions in `text` when `original` has none, so a rewrite does
/// not turn "I will" into "I'll". Possessive `'s` and ambiguous `'d` stay.
pub fn expand_contractions(original: &str, text: &str) -> String {
    if contractions(original) > 0 || !text.contains(['\'', '’']) {
        return text.to_string();
    }
    static RULES: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
        [
            (r"(?i)\bwon['’]t\b", "will not"),
            (r"(?i)\bcan['’]t\b", "cannot"),
            (r"(?i)\bshan['’]t\b", "shall not"),
            (r"(?i)\b(\w+)n['’]t\b", "$1 not"),
            (r"(?i)\b(\w+)['’]ll\b", "$1 will"),
            (r"(?i)\b(\w+)['’]re\b", "$1 are"),
            (r"(?i)\b(\w+)['’]ve\b", "$1 have"),
            (r"(?i)\bI['’]m\b", "I am"),
            (r"(?i)\b(it|that|there|here|what|who)['’]s\b", "$1 is"),
        ]
        .into_iter()
        .map(|(p, r)| (Regex::new(p).unwrap(), r))
        .collect()
    });
    RULES.iter().fold(text.to_string(), |acc, (re, rep)| {
        re.replace_all(&acc, *rep).into_owned()
    })
}

/// Tokens of `text` that start with an uppercase letter.
fn capitalised_tokens(text: &str) -> HashSet<&str> {
    text.split(|c: char| !(c.is_alphanumeric() || matches!(c, '\'' | '’' | '-' | '_')))
        .filter(|t| t.chars().next().is_some_and(char::is_uppercase))
        .collect()
}

const ALWAYS_CAPITALISED: &[&str] = &[
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "January",
    "February",
    "March",
    "April",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Whether a sentence-initial `token` may be lowercased: not `I`, an acronym,
/// an ID or a number-bearing word, a name the original capitalises, or a
/// weekday or month.
fn lowercasable(token: &str, original_caps: &HashSet<&str>) -> bool {
    let uppers = token.chars().filter(|c| c.is_uppercase()).count();
    !(uppers > 1
        || token.chars().any(|c| c.is_ascii_digit())
        || token == "I"
        || token.starts_with("I'")
        || token.starts_with("I’")
        || ALWAYS_CAPITALISED.contains(&token)
        || original_caps.contains(token))
}

fn lowercase_starts(original: &str, text: &str) -> String {
    let caps = capitalised_tokens(original);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let (mut at_start, mut in_code, mut in_fence) = (true, false, false);
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        if c == '`' {
            in_code = !in_code;
        }
        if at_start && c.is_uppercase() && !in_code && !in_fence {
            let end = (k..chars.len())
                .find(|&j| {
                    !(chars[j].is_alphanumeric() || matches!(chars[j], '\'' | '’' | '-' | '_'))
                })
                .unwrap_or(chars.len());
            let token: String = chars[k..end].iter().collect();
            // A path, URL or dotted ID continues past the token.
            let glued = chars.get(end).is_some_and(|n| {
                matches!(n, '/' | '\\' | '.' | ':')
                    && chars.get(end + 1).is_some_and(|m| m.is_alphanumeric())
            });
            if !glued && lowercasable(&token, &caps) {
                out.extend(c.to_lowercase());
                k += 1;
                at_start = false;
                continue;
            }
        }
        if c == '\n' {
            if chars[k + 1..].starts_with(&['`', '`', '`']) {
                in_fence = !in_fence;
            }
            at_start = true;
        } else if matches!(c, '.' | '!' | '?') && chars.get(k + 1).is_none_or(|n| n.is_whitespace())
        {
            at_start = true;
        } else if !c.is_whitespace() && !matches!(c, '"' | '\'' | '(' | '“' | '‘' | '*' | '_') {
            at_start = false;
        }
        out.push(c);
        k += 1;
    }
    out
}

/// Drop the single full stop that ends a line, keeping `...`, `?`, `!`,
/// abbreviations and lines inside code fences.
fn strip_final_stops(text: &str) -> String {
    let mut in_fence = false;
    let lines: Vec<String> =
        text.split('\n')
            .map(|line| {
                if line.trim_start().starts_with("```") {
                    in_fence = !in_fence;
                    return line.to_string();
                }
                let body = line.trim_end();
                let rest = body.strip_suffix('.');
                match rest {
                    Some(r)
                        if !in_fence
                            && r.chars().last().is_some_and(char::is_alphanumeric)
                            && !r.rsplit(char::is_whitespace).next().is_some_and(|w| {
                                w.contains('.') || w.eq_ignore_ascii_case("etc")
                            }) =>
                    {
                        format!("{r}{}", &line[body.len()..])
                    }
                    _ => line.to_string(),
                }
            })
            .collect();
    lines.join("\n")
}

/// Put back the casing and full-stop habits of a casual `original` that the
/// round trip tidied: lowercase sentence starts (when most of the original's
/// are lowercase) and no full stop at line ends (when most of its lines end
/// without one). Proper nouns, acronyms, IDs and `I` keep their case.
/// Other registers pass through unchanged.
pub fn restore_surface(original: &str, text: &str) -> String {
    let profile = detect(original);
    if profile.register != Register::Casual {
        return text.to_string();
    }
    let mut out = text.to_string();
    if profile.markers.contains(&"lowercase sentence starts") {
        out = lowercase_starts(original, &out);
    }
    if profile.markers.contains(&"no final full stops") {
        out = strip_final_stops(&out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASUAL: &str = include_str!("../../tests/fixtures/register/casual-dictation.txt");
    const FORMAL: &str = include_str!("../../tests/fixtures/register/formal-notice.txt");
    const TECHNICAL: &str = "The dispatcher coalesces writes that target the same key within a \
        50 ms window. Set `coalesce.enabled` to true to turn it on; the flush interval is read \
        from the environment.\n\n- batches are flushed in order\n- errors are retried twice";

    #[test]
    fn dictated_text_is_casual_with_its_markers() {
        let p = detect(CASUAL);
        assert_eq!(p.register, Register::Casual);
        for m in [
            "lowercase sentence starts",
            "no final full stops",
            "self-corrections and false starts",
            "trailing-off ellipses",
        ] {
            assert!(p.markers.contains(&m), "{m} missing from {:?}", p.markers);
        }
    }

    #[test]
    fn formal_correspondence_is_formal() {
        let p = detect(FORMAL);
        assert_eq!(p.register, Register::Formal, "{:?}", p.markers);
    }

    #[test]
    fn technical_prose_and_short_or_uncased_text_are_neutral() {
        assert_eq!(detect(TECHNICAL).register, Register::Neutral);
        assert_eq!(detect("ok sounds good").register, Register::Neutral);
        assert_eq!(
            detect("请在周五之前确认评审时间，谢谢大家。").register,
            Register::Neutral
        );
        assert_eq!(detect("").register, Register::Neutral);
    }

    #[test]
    fn contractions_alone_are_neither_casual_nor_formal() {
        let t = "We can't ship this week. The review didn't finish, and we'll know more on Friday.";
        assert_eq!(detect(t).register, Register::Neutral);
    }

    #[test]
    fn version_numbers_do_not_split_segments() {
        assert_eq!(
            segments("it is 4.19.0 now. ok"),
            vec!["it is 4.19.0 now", "ok"]
        );
    }

    #[test]
    fn surface_is_restored_for_casual_sources() {
        let tidy = "Hey, Grace Hughes here. It's Monday 1 Dec today.\n\nI need you to dig into INC-2291, \
            the coalescing one. Olivia's booking the review for Thursday. Sam will check every single one, haha.";
        assert_eq!(
            restore_surface(CASUAL, tidy),
            "hey, Grace Hughes here. it's Monday 1 Dec today\n\nI need you to dig into INC-2291, \
            the coalescing one. Olivia's booking the review for Thursday. Sam will check every single one, haha"
        );
    }

    #[test]
    fn surface_keeps_acronyms_ids_and_i_and_leaves_other_registers_alone() {
        let out = restore_surface(
            CASUAL,
            "UTC is fine. I agree. INC-2291 is open. Version 4 ships.",
        );
        assert_eq!(
            out,
            "UTC is fine. I agree. INC-2291 is open. version 4 ships"
        );
        assert_eq!(
            restore_surface(FORMAL, "Dear Ms Hughes,\n\nWe write."),
            "Dear Ms Hughes,\n\nWe write."
        );
        let fenced = restore_surface(CASUAL, "Run this.\n```\nLine one.\n```\ndone.");
        assert!(fenced.contains("Line one."), "{fenced}");
    }

    #[test]
    fn contractions_are_expanded_only_when_the_original_has_none() {
        let o = "I will carry this and we do not retry.";
        assert_eq!(
            expand_contractions(
                o,
                "I'll carry this and we don't retry. It's fine, can’t fail, Olivia's note."
            ),
            "I will carry this and we do not retry. It is fine, cannot fail, Olivia's note."
        );
        let casual = "it's fine, I won't retry";
        assert_eq!(
            expand_contractions(casual, "it's fine, I'll skip"),
            "it's fine, I'll skip"
        );
    }

    #[test]
    fn surface_strips_only_the_final_single_full_stop() {
        assert_eq!(
            strip_final_stops("a b.\nwait...\nsee e.g.\nreally?\nend."),
            "a b\nwait...\nsee e.g.\nreally?\nend"
        );
    }
}
