# Agent integrations

`translatex` integrates as **a CLI plus a skill** — one binary the agent can run,
one document telling it when and how. No MCP server, no protocol handshake: the
agent calls the command and reads JSON.

```bash
npm install -g translatex     # or: cargo install translatex
translatex --version
```

The skill text is identical for every harness; only the place it goes differs.
[`skill/translatex/SKILL.md`](skill/translatex/SKILL.md) is the canonical copy,
and [`guidance/AGENTS.md`](guidance/AGENTS.md) is the short block for harnesses
that take a project instructions file.

## Harness matrix

Every harness supported by FreeLLMAPI is listed. "Guidance" is how it learns the
workflow; "Where" is the file to write.

| Harness | Guidance | Where |
| --- | --- | --- |
| Claude Code | Skill | `.claude/skills/translatex/SKILL.md` (project) or `~/.claude/skills/translatex/SKILL.md` |
| Claude Code | Plugin | `/plugin marketplace add w21180239/translatex` then `/plugin install translatex@translatex` |
| Codex CLI | Skill | `~/.agents/skills/translatex/SKILL.md` |
| Codex CLI | Instructions | `AGENTS.md` at the repo root, or `~/.codex/AGENTS.md` |
| Gemini CLI | Skill | `~/.agents/skills/translatex/SKILL.md` |
| Gemini CLI | Instructions | `GEMINI.md` |
| OpenCode | Skill | `~/.config/opencode/skills/translatex/SKILL.md` |
| OpenCode | Instructions | `AGENTS.md` |
| Cursor | Rule | `.cursor/rules/translatex.mdc` |
| Cline | Rule | `.clinerules/translatex.md` |
| Roo Code | Rule | `.roo/rules/translatex.md` |
| Kilo Code | Rule | `.kilocode/rules/translatex.md` |
| Continue | Rule | `.continue/rules/translatex.md` |
| JetBrains AI | Rule | `.aiassistant/rules/translatex.md` |
| Zed | Rule | `.rules` |
| Aider | Instructions | `CONVENTIONS.md`, read with `--read CONVENTIONS.md` |
| Qwen Code | Instructions | `QWEN.md` |
| Crush | Instructions | `CRUSH.md` |
| Goose | Instructions | project instructions file |
| DeepSeek Harness | Instructions | `AGENTS.md` |
| MiMo Code | Instructions | `AGENTS.md` |
| AtomCode | Instructions | `AGENTS.md` |
| OpenClaw | Instructions | `AGENTS.md` |
| Hermes Agent | Instructions | `AGENTS.md` |
| Pi | Instructions | `AGENTS.md` |
| Reasonix | Instructions | `AGENTS.md` |
| ZCode | Instructions | `AGENTS.md` |
| QwenPaw | Instructions | project instructions file |
| Anything else | CLI | the agent runs `translatex … --format json` directly |

`AGENTS.md` is the cross-vendor convention; a tool that documents a different
instructions filename reads the same content under that name. Paths for Claude
Code, Codex and OpenCode were verified against the installed CLIs; the rest are
the vendor-documented locations for the same convention.

## Install the skill once

```bash
# Claude Code
mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/

# OpenCode
mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/

# Codex and Gemini CLI (shared cross-runtime location)
mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/

# Anything with an instructions file
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

PowerShell, same operations: `New-Item -ItemType Directory -Force <dir>` then
`Copy-Item <src> <dir>`.

## Which channel for which job

| Need | Command |
| --- | --- |
| The agent translates while answering | `translatex translate "…" -t <lang> --format json` |
| Remove machine cadence from prose | `translatex roundtrip "…" --from <lang> --format json` |
| A script or CI job needs a translation | same CLI, parse `text` |
| Bulk work | `translatex batch --file jobs.ndjson` |
| Capability discovery | `translatex providers`, `translatex languages`, `translatex health` |

The CLI reads `~/.config/translatex/config.toml` and `credentials.toml`, or
`TRANSLATEX_*` environment variables. Free web endpoints need neither, so the
skill works on a fresh machine with only the binary installed.

## Per-harness pages

- [Claude Code](claude-code/README.md) — skill, plugin, project instructions
- [Codex CLI](codex/README.md)
- [OpenCode](opencode/README.md)
- [Gemini CLI](gemini-cli/README.md)
- [Cursor](cursor/README.md)
- [Rules-file harnesses](harnesses/rules.md) — Cline, Roo, Kilo, Continue, JetBrains, Zed, Aider
- [Instructions-file harnesses](harnesses/instructions.md) — Qwen Code, Crush, Goose and the AGENTS.md tools
