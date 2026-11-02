use crate::core::creds::Credential;
use crate::core::engine::Engine;
use crate::core::types::TranslateRequest;
use crate::core::error::{Error, Result};
use serde_json::Value;

async fn handle_tool_call(name: &str, args: Value) -> Value {
    // Handle case where arguments might be a JSON string instead of an object
    let args = match args {
        Value::String(s) => serde_json::from_str(&s).unwrap_or(Value::Null),
        other => other,
    };
    match name {
        "translate" => {
            let req: TranslateRequest = serde_json::from_value(args).unwrap_or(TranslateRequest {
                text: String::new(),
                source_lang: "auto".into(),
                target_lang: "zh".into(),
                provider: None,
                fallback_chain: Vec::new(),
                preserve: Vec::new(),
                timeout_ms: None,
                idempotency_key: None,
                metadata: Default::default(),
            });
            let settings = crate::core::config::Settings::default();
            let creds = crate::core::creds::CredentialStore::default();
            let engine = Engine::new(
                std::sync::Arc::new(crate::cli::build_registry(&creds, &settings)),
                creds,
                crate::core::config::cache_dir(),
                settings.retries(),
            );
            match engine.translate(&req).await {
                Ok(out) => serde_json::to_value(&out.response).unwrap_or(Value::Null),
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            }
        }
        "providers" => {
            let settings = crate::core::config::Settings::default();
            let creds = crate::core::creds::CredentialStore::default();
            let registry = crate::cli::build_registry(&creds, &settings);
            let providers = registry.describe();
            serde_json::json!({ "providers": providers })
        }
        "languages" => {
            let provider = args.get("provider").and_then(|v| v.as_str()).unwrap_or("all");
            let codes: Vec<String> = if provider == "all" {
                crate::core::lang::LANGUAGES.iter().map(|(c, _)| c.to_string()).collect()
            } else {
                crate::core::lang::LANGUAGES
                    .iter()
                    .filter(|(c, _)| crate::core::lang::provider_supports(provider, c))
                    .map(|(c, _)| c.to_string())
                    .collect()
            };
            serde_json::json!({ "provider": provider, "languages": codes })
        }
        "health" => {
            let settings = crate::core::config::Settings::default();
            let creds = crate::core::creds::CredentialStore::default();
            let registry = crate::cli::build_registry(&creds, &settings);
            let mut records = Vec::new();
            for provider in registry.iter() {
                let info = provider.info();
                let keys = creds.keys(&info.name);
                if info.needs_key && keys.is_empty() {
                    records.push(serde_json::json!({
                        "provider": info.name,
                        "configured": false,
                        "healthy": false,
                        "latency_ms": Value::Null,
                        "error": Some("no credentials")
                    }));
                    continue;
                }
                let cred = keys.first().cloned().unwrap_or_else(|| Credential {
                    provider: info.name.clone(),
                    label: "default".into(),
                    fields: Default::default(),
                });
                let started = std::time::Instant::now();
                match provider.probe(&cred).await {
                    Ok(()) => records.push(serde_json::json!({
                        "provider": info.name,
                        "configured": true,
                        "healthy": true,
                        "latency_ms": started.elapsed().as_millis() as u64,
                        "error": Value::Null
                    })),
                    Err(e) => records.push(serde_json::json!({
                        "provider": info.name,
                        "configured": !info.needs_key,
                        "healthy": false,
                        "latency_ms": Value::Null,
                        "error": Some(e.to_string())
                    })),
                }
            }
            let all_healthy = records.iter().all(|r| !r["configured"].as_bool().unwrap_or(false) || r["healthy"].as_bool().unwrap_or(false));
            serde_json::json!({ "healthy": all_healthy, "providers": records })
        }
        _ => serde_json::json!({ "error": "Unknown tool" }),
    }
}

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
                "tools/call" => {
                    let params = req.get("params").cloned().unwrap_or_default();
                    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let arguments = params.get("arguments").cloned().unwrap_or_default();
                    let result = crate::mcp::handle_tool_call(tool_name, arguments).await;
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": result
                    })
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
