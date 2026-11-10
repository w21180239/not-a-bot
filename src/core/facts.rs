//! Fact guard for rewrites: tokens of the input that must survive verbatim.
//!
//! Machine translation respells names (Aroha -> Aloha) and expands codes (NZD
//! -> "New Zealand Dinar"); an LLM can drop a number. This lists what the
//! output lost so a caller can reject or patch it.

use once_cell::sync::Lazy;
use regex::Regex;
use std::ops::Range;

static CODE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`([^`\n]+)`").unwrap());
static URL: Lazy<Regex> = Lazy::new(|| Regex::new(r#"https?://[^\s<>"'`]+"#).unwrap());
static PATH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?:[A-Za-z]:\\|\.{1,2}/|~/|/)?[A-Za-z0-9_.-]+(?:[/\\][A-Za-z0-9_.-]+)+|\b[A-Za-z0-9_-]+\.(?:rs|py|js|ts|tsx|jsx|md|json|toml|ya?ml|txt|csv|html|css|sh|ps1|go|java|c|h|cpp|log|xml|pdf|png|jpe?g|zip|exe|dll|lock|sql)\b",
    )
    .unwrap()
});
static TICKET: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b[A-Z][A-Z0-9]{1,9}-\d+\b").unwrap());
static MENTION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:^|[^A-Za-z0-9_@])(@[A-Za-z0-9_](?:[A-Za-z0-9_.-]*[A-Za-z0-9_])?)").unwrap()
});
static NUMBER: Lazy<Regex> = Lazy::new(|| Regex::new(r"\d+(?:[.,:]\d+)*").unwrap());
static WORD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Z\u{C0}-\u{DE}][A-Za-z\u{C0}-\u{24F}]+").unwrap());

/// Capitalised words that are not names even mid-sentence.
const STOP: &[&str] = &["OK", "Mr", "Mrs", "Ms", "Dr", "St", "AM", "PM", "The", "An"];

/// Lowercase English function words and common sentence starters. A
/// sentence-initial capitalised word in this list is not a name. Weekdays and
/// months are deliberately absent: they are facts.
pub const FUNCTION_WORDS: &[&str] = &[
    "a",
    "about",
    "above",
    "across",
    "after",
    "afterwards",
    "again",
    "against",
    "all",
    "almost",
    "also",
    "although",
    "always",
    "am",
    "among",
    "an",
    "and",
    "another",
    "any",
    "anyone",
    "anything",
    "anyway",
    "are",
    "around",
    "as",
    "ask",
    "at",
    "back",
    "be",
    "because",
    "been",
    "before",
    "being",
    "below",
    "besides",
    "between",
    "both",
    "but",
    "by",
    "can",
    "could",
    "currently",
    "did",
    "do",
    "does",
    "done",
    "down",
    "during",
    "each",
    "either",
    "else",
    "even",
    "ever",
    "every",
    "everyone",
    "everything",
    "finally",
    "first",
    "for",
    "from",
    "further",
    "furthermore",
    "get",
    "go",
    "had",
    "has",
    "have",
    "having",
    "he",
    "her",
    "here",
    "hers",
    "herself",
    "hi",
    "him",
    "himself",
    "his",
    "how",
    "however",
    "i",
    "if",
    "in",
    "indeed",
    "instead",
    "into",
    "is",
    "it",
    "its",
    "itself",
    "just",
    "last",
    "later",
    "least",
    "less",
    "let",
    "like",
    "likewise",
    "many",
    "may",
    "maybe",
    "me",
    "meanwhile",
    "might",
    "more",
    "moreover",
    "most",
    "much",
    "must",
    "my",
    "myself",
    "neither",
    "never",
    "nevertheless",
    "next",
    "no",
    "none",
    "nor",
    "not",
    "note",
    "now",
    "of",
    "off",
    "often",
    "ok",
    "on",
    "once",
    "one",
    "only",
    "or",
    "other",
    "others",
    "otherwise",
    "our",
    "ours",
    "ourselves",
    "out",
    "over",
    "overall",
    "perhaps",
    "please",
    "plus",
    "rather",
    "really",
    "regardless",
    "second",
    "see",
    "she",
    "should",
    "similarly",
    "since",
    "so",
    "some",
    "someone",
    "something",
    "sometimes",
    "soon",
    "still",
    "such",
    "sure",
    "than",
    "thank",
    "thanks",
    "that",
    "the",
    "their",
    "theirs",
    "them",
    "themselves",
    "then",
    "there",
    "therefore",
    "these",
    "they",
    "this",
    "those",
    "though",
    "through",
    "thus",
    "to",
    "today",
    "together",
    "tomorrow",
    "too",
    "toward",
    "towards",
    "under",
    "unless",
    "until",
    "up",
    "upon",
    "us",
    "usually",
    "very",
    "was",
    "we",
    "well",
    "were",
    "what",
    "whatever",
    "when",
    "whenever",
    "where",
    "whereas",
    "wherever",
    "whether",
    "which",
    "while",
    "who",
    "whoever",
    "whom",
    "whose",
    "why",
    "will",
    "with",
    "within",
    "without",
    "would",
    "yes",
    "yesterday",
    "yet",
    "you",
    "your",
    "yours",
    "yourself",
    "yourselves",
];

