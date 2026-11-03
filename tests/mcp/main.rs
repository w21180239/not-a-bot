//! MCP protocol tests.
//!
//! Spawns the real binary and speaks JSON-RPC over stdio, because the failure
//! mode this guards against (a tool result a client cannot parse) only shows up
//! across a process boundary.

use std::io::Write;
use std::process::{Command, Stdio};

/// Run one MCP session: write every request, close stdin, collect replies.
fn exchange(messages: &[String]) -> Vec<serde_json::Value> {
    let home = tempfile::TempDir::new().expect("temp home");
    let mut child = Command::new(env!("CARGO_BIN_EXE_translatex"))
        .arg("mcp")
        .env("TRANSLATEX_HOME", home.path())
        .env_remove("TRANSLATEX_CONFIG")
        .env_remove("TRANSLATEX_CREDENTIALS")
        .env_remove("TRANSLATEX_CHAIN")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn translatex mcp");

    {
        let mut stdin = child.stdin.take().expect("stdin");
        for message in messages {
            writeln!(stdin, "{message}").expect("write request");
        }
    }

    let output = child.wait_with_output().expect("wait for mcp");
    assert!(
        output.status.success(),
        "mcp exited with {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("stdout is not one JSON object per line: {line:?}: {e}"))
        })
        .collect()
}

fn rpc(id: u64, method: &str, params: serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
    .to_string()
}

fn initialize(id: u64) -> String {
    rpc(
        id,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "translatex-tests", "version": "0" }
        }),
    )
}

fn by_id(replies: &[serde_json::Value], id: u64) -> &serde_json::Value {
    replies
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no reply for id {id}: {replies:?}"))
}

#[test]
fn initialize_negotiates_the_protocol() {
    let replies = exchange(&[initialize(1)]);
    assert_eq!(replies.len(), 1);

    let init = by_id(&replies, 1);
    assert_eq!(init["jsonrpc"], "2.0");
    assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(init["result"]["serverInfo"]["name"], "translatex");
    assert_eq!(init["result"]["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(init["result"]["capabilities"]["tools"]["listChanged"], false);
}

#[test]
fn notifications_are_never_answered() {
    let replies = exchange(&[
        initialize(1),
        serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
            .to_string(),
    ]);
    assert_eq!(
        replies.len(),
        1,
        "a notification must not produce a reply: {replies:?}"
    );
}

#[test]
fn tools_list_is_spec_shaped() {
    let replies = exchange(&[initialize(1), rpc(2, "tools/list", serde_json::json!({}))]);
    let tools = by_id(&replies, 2)["result"]["tools"]
        .as_array()
        .expect("tools array");

    let mut names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().expect("name"))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec!["health", "languages", "providers", "roundtrip", "translate"]
    );

    for tool in tools {
        let schema = tool
            .get("inputSchema")
            .unwrap_or_else(|| panic!("{} has no inputSchema", tool["name"]));
        assert!(
            schema.is_object(),
            "{} inputSchema must be an object, got {schema}",
            tool["name"]
        );
        assert_eq!(schema["type"], "object");
        assert!(
            tool.get("description").and_then(|d| d.as_str()).is_some(),
            "{} has no description",
            tool["name"]
        );
    }
}

#[test]
fn translate_tool_rejects_empty_text_with_a_typed_error() {
    let replies = exchange(&[
        initialize(1),
        rpc(
            2,
            "tools/call",
            serde_json::json!({
                "name": "translate",
                "arguments": { "text": "   ", "targetLang": "zh" }
            }),
        ),
    ]);

    let result = &by_id(&replies, 2)["result"];
    assert_eq!(result["isError"], true);

    let content = result["content"].as_array().expect("content array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "text");
    assert!(content[0]["text"].as_str().is_some_and(|t| !t.is_empty()));

    // The same payload is available structured, which is what agents read.
    assert_eq!(result["structuredContent"]["error"]["code"], "INVALID_INPUT");
    assert_eq!(result["structuredContent"]["error"]["retryable"], false);
    assert_eq!(
        result["structuredContent"]["error"]["suggestedAction"],
        "fix_input"
    );
}

#[test]
fn roundtrip_tool_rejects_an_unknown_pivot() {
    let replies = exchange(&[
        initialize(1),
        rpc(
            2,
            "tools/call",
            serde_json::json!({
                "name": "roundtrip",
                "arguments": { "text": "hello", "pivot": "klingon" }
            }),
        ),
    ]);

    let result = &by_id(&replies, 2)["result"];
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["error"]["code"], "INVALID_INPUT");
}

#[test]
fn languages_tool_returns_canonical_codes() {
    let replies = exchange(&[
        initialize(1),
        rpc(
            2,
            "tools/call",
            serde_json::json!({
                "name": "languages",
                "arguments": { "provider": "google" }
            }),
        ),
    ]);

    let payload = &by_id(&replies, 2)["result"]["structuredContent"];
    assert_eq!(payload["provider"], "google");
    let codes: Vec<&str> = payload["languages"]
        .as_array()
        .expect("languages array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(codes.contains(&"en"));
    assert!(codes.contains(&"zh"));
    assert!(codes.contains(&"auto"));
}

#[test]
fn ping_and_unknown_methods_follow_json_rpc() {
    let replies = exchange(&[
        initialize(1),
        rpc(2, "ping", serde_json::json!({})),
        rpc(3, "does/not/exist", serde_json::json!({})),
    ]);

    assert_eq!(by_id(&replies, 2)["result"], serde_json::json!({}));
    let err = &by_id(&replies, 3)["error"];
    assert_eq!(err["code"], -32601);
    assert!(err["message"].as_str().is_some_and(|m| m.contains("does/not/exist")));
}

#[test]
fn unknown_tools_report_an_error_result_not_a_crash() {
    let replies = exchange(&[
        initialize(1),
        rpc(
            2,
            "tools/call",
            serde_json::json!({ "name": "nope", "arguments": {} }),
        ),
    ]);

    let result = &by_id(&replies, 2)["result"];
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"]["error"]["code"],
        "PROVIDER_NOT_FOUND"
    );
}
