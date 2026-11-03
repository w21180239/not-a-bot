# Codex

Codex reads MCP servers from `~/.codex/config.toml` and project guidance from
`AGENTS.md`.

## MCP server

Add to `~/.codex/config.toml`:

```toml
[mcp_servers.translatex]
command = "translatex"
args = ["mcp"]
```

Pass credentials without touching the host config:

```toml
[mcp_servers.translatex]
command = "translatex"
args = ["mcp"]

[mcp_servers.translatex.env]
TRANSLATEX_DEEPL_API_KEY = "…"
```

Tool calls may be slow on a cascade: give the server enough time.

```toml
[mcp_servers.translatex]
command = "translatex"
args = ["mcp"]
tool_timeout_sec = 90
```

## Project guidance

Append [`AGENTS.md`](AGENTS.md) to the repository's `AGENTS.md` so Codex knows
the CLI contract without a prompt. It documents the JSON shapes, the error
codes and the round-trip rule.

## CLI

Codex can call the binary directly when it needs structured output:

```bash
translatex translate "Deploy failed" -t zh --format json
translatex roundtrip "$(cat draft.md)" --from en --format json
```

Use `--format json` and read `text`; keep stderr for diagnostics.
