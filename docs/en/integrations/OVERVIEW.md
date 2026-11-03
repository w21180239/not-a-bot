**English** · [简体中文](../../zh-cn/integrations/OVERVIEW.md)

# Integrations — Overview

`translatex` ships as one binary with two surfaces: a CLI and an MCP server.
Harnesses differ in which one they can consume, so both are documented with
ready-to-copy config.

## Distribution

| Channel | Command | Notes |
| --- | --- | --- |
| npm | `npm install -g translatex` | Downloads the prebuilt binary for the platform at install time |
| Cargo | `cargo install translatex --locked` | Builds from source; any platform with Rust 1.80+ |
| Release archive | [Releases](https://github.com/w21180239/translatex/releases/latest) | `translatex-<target>.tar.gz` with the binary at the root |

The npm package is a shim: `postinstall` fetches the archive from the matching
GitHub release, and `bin/translatex.js` execs it. Offline installs can set
`TRANSLATEX_BINARY_PATH` to any local build.

## Harness matrix

| Harness | Recommended | Files |
| --- | --- | --- |
| Claude Code | Plugin (MCP + skill) | [`integrations/claude-code/`](../../../integrations/claude-code/README.md) |
| OpenCode | MCP server + skill | [`integrations/opencode/`](../../../integrations/opencode/README.md) |
| Codex | MCP server + `AGENTS.md` | [`integrations/codex/`](../../../integrations/codex/README.md) |
| Cursor | Project MCP server | [`integrations/cursor/`](../../../integrations/cursor/README.md) |
| VS Code / Copilot | Workspace MCP server | [`integrations/vscode/`](../../../integrations/vscode/README.md) |
| Any other MCP client | JSON-RPC over stdio | [`integrations/generic/`](../../../integrations/generic/README.md) |

Verified with the real CLIs: Claude Code 2.1.317 (`claude mcp list` →
Connected; plugin market install → 1 skill + 1 MCP server), Codex 0.164.0
(`codex mcp get` → stdio transport) and OpenCode 2.2.5 (`opencode mcp list` →
connected, five tools). Cursor and VS Code were not installed on the test
machine, so their configs follow the published schema without a live check.

Skills for harnesses that read `SKILL.md` (Claude Code, OpenCode):
[`integrations/skill/SKILL.md`](../../../integrations/skill/SKILL.md).

## Which surface

| Need | Surface |
| --- | --- |
| Model translates mid-conversation | MCP `translate` |
| Model removes machine cadence | MCP `roundtrip` |
| Script, CI, pre-commit | CLI with `--format json` |
| Bulk | CLI `batch` (NDJSON in, NDJSON out) |
| Capability discovery | MCP `providers` / `languages` / `health` |

Both surfaces read the same `config.toml` and `credentials.toml`, so a key
configured for the CLI is visible to the MCP server after a restart.

## Credentials in agent environments

GUI-launched agents (Cursor, VS Code) often do not inherit shell environment
variables. Two reliable options:

1. Write `~/.config/translatex/credentials.toml`; it is read by both surfaces.
2. Add an `env` block to the MCP server entry — every client that supports
   `mcpServers` accepts one.

Free web providers need neither; the keyed APIs are the only ones that require
configuration.

## Navigation

- ← [Documentation index](../README.md)
- → [Round trips](../roundtrip/OVERVIEW.md)
