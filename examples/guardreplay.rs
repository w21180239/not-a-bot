//! Replays the drift guard on dumps written by a rewrite run with `NAB_EXP_DUMP=<dir>`.
//!
//! usage: guardreplay <corpus.jsonl> <dump-dir>... > out.jsonl
//! Each output line: {"id","text","putbacks","same"}; `same` is true when the replay
//! matches the text the run returned. The repair hop is not re-run: its saved output
//! is guarded again and kept when it leaves no more put-backs, as the engine does.

use not_a_bot::core::drift::guard;
use serde_json::{json, Value};
use std::collections::HashMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (corpus, dirs) = args
        .split_first()
        .expect("usage: guardreplay <corpus.jsonl> <dump-dir>...");
    let ids: HashMap<String, String> = std::fs::read_to_string(corpus)
        .expect("corpus")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let v: Value = serde_json::from_str(l).expect("corpus line");
            (
                v["text"].as_str().unwrap().trim().to_string(),
                v["id"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    for dir in dirs {
        let mut files: Vec<_> = std::fs::read_dir(dir)
            .expect("dump dir")
            .flatten()
            .map(|e| e.path())
            .collect();
        files.sort();
        for path in files {
            let d: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let source = d["source"].as_str().unwrap();
            let Some(id) = ids.get(source.trim()) else {
                continue;
            };
            let (mut text, mut drift) = guard(source, d["surface"].as_str().unwrap());
            if let Some(fixed) = d["repaired"].as_str() {
                let (g2, d2) = guard(source, fixed);
                if d2.len() <= drift.len() {
                    text = g2;
                    drift = d2;
                }
            }
            let same = d["final"].as_str() == Some(text.as_str());
            println!(
                "{}",
                json!({"id": id, "text": text, "putbacks": drift.len(), "same": same, "drift": drift})
            );
        }
    }
}
