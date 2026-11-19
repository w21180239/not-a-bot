//! Replays the fix hop's edit review and the drift guard on saved rewrite outputs.
//!
//! usage: fixreplay <src-dir> <out-dir> > out.jsonl
//! `<src-dir>/<name>` holds the source text, `<out-dir>/<name>.json` the rewrite JSON
//! (last hop = fix, with edits). Recorded `fact` kinds are reset to meaning, since the
//! review assigns that kind itself. Each line: {"id","facts","logic","fixed","guarded","applied"}.

use not_a_bot::core::drift::{content, fix_facts, guard, logic_issues};
use not_a_bot::core::repair::apply_edits;
use not_a_bot::core::types::{EditKind, HopEdit};
use serde_json::{json, Value};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [src, out] = args.as_slice() else {
        panic!("usage: fixreplay <src-dir> <out-dir>");
    };
    let mut names: Vec<_> = std::fs::read_dir(src)
        .expect("src dir")
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    for name in names {
        let source = std::fs::read_to_string(format!("{src}/{name}")).expect("source");
        let Ok(raw) = std::fs::read_to_string(format!("{out}/{name}.json")) else {
            continue;
        };
        let v: Value = serde_json::from_str(&raw).expect("rewrite json");
        let hops = v["hops"].as_array().expect("hops");
        let (Some(fix), Some(pre)) = (hops.last(), hops.get(hops.len().wrapping_sub(2))) else {
            continue;
        };
        let pre = pre["text"].as_str().unwrap_or_default();
        let edits: Vec<HopEdit> = serde_json::from_value::<Vec<HopEdit>>(fix["edits"].clone())
            .unwrap_or_default()
            .into_iter()
            .map(|mut e| {
                if e.kind == EditKind::Fact {
                    e.kind = EditKind::Meaning;
                }
                e
            })
            .collect();
        // FIXREPLAY_NONE: skip every edit, to bound what the fix hop costs.
        let (fixed, edits) = if std::env::var("FIXREPLAY_NONE").is_ok() {
            (pre.to_string(), edits)
        } else {
            apply_edits(pre, &source, edits)
        };
        let (guarded, drift) = guard(&source, &fixed);
        let applied: Vec<Value> = edits
            .iter()
            .map(|e| json!({"kind": e.kind, "applied": e.applied, "reason": e.reason, "from": e.from, "to": e.to}))
            .collect();
        println!(
            "{}",
            json!({
                "id": name.trim_end_matches(".txt"),
                "facts": fix_facts(&source, pre),
                "logic": logic_issues(&content(&source), pre),
                "fixed": fixed, "guarded": guarded, "drift": drift, "applied": applied,
            })
        );
    }
}
