**English** · [简体中文](../../zh-cn/integrations/OVERVIEW.md)

# Integrations — Overview

[← Documentation index](../README.md)

`not-a-bot` integrates as a CLI plus a skill. The agent runs one command and
reads JSON; a short document tells it when to run it. There is no server to
start and no protocol to negotiate.

## Distribution

| Channel | Command | Notes |
| --- | --- | --- |
| npm | `npm install -g not-a-bot` | Pulls the per-platform binary package; no install script runs |
| Cargo | `cargo install not-a-bot --locked` | Builds from source; any platform with Rust 1.89+ |
| Release archive | [Releases](https://github.com/w21180239/not-a-bot/releases/latest) | `not-a-bot-<target>.tar.gz` with the binary at the root |

The npm package is a shim. The binary lives in a per-platform package
(`not-a-bot-linux-x64`, `not-a-bot-darwin-arm64`, `not-a-bot-windows-x64`, …)
declared as an optional dependency, so npm installs the right one and nothing
has to run at install time. `bin/not-a-bot.js` execs it. Offline installs can
set `NOT_A_BOT_BINARY_PATH` to any local build.

## Two documents, every harness

| Document | Use for |
| --- | --- |
| [`integrations/skill/not-a-bot/SKILL.md`](../../../integrations/skill/not-a-bot/SKILL.md) | Harnesses that load skills: Claude Code, Codex, Gemini CLI, OpenCode |
| [`integrations/guidance/AGENTS.md`](../../../integrations/guidance/AGENTS.md) | Any harness with a project instructions or rules file |

The full matrix — Claude Code, Codex CLI, Gemini CLI, OpenCode, Cursor, Cline,
Roo Code, Kilo Code, Continue, JetBrains AI, Zed, Aider, Qwen Code, Crush, Goose,
DeepSeek Harness, MiMo Code, AtomCode, OpenClaw, Hermes Agent, Pi, Reasonix,
ZCode, QwenPaw — with the file path for each is
[`integrations/README.md`](../../../integrations/README.md).

| Page | Contents |
| --- | --- |
| [02-supported-harnesses.md](02-supported-harnesses.md) | Logo grid, the four channels, and a path plus command for each of the 24 harnesses |

## Which surface

| Need | Command |
| --- | --- |
| Model translates mid-conversation | `not-a-bot translate "…" -t <lang> --format json` |
| Model humanises AI-sounding text | `not-a-bot rewrite "…" --from <lang> --preset standard --preserve @code --format json` |
| Script, CI, pre-commit | same CLI, parse `text` |
| Bulk | `not-a-bot batch` (NDJSON in, NDJSON out) |
| Capability discovery | `not-a-bot providers`, `not-a-bot languages`, `not-a-bot health` |

## Credentials in agent environments

GUI-launched agents (Cursor, VS Code) often do not inherit shell environment
variables. Write `~/.config/not-a-bot/credentials.toml`; the CLI reads it from
any process. Free web providers need no credentials at all, so the skill works
on a fresh machine with only the binary installed.

## Navigation

- ← [Documentation index](../README.md)
- → [Round trips](../rewrite/OVERVIEW.md)
