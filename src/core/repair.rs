//! Structural minimality for `fix` hops: the model proposes span edits and
//! this module decides which to apply, so a repair cannot quietly turn into
//! copying the original back.

use crate::core::facts::{missing_facts, FUNCTION_WORDS};
use crate::core::types::{EditKind, HopEdit};
use serde::Deserialize;
use std::collections::HashSet;

/// Largest total rise in content-word overlap with the original that the
/// non-fact edits of one fix hop may cause together.
pub const MAX_STYLE_OVERLAP_RISE: f64 = 0.10;

#[derive(Debug, Deserialize)]
struct RawEdit {
    from: String,
    to: String,
    #[serde(default)]
    why: String,
}

/// Edits from model output: tolerates a code fence or prose around the
/// outermost `[...]`. `Err` carries the reason the output is unusable.
pub fn parse_edits(raw: &str) -> Result<Vec<HopEdit>, String> {
    let (Some(start), Some(end)) = (raw.find('['), raw.rfind(']')) else {
        return Err("no JSON array in output".into());
    };
    if end < start {
        return Err("no JSON array in output".into());
    }
    let edits: Vec<RawEdit> =
        serde_json::from_str(&raw[start..=end]).map_err(|e| format!("bad edit JSON: {e}"))?;
    Ok(edits
        .into_iter()
        .map(|e| HopEdit {
            from: e.from,
            to: e.to,
            why: e.why,
            kind: EditKind::Meaning,
            applied: false,
            reason: None,
        })
        .collect())
}

/// Apply edits in two passes, each in text order (position in `text`); the
/// result lists edits in text order. An edit is skipped, with `reason`, when
/// its `from` is empty, unchanged, not exactly once in the current text,
/// longer than max(40 chars, 25% of the text), or when `to` exceeds 3x `from`
/// + 80 chars.
///
/// 1. Fact edits (applying one shrinks [`missing_facts`] against `original`)
///    always apply and spend no budget.
/// 2. The rest share one budget: content-word overlap with `original` may
///    rise at most [`MAX_STYLE_OVERLAP_RISE`] above the text after pass 1, so
///    many small edits cannot together copy the original back.
pub fn apply_edits(text: &str, original: &str, edits: Vec<HopEdit>) -> (String, Vec<HopEdit>) {
    let max_from = 40.max(text.chars().count() / 4);
    let mut ordered: Vec<(usize, HopEdit)> = edits
        .into_iter()
        .map(|e| (text.find(&e.from).unwrap_or(usize::MAX), e))
        .collect();
    ordered.sort_by_key(|(pos, _)| *pos);
    let mut edits: Vec<HopEdit> = ordered.into_iter().map(|(_, e)| e).collect();

    for edit in &mut edits {
        let from_len = edit.from.chars().count();
        edit.applied = false;
        edit.kind = EditKind::Meaning;
        edit.reason = if edit.from.is_empty() {
            Some("empty from".to_string())
        } else if edit.from == edit.to {
            Some("no change".into())
        } else if from_len > max_from {
            Some(format!("from longer than {max_from} chars"))
        } else if edit.to.chars().count() > 3 * from_len + 80 {
            Some("to too long for from".into())
        } else {
            None
        };
    }
    // The single replacement of `from`, or why there is none.
    let replace = |current: &str, edit: &HopEdit| match current.matches(edit.from.as_str()).count()
    {
        0 => Err("from not found"),
        1 => Ok(current.replacen(edit.from.as_str(), &edit.to, 1)),
        _ => Err("from not unique"),
    };

    let mut current = text.to_string();
    let mut missing = missing_facts(original, &current).len();
    for edit in edits.iter_mut().filter(|e| e.reason.is_none()) {
        if let Ok(candidate) = replace(&current, edit) {
            let left = missing_facts(original, &candidate).len();
            if left < missing {
                missing = left;
                current = candidate;
                edit.kind = EditKind::Fact;
                edit.applied = true;
            }
        }
    }

    let baseline = overlap(&current, original);
    for edit in edits
        .iter_mut()
        .filter(|e| e.reason.is_none() && !e.applied)
    {
        match replace(&current, edit) {
            Err(reason) => edit.reason = Some(reason.into()),
            Ok(candidate) => {
                if overlap(&candidate, original) - baseline > MAX_STYLE_OVERLAP_RISE {
                    edit.reason = Some("copies the original back".into());
                } else {
                    current = candidate;
                    edit.applied = true;
                }
            }
        }
    }
    (current, edits)
}

