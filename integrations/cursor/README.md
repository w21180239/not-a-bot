# Cursor

Cursor reads MCP servers from `.cursor/mcp.json` (project) or
`~/.cursor/mcp.json` (global).

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

Reload the window, then the tools are available in Agent mode as
`translate`, `roundtrip`, `providers`, `languages` and `health`.

Credentials resolve from `~/.config/translatex/credentials.toml` and
`TRANSLATEX_*` variables in the environment Cursor inherits. On macOS a GUI
launch does not read your shell profile, so either write the credentials file
or add an `env` block:

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"],
      "env": { "TRANSLATEX_DEEPL_API_KEY": "…" }
    }
  }
}
```
