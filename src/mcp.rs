use crate::core::error::{Error, Result};
use serde_json::Value;

pub async fn serve() -> Result<()> {
    use std::io::{BufRead, Write};

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdin_lock = stdin.lock();
    let mut stdout_lock = stdout.lock();
    let mut line = String::new();

    while stdin_lock.read_line(&mut line).map_err(|e| Error::Internal(e.to_string()))? > 0 {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            line.clear();
            continue;
        }

        // Strip BOM if present
        let trimmed = trimmed.trim_start_matches('\u{FEFF}');

        if let Ok(req) = serde_json::from_str::<Value>(trimmed) {
            let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = req.get("id").cloned();

            let resp = match method {
                "initialize" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "translatex", "version": crate::core::types::VERSION }
                    }
                }),
                "tools/list" => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": crate::cli::manifest_document().unwrap_or_default()
                }),
                "notifications/initialized" => {
                    line.clear();
                    continue;
                }
                _ => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": "Method not found" }
                }),
            };

            let out = serde_json::to_string(&resp).unwrap_or_default();
            let _ = writeln!(stdout_lock, "{}", out);
            let _ = stdout_lock.flush();
        }

        line.clear();
    }

    Ok(())
}
