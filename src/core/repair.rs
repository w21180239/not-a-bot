//! Structural minimality for `fix` hops: the model proposes span edits and
//! this module decides which to apply, so a repair cannot quietly turn into
//! copying the original back.

use crate::core::drift::drift_count;
use crate::core::facts::FUNCTION_WORDS;
use crate::core::types::{EditKind, HopEdit};
use serde::Deserialize;
use std::collections::HashSet;

/// Largest total rise in content-word overlap with the original that the
/// non-fact edits of one fix hop may cause together.
pub const MAX_STYLE_OVERLAP_RISE: f64 = 0.10;

/// Words that pinpoint edits may bring back from the original, as a share of
/// the rewrite's word count.
pub const MAX_PINPOINT_SHARE: f64 = 0.30;

#[derive(Debug, Deserialize)]
struct RawEdit {
    from: String,
    to: String,
    #[serde(default)]
    why: String,
    /// `register` marks a voice repair, `term` a changed term, form or condition put
    /// back; anything else is a meaning repair.
    #[serde(default)]
    kind: String,
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
            kind: match e.kind.to_lowercase().as_str() {
                "register" => EditKind::Register,
                "term" => EditKind::Term,
                _ => EditKind::Meaning,
            },
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
/// 1. Fact edits (applying one shrinks [`drift_count`] against `original`: lost
///    facts, flipped negations, changed hedges and the like)
///    always apply and spend no budget.
/// 2. The rest share one budget: content-word overlap with `original` may
///    rise at most [`MAX_STYLE_OVERLAP_RISE`] above the text after pass 1, so
///    many small edits cannot together copy the original back.
///    Pinpoint `term` edits (see [`pinpoint_words`]) are exempt up to
///    [`MAX_PINPOINT_SHARE`] of the text's words.
/// 3. Register edits (`kind: register`, restoring voice markers) get the same
///    budget again, counted from the text after pass 2, and are skipped when
///    they would lose a fact.
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
        if !matches!(edit.kind, EditKind::Register | EditKind::Term) {
            edit.kind = EditKind::Meaning;
        }
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
    let mut missing = drift_count(original, &current);
    for edit in edits
        .iter_mut()
        .filter(|e| e.reason.is_none() && e.kind != EditKind::Register)
    {
        if let Ok(candidate) = replace(&current, edit) {
            let left = drift_count(original, &candidate);
            if left < missing {
                missing = left;
                current = candidate;
                edit.kind = EditKind::Fact;
                edit.applied = true;
            }
        }
    }

    let mut baseline = overlap(&current, original);
    let max_pinpoint = (MAX_PINPOINT_SHARE * current.split_whitespace().count() as f64) as usize;
    let mut pinpointed = 0;
    for edit in edits
        .iter_mut()
        .filter(|e| e.reason.is_none() && !e.applied && e.kind != EditKind::Register)
    {
        match replace(&current, edit) {
            Err(reason) => edit.reason = Some(reason.into()),
            Ok(candidate) => {
                let pin = (edit.kind == EditKind::Term)
                    .then(|| pinpoint_words(edit, original))
                    .flatten();
                if pin.is_some_and(|n| pinpointed + n <= max_pinpoint) {
                    pinpointed += pin.unwrap_or(0);
                    current = candidate;
                    baseline = overlap(&current, original);
                    edit.applied = true;
                } else if overlap(&candidate, original) - baseline > MAX_STYLE_OVERLAP_RISE {
                    edit.reason = Some("copies the original back".into());
                } else {
                    current = candidate;
                    edit.applied = true;
                }
            }
        }
    }

    // Register edits bring voice markers back, so they must not starve the
    // meaning edits above: own budget, and never at the cost of a fact.
    let baseline = overlap(&current, original);
    let mut missing = drift_count(original, &current);
    for edit in edits
        .iter_mut()
        .filter(|e| e.reason.is_none() && e.kind == EditKind::Register)
    {
        match replace(&current, edit) {
            Err(reason) => edit.reason = Some(reason.into()),
            Ok(candidate) => {
                let left = drift_count(original, &candidate);
                if left > missing {
                    edit.reason = Some("drops a fact".into());
                } else if overlap(&candidate, original) - baseline > MAX_STYLE_OVERLAP_RISE {
                    edit.reason = Some("copies the original back".into());
                } else {
                    missing = left;
                    current = candidate;
                    edit.applied = true;
                }
            }
        }
    }
    (current, edits)
}

