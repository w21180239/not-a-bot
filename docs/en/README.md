**English** · [简体中文](../zh-cn/README.md)

# translatex — Documentation

`translatex` is an agent-first translation CLI. It tries free web endpoints before official APIs, rotates credentials, and answers in structured JSON.

## Start here

| If you want to… | Read |
| --- | --- |
| Install and translate something in one minute | [install/OVERVIEW.md](install/OVERVIEW.md) |
| Learn every command and flag | [cli/OVERVIEW.md](cli/OVERVIEW.md) |
| Know which providers exist and in what order | [providers/OVERVIEW.md](providers/OVERVIEW.md) |
| Wire it into Claude Code, Codex, OpenCode or another agent | [integrations/OVERVIEW.md](integrations/OVERVIEW.md) |
| See the 24 supported harnesses with their paths | [integrations/02-supported-harnesses.md](integrations/02-supported-harnesses.md) |
| Parse the output safely | [api/OVERVIEW.md](api/OVERVIEW.md) |
| Fix a failing channel | [troubleshooting/OVERVIEW.md](troubleshooting/OVERVIEW.md) |

## Domains

| Domain | Scope |
| --- | --- |
| [install/](install/OVERVIEW.md) | Cargo, prebuilt binaries, first request, shell completion |
| [cli/](cli/OVERVIEW.md) | Commands, flags, exit codes, config and credential files |
| [providers/](providers/OVERVIEW.md) | Provider catalogue, priority scheme, fallback semantics, adding a provider |
| [roundtrip/](roundtrip/OVERVIEW.md) | De-AI rewriting via a pivot language; measured pivot choice |
| [integrations/](integrations/OVERVIEW.md) | npm, release archives, per-harness CLI and skill setup |
| [api/](api/OVERVIEW.md) | JSON and NDJSON shapes, error taxonomy, agent decision rules |
| [env/](env/OVERVIEW.md) | Environment variables and their precedence |
| [architecture/](architecture/OVERVIEW.md) | Request flow, key rotation, cooldown state, preserve engine |
| [troubleshooting/](troubleshooting/OVERVIEW.md) | Symptom → cause → fix table for each failure class |

## Project information

- **Version:** 0.1.0
- **Repository:** [github.com/w21180239/translatex](https://github.com/w21180239/translatex)
- **License:** MIT OR Apache-2.0
- **Agent integration:** [AGENTS.md](../../AGENTS.md)
- **Changelog:** [CHANGELOG.md](CHANGELOG.md)