/// Jaccard similarity of lowercase content words (function words dropped).
pub fn overlap(a: &str, b: &str) -> f64 {
    let words = |s: &str| -> HashSet<String> {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.chars().count() > 1)
            .map(str::to_lowercase)
            .filter(|w| !FUNCTION_WORDS.contains(&w.as_str()))
            .collect()
    };
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL: &str = "This update is not just a bug fix, it's a fundamental shift. \
        Aroha needs approval from Tom by Friday for NZD 18,900, before the board meets. \
        The team ships after approval.";
    const REWRITE: &str = "This release reshapes how the product works at a basic level. \
        Aloha needs Tom's sign-off by Friday for NZD 18,000. \
        The team will ship once that lands.";

    fn edit(from: &str, to: &str) -> HopEdit {
        HopEdit {
            from: from.into(),
            to: to.into(),
            why: "test".into(),
            kind: EditKind::Meaning,
            applied: false,
            reason: None,
        }
    }

    #[test]
    fn exact_single_matches_apply_left_to_right() {
        let (text, edits) = apply_edits(
            REWRITE,
            ORIGINAL,
            vec![edit("18,000", "18,900"), edit("Aloha", "Aroha")],
        );
        assert!(text.contains("Aroha needs Tom's sign-off by Friday for NZD 18,900."));
        assert!(text.starts_with("This release reshapes"));
        assert_eq!(edits[0].from, "Aloha", "ordered by position");
        assert!(edits.iter().all(|e| e.applied && e.reason.is_none()));
    }

    #[test]
    fn duplicate_or_missing_matches_are_skipped() {
        let (text, edits) = apply_edits(
            REWRITE,
            ORIGINAL,
            vec![edit("at", "on"), edit("Bob", "Tom")],
        );
        assert_eq!(text, REWRITE);
        assert_eq!(edits[0].reason.as_deref(), Some("from not unique"));
        assert_eq!(edits[1].reason.as_deref(), Some("from not found"));
    }

    #[test]
    fn oversize_edits_are_rejected() {
        let sentence = "This release reshapes how the product works at a basic level.";
        let (text, edits) = apply_edits(REWRITE, ORIGINAL, vec![edit(sentence, "x")]);
        assert_eq!(text, REWRITE);
        assert!(edits[0]
            .reason
            .as_deref()
            .unwrap()
            .starts_with("from longer than"));

        let long = "y".repeat(200);
        let (_, edits) = apply_edits(REWRITE, ORIGINAL, vec![edit("basic", &long)]);
        assert_eq!(edits[0].reason.as_deref(), Some("to too long for from"));
    }

    #[test]
    fn copy_back_is_rejected() {
        let original = "Aroha pays. The fundamental shift happened.";
        let rewrite = "Aroha pays. The big change.";
        let (text, edits) = apply_edits(
            rewrite,
            original,
            vec![edit("big change", "fundamental shift happened")],
        );
        assert_eq!(text, rewrite);
        assert_eq!(edits[0].reason.as_deref(), Some("copies the original back"));
    }

    // Each style edit below lifts overlap by about 0.04: two fit the 0.10
    // budget, a third does not. Under a per-edit rule all of them would pass.
    const WORDS: &str = "we ship alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima for Aroha.";
    const COLOURS: &str =
        "we ship red orange yellow green blue indigo violet black white grey brown pink for Aloha.";

    fn applied(edits: &[HopEdit]) -> Vec<(&str, bool, EditKind)> {
        edits
            .iter()
            .map(|e| (e.from.as_str(), e.applied, e.kind))
            .collect()
    }

    #[test]
    fn style_edits_share_one_cumulative_budget() {
        let (text, edits) = apply_edits(
            COLOURS,
            WORDS,
            vec![
                edit("red", "alpha"),
                edit("orange", "bravo"),
                edit("yellow", "charlie"),
                edit("green", "delta"),
            ],
        );
        use EditKind::Meaning as M;
        assert_eq!(
            applied(&edits),
            vec![
                ("red", true, M),
                ("orange", true, M),
                ("yellow", false, M),
                ("green", false, M)
            ]
        );
        assert_eq!(edits[2].reason.as_deref(), Some("copies the original back"));
        assert!(text.contains("alpha bravo yellow green"));
    }

    #[test]
    fn fact_edit_applies_after_style_edits_used_the_budget() {
        let (text, edits) = apply_edits(
            COLOURS,
            WORDS,
            vec![
                edit("red", "alpha"),
                edit("orange", "bravo"),
                edit("yellow", "charlie"),
                edit("Aloha", "Aroha"),
            ],
        );
        use EditKind::{Fact as F, Meaning as M};
        assert_eq!(
            applied(&edits),
            vec![
                ("red", true, M),
                ("orange", true, M),
                ("yellow", false, M),
                ("Aloha", true, F)
            ]
        );
        assert!(text.ends_with("for Aroha."));
        assert!(missing_facts(WORDS, &text).is_empty());
    }

    #[test]
    fn fact_edit_goes_first_whatever_the_json_order() {
        let original = "Aroha signs off alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima.";
        let rewrite = "Aloha signs off red orange yellow green blue indigo violet black white grey brown pink.";
        let (text, edits) = apply_edits(
            rewrite,
            original,
            vec![
                edit("yellow", "charlie"),
                edit("orange", "bravo"),
                edit("red", "alpha"),
                edit("Aloha", "Aroha"),
            ],
        );
        let order: Vec<_> = edits.iter().map(|e| e.from.as_str()).collect();
        assert_eq!(order, vec!["Aloha", "red", "orange", "yellow"]);
        assert_eq!(edits[0].kind, EditKind::Fact);
        assert!(edits[0].applied);
        assert!(edits[1].applied && edits[2].applied && !edits[3].applied);
        assert!(text.starts_with("Aroha signs off alpha bravo yellow"));
    }

    #[test]
    fn clause_insertion_is_accepted() {
        let (text, edits) = apply_edits(
            REWRITE,
            ORIGINAL,
            vec![edit("18,000", "18,000, before the board meets")],
        );
        assert!(edits[0].applied, "{:?}", edits[0].reason);
        assert!(text.contains("NZD 18,000, before the board meets."));
    }

    #[test]
    fn parsing_is_lenient_but_never_falls_back_to_text() {
        let fenced = "```json\n[{\"from\": \"Aloha\", \"to\": \"Aroha\", \"why\": \"name\"}]\n```";
        let edits = parse_edits(fenced).unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].to, "Aroha");
        assert!(parse_edits("[]").unwrap().is_empty());
        assert!(parse_edits("Here is the fixed text.").is_err());
        assert!(parse_edits("[not json]").is_err());
    }
}
