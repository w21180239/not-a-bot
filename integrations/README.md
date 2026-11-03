# Agent integrations

`translatex` is one binary with two faces: a CLI and an MCP server. Every
harness below can use either, and most want both — the MCP server for the model,
the CLI for scripts.

| Harness | Recommended setup | Config file |
| --- | --- | --- |
| [Claude Code](claude-code/README.md) | Plugin (MCP + skill), or `claude mcp add` | `.mcp.json`, `.claude/skills/` |
| [OpenCode](opencode/README.md) | MCP server, plus a skill for the CLI | `opencode.json`, `~/.config/opencode/skills/` |
| [Codex](codex/README.md) | MCP server, plus an `AGENTS.md` note | `~/.codex/config.toml` |
| [Cursor](cursor/README.md) | Project-level MCP server | `.cursor/mcp.json` |
| [VS Code / Copilot](vscode/README.md) | Workspace MCP server | `.vscode/mcp.json` |
| [Anything else](generic/README.md) | JSON-RPC over stdio | — |

## Verified against

Configs were checked with the real CLIs, not only parsed as JSON:

| Harness | Version | Check | Result |
| --- | --- | --- | --- |
| Claude Code | 2.1.317 | `claude mcp add --scope user` then `claude mcp list` | `✔ Connected` |
| Claude Code | 2.1.317 | `claude plugin marketplace add` + `claude plugin install translatex@translatex` | installed; 1 skill, 1 MCP server, ~84 always-on tokens |
| Codex | 0.164.0 | `codex mcp add` / `list` / `get` | enabled, transport stdio, TOML shape as documented |
| OpenCode | 2.2.5 | `opencode mcp add` then `opencode mcp list` | `✓ translatex connected`, 5 tools discovered |
| Cursor, VS Code | — | not installed on this machine | JSON validated; config follows each client's published schema |

The OpenCode entry in the schema is `mcp.servers.<name>`; the flat
`mcp.<name>` form is still accepted for older configs.

## Install the binary first

```bash
npm install -g translatex          # prebuilt binary for this platform
# or
cargo install translatex --locked
```

Then confirm it is on `PATH`:

```bash
translatex --version
translatex providers --format text
```

## Which channel for which job

| Need | Use |
| --- | --- |
| The model should translate while answering | MCP tool `translate` |
| Strip machine-flavoured phrasing before publishing | MCP tool `roundtrip`, or `translatex roundtrip` |
| A script or CI job needs a translation | `translatex translate --format json` |
| Bulk work from a file | `translatex batch` |
| Health or capability discovery | `translatex providers`, `translatex health` |

The MCP server and the CLI share one config, so a key added for one is visible
to the other. Restart the MCP server after changing credentials: the engine is
built once at startup.
