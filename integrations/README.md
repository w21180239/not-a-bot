# Agent integrations

`translatex` integrates as **a CLI plus a skill** — one binary the agent runs,
one document telling it when and how. No server, no protocol handshake: the agent
calls the command and reads JSON. The same setup works in 24 harnesses; only the
place the document goes differs.

```bash
npm install -g translatex     # or: cargo install translatex
translatex --version
```

[`skill/translatex/SKILL.md`](skill/translatex/SKILL.md) is the canonical skill,
[`guidance/AGENTS.md`](guidance/AGENTS.md) the short block for harnesses that
read project instructions, and [`cursor/translatex.mdc`](cursor/translatex.mdc)
the Cursor rule with frontmatter.

<div align="center">
<table>
<tr>
<td align="center" width="150"><img src="../repo-assets/agents/claude-code.png" width="44" alt="Claude Code"><br/><b>Claude Code</b><br/><sub>skill · plugin</sub></td>
<td align="center" width="150"><img src="../repo-assets/agents/codex.png" width="44" alt="Codex CLI"><br/><b>Codex CLI</b><br/><sub>skill · AGENTS.md</sub></td>
<td align="center" width="150"><img src="../repo-assets/agents/gemini-cli.png" width="44" alt="Gemini CLI"><br/><b>Gemini CLI</b><br/><sub>skill · GEMINI.md</sub></td>
<td align="center" width="150"><img src="../repo-assets/agents/opencode.png" width="44" alt="OpenCode"><br/><b>OpenCode</b><br/><sub>skill · AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../repo-assets/agents/cursor.png" width="44" alt="Cursor"><br/><b>Cursor</b><br/><sub>.cursor/rules</sub></td>
<td align="center"><img src="../repo-assets/agents/cline.png" width="44" alt="Cline"><br/><b>Cline</b><br/><sub>.clinerules</sub></td>
<td align="center"><img src="../repo-assets/agents/roo-code.png" width="44" alt="Roo Code"><br/><b>Roo Code</b><br/><sub>.roo/rules</sub></td>
<td align="center"><img src="../repo-assets/agents/kilo-code.png" width="44" alt="Kilo Code"><br/><b>Kilo Code</b><br/><sub>.kilocode/rules</sub></td>
</tr>
<tr>
<td align="center"><img src="../repo-assets/agents/continue.png" width="44" alt="Continue"><br/><b>Continue</b><br/><sub>.continue/rules</sub></td>
<td align="center"><img src="../repo-assets/agents/jetbrains.png" width="44" alt="JetBrains AI"><br/><b>JetBrains AI</b><br/><sub>.aiassistant/rules</sub></td>
<td align="center"><img src="../repo-assets/agents/zed.png" width="44" alt="Zed"><br/><b>Zed</b><br/><sub>.rules</sub></td>
<td align="center"><img src="../repo-assets/agents/aider.png" width="44" alt="Aider"><br/><b>Aider</b><br/><sub>CONVENTIONS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../repo-assets/agents/qwen-code.png" width="44" alt="Qwen Code"><br/><b>Qwen Code</b><br/><sub>QWEN.md</sub></td>
<td align="center"><img src="../repo-assets/agents/crush.png" width="44" alt="Crush"><br/><b>Crush</b><br/><sub>CRUSH.md</sub></td>
<td align="center"><img src="../repo-assets/agents/goose.png" width="44" alt="Goose"><br/><b>Goose</b><br/><sub>instructions</sub></td>
<td align="center"><img src="../repo-assets/agents/deepseek-harness.png" width="44" alt="DeepSeek Harness"><br/><b>DeepSeek Harness</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../repo-assets/agents/mimo-code.svg" width="44" alt="MiMo Code"><br/><b>MiMo Code</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../repo-assets/agents/atomcode.png" width="44" alt="AtomCode"><br/><b>AtomCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../repo-assets/agents/openclaw.png" width="44" alt="OpenClaw"><br/><b>OpenClaw</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../repo-assets/agents/hermes-agent.png" width="44" alt="Hermes Agent"><br/><b>Hermes Agent</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../repo-assets/agents/pi.png" width="44" alt="Pi"><br/><b>Pi</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../repo-assets/agents/reasonix.png" width="44" alt="Reasonix"><br/><b>Reasonix</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../repo-assets/agents/zcode.png" width="44" alt="ZCode"><br/><b>ZCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../repo-assets/agents/qwenpaw.svg" width="44" alt="QwenPaw"><br/><b>QwenPaw</b><br/><sub>instructions</sub></td>
</tr>
</table>
</div>

Logos belong to their projects and identify the harness they point at.

## The four channels