/// Tokens of `source` absent from `output`: code spans (inner text), URLs,
/// paths, ticket IDs, @mentions and numbers always; when both texts are Latin
/// script, capitalised words as whole words: case-sensitive mid-sentence, and
/// case-insensitive at sentence start unless the word is a common starter or
/// appears lowercase elsewhere in the source.
/// Numbers compare by digits, so `18,900` matches `18900` and `4:50pm` matches
/// `4:50 PM`.
pub fn missing_facts(source: &str, output: &str) -> Vec<String> {
    let mut missing: Vec<String> = Vec::new();
    let mut taken: Vec<Range<usize>> = Vec::new();
    let flag = |token: &str, missing: &mut Vec<String>| {
        if !missing.iter().any(|m| m == token) {
            missing.push(token.to_string());
        }
    };

    for cap in CODE.captures_iter(source) {
        let whole = cap.get(0).unwrap();
        taken.push(whole.range());
        let inner = cap.get(1).unwrap().as_str();
        if !output.contains(inner) {
            flag(inner, &mut missing);
        }
    }
    let structured: [&Lazy<Regex>; 3] = [&URL, &PATH, &TICKET];
    for re in structured {
        for m in re.find_iter(source) {
            if overlaps(&taken, &m.range()) {
                continue;
            }
            let token = m
                .as_str()
                .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}', '\'', '"']);
            taken.push(m.range());
            if !output.contains(token) {
                flag(token, &mut missing);
            }
        }
    }
    for cap in MENTION.captures_iter(source) {
        let m = cap.get(1).unwrap();
        if overlaps(&taken, &m.range()) {
            continue;
        }
        taken.push(m.range());
        if !output.contains(m.as_str()) {
            flag(m.as_str(), &mut missing);
        }
    }

    let output_numbers: Vec<String> = NUMBER
        .find_iter(output)
        .map(|m| digits(m.as_str()))
        .collect();
    for m in NUMBER.find_iter(source) {
        if overlaps(&taken, &m.range()) {
            continue;
        }
        let token = m.as_str().trim_end_matches(['.', ',', ':']);
        if !output_numbers.contains(&digits(token)) {
            flag(token, &mut missing);
        }
    }

    if is_latin(source) && is_latin(output) {
        for m in WORD.find_iter(source) {
            let word = m.as_str();
            if overlaps(&taken, &m.range())
                || STOP.contains(&word)
                || touches_word(source, &m.range())
            {
                continue;
            }
            if sentence_initial(source, m.start()) {
                // Capitalised only by position: a name unless it is a common
                // starter or appears lowercase elsewhere. Case-insensitive, so
                // "Deployment" survives as "deployment".
                let lower = word.to_lowercase();
                if FUNCTION_WORDS.contains(&lower.as_str()) || contains_word(source, &lower) {
                    continue;
                }
                if !contains_word(&output.to_lowercase(), &lower) {
                    flag(word, &mut missing);
                }
                continue;
            }
            if !contains_word(output, word) {
                flag(word, &mut missing);
            }
        }
    }
    missing
}

