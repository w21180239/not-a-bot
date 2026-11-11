# AGENTS.md

How `not-a-bot` is meant to be used by an agent: every interface is machine-readable, every failure is actionable, and every behaviour is deterministic.

## Design goal

An agent should never parse human-oriented output or guess a retry strategy. Read the exit code, then `error.code`, then act.

## Interfaces

### 1. CLI

```bash
not-a-bot translate "Hello world" -t zh --format json
not-a-bot rewrite "In today's fast-paced landscape, …" --from en --preset standard --preserve @code --format json
not-a-bot batch --file jobs.ndjson --parallel 4
not-a-bot providers --format json
not-a-bot languages --provider google --format json
not-a-bot health --format json
not-a-bot schema --format json
```

**Request**

```json
{
  "text": "Hello world",
  "sourceLang": "auto",
  "targetLang": "zh",
  "provider": "auto",
  "fallbackChain": ["edge", "iciba"],
  "preserve": ["`[^`]*`"],
  "timeoutMs": 20000,
  "idempotencyKey": "optional-correlation-id"
}
```

All field names are camelCase. Unknown fields are rejected with `INVALID_INPUT`, so a typo fails loudly.

**Success**

```json
{
  "text": "你好世界",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 245,
  "requestId": "uuid",
  "chars": 11
}
```

**Failure**

```json
{
  "error": {
    "code": "QUOTA_EXHAUSTED",
    "message": "quota exhausted for deepl",
    "retryable": true,
    "retryAfterSecs": 3600,
    "suggestedAction": "switch_key_or_upgrade_plan"
  },
  "requestId": "uuid"
}
```

**Batch NDJSON** — one request per line, one result per line, input order preserved:

```json
{"index":0,"ok":true,"result":{…}}
{"index":1,"ok":false,"error":{…}}
```

### 2. JSON Schema

```bash
not-a-bot schema --format json
```

Draft 2020-12 definitions for `TranslateRequest`, `TranslateResponse`, `ErrorResponse`, `ProviderInfo`, `HealthRecord`, `AttemptRecord`.

## Error handling

| `code` | Retryable | Agent action |
| --- | --- | --- |
| `RATE_LIMITED` | yes | Wait `retryAfterSecs`, retry |
| `QUOTA_EXHAUSTED` | no | Rotate credential or wait for the quota window |
| `NETWORK_ERROR` | yes | Exponential backoff |
| `UPSTREAM_ERROR` | 5xx only | Retry on 5xx, fix input on 4xx |
| `TIMEOUT` | yes | Raise `timeoutMs` and retry, or switch provider |
| `AUTH_FAILED` | no | Refresh credentials |
| `AUTO_DETECT_UNSUPPORTED` | no | Pass an explicit `sourceLang` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | no | Change provider or target |
| `PROVIDER_NOT_CONFIGURED` | no | Add credentials |
| `PROVIDER_NOT_FOUND` | no | Fix the provider name |
| `INVALID_INPUT` | no | Fix the request |
| `CHAIN_EXHAUSTED` | no | Read `attempts` from `--verbose` |
| `CONFIG_ERROR` | no | Fix the TOML file |

Exit codes: `0` success, `1` request error, `2` config error, `3` chain exhausted, `4` network error.

## Provider order

Free web endpoints occupy priorities 1-12 and are always available. Official APIs occupy 20-27 and are registered only when credentials exist. LLM providers occupy 40-42; the local agent CLIs `claude-cli` and `opencode-cli` occupy 60-61. Lower is tried first.

Inspect the live set rather than assuming:

```bash
not-a-bot providers --format json
not-a-bot health --format json
```

`claude-cli` and `opencode-cli` run the local Claude Code or OpenCode CLI on the user's subscription (slow, spends quota); like `reverso` they never join the automatic chain and run only when named.

`reverso` is registered but excluded from the automatic chain: its endpoint rejects this client's TLS fingerprint, so an automatic request would always pay for a failed attempt. Pin it with `--provider reverso`, list it in `--fallback`, or name it in `chain` to try it anyway.

Up to `race_width` providers (default 3) run concurrently: the first success wins and the rest are cancelled. A pinned provider always runs alone. Set `race_width = 1` (config) or `--race 1` for strict sequencing.

## Credential resolution

Later wins:

1. Built-in free endpoints.
2. `~/.config/not-a-bot/credentials.toml` (or `--credentials`, `NOT_A_BOT_CREDENTIALS`).
3. `NOT_A_BOT_<PROVIDER>_<FIELD>` environment variables.
4. OS keyring when a field is set to `@keyring`.

Cooldown state lives in `~/.config/not-a-bot/cache/keys-<provider>.json` and is shared across processes. `not-a-bot keys --reset` clears it.

Credentials never appear in output. Do not pass secrets as command-line flags; use the credentials file, environment variables, or the OS keyring.

## Language codes

Canonical ISO 639-1 with a few extensions. Aliases accepted:

| Input | Canonical |
| --- | --- |
| `zh-CN`, `zh-Hans`, `chinese` | `zh` |
| `zh-TW`, `zh-Hant`, `tw` | `zh-TW` |
| `jp`, `japanese` | `ja` |
| `kr`, `korean` | `ko` |
| `iw` | `he` |
| `auto`, `detect` | `auto` |

Full list: `not-a-bot languages --format json`.

## Preserve patterns

`preserve` takes Rust `regex` patterns, or the named patterns `@code` and `@url`. For machine translation providers, matched spans are masked before translation and restored afterwards; a provider that drops or duplicates a token fails the attempt and the chain moves on, so corrupted output never reaches the caller. LLM providers get the text unmasked plus the list of spans to keep verbatim; a span missing from their output adds a `preserve: <provider> dropped <span>` warning instead of failing.

## Round trips

`rewrite` translates out through one or more hops and back. Use it when the
user wants prose that reads less like a model wrote it. The result is in the
source language; the response carries `chain` (languages visited), `hops`
(per-hop `from`, `to`, `provider`, `text`, `latencyMs`, and `edits` on a repair
hop), `provider` (final hop), `missingFacts`, `warnings`, `latencyMs`,
`requestId`, `chars`, `score` (`null` without a usable scorer), `scorer` and, when
more than one run happened, `candidates` (`presetOrChain`, `score`,
`missingFacts` count).

- Hop selection, first match wins: `--chain`, `--preset`, `--depth`, `--pivot`, configured `rewrite_preset` (default `standard`).
- `--chain "claude-cli:zh,google:fi,edge:en"`: hops `[engine:]lang`; no engine races the fallback chain; a named MT engine is preferred, and a retryable failure (`RATE_LIMITED`, `TIMEOUT`, `NETWORK_ERROR`, `QUOTA_EXHAUSTED`) hands the hop to the automatic chain, avoiding the previous hop's provider, with a `hop <n>: <engine> failed (<code>), served by <other>` warning; named LLM engines never fall back; a hop back to the source is appended when needed. `engine:fix` (LLM engines only) stays in the current language and repairs meaning against the original.
- `fix` hop edits carry `kind`: `fact` edits restore a lost name, number or other fact and always apply; `meaning` edits share one budget, so together they cannot copy the original back.
- `--depth 1-3`: machine translation only, zh -> ja -> fi (en -> ja -> fi for Chinese). The default preset `light` is depth 2.
- Presets: `light` (two machine hops), `standard` (default; LLM out and back through `rewrite_llm`, default `opencode-cli`, alternative `claude-cli`), `strong` (LLM zh, LLM ja, google fi, edge back, then a repair hop). Any other name fails with `INVALID_INPUT` listing `light, standard, strong`. Config `rewrite_preset` (default `standard`), env `NOT_A_BOT_REWRITE_PRESET`, `NOT_A_BOT_REWRITE_LLM`.
- LLM hops rewrite in humanize style; later LLM hops also get the previous LLM hop as history and the original text.
- Machine hops change words but keep sentence shapes ("not just X, it's Y" survives) and respell names (Aroha -> Aloha). Read `missingFacts` (also a `fact check:` warning) and `warnings`, and report them; names are checked as whole words in Latin-script text, and a name that doubles as a common sentence-starting word can slip through.
- LLM presets rebuild the sentences and remove the structural tells; `strong` rewrites most. Use `--preserve @code` and `--preserve @url` for code and links. No preset is a claim about AI detectors.
- `--scorer "<command>"` (config `rewrite_scorer`), for benchmarks and tuning rather than live requests: a detector reads each result on stdin and prints a 0-1 score (higher = more AI-like) on its last stdout line; no shell; a failure adds a `scorer: ... unscored` warning. `--best-of 1-5` keeps the run with no missing facts (else the fewest), then the lowest score. `--target-score 0-1` then climbs `rewrite_ladder` (default `standard,strong`) and warns `target score not met` when no rung reaches it. Both need a scorer, else `INVALID_INPUT`. A score is one detector's measurement; report it per detector.
- `--pivot ja` rewrites as strongly and keeps more grammar; use it for technical English.
- Never round-trip legal text, API reference, configuration or numbers the reader will check; the fact guard is a heuristic.

## Installation

```bash
npm install -g not-a-bot
cargo install not-a-bot
# or: https://github.com/w21180239/not-a-bot/releases/latest
```

Single binary, no runtime dependencies. Per-harness CLI and skill setup:
[`integrations/`](./integrations/README.md).

## Testing an integration

```bash
echo '{"text":"hi","targetLang":"zh"}' | not-a-bot batch --format json
not-a-bot schema --format json | sha256sum    # schema stability
not-a-bot health --format json
```

## Versioning

`not-a-bot --version` returns semver. Field names and error codes are the public contract; adding optional fields is not a breaking change, removing or renaming one is.

## License

MIT OR Apache-2.0
