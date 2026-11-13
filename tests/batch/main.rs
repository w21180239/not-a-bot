//! `nab batch --kind lint|rewrite` end to end: ids, order, isolation and per-item errors.

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

fn nab(args: &[&str], stdin: &str) -> (i32, Vec<Value>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nab"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nab");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    let lines = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("ndjson line"))
        .collect();
    (out.status.code().unwrap_or(-1), lines)
}

const REPEATED: &str =
    "Honestly the build is slow. Honestly the tests are flaky. Honestly the docs lag.";

#[test]
fn lint_items_are_isolated_ordered_and_tagged_with_id() {
    let input = format!(
        "{}\n\n{}\n{}\n",
        serde_json::json!({"id": "a", "text": REPEATED}),
        serde_json::json!({"id": 7, "text": "Short plain text."}),
        serde_json::json!({"id": "c", "text": "Draft with a fact 42.", "source": "Draft with a fact 42 and 99."}),
    );
    let (code, rows) = nab(&["batch", "--kind", "lint", "--parallel", "3"], &input);
    assert_eq!(code, 0);
    assert_eq!(rows.len(), 3);
    // Input order, ids echoed, blank lines keep their index slot.
    assert_eq!(rows[0]["id"], "a");
    assert_eq!(rows[1]["id"], 7);
    assert_eq!(rows[2]["id"], "c");
    assert_eq!(rows[0]["index"], 0);
    assert_eq!(rows[1]["index"], 2);
    assert!(rows.iter().all(|r| r["ok"] == true));
    // Same tells as a standalone run: nothing leaks between items.
    let (_, solo) = nab(
        &["batch", "--kind", "lint"],
        &format!("{}\n", serde_json::json!({"text": REPEATED})),
    );
    assert_eq!(rows[0]["result"], solo[0]["result"]);
    assert_eq!(rows[1]["result"]["tells"], serde_json::json!([]));
    assert!(!rows[2]["result"]["missingFromDraft"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn bad_lines_become_error_items_without_aborting() {
    let input = "not json\n{\"id\":\"x\",\"text\":\"ok\"}\n{\"id\":\"y\",\"txt\":\"typo\"}\n[1]\n";
    let (code, rows) = nab(&["batch", "--kind", "lint"], input);
    assert_eq!(code, 0);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["ok"], false);
    assert_eq!(rows[0]["error"]["code"], "INVALID_INPUT");
    assert_eq!(rows[1]["ok"], true);
    assert_eq!(rows[2]["ok"], false);
    assert_eq!(rows[2]["id"], "y");
    assert_eq!(rows[3]["ok"], false);
}

#[test]
fn rewrite_items_take_flag_defaults_and_fail_one_by_one() {
    // An unknown preset fails validation before any provider call, so the
    // flag default reaching each item shows without network access.
    let input = "{\"id\":1,\"text\":\"Hello there.\"}\n{\"id\":2,\"text\":\"Bye.\",\"preset\":\"light\",\"bogus\":1}\n";
    let (code, rows) = nab(
        &[
            "batch",
            "--kind",
            "rewrite",
            "--from",
            "en",
            "--preset",
            "nope",
            "--preserve",
            "@code",
        ],
        input,
    );
    assert_eq!(code, 0);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], 1);
    assert_eq!(rows[0]["ok"], false);
    assert_eq!(rows[0]["error"]["code"], "INVALID_INPUT");
    assert!(rows[0]["error"]["message"]
        .as_str()
        .unwrap()
        .contains("light, standard"));
    // Unknown field is rejected for that line only.
    assert_eq!(rows[1]["id"], 2);
    assert_eq!(rows[1]["error"]["code"], "INVALID_INPUT");
}

#[test]
fn empty_input_is_a_request_error() {
    let (code, _) = nab(&["batch", "--kind", "lint"], "\n\n");
    assert_eq!(code, 1);
}