/// Words an edit brings back, when it is a pinpoint repair: `from` of at most
/// 3 words, `to` of at most 4, and every word `to` adds appears in `original`
/// (a changed term put back). `None` for any other edit.
fn pinpoint_words(edit: &HopEdit, original: &str) -> Option<usize> {
    let norm = |s: &str| -> Vec<String> {
        s.split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect()
    };
    let (from, to) = (norm(&edit.from), norm(&edit.to));
    if from.len() > 3 || to.len() > 4 {
        return None;
    }
    let known = norm(original);
    let added: Vec<_> = to.iter().filter(|w| !from.contains(w)).collect();
    (!added.is_empty() && added.iter().all(|w| known.contains(w))).then_some(added.len())
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
    use crate::core::facts::missing_facts;

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

    fn register_edit(from: &str, to: &str) -> HopEdit {
        HopEdit {
            kind: EditKind::Register,
            ..edit(from, to)
        }
    }

    #[test]
    fn register_kind_is_parsed_and_defaults_to_meaning() {
        let raw = r#"[{"from": "Hey", "to": "hey", "kind": "register"},
            {"from": "Aloha", "to": "Aroha", "kind": "meaning"}, {"from": "a", "to": "b"}]"#;
        let kinds: Vec<_> = parse_edits(raw).unwrap().iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![EditKind::Register, EditKind::Meaning, EditKind::Meaning]
        );
    }

    #[test]
    fn register_edits_have_their_own_budget_and_keep_their_kind() {
        let original = "hey, we ship alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima for Aroha";
        let rewrite = "Hey, we ship red orange yellow green blue indigo violet black white grey brown pink for Aroha.";
        let (text, edits) = apply_edits(
            rewrite,
            original,
            vec![
                edit("red", "alpha"),
                edit("orange", "bravo"),
                edit("yellow", "charlie"),
                register_edit("Hey,", "hey,"),
                register_edit("Aroha.", "Aroha"),
            ],
        );
        // The third style edit is over budget, the register edits are not.
        let got: Vec<_> = edits
            .iter()
            .map(|e| (e.from.as_str(), e.applied, e.kind))
            .collect();
        use EditKind::{Meaning as M, Register as R};
        assert_eq!(
            got,
            vec![
                ("Hey,", true, R),
                ("red", true, M),
                ("orange", true, M),
                ("yellow", false, M),
                ("Aroha.", true, R)
            ]
        );
        assert!(text.starts_with("hey, we ship alpha bravo yellow"));
        assert!(text.ends_with("for Aroha"));
    }

    #[test]
    fn register_edit_that_drops_a_fact_is_skipped() {
        let original = "hey, pay NZD 18,900 to Aroha";
        let rewrite = "Hey, pay NZD 18,900 to Aroha.";
        let (text, edits) = apply_edits(
            rewrite,
            original,
            vec![
                register_edit("NZD 18,900", "money"),
                register_edit("Hey", "hey"),
            ],
        );
        assert_eq!(text, "hey, pay NZD 18,900 to Aroha.");
        assert_eq!(edits[0].reason.as_deref(), None);
        assert_eq!(edits[1].reason.as_deref(), Some("drops a fact"));
    }

    fn term_edit(from: &str, to: &str) -> HopEdit {
        HopEdit {
            kind: EditKind::Term,
            ..edit(from, to)
        }
    }

    #[test]
    fn small_term_edits_skip_the_budget_but_untagged_ones_do_not() {
        let original = "we ship alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima after 6 attempts";
        let rewrite = "we ship red orange yellow green blue indigo violet black white grey brown pink after 6 retries";
        let (text, edits) = apply_edits(
            rewrite,
            original,
            vec![
                edit("red", "alpha"),
                edit("orange", "bravo"),
                edit("yellow", "charlie"),
                term_edit("retries", "attempts"),
            ],
        );
        let got: Vec<_> = edits.iter().map(|e| (e.from.as_str(), e.applied)).collect();
        assert_eq!(
            got,
            vec![
                ("red", true),
                ("orange", true),
                ("yellow", false),
                ("retries", true)
            ]
        );
        assert!(text.ends_with("6 attempts"));
        assert_eq!(edits[3].kind, EditKind::Term);
        // The same edit untagged shares the exhausted budget.
        let (_, edits) = apply_edits(
            rewrite,
            original,
            vec![
                edit("red", "alpha"),
                edit("orange", "bravo"),
                edit("yellow", "charlie"),
                edit("retries", "attempts"),
            ],
        );
        assert!(!edits[2].applied && !edits[3].applied);
    }

    #[test]
    fn term_edits_that_bring_in_words_not_in_the_original_use_the_budget() {
        let original = "we ship alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima after 6 attempts";
        assert_eq!(
            pinpoint_words(&term_edit("retries", "tries"), original),
            None
        );
        assert_eq!(
            pinpoint_words(&term_edit("retries", "attempts"), original),
            Some(1)
        );
        assert_eq!(
            pinpoint_words(&term_edit("a b c d", "attempts"), original),
            None,
            "from of four words"
        );
    }

    #[test]
    fn restoring_a_flipped_negation_is_a_free_fact_style_edit() {
        let original = "Do not push the change before the window opens on Wednesday";
        let rewrite = "Push the change once the window opens on Wednesday";
        let (text, edits) = apply_edits(
            rewrite,
            original,
            vec![edit(
                "Push the change once",
                "Do not push the change before",
            )],
        );
        assert!(edits[0].applied, "{:?}", edits[0].reason);
        assert_eq!(edits[0].kind, EditKind::Fact);
        assert!(text.starts_with("Do not push the change before the window"));
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