/// `word` as a whole word, so `Tomi` does not vouch for `Tom` (`Tom's` does).
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let after = text[i + word.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn overlaps(taken: &[Range<usize>], r: &Range<usize>) -> bool {
    taken.iter().any(|t| t.start < r.end && r.start < t.end)
}

fn digits(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

/// Mid-word capitals (`iPhone`, `macOS`) are not standalone names.
fn touches_word(text: &str, r: &Range<usize>) -> bool {
    text[..r.start]
        .chars()
        .next_back()
        .is_some_and(char::is_alphanumeric)
}

/// First word of the text, a sentence, a line or a list item.
fn sentence_initial(text: &str, start: usize) -> bool {
    for c in text[..start].chars().rev() {
        match c {
            '\n' => return true,
            c if c.is_whitespace() => continue,
            '"' | '\'' | '\u{201C}' | '\u{2018}' | '(' | '[' | '*' | '-' | '\u{2022}' | '>'
            | '#' => continue,
            '.' | '!' | '?' | ':' | ';' => return true,
            _ => return false,
        }
    }
    true
}

/// Most letters are Latin script; texts with no letters count as Latin.
fn is_latin(text: &str) -> bool {
    let (mut latin, mut total) = (0usize, 0usize);
    for c in text.chars().filter(|c| c.is_alphabetic()) {
        total += 1;
        if c.is_ascii_alphabetic() || ('\u{C0}'..='\u{24F}').contains(&c) {
            latin += 1;
        }
    }
    total == 0 || latin * 10 >= total * 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respelled_name_is_missing() {
        let src = "Please ask Aroha to approve it.";
        assert_eq!(
            missing_facts(src, "Please ask Aloha to approve it."),
            vec!["Aroha"]
        );
        assert!(missing_facts(src, "Ask Aroha for approval.").is_empty());
    }

    #[test]
    fn numbers_compare_by_digits() {
        let src = "Budget is NZD 18,900, due at 4:50pm.";
        assert!(missing_facts(src, "The budget, NZD 18900, is due 4:50 PM.").is_empty());
        assert_eq!(
            missing_facts(src, "The budget, New Zealand Dinar 18900, is due 4:50 PM."),
            vec!["NZD"]
        );
        assert_eq!(
            missing_facts(src, "Budget NZD 18,000 due 4:50pm."),
            vec!["18,900"]
        );
        assert!(missing_facts("Meet at 4:50.", "Meet at 4:50 today.").is_empty());
    }

    #[test]
    fn structured_tokens_are_checked() {
        let src = "See `cargo test` in src/core/engine.rs, ticket ABC-123, \
                   https://example.com/a?b=1. Ping @wei.";
        let out = "See cargo in engine, ticket ABC-12, https://example.com/a. Ping wei.";
        let missing = missing_facts(src, out);
        assert_eq!(
            missing,
            vec![
                "cargo test",
                "https://example.com/a?b=1",
                "src/core/engine.rs",
                "ABC-123",
                "@wei"
            ]
        );
    }

    #[test]
    fn sentence_initial_names_are_checked() {
        let src = "Ship it today. Aroha signs off.";
        assert_eq!(
            missing_facts(src, "Ship it today. Aloha signs off."),
            vec!["Aroha"]
        );
        assert!(missing_facts(src, "Ship it today; Aroha signs off.").is_empty());
    }

    #[test]
    fn sentence_initial_common_words_compare_case_insensitively() {
        let src = "Deployment starts Monday.";
        assert!(missing_facts(src, "The deployment starts Monday.").is_empty());
        assert_eq!(
            missing_facts(src, "Rollout starts Monday."),
            vec!["Deployment"]
        );
        // Its lowercase form elsewhere marks it as an ordinary word.
        let src = "Release notes. The release is late.";
        assert!(missing_facts(src, "It is late.").is_empty());
    }

    #[test]
    fn names_match_whole_words() {
        let src = "Approval from Tom by Friday.";
        assert!(missing_facts(src, "Tom's approval by Friday.").is_empty());
        assert_eq!(
            missing_facts(src, "Tomi's approval by Friday."),
            vec!["Tom"]
        );
    }

    #[test]
    fn sentence_initial_and_stop_words_are_skipped() {
        let src = "This works. However it needs Tom. OK then.";
        assert_eq!(
            missing_facts(src, "It works but needs someone."),
            vec!["Tom"]
        );
    }

    #[test]
    fn names_skip_non_latin_output_but_numbers_do_not() {
        let src = "Ask Aroha about NZD 18,900.";
        assert!(missing_facts(src, "问问 Aroha 关于 18900 新西兰元的事").is_empty());
        assert_eq!(
            missing_facts(src, "问问阿罗哈关于新西兰元的事"),
            vec!["18,900"]
        );
    }
}
