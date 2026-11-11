<div align="center">

# translatex

**Humanize AI-sounding text from the command line. Translation for your agent too: one binary, no API key.**

Rewrite AI-generated text so it reads like a person wrote it, using your own Claude or OpenCode subscription. Names, numbers and code stay intact, and translatex tells you if anything went missing. The same 6 MB binary translates through 28 engines, 14 of them free web endpoints that need no key, and ships one skill file that 24 coding agents can install with a copy.

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)
[![Tests](https://img.shields.io/badge/tests-319-brightgreen.svg)](#development)
[![Providers](https://img.shields.io/badge/providers-28-blueviolet.svg)](#provider-matrix)
[![Binary](https://img.shields.io/badge/binary-6%20MB-lightgrey.svg)](#install)
[![Agent-native](https://img.shields.io/badge/agent--native-JSON-purple.svg)](#supported-harnesses)

`translatex roundtrip "<AI-sounding text>" --from en --preset standard`

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [Docs](./docs/README.md)

</div>

---

## Contents

- [What you get](#what-you-get)
- [Before and after](#before-and-after)
- [Which preset?](#which-preset)
- [Measured results](#measured-results)
- [Why translatex](#why-translatex)
- [Features](#features)
- [Install](#install)
- [Quick start](#quick-start)
- [Supported harnesses](#supported-harnesses)
- [Round trip: how it works](#round-trip-how-it-works)
- [Provider matrix](#provider-matrix)
- [Live channel status](#live-channel-status)
- [Configuration](#configuration)
- [CLI reference](#cli-reference)
- [Error contract](#error-contract)
- [Documentation](#documentation)
- [Development](#development)

## What you get

A humanizer for AI-generated English copy and a free translation CLI, in one binary.

- **Humanize AI text.** `translatex roundtrip` rewrites AI-sounding English prose (stock openers, "not just X, it's Y", "it's worth noting") so it reads like a person wrote it. It runs on your own Claude Code or OpenCode subscription: no API key and no extra bill. Under the hood it is back-translation done by an LLM, out through another language and back, rewritten on both hops.
- **A free round trip.** The default preset goes through free web engines and costs nothing. It changes the words and keeps the sentence structure, so it is the quick option and the lighter rewrite.
- **Facts stay put.** Names, numbers, IDs, URLs and code are checked against the source after every round trip. Anything that went missing is listed in `missingFacts`, with a warning.
- **A repair pass.** `strong` adds a last hop in which the LLM proposes small edits. translatex applies only exact, small ones, so the rewrite stays and lost facts come back.
- **Translation for agents.** `translatex translate` races up to three free engines (Google, Edge, Papago, Sogou and more), answers in JSON and returns typed errors. No signup, no config.
- **No claim about AI detectors.** The goal is prose that reads like a person wrote it, with the facts intact. No method makes text reliably undetectable, and translatex does not promise it.

If you are looking for an AI humanizer, a de-AI tool, a way to remove AI tone or to rewrite AI-generated text from a script or a coding agent, this is a CLI that Claude Code, Codex, OpenCode, Cursor and 20 more harnesses can call, with DeepSeek, Haiku or whichever model your subscription offers doing the rewriting.

## Before and after

Input, an AI-flavoured paragraph with a name and a number:

```
In today's fast-paced landscape, Aroha's team isn't just shipping a feature, they're fundamentally transforming how 4,200 customers manage their invoices. It's worth noting that this comprehensive solution leverages cutting-edge automation to empower finance teams and unlock their full potential.
```

```bash
translatex roundtrip "<the paragraph above>" --from en --preset standard --format json
```

Output of `--preset standard` (OpenCode CLI with DeepSeek, 31 s for the two hops), one run on 2026-11-11, trimmed to the main fields:

```json
{
  "text": "Aroha's team is doing more than shipping one feature: they're changing how 4,200 customers manage their invoices. Automation takes the manual work off finance teams, so their time goes to work that's worth more.",
  "chain": ["zh", "en"],
  "hops": [
    {"from": "en", "to": "zh", "provider": "opencode-cli", "text": "Aroha 的团队正在做的，是让 4,200 家客户换一种方式管理发票，这次不只是上线一个功能。方案用自动化替财务团队省下手工活，让他们的时间用在更有价值的地方。", "latencyMs": 11027},
    {"from": "zh", "to": "en", "provider": "opencode-cli", "text": "Aroha's team is doing more than shipping one feature: they're changing how 4,200 customers manage their invoices. Automation takes the manual work off finance teams, so their time goes to work that's worth more.", "latencyMs": 20508}
  ],
  "missingFacts": [],
  "latencyMs": 31544
}
```

The opener, the hedge and the buzzwords are gone, and Aroha and 4,200 survived. One contrast ("more than shipping one feature") stayed, so the result is a better first draft, and a human edit can still improve it.

The same paragraph through the default `light` preset (two free machine-translation hops, 1.0 s), one run on 2026-11-11:

```json
{
  "text": "In today's fast-paced environment, the Araha team is not just releasing features—they are fundamentally changing how their 4,200 customers manage their invoices. Notably, this integrated solution leverages cutting-edge automation technologies to empower finance teams and maximize their potential.",
  "chain": ["zh", "ja", "en"],
  "missingFacts": ["Aroha"],
  "warnings": ["fact check: missing Aroha"]
}
```

Machine translation changed words and kept every sentence. "Not just X" and "in today's" survived, and the name came back as Araha. The fact guard caught it.

## Which preset?

| Preset | What it does | Rewrite strength | Speed | Facts lost (10 texts) | Needs |
| --- | --- | --- | --- | --- | --- |
| `light` (default) | Two machine-translation hops (en to zh to ja to en) | Light: words change, sentences stay | about 1 s | 5 | nothing |
| `standard` | LLM out and back; both hops "translate and rewrite like a person" | Removes the structural tells | 8 to 14 s | 0 | Claude or OpenCode subscription |
| `strong` | LLM hops into zh and ja, machine-translation hops through fi and back, then a repair hop | Strongest rewrite, with lost facts restored | about 25 s | 1 | subscription |

Pick `standard` to humanise English prose, `strong` for the most rewriting, and `light` when speed or a zero-cost run matters more. Add `--preserve @code --preserve @url` when the text holds code or links. `--depth 1-3` and `--chain` build other machine-translation or mixed chains by hand.

## Measured results

Run on 2026-11-11: 10 texts, one run per configuration (5 AI-flavoured English, 3 fact-heavy workplace English, 2 Chinese). Script and corpus: [`benches/roundtrip-depth/`](./benches/roundtrip-depth/README.md).

| Configuration | Word overlap with source | Tells left (of 10) | Facts missing | Latency |
| --- | --- | --- | --- | --- |
| light, 1 hop (`--depth 1`, 0.1.x default) | 0.556 | 7 | 5 | 0.5 s |
| light, 2 hops (0.3 default) | 0.520 | 5 | 5 | 0.9 s |
| 3 machine-translation hops (`--depth 3`) | 0.480 | 6 | 8 | 1.1 s |
| standard via opencode-cli (DeepSeek) | 0.446 | 0 | 0 | 14 s |
| standard via claude-cli (Haiku) | 0.535 | 0 | 0 | 8 s |
| strong via claude-cli / opencode-cli | 0.369 / 0.396 | 0 / 1 | 1 / 1 | 24 / 28 s |

Overlap is content-word Jaccard; lower means more rewriting. Tells are structural AI patterns ("not just/only X", "more than just", "it's worth noting", "in today's", "at its core", 不仅, 更是, 值得注意, 当今). Facts missing counts the names, numbers, IDs and code spans the guard reported as absent.

What the data shows:

- Machine translation changes words and keeps sentence structure, so "not just X, it's Y" survives every machine-translation depth.
- Even one machine-translation hop respells names (Aroha to Aloha, Tom to Tomi) and expands codes (NZD to "New Zealand dollars").
- A third machine-translation hop costs more facts than it gains in rewriting, so the default is two hops.
- LLM hops remove the structural tells and kept every fact on this corpus.
- Machine-translation hops placed after an LLM hop can bring a foil back ("ei ainoastaan ... vaan myös" returned as "not only ... but also"), so the same chain without a repair hop kept a few.
- The repair hop cut that chain's fact loss from 3 to 5 down to 1 and kept most of the rewrite; that is `strong`.
- Earlier finding (2026-11-04): German, French and Spanish pivots barely rewrite; Chinese and Japanese rewrite most. See [docs/en/roundtrip/01-pivot-choice.md](./docs/en/roundtrip/01-pivot-choice.md).

Caveats: 10 texts and one run is a small sample; the tell list and the fact guard are heuristics; overlap rewards change, not quality. Nothing here measures AI detectors.

## Why translatex

| Why it gets picked | What that means |
| --- | --- |
| **Humanizes with your subscription** | `claude-cli` and `opencode-cli` run `claude -p` and `opencode run` on the plan you already pay for. No API key, no new account. |
| **Facts checked** | Every round trip lists the names, numbers, IDs, URLs and code spans that did not make it back. |
| **Free, not a trial** | 14 public endpoints (Google Translate, Bing/Edge, Papago, Reverso, Sogou and more) need no key, no account and no quota sheet. |
| **Zero config** | Install and translate. Config files exist for pinning providers and adding keys, not for getting started. |
| **Fast by construction** | Up to three providers race; the first good answer wins and the rest are cancelled, so a hung endpoint cannot stall the request. |
| **Agent-native** | JSON or NDJSON on stdout, logs on stderr, exit codes, typed errors with `retryable` and `suggestedAction`, and a `manifest` that lists every operation. |
| **No MCP server** | The integration is the CLI plus one skill document: no process, no port, no protocol handshake, nothing to keep in sync. |
| **Grows with your keys** | Add DeepL, Azure, Google Cloud, Baidu, Tencent and four more when you need an SLA; keys rotate and cool down on their own. |
| **Nothing to host** | One static binary, 6 MB, no runtime, no Docker, no database. Laptop, CI runner or container. |
| **Code survives** | Regex spans, or the named patterns `@code` and `@url`, are masked before machine translation and restored after; a corrupted restore fails the hop instead of shipping. |

Drop-in for shell scripts, CI pipelines and coding agents. If you were about to sign up for a translation API, or to paste AI text into a rewriting site, start here.

### How the chain works

```
request ─▶ priority 1-12  free public endpoints     (no keys, no signup)
        ─▶ priority 20-27 official APIs             (only if credentials exist)
        ─▶ priority 40-42 LLM providers             (last resort, opt-in)
        ─▶ priority 60-61 claude-cli, opencode-cli  (local agent CLIs, only when named)
```

Lower number means tried earlier. Every hop is recorded, so a failure names the engines that were consulted and why each one declined.

## Features

| Capability | What you get |
| --- | --- |
| **Humanize / de-AI** | `roundtrip` goes out through one or more hops and back. Machine-translation hops are free; LLM hops rebuild the sentences. Presets `light`, `standard`, `strong`, or your own `--chain`. |
| **Fact guard** | `missingFacts` and a warning after every round trip: names, numbers, IDs, URLs and code that the result lost. |
| **Repair hop** | `<llm>:fix` asks the LLM for small edits and applies only exact ones. Lost facts always come back; other edits share a budget so the original cannot be copied back. |
| **28 providers** | 14 free web endpoints, 9 official APIs, 5 LLM providers (OpenAI, Anthropic, DeepSeek, and the local `claude-cli` and `opencode-cli`). LibreTranslate is opt-in via `base_url`. |
| **Fallback chain** | Ordered by priority; up to three providers in flight at once, so a hung endpoint cannot hide the healthy ones behind it. Explicit `--fallback` order is preserved. |
| **Key rotation** | Several credentials per provider, round-robin, with cooldowns shared across processes through `~/.config/translatex/cache/`. |
| **Structured output** | JSON or NDJSON on stdout, logs on stderr, exit codes an agent can branch on. |
| **Preserve patterns** | Regex spans or `@code` and `@url` are masked before machine translation and restored after; LLM hops get the spans listed and are checked afterwards. |
| **24 harnesses** | Claude Code, Codex, Cursor, Zed and 20 more read one skill or instructions block; no server to run. Paths: [integrations/](./integrations/README.md). |
| **Timeouts that hold** | `timeoutMs` from the request (or `timeout_ms` in config) caps the whole provider call, not just the socket read. |
| **Honest errors** | `code`, `retryable`, `retryAfterSecs`, `suggestedAction` on every failure. |

## Install

```bash
npm install -g translatex                 # prebuilt binary for this platform
cargo install translatex                 # or build from source, Rust 1.89+
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
translatex translate 'Run `make test` before pushing.' -t zh --preserve @code

# Humanize AI-sounding text: LLM out through Chinese and back (your Claude or OpenCode subscription)
translatex roundtrip "In today's fast-paced landscape, it's important to note that …" --from en --preset standard --preserve @code

# Free and fast: two machine-translation hops, then read missingFacts
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
| `translatex roundtrip "…" --from en --preset standard --format json` | Humanize: out through one or more hops and back |
| `translatex providers --format json` | What can serve this machine |
| `translatex health --format json` | Probe every configured provider |

## Round trip: how it works

`roundtrip` translates text out through one or more hops and back into the source language. Translating into a language with different syntax forces the text to be rebuilt, so the stock phrasing of generated text does not come back unchanged. Machine-translation hops do that as a side effect; LLM hops are told to translate and rewrite like a person.

```bash
translatex roundtrip "<text>" --from en --preset standard --preserve @code --format json
translatex roundtrip "<text>" --from en --depth 3                    # zh, ja, fi, back; machine translation only
translatex roundtrip "<text>" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
translatex roundtrip --from en --pivot ja < draft.md                 # alias: translatex polish
translatex roundtrip "<text>" --from en --preset standard --best-of 3 --scorer "python score.py" --format json
```

- **Choosing hops.** First match wins: `--chain "<[engine:]lang,...>"`, `--preset <name>`, `--depth 1-3`, `--pivot <lang>`, then `roundtrip_preset` in the config (default `light`). A final hop back to the source is appended when a chain lacks one.
- **LLM providers.** `claude-cli` runs `claude -p` on your Claude subscription (default model haiku); `opencode-cli` runs `opencode run` on your OpenCode quota (default model `opencode-go/deepseek-v4.1-flash`). They need no key, take seconds per hop, and run only when named: `auto` never picks them. `roundtrip_llm` (default `opencode-cli`) chooses the engine for the `standard` and `strong` presets.
- **Anchored to the original.** LLM hops also receive the source text and repair meaning that earlier machine-translation hops lost, with the smallest change, before rewriting.
- **Repair hop.** `<llm>:fix` returns a JSON list of small edits (`from`, `to`, `why`). translatex applies only exact, small ones. Edits that restore a missing fact always apply; other edits share a small budget, so the hop cannot copy the original back. The hop reports each edit under `edits` with `kind` (`fact` or `meaning`), `applied` and `reason`.
- **Fact guard.** `missingFacts` lists the names, numbers, IDs, code spans and URLs of the source that the result lacks (numbers compared by digits, names as whole words), and a `fact check:` warning is added. A name that doubles as a common sentence-starting word can slip through.
- **Preferred engines.** A named machine-translation engine is preferred, not pinned. On a retryable error (rate limit, timeout, network, quota) the hop falls back to another engine, avoiding the neighbouring hops' engines, with a warning. LLM hops never fall back.
- **Placeholders.** Machine-translation hops mask protected spans. LLM hops get the spans as-is, listed in the prompt, and a dropped span adds a warning.
- **Detector score.** Meant for benchmarking and tuning presets, not every request: a local detector loads a model per call. `--scorer "<command>"` (config `roundtrip_scorer`) runs a detector of your choice on each result: the text goes to its stdin and it prints a score from 0 to 1 (higher = more AI-like) as the last line of stdout. The command is split into words and run without a shell. A scorer that fails or times out (`--timeout-ms`) adds a warning and leaves that result unscored. The score is a measurement from that one detector; another detector can disagree.
- **Best of N.** `--best-of 1-5` runs the chain N times and keeps the run that lost no facts (else the fewest), then the lowest score. Above 1 it needs a scorer.
- **Target score.** `--target-score 0-1` tries the requested chain first, then each preset of `roundtrip_ladder` (default `standard,strong`) not yet tried, and stops at the first result at or below the target. Otherwise it returns the best result with a `target score not met` warning. Facts still outrank score at every step.

Use `--preserve @code` and `--preserve @url` rather than a regex such as `` `[^`]*` ``: on Windows the npm `.cmd` shim runs through cmd.exe, which strips `^` when Python or Node spawns translatex without a shell.

Response fields: `text`, `detectedSource`, `chain` (languages visited), `hops` (`from`, `to`, `provider`, `text`, `latencyMs`, and `edits` for a repair hop), `provider` (final hop), `missingFacts`, `warnings`, `latencyMs`, `requestId`, `chars`, `score` (`null` without a usable scorer), `scorer` (program name) and, when more than one run happened, `candidates` (`presetOrChain`, `score`, `missingFacts` as a count; only the selected text is returned).

Never round-trip legal text, API reference, configuration or numbers the reader will check; every hop paraphrases, and the fact guard is a heuristic. Chains, presets and the edit contract: [docs/en/roundtrip/02-chains-presets-and-llm.md](./docs/en/roundtrip/02-chains-presets-and-llm.md). Measurements and method: [docs/en/roundtrip/01-pivot-choice.md](./docs/en/roundtrip/01-pivot-choice.md).

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
| 60 | Claude Code CLI | `claude-cli` | none; runs `claude -p` on your Claude subscription, optional `model` (default haiku) and `bin` |
| 61 | OpenCode CLI | `opencode-cli` | none; runs `opencode run` on your OpenCode quota, optional `model` (default `opencode-go/deepseek-v4.1-flash`) and `bin` |

`claude-cli` and `opencode-cli` never join the automatic chain. They run when named: a `roundtrip` hop, a preset, `--provider` or `--fallback`.

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
roundtrip_preset = "light"          # light, standard, strong
roundtrip_llm = "opencode-cli"   # or "claude-cli"; engine for the standard and strong presets
# roundtrip_scorer = "python score.py"    # detector for --best-of and --target-score
# roundtrip_ladder = ["standard", "strong"] # presets tried until --target-score is met
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[provider.claude-cli]
model = "haiku"
bin = "claude"

[provider.opencode-cli]
model = "opencode-go/deepseek-v4.1-flash"
bin = "opencode"
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
| `roundtrip [TEXT]` | Out through one or more hops and back; humanizes AI-sounding text (`--chain`, `--preset`, `--depth`, `--pivot`, `--scorer`, `--best-of`, `--target-score`) |
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
| Round trip: overview, chains and presets, measured results | [docs/en/roundtrip/OVERVIEW.md](./docs/en/roundtrip/OVERVIEW.md) |
| Bench script and results | [benches/roundtrip-depth/README.md](./benches/roundtrip-depth/README.md) |
| Harness setup | [integrations/README.md](./integrations/README.md) |
| Agent integration | [AGENTS.md](./AGENTS.md) |

## Development

```bash
cargo test                # 319 tests
cargo build --release     # ~5 min with LTO
cargo clippy --all-targets
```

Adding a provider: implement `Provider` in `src/providers/<name>.rs`, set `priority` in the 1-12 range for web endpoints or 20-27 for APIs, register it in `src/cli.rs::build_registry`, and add the name to `KNOWN_PROVIDERS`.

## License

MIT OR Apache-2.0

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.
