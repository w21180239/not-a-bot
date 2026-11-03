//! MCP server over stdio.
//!
//! Implements the subset agents need: `initialize`, `tools/list`, `tools/call`,
//! `ping`. Results follow the MCP tool contract — `content` blocks plus
//! `structuredContent` — so clients that validate the schema still work.

use crate::core::config::Settings;
use crate::core::creds::{Credential, CredentialStore};
use crate::core::engine::Engine;
use crate::core::error::{Error, Result};
use crate::core::types::TranslateRequest;
use crate::providers::Registry;
use serde_json::{json, Value};
use std::sync::Arc;

/// Longest text block a tool result will carry; agents read structured payloads.
const MAX_INLINE: usize = 1 << 20;

pub struct Server {
    engine: Engine,
    registry: Arc<Registry>,
    creds: CredentialStore,
}

impl Server {
    /// Build the server from the same config and credential files the CLI uses,
    /// so an MCP client sees configured keys and the configured chain.
    pub fn build() -> Result<Self> {
        let settings = Settings::load(None)?;
        let creds = CredentialStore::load(None)?;
        if let Some(w) = creds.warning() {
            tracing::warn!("{w}");
        }
        let registry = Arc::new(crate::cli::build_registry(&creds, &settings));
        let engine = Engine::new(
            registry.clone(),
            creds.clone(),
            crate::core::config::cache_dir(),
            settings,
        );
        Ok(Self { engine, registry, creds })
    }

