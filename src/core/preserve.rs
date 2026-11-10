//! Preserve spans: replace matched regions with tokens, translate, restore, verify.

use crate::core::error::{Error, Result};
use regex::Regex;

/// Marker alphabet avoids sequences that translators commonly emit.
const ALPHABET: &[&str] = &[
    "AQ", "ZQ", "QX", "XQ", "VQ", "QV", "WQ", "QW", "JQ", "QJ", "KQ", "QK", "ZV", "VZ", "KW", "WK",
];

#[derive(Debug)]
pub struct Preserver {
    patterns: Vec<Regex>,
    tokens: Vec<String>,
}

impl Preserver {
    pub fn new(patterns: &[String]) -> Result<Option<Self>> {
        if patterns.is_empty() {
            return Ok(None);
        }
        let mut compiled = Vec::with_capacity(patterns.len());
        for p in patterns {
            let pattern = builtin(p).unwrap_or(p.as_str());
            compiled.push(Regex::new(pattern).map_err(|e| Error::BadPattern(format!("{p}: {e}")))?);
        }
        Ok(Some(Self {
            patterns: compiled,
            tokens: Vec::new(),
        }))
    }

    /// Every distinct protected span in `text`, in match order, without
    /// masking: for providers that keep real spans better than placeholders.
    pub fn spans(&self, text: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for span in self.cover(text).1 {
            if !out.contains(&span) {
                out.push(span);
            }
        }
        out
    }

    /// Per-byte token index plus the matched text of each token. Patterns
    /// apply in order; a match that adds no uncovered byte gets no token.
    fn cover(&self, text: &str) -> (Vec<Option<usize>>, Vec<String>) {
        let mut matches: Vec<Option<usize>> = vec![None; text.len()];
        let mut tokens: Vec<String> = Vec::new();
        for re in self.patterns.iter() {
            for m in re.find_iter(text) {
                let contributes = matches[m.start()..m.end()]
                    .iter()
                    .any(|slot| slot.is_none());
                if contributes {
                    let idx = tokens.len();
                    tokens.push(text[m.start()..m.end()].to_string());
                    for slot in &mut matches[m.start()..m.end()] {
                        if slot.is_none() {
                            *slot = Some(idx);
                        }
                    }
                }
            }
        }
        (matches, tokens)
    }

    /// Replace every match with a token. Patterns apply in order; later
    /// patterns cannot re-match text already tokenised.
    pub fn mask(&mut self, text: &str) -> String {
        let (matches, tokens) = self.cover(text);
        self.tokens = tokens;
        // Build the masked string.
        let mut out = String::with_capacity(text.len());
        let mut i = 0;
        while i < text.len() {
            if let Some(idx) = matches[i] {
                out.push_str(&Self::token(idx));
                // Skip the entire original match.
                let mut j = i + 1;
                while j < text.len() && matches[j] == Some(idx) {
                    j += 1;
                }
                i = j;
            } else {
                // `i` is a byte offset; advance by one whole UTF-8 character or
                // multi-byte input gets indexed as if it were ASCII.
                let ch = text[i..]
                    .chars()
                    .next()
                    .expect("byte offset on a char boundary");
                out.push(ch);
                i += ch.len_utf8();
            }
        }
        out
    }

    fn token(idx: usize) -> String {
        let base = ALPHABET.len();
        let suffix = idx.to_string();
        format!(
            "{}{}{}",
            ALPHABET[idx % base],
            suffix,
            ALPHABET[(idx + 7) % base]
        )
    }

    /// Restore tokens. Duplicate or missing tokens are treated as engine
    /// corruption so the caller can fall through to the next provider.
    pub fn restore(&self, translated: &str) -> Result<String> {
        let mut out = translated.to_string();
        for (idx, original) in self.tokens.iter().enumerate() {
            let token = Self::token(idx);
            let hits = out.matches(&token).count();
            if hits == 0 {
                return Err(Error::BadResponse {
                    provider: String::new(),
                    reason: format!("lost preserve token {token}"),
                });
            }
            if hits > 1 {
                return Err(Error::BadResponse {
                    provider: String::new(),
                    reason: format!("duplicated preserve token {token}"),
                });
            }
            out = out.replace(&token, original);
        }
        if let Some(left) = find_stray_token(&out) {
            return Err(Error::BadResponse {
                provider: String::new(),
                reason: format!("invented preserve token {left}"),
            });
        }
        Ok(out)
    }
}

fn find_stray_token(text: &str) -> Option<String> {
    static STRAY: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re = STRAY.get_or_init(|| {
        Regex::new(r"(?:AQ|ZQ|QX|XQ|VQ|QV|WQ|QW|JQ|QJ|KQ|QK|ZV|VZ|KW|WK)\d+(?:AQ|ZQ|QX|XQ|VQ|QV|WQ|QW|JQ|QJ|KQ|QK|ZV|VZ|KW|WK)")
            .unwrap()
    });
    re.find(text).map(|m| m.as_str().to_string())
}

