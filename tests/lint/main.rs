//! `nab lint` end to end: JSON shape, exit codes and the error contract.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn nab(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nab"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn nab");
    {
        let mut pipe = child.stdin.take().expect("stdin");
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).expect("write stdin");
        }
    }
    child.wait_with_output().expect("wait")
}

fn write(dir: &tempfile::TempDir, name: &str, text: &str) -> String {
    let path = dir.path().join(name);
    std::fs::write(&path, text).expect("write file");
    path.to_string_lossy().into_owned()
}

#[test]
fn json_output_has_stable_shape() {
    let dir = tempfile::tempdir().unwrap();
    let draft = write(&dir, "draft.md", "We delve into it.\nRan 3 times.");
    let source = write(&dir, "source.md", "Ran 5 times in src/cli.rs.");
    let out = nab(
        &["lint", &draft, "--source", &source, "--format", "json"],
        None,
    );
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json stdout");
    assert_eq!(v["tells"][0]["kind"], "vocab");
    assert_eq!(v["tells"][0]["line"], 1);
    assert_eq!(v["tells"][0]["match"], "delve");
    assert_eq!(v["tells"][0]["hint"], "look at / dig into");
    assert_eq!(
        v["missingFromDraft"],
        serde_json::json!(["path:src/cli.rs", "number:5"])
    );
    assert_eq!(v["newInDraft"], serde_json::json!(["number:3"]));
}

#[test]
fn text_output_reads_stdin_and_ends_with_footer() {
    let out = nab(
        &["lint", "-", "--format", "text"],
        Some("We delve into it.\n"),
    );
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.starts_with("1: vocab: delve -> look at / dig into\n"),
        "{text}"
    );
    assert!(text.trim_end().ends_with("1 hits"), "{text}");
}

#[test]
fn strict_exits_one_on_tell_or_missing_fact() {
    let dir = tempfile::tempdir().unwrap();
    let clean = write(&dir, "clean.md", "It ran 5 times.");
    let tell = write(&dir, "tell.md", "We delve into it.");
    let lost = write(&dir, "lost.md", "It ran often.");
    let source = write(&dir, "source.md", "It ran 5 times.");

    let ok = nab(&["lint", &clean, "--source", &source, "--strict"], None);
    assert_eq!(ok.status.code(), Some(0));

    let relaxed = nab(&["lint", &tell], None);
    assert_eq!(relaxed.status.code(), Some(0));
    let strict = nab(&["lint", &tell, "--strict"], None);
    assert_eq!(strict.status.code(), Some(1));

    let missing = nab(&["lint", &lost, "--source", &source, "--strict"], None);
    assert_eq!(missing.status.code(), Some(1));
}

#[test]
fn missing_file_follows_json_error_contract() {
    let out = nab(&["lint", "no-such-draft.md"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let v: serde_json::Value = serde_json::from_slice(&out.stderr).expect("json on stderr");
    assert_eq!(v["error"]["code"], "INVALID_INPUT");
    assert!(v["error"]["message"]
        .as_str()
        .unwrap()
        .contains("no-such-draft.md"));
}
