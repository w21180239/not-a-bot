**English** · [简体中文](../zh-cn/OVERVIEW.md)

# Overview

[← Documentation index](../README.md)

`not-a-bot` answers translation requests by walking a priority-ordered list of providers. Free web endpoints come first; official APIs join only when credentials exist; LLM providers sit at the end.

## What the binary does

- Translates text through 28 providers: 14 free web endpoints, 9 official APIs, 5 LLM providers (two of them run your local Claude Code or OpenCode CLI).
- Rotates API keys and cools failing ones down across processes.
- Keeps up to three providers in flight, so a hung endpoint cannot hide the healthy ones.
- Round-trips text through a chain of machine-translation or LLM hops to humanise AI-sounding prose, and reports names, numbers and code that went missing.
- Emits JSON, NDJSON, or bare text on stdout; diagnostics go to stderr.
- Masks regex-defined spans before translation and restores them afterwards.
- Serves the same engine to agents as a CLI with structured output.

## Request flow

```
agent ──▶ CLI
            │
            ├─ validate input, normalise language codes
            ├─ mask preserve patterns
            ├─ build chain: fallback ─▶ configured chain ─▶ remaining by priority
            ├─ for each provider: pick key ─▶ call with timeout ─▶ map error
            └─ return response or a typed error with attempts
```

## Priority scheme

| Band | Kind | Examples |
| --- | --- | --- |
| 1-12 | Free web endpoints | `google` (1), `edge` (2), `iciba` (3), `mirai` (7), `systran` (12) |
| 20-27 | Official APIs | `deepl` / `baidu` (20), `azure` (22), `tencent` (26), `youdao` (27) |
| 40-42 | LLM providers | `openai` (40), `anthropic` (41), `deepseek` (42) |
| 60-61 | Local agent CLIs, named only | `claude-cli` (60), `opencode-cli` (61) |

Lower numbers are tried first. See [providers/02-priorities-and-fallback.md](providers/02-priorities-and-fallback.md) for how the chain is assembled.

## Conventions used across these docs

| Term | Meaning |
| --- | --- |
| Provider | One translation backend, identified by a stable lowercase id |
| Chain | The ordered list of providers a request will try |
| Key label | The name of one credential set inside a provider (`default`, `backup`, …) |
| Cooldown | Temporary skip of one credential after auth/quota/rate-limit failures |
| Preserve pattern | Regex whose matches must survive translation untouched |
| Pinned provider | A provider passed explicitly with `--provider` |

## Navigation

- → [Installation](install/OVERVIEW.md)
- → [CLI reference](cli/OVERVIEW.md)
- → [Providers](providers/OVERVIEW.md)
- → [API and errors](api/OVERVIEW.md)
- ↑ [Documentation index](README.md)