/// Named patterns, so callers need no regex metacharacters: a Windows npm
/// `.cmd` shim runs through cmd.exe, which strips `^` from `[^`]*`.
fn builtin(name: &str) -> Option<&'static str> {
    match name {
        "@code" => Some(r"(?s)```.*?```|`[^`\n]+`"),
        "@url" => Some(r#"https?://[^\s<>"'`)\]]+"#),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_code_and_url_patterns() {
        let mut p = Preserver::new(&["@code".into(), "@url".into()])
            .unwrap()
            .unwrap();
        let src = "Run `make test`, see https://example.com/a?b=1 and\n```\nfn x() {}\n```";
        let masked = p.mask(src);
        assert!(!masked.contains("make test"));
        assert!(!masked.contains("example.com"));
        assert!(!masked.contains("fn x()"));
        assert_eq!(p.restore(&masked).unwrap(), src);
    }

    #[test]
    fn spans_lists_matches_without_masking() {
        let p = Preserver::new(&["@code".into()]).unwrap().unwrap();
        let src = "Run `make test`, then `make test` and `ls`.";
        assert_eq!(p.spans(src), vec!["`make test`", "`ls`"]);
        assert!(p.tokens.is_empty(), "spans must not mask");
    }

    #[test]
    fn no_patterns_is_noop() {
        assert!(Preserver::new(&[]).unwrap().is_none());
    }

    #[test]
    fn masks_and_restores_code_spans() {
        let mut p = Preserver::new(&[r"`[^`]*`".to_string()]).unwrap().unwrap();
        let masked = p.mask("Run `make test` before you push.");
        assert!(!masked.contains("make test"));
        assert!(find_stray_token(&masked).is_some());
        assert_eq!(
            p.restore(&masked).unwrap(),
            "Run `make test` before you push."
        );
    }

    #[test]
    fn multibyte_text_survives_masking() {
        // Byte offsets and char offsets diverge here; indexing one as the other
        // either panics or silently drops characters.
        let mut p = Preserver::new(&[r"`[^`]*`".to_string()]).unwrap().unwrap();
        let src = "推送前先跑 `make test` 再提交。";
        let masked = p.mask(src);
        assert!(!masked.contains("make test"));
        assert!(masked.contains("推送前先跑"));
        assert!(masked.contains("再提交。"));
        assert_eq!(p.restore(&masked).unwrap(), src);
    }

    #[test]
    fn restores_multiple_distinct_tokens() {
        let mut p = Preserver::new(&[r"\{\{\s*\w+\s*\}\}".to_string()])
            .unwrap()
            .unwrap();
        let src = "Hi {{ first }} {{ last }}, ok {{ last2 }}";
        let masked = p.mask(src);
        assert_eq!(p.tokens.len(), 3);
        assert_eq!(p.restore(&masked).unwrap(), src);
    }

    #[test]
    fn adjacent_tokens_survive_restore() {
        let mut p = Preserver::new(&[r"\{\{\s*\w+\s*\}\}".to_string()])
            .unwrap()
            .unwrap();
        let src = "{{a}}{{b}}{{c}}";
        let masked = p.mask(src);
        assert_eq!(p.restore(&masked).unwrap(), src);
    }

    #[test]
    fn repeated_token_is_corruption() {
        let mut p = Preserver::new(&[r"`[^`]*`".to_string()]).unwrap().unwrap();
        let masked = p.mask("a `x` b");
        let broken = format!("{masked}{masked}");
        let err = p.restore(&broken).unwrap_err();
        assert!(err.to_string().contains("duplicated"));
    }

    #[test]
    fn lost_token_is_corruption() {
        let mut p = Preserver::new(&[r"`[^`]*`".to_string()]).unwrap().unwrap();
        p.mask("a `x` b");
        let err = p.restore("translated without token").unwrap_err();
        assert!(err.to_string().contains("lost"));
    }

    #[test]
    fn invented_token_is_corruption() {
        let mut p = Preserver::new(&[r"`[^`]*`".to_string()]).unwrap().unwrap();
        let masked = p.mask("a `x` b");
        let err = p.restore(&format!("{masked} QX99ZQ")).unwrap_err();
        assert!(err.to_string().contains("invented"));
    }

    #[test]
    fn bad_pattern_reports_invalid_pattern() {
        let err = Preserver::new(&["[".to_string()]).unwrap_err();
        assert_eq!(err.code(), "INVALID_PRESERVE_PATTERN");
    }

    #[test]
    fn later_pattern_skips_masked_text() {
        let mut p = Preserver::new(&[r"`[^`]*`".to_string(), r"\S+".to_string()])
            .unwrap()
            .unwrap();
        let masked = p.mask("keep `a b` word");
        assert_eq!(p.restore(&masked).unwrap(), "keep `a b` word");
    }
}