    /// Tool catalogue. `inputSchema` is an object, as the MCP schema requires;
    /// the previous `#/definitions/...` string was rejected by strict clients.
    pub fn tools() -> Value {
        let translate_schema = serde_json::to_value(schemars::schema_for!(
            crate::core::types::TranslateRequest
        ))
        .unwrap_or_else(|_| json!({ "type": "object" }));

        json!({
            "tools": [
                {
                    "name": "translate",
                    "description": "Translate text between languages. Walks the provider fallback chain and rotates keys automatically. Returns the translated text plus the provider that served it.",
                    "inputSchema": translate_schema,
                    "outputSchema": serde_json::to_value(schemars::schema_for!(
                        crate::core::types::TranslateResponse
                    )).unwrap_or_else(|_| json!({ "type": "object" })),
                    "annotations": { "readOnlyHint": true, "idempotentHint": true }
                },
                {
                    "name": "providers",
                    "description": "List translation providers with kind, priority, language support and whether credentials are configured.",
                    "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
                },
                {
                    "name": "languages",
                    "description": "List canonical language codes, optionally restricted to one provider's advertised set.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "provider": { "type": "string", "description": "Provider name, or 'all'" }
                        },
                        "additionalProperties": false
                    }
                },
                {
                    "name": "health",
                    "description": "Probe every configured provider and report latency and errors.",
                    "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
                }
            ]
        })
    }

    async fn call(&self, name: &str, args: Value) -> std::result::Result<Value, Error> {
        // Some clients send `arguments` as a JSON string.
        let args = match args {
            Value::String(s) => serde_json::from_str(&s)
                .map_err(|e| Error::InvalidInput(format!("arguments: {e}")))?,
            other => other,
        };

        match name {
            "translate" => {
                let req: TranslateRequest = serde_json::from_value(args)
                    .map_err(|e| Error::InvalidInput(format!("arguments: {e}")))?;
                let out = self.engine.translate(&req).await?;
                Ok(serde_json::to_value(&out.response).unwrap_or(Value::Null))
            }
            "providers" => Ok(json!({ "providers": self.registry.describe() })),
            "languages" => {
                let provider = args
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("all");
                let codes: Vec<String> = if provider == "all" {
                    crate::core::lang::LANGUAGES
                        .iter()
                        .map(|(c, _)| c.to_string())
                        .collect()
                } else {
                    let info = self
                        .registry
                        .get(provider)
                        .ok_or_else(|| Error::ProviderNotFound {
                            name: provider.to_string(),
                        })?
                        .info();
                    if info.languages.iter().any(|l| l == "*") {
                        crate::core::lang::LANGUAGES
                            .iter()
                            .map(|(c, _)| c.to_string())
                            .collect()
                    } else {
                        info.languages
                            .iter()
                            .filter(|c| crate::core::lang::is_known(c))
                            .cloned()
                            .collect()
                    }
                };
                Ok(json!({ "provider": provider, "languages": codes }))
            }
            "health" => {
                let mut records = Vec::new();
                for provider in self.registry.iter() {
                    let info = provider.info();
                    let keys = self.creds.keys(&info.name);
                    if info.needs_key && keys.is_empty() {
                        records.push(json!({
                            "provider": info.name,
                            "configured": false,
                            "healthy": false,
                            "error": "no credentials"
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
                        Ok(()) => records.push(json!({
                            "provider": info.name,
                            "configured": true,
                            "healthy": true,
                            "latencyMs": started.elapsed().as_millis() as u64
                        })),
                        Err(e) => records.push(json!({
                            "provider": info.name,
                            "configured": !info.needs_key,
                            "healthy": false,
                            "error": e.to_string()
                        })),
                    }
                }
                let healthy = records.iter().all(|r| {
                    !r["configured"].as_bool().unwrap_or(false)
                        || r["healthy"].as_bool().unwrap_or(false)
                });
                Ok(json!({ "healthy": healthy, "providers": records }))
            }
            other => Err(Error::ProviderNotFound { name: other.to_string() }),
        }
    }

    /// A tool result per the MCP contract: one text block carrying the JSON,
    /// the same payload as `structuredContent`, and `isError` on failure.
    async fn call_tool(&self, name: &str, args: Value) -> Value {
        match self.call(name, args).await {
            Ok(payload) => {
                let text = payload.to_string();
                json!({
                    "content": [{
                        "type": "text",
                        "text": if text.len() > MAX_INLINE { text[..MAX_INLINE].to_string() } else { text }
                    }],
                    "structuredContent": payload,
                    "isError": false
                })
            }
            Err(err) => {
                let payload = json!({ "error": err.wire() });
                json!({
                    "content": [{ "type": "text", "text": payload.to_string() }],
                    "structuredContent": payload,
                    "isError": true
                })
            }
        }
    }
}

fn write_line(out: &mut impl std::io::Write, value: &Value) -> Result<()> {
    writeln!(out, "{value}").map_err(|e| Error::Internal(format!("write: {e}")))?;
    out.flush().map_err(|e| Error::Internal(format!("flush: {e}")))
}

pub async fn serve() -> Result<()> {
    use std::io::BufRead;

    let server = Server::build()?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| Error::Internal(e.to_string()))?;
        if read == 0 {
            break;
        }
        let trimmed = line.trim().trim_start_matches('\u{FEFF}');
        if trimmed.is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = req.get("id").cloned();

        // Notifications carry no id and must not be answered.
        if id.is_none() || method.starts_with("notifications/") {
            continue;
        }
        let id = id.unwrap();

        let reply = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": {
                        "name": "translatex",
                        "version": crate::core::types::VERSION
                    }
                }
            }),
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": Server::tools()
            }),
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or_default();
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
                let result = server.call_tool(name, arguments).await;
                json!({ "jsonrpc": "2.0", "id": id, "result": result })
            }
            other => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("method not found: {other}") }
            }),
        };

        write_line(&mut writer, &reply)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_catalogue_is_spec_shaped() {
        let doc = Server::tools();
        let tools = doc["tools"].as_array().expect("tools array");
        assert_eq!(tools.len(), 4);
        for tool in tools {
            assert!(tool.get("name").and_then(|n| n.as_str()).is_some());
            let schema = tool.get("inputSchema").expect("inputSchema");
            assert!(
                schema.is_object(),
                "inputSchema must be an object, got {schema}"
            );
            assert_eq!(schema["type"], "object");
        }
    }

    #[test]
    fn translate_schema_lists_required_fields() {
        let doc = Server::tools();
        let translate = doc["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == "translate")
            .unwrap();
        let required = translate["inputSchema"]["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "text"));
        assert!(required.iter().any(|v| v == "targetLang"));
    }
}