| Channel | What the harness reads | Loaded | Harnesses |
| --- | --- | --- | --- |
| Skill | `SKILL.md` from a skills directory | On demand, when its description matches the task | Claude Code, Codex CLI, Gemini CLI, OpenCode |
| Rule | A markdown file in a project rules directory | Per project | Cursor, Cline, Roo Code, Kilo Code, Continue, JetBrains AI, Zed |
| Instructions | The project instructions file, appended | Whole session | Aider, Qwen Code, Crush, Goose, DeepSeek Harness, MiMo Code, AtomCode, OpenClaw, Hermes Agent, Pi, Reasonix, ZCode, QwenPaw |
| CLI | Nothing | — | Any host that can run a shell command |

`AGENTS.md` is the cross-vendor convention; a tool that documents a different
filename reads the same content under that name. Claude Code, Codex and OpenCode
paths were verified against the installed CLIs; the rest follow each vendor's
documented location for the same convention.

## Install

Skill harnesses:

```bash
# Claude Code, project scope
mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/

# OpenCode
mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/

# Codex and Gemini CLI share the cross-runtime location
mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/
```

Rule and instructions harnesses read the same block under their own filename:

```bash
cat integrations/guidance/AGENTS.md >> AGENTS.md          # Codex, OpenCode, and the AGENTS.md tools
cat integrations/guidance/AGENTS.md >> GEMINI.md         # Gemini CLI
mkdir -p .clinerules && cat integrations/guidance/AGENTS.md > .clinerules/translatex.md
mkdir -p .cursor/rules && cp integrations/cursor/translatex.mdc .cursor/rules/
```

PowerShell: `New-Item -ItemType Directory -Force <dir>` then
`Copy-Item <src> <dir>`, or `Get-Content <src> | Add-Content <dst>` to append.

## Harness matrix

"Where" is the file to write, relative to the project the agent works in, or
under `$HOME` where marked.

| Harness | Channel | Where |
| --- | --- | --- |
| Claude Code | Skill | `.claude/skills/translatex/SKILL.md`, or `~/.claude/skills/translatex/SKILL.md` |
| Claude Code | Plugin | `/plugin marketplace add w21180239/translatex` then `/plugin install translatex@translatex` |
| Codex CLI | Skill | `~/.agents/skills/translatex/SKILL.md` |
| Codex CLI | Instructions | `AGENTS.md`, or `~/.codex/AGENTS.md` |
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
| Goose | Instructions | the project instructions file |
| DeepSeek Harness | Instructions | `AGENTS.md` |
| MiMo Code | Instructions | `AGENTS.md` |
| AtomCode | Instructions | `AGENTS.md` |
| OpenClaw | Instructions | `AGENTS.md` |
| Hermes Agent | Instructions | `AGENTS.md` |
| Pi | Instructions | `AGENTS.md` |
| Reasonix | Instructions | `AGENTS.md` |
| ZCode | Instructions | `AGENTS.md` |
| QwenPaw | Instructions | the project instructions file |
| Anything else | CLI | the agent runs `translatex … --format json` directly |

## Which channel for which job

| Need | Command |
| --- | --- |
| The agent translates while answering | `translatex translate "…" -t <lang> --format json` |
| Humanise AI-sounding prose | `translatex roundtrip "…" --from <lang> --preset standard --preserve @code --format json` |
| A script or CI job needs a translation | same CLI, parse `text` |
| Bulk work | `translatex batch --file jobs.ndjson` |
| Capability discovery | `translatex providers`, `translatex languages`, `translatex health` |

The CLI reads `~/.config/translatex/config.toml` and `credentials.toml`, or
`TRANSLATEX_*` environment variables. Free web endpoints need neither, so the
skill works on a fresh machine with only the binary installed.

## Verify

```bash
translatex translate "Hello world" -t zh --format json
```

Then ask the harness to translate a sentence. A correct answer contains a
`provider` name from the tool; an answer without one came from the model. The
document is then in the wrong place, or the harness cannot see the binary.

## Per-harness pages

- [Claude Code](claude-code/README.md) — skill, plugin, project instructions
- [Codex CLI](codex/README.md)
- [OpenCode](opencode/README.md)
- [Gemini CLI](gemini-cli/README.md)
- [Cursor](cursor/README.md)
- [Rules-file harnesses](harnesses/rules.md) — Cline, Roo, Kilo, Continue, JetBrains, Zed, Aider
- [Instructions-file harnesses](harnesses/instructions.md) — Qwen Code, Crush, Goose and the AGENTS.md tools

Full documentation, with the same grid and the install command for each harness:
[docs/en/integrations/02-supported-harnesses.md](../docs/en/integrations/02-supported-harnesses.md).
