<div align="center">

# translatex

**Free translation for your agent — one command, no API key, no signup, no config.**

A single 6 MB binary with 26 translation engines, raced in parallel. JSON out, typed errors in, and one skill file that 24 coding agents can install with a copy.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Tests](https://img.shields.io/badge/tests-245-brightgreen.svg)](#development)
[![Providers](https://img.shields.io/badge/providers-26-blueviolet.svg)](#provider-matrix)
[![Binary](https://img.shields.io/badge/binary-6%20MB-lightgrey.svg)](#install)
[![Agent-native](https://img.shields.io/badge/agent--native-JSON-purple.svg)](#supported-harnesses)

`translatex translate "Hello world" -t zh`

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [Docs](./docs/README.md)

</div>

---

## Contents

- [Why translatex](#why-translatex)
- [Features](#features)
- [Install](#install)
- [Quick start](#quick-start)
- [Supported harnesses](#supported-harnesses)
- [Round trip: de-AI prose](#round-trip-de-ai-prose)
- [Provider matrix](#provider-matrix)
- [Live channel status](#live-channel-status)
- [Configuration](#configuration)
- [CLI reference](#cli-reference)
- [Error contract](#error-contract)
- [Documentation](#documentation)
- [Development](#development)

## Why translatex

| Why it gets picked | What that means |
| --- | --- |
| **Free, not a trial** | 14 public endpoints — Google Translate, Bing/Edge, Papago, Reverso, Sogou and more — need no key, no account and no quota sheet. |
| **Zero config** | Install and translate. Config files exist for pinning providers and adding keys, not for getting started. |
| **Fast by construction** | Up to three providers race; the first good answer wins and the rest are cancelled, so a hung endpoint cannot stall the request. |
| **Agent-native** | JSON or NDJSON on stdout, logs on stderr, exit codes, typed errors with `retryable` and `suggestedAction`, and a `manifest` that lists every operation. |
| **No MCP server** | The integration is the CLI plus one skill document: no process, no port, no protocol handshake, nothing to keep in sync. |
| **De-AI prose** | `roundtrip` sends text out through a pivot language and back, so stock machine phrasing cannot survive the trip. |
| **Grows with your keys** | Add DeepL, Azure, Google Cloud, Baidu, Tencent and four more when you need an SLA; keys rotate and cool down on their own. |
| **Nothing to host** | One static binary, 6 MB, no runtime, no Docker, no database. Laptop, CI runner or container. |
| **Code survives** | Regex spans — code, identifiers, URLs, version strings — are masked before translation and restored after; a corrupted restore fails the hop instead of shipping. |

Drop-in for shell scripts, CI pipelines and coding agents. If you were about to sign up for a translation API, start here.

### How the chain works

```
request ─▶ priority 1-12  free public endpoints     (no keys, no signup)
        ─▶ priority 20-27 official APIs             (only if credentials exist)
        ─▶ priority 40-42 LLM providers             (last resort, opt-in)
```

Lower number means tried earlier. Every hop is recorded, so a failure names the engines that were consulted and why each one declined.

## Features

| Capability | What you get |
| --- | --- |
| **26 providers** | 14 free web endpoints, 9 official APIs, 3 LLM providers. LibreTranslate is opt-in via `base_url`. |
| **Fallback chain** | Ordered by priority; up to three providers in flight at once, so a hung endpoint cannot hide the healthy ones behind it. Explicit `--fallback` order is preserved. |
| **Round trip** | `roundtrip` translates out through a pivot language and back, which rewrites machine-flavoured prose. Pivot measured, defaults to zh for non-Chinese and en for Chinese. |
| **Key rotation** | Several credentials per provider, round-robin, with cooldowns shared across processes through `~/.config/translatex/cache/`. |
| **Structured output** | JSON or NDJSON on stdout, logs on stderr, exit codes an agent can branch on. |
| **Preserve patterns** | Regex spans (code, variables, URLs) are masked before translation and restored after; a corrupted restore triggers fallback. |
| **24 harnesses** | Claude Code, Codex, Cursor, Zed and 20 more read one skill or instructions block; no server to run. Paths: [integrations/](./integrations/README.md). |
| **Timeouts that hold** | `timeoutMs` from the request (or `timeout_ms` in config) caps the whole provider call, not just the socket read. |
| **Honest errors** | `code`, `retryable`, `retryAfterSecs`, `suggestedAction` on every failure. |

## Install

```bash
npm install -g translatex                 # prebuilt binary for this platform
cargo install translatex                 # or build from source, Rust 1.80+
# or grab a prebuilt binary
# https://github.com/w21180239/translatex/releases/latest
```

Verify:

```bash
translatex --version
translatex providers --format text
```

## Quick start

```bash
# Web endpoints only, no configuration at all
translatex translate "Hello world" -t zh

# Pin a provider and keep a fallback list
translatex translate "Deploy failed" -t ja --provider deepl --fallback google,edge

# Read stdin
echo "Good morning" | translatex translate -t fr

# Force detection off; pass the source
translatex translate "Guten Tag" -s de -t en

# NDJSON batch, four at a time
translatex batch --file examples/batch.ndjson --parallel 4

# Keep code spans intact
translatex translate 'Run `make test` before pushing.' -t zh --preserve '`[^`]*`'

# Strip the machine cadence: out through Chinese, back to English
translatex roundtrip "In today's fast-paced landscape, it's important to note that …" --from en
```

Successful response:

```json
{
  "text": "你好，世界",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "c506d3bb-8eb2-451a-9e84-f2be63833f79",
  "chars": 11
}
```

Failures carry a machine-readable reason:

```json
{
  "error": {
    "code": "PROVIDER_NOT_CONFIGURED",
    "message": "provider deepl is implemented but not configured; add credentials or a base_url",
    "retryable": false,
    "suggestedAction": "configure_a_provider"
  },
  "requestId": ""
}
```

## Supported harnesses

The integration is the command plus one document that tells the agent when to
run it: a skill where the harness loads skills, a rule file where it reads rules,
an instructions block where it keeps project instructions. No server, no protocol.

<div align="center">
<table>
<tr>
<td align="center" width="150"><img src="repo-assets/agents/claude-code.png" width="44" alt="Claude Code"><br/><b>Claude Code</b><br/><sub>skill · plugin</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/codex.png" width="44" alt="Codex CLI"><br/><b>Codex CLI</b><br/><sub>skill · AGENTS.md</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/gemini-cli.png" width="44" alt="Gemini CLI"><br/><b>Gemini CLI</b><br/><sub>skill · GEMINI.md</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/opencode.png" width="44" alt="OpenCode"><br/><b>OpenCode</b><br/><sub>skill · AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/cursor.png" width="44" alt="Cursor"><br/><b>Cursor</b><br/><sub>.cursor/rules</sub></td>
<td align="center"><img src="repo-assets/agents/cline.png" width="44" alt="Cline"><br/><b>Cline</b><br/><sub>.clinerules</sub></td>
<td align="center"><img src="repo-assets/agents/roo-code.png" width="44" alt="Roo Code"><br/><b>Roo Code</b><br/><sub>.roo/rules</sub></td>
<td align="center"><img src="repo-assets/agents/kilo-code.png" width="44" alt="Kilo Code"><br/><b>Kilo Code</b><br/><sub>.kilocode/rules</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/continue.png" width="44" alt="Continue"><br/><b>Continue</b><br/><sub>.continue/rules</sub></td>
<td align="center"><img src="repo-assets/agents/jetbrains.png" width="44" alt="JetBrains AI"><br/><b>JetBrains AI</b><br/><sub>.aiassistant/rules</sub></td>
<td align="center"><img src="repo-assets/agents/zed.png" width="44" alt="Zed"><br/><b>Zed</b><br/><sub>.rules</sub></td>
<td align="center"><img src="repo-assets/agents/aider.png" width="44" alt="Aider"><br/><b>Aider</b><br/><sub>CONVENTIONS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/qwen-code.png" width="44" alt="Qwen Code"><br/><b>Qwen Code</b><br/><sub>QWEN.md</sub></td>
<td align="center"><img src="repo-assets/agents/crush.png" width="44" alt="Crush"><br/><b>Crush</b><br/><sub>CRUSH.md</sub></td>
<td align="center"><img src="repo-assets/agents/goose.png" width="44" alt="Goose"><br/><b>Goose</b><br/><sub>instructions</sub></td>
<td align="center"><img src="repo-assets/agents/deepseek-harness.png" width="44" alt="DeepSeek Harness"><br/><b>DeepSeek Harness</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/mimo-code.svg" width="44" alt="MiMo Code"><br/><b>MiMo Code</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/atomcode.png" width="44" alt="AtomCode"><br/><b>AtomCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/openclaw.png" width="44" alt="OpenClaw"><br/><b>OpenClaw</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/hermes-agent.png" width="44" alt="Hermes Agent"><br/><b>Hermes Agent</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/pi.png" width="44" alt="Pi"><br/><b>Pi</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/reasonix.png" width="44" alt="Reasonix"><br/><b>Reasonix</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/zcode.png" width="44" alt="ZCode"><br/><b>ZCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/qwenpaw.svg" width="44" alt="QwenPaw"><br/><b>QwenPaw</b><br/><sub>instructions</sub></td>
</tr>
</table>

<i>24 harnesses, two documents. Anything else that can run a shell uses the CLI directly.</i>
</div>

```bash
# Skill harnesses
mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/
mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/
mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/   # Codex, Gemini CLI

# Everything else reads the same block under its own filename
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

| Need | Read |
| --- | --- |
| Exact path and command per harness | [integrations/README.md](./integrations/README.md) |
| Why a skill beats an instructions file | [docs/en/integrations/02-supported-harnesses.md](./docs/en/integrations/02-supported-harnesses.md) |
| One-command plugin install for Claude Code | [integrations/claude-code/README.md](./integrations/claude-code/README.md) |

Agents that prefer a command reference over a skill can call the CLI directly:

| Command | Purpose |
| --- | --- |
| `translatex translate "…" -t zh --format json` | One translation |
| `translatex roundtrip "…" --from en --format json` | Out through a pivot and back |
| `translatex providers --format json` | What can serve this machine |
| `translatex health --format json` | Probe every configured provider |

## Round trip: de-AI prose

Translating out through a language with different syntax and back forces the text
to be rebuilt, so the result loses the stock phrasing that makes model output
recognisable. It is one command, not a prompt:

```bash
translatex roundtrip "Our comprehensive solution leverages cutting-edge technology to empower teams." --from en --format json
```

```json
{
  "text": "Our integrated solutions use cutting-edge technology to give teams more power.",
  "detectedSource": "en",
  "pivotLang": "zh",
  "pivotText": "我们综合的解决方案利用尖端技术来增强团队的能力。",
  "provider": "google",
  "providers": ["google", "edge"],
  "latencyMs": 612,
  "requestId": "…",
  "chars": 82
}
```

The default pivot is Chinese for non-Chinese text and English for Chinese text.
That pair was chosen by measurement: German, French and Spanish round-trip
almost verbatim, Japanese rewrites as strongly as Chinese, and Russian rewrites
the most but drifts the most. Use `--pivot ja` for technical English that has to
stay precise, and never round-trip legal text, API docs or anything with numbers
you cannot verify. Evidence and method: [docs/en/roundtrip/01-pivot-choice.md](./docs/en/roundtrip/01-pivot-choice.md).

It rewrites wording and keeps sentences: an AI opener or hedge survives every
language, so delete those clauses before translating and tell the user you did.

## Provider matrix

Priorities are the numbers the chain sorts by. `chars` is the request limit the adapter enforces before splitting.

### Free web endpoints — primary path

| # | Provider | ID | Auto-detect | Max chars | Notes |
| --- | --- | --- | --- | --- | --- |
| 1 | Google Translate (web) | `google` | yes | 5 000 | `translate.googleapis.com/translate_a/single`, best effort |
| 2 | Microsoft Edge Translator | `edge` | yes | 50 000 | `edge.microsoft.com/translate`, no key |
| 3 | Iciba | `iciba` | yes | 3 000 | AES-ECB signed payload |
| 4 | iFlytek iFlyrec | `iflyrec` | yes | 2 000 | zh-anchored pairs only |
| 4 | ModernMT | `modernmt` | yes | 5 000 | md5 verify + timestamp |
| 5 | Volcengine | `volcengine` | yes | 5 000 | `crx/translate/v1/` |
| 5 | Lara Translate | `lara` | yes | 500 | daily per-IP credit |
| 6 | Lingvanex | `lingvanex` | no | 1 000 | `API_BEARER_TOKEN` |
| 7 | Mirai Translate | `mirai` | yes | 500 | Japanese-anchored pairs only, trial key scraped per call |
| 8 | Papago (Naver) | `papago` | yes | 1 000 | cookie-primed detect + form POST |
| 9 | Reverso Context | `reverso` | no | 2 000 | three-letter codes; excluded from the auto chain, see status |
| 10 | Sogou | `sogou` | yes | 5 000 | md5 sign; per-IP throttle returns `RATE_LIMITED` |
| 11 | MyMemory | `mymemory` | no | 5 000 | RFC 3066 codes, daily cap |
| 12 | SysTran | `systran` | no | 5 000 | OAuth client credentials scraped from the bundle |
| — | LibreTranslate | `libretranslate` | yes | 5 000 | self-hosted only: set `[provider.libretranslate] base_url` |

### Official APIs — fallback when credentials exist

| # | Provider | ID | Credentials |
| --- | --- | --- | --- |
| 20 | DeepL | `deepl` | `api_key` |
| 20 | Baidu | `baidu` | `appid`, `secret_key` |
| 21 | Google Cloud Translation | `googlecloud` | `api_key` |
| 22 | Azure Translator | `azure` | `api_key`, `region` |
| 23 | Alibaba Cloud | `aliyun` | `access_key_id`, `access_key_secret` |
| 24 | Caiyun | `caiyun` | `token` |
| 25 | Niutrans | `niutrans` | `apikey` |
| 26 | Tencent | `tencent` | `secret_id`, `secret_key` |
| 27 | Youdao | `youdao` | `app_key`, `app_secret` |

### LLM — last resort

| # | Provider | ID | Credentials |
| --- | --- | --- | --- |
| 40 | OpenAI | `openai` | `api_key`, optional `base_url`, `model` |
| 41 | Anthropic | `anthropic` | `api_key`, optional `model` |
| 42 | DeepSeek | `deepseek` | `api_key`, optional `model` |

## Live channel status

Endpoints drift. This is what a full sweep against the free web endpoints returned on 2026-11-03:

| Status | Providers |
| --- | --- |
| Verified end-to-end | `google`, `edge`, `iciba`, `modernmt`, `lara`, `volcengine`, `lingvanex`, `papago`, `sogou`, `mymemory`, `systran` |
| Restricted pairs | `iflyrec` (zh ↔ any), `mirai` (ja-anchored) |
| Throttles under load | `sogou` — per-IP limit surfaces as `RATE_LIMITED` with `retryAfterSecs`, never a fake translation |
| Blocked by fingerprinting | `reverso` — Cloudflare rejects the rustls JA3; it stays registered for explicit use but is skipped by the auto chain, so an automatic request never spends a round trip on it |
| Opt-in only | `libretranslate` — no public instance is alive; self-host and set `base_url` |
| Retired upstream | `iflytek` — aliased to `iflyrec` so old configs keep working |

Run `translatex health --format json` on your own network before trusting this table.

## Configuration

`~/.config/translatex/config.toml` (or `--config`, or `TRANSLATEX_CONFIG`):

```toml
# Replaces the built-in auto chain. Keyed providers are still appended
# afterwards if credentials exist.
chain = ["edge", "google", "iciba", "deepl", "openai"]

default_target = "zh"
timeout_ms = 20000
retries = 1
race_width = 3
roundtrip_pivot = "auto"
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
```

`~/.config/translatex/credentials.toml`:

```toml
[deepl.default]
api_key = "..."

[azure.default]
api_key = "..."
region = "eastasia"

# A second key takes over when the first is cooling.
[deepl.backup]
api_key = "..."
```

Resolution: `credentials.toml` labels are tried in file order, a `TRANSLATEX_<PROVIDER>_<FIELD>` variable adds an `env` label after them, and a field set to `@keyring` is read from the OS keyring. Free endpoints need none of this.

## CLI reference

| Command | Purpose |
| --- | --- |
| `translate [TEXT]` | One request; reads stdin when text is omitted |
| `roundtrip [TEXT]` | Out through a pivot language and back; strips machine cadence |
| `batch` | NDJSON in, NDJSON out, order preserved |
| `providers` | Capability and configuration state per provider |
| `languages` | Canonical codes, or one provider's advertised set |
| `keys [PROVIDER]` | Cooldown and failure state; `--reset` clears it |
| `health` | Probe every configured provider |
| `schema` | JSON Schema (draft 2020-12) for all wire types |
| `manifest` | Machine-readable list of CLI operations |
| `init` | Write a config template |
| `completions SHELL` | Bash/zsh/fish/PowerShell completion |

Global flags: `--format json|text`, `--quiet`, `--verbose`, `--race N`, `--config`, `--credentials`, `--log`.

Exit codes: `0` success, `1` request error, `2` config error, `3` chain exhausted, `4` network error.

## Error contract

| `code` | Retryable | Agent action |
| --- | --- | --- |
| `RATE_LIMITED` | yes | Wait `retryAfterSecs`, then retry |
| `QUOTA_EXHAUSTED` | no | Rotate key or wait for the quota window |
| `NETWORK_ERROR` | yes | Exponential backoff |
| `UPSTREAM_ERROR` | 5xx only | Retry on 5xx, fix input on 4xx |
| `TIMEOUT` | yes | Raise `timeoutMs` and retry, or switch provider |
| `AUTH_FAILED` | no | Refresh credentials |
| `PROVIDER_NOT_CONFIGURED` | no | Add credentials |
| `PROVIDER_NOT_FOUND` | no | Fix the provider name |
| `AUTO_DETECT_UNSUPPORTED` | no | Pass an explicit `sourceLang` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | no | Change provider or target |
| `INVALID_INPUT` | no | Fix the request |
| `CHAIN_EXHAUSTED` | no | Inspect `--verbose` attempts |

Full reference: [docs/en/api/OVERVIEW.md](./docs/en/api/OVERVIEW.md).

## Documentation

| Language | Entry point |
| --- | --- |
| English | [docs/en/OVERVIEW.md](./docs/en/OVERVIEW.md) |
| 简体中文 | [docs/zh-cn/OVERVIEW.md](./docs/zh-cn/OVERVIEW.md) |
| Round trip and pivot choice | [docs/en/roundtrip/OVERVIEW.md](./docs/en/roundtrip/OVERVIEW.md) |
| Harness setup | [integrations/README.md](./integrations/README.md) |
| Agent integration | [AGENTS.md](./AGENTS.md) |

## Development

```bash
cargo test                # 245 tests
cargo build --release     # ~5 min with LTO
cargo clippy --all-targets
```

Adding a provider: implement `Provider` in `src/providers/<name>.rs`, set `priority` in the 1-12 range for web endpoints or 20-27 for APIs, register it in `src/cli.rs::build_registry`, and add the name to `KNOWN_PROVIDERS`.

## License

MIT OR Apache-2.0
