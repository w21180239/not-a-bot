# AGENTS.md

How `translatex` is meant to be used by an agent: every interface is machine-readable, every failure is actionable, and every behaviour is deterministic.

## Design goal

An agent should never parse human-oriented output or guess a retry strategy. Read the exit code, then `error.code`, then act.

## Interfaces

### 1. CLI

```bash
translatex translate "Hello world" -t zh --format json
translatex batch --file jobs.ndjson --parallel 4
translatex providers --format json
translatex languages --provider google --format json
translatex health --format json
translatex schema --format json
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

### 2. MCP server

```bash
translatex mcp
```

| Tool | Input | Output |
| --- | --- | --- |
| `translate` | `TranslateRequest` | `TranslateResponse` |
| `providers` | `{}` | `{providers: ProviderInfo[]}` |
| `languages` | `{provider?: string}` | `{provider, languages: string[]}` |
| `health` | `{}` | `{healthy, providers: HealthRecord[]}` |

Results follow the MCP contract: `content` with one text block, plus `structuredContent` carrying the same JSON. Failures set `isError: true` and put the typed error in `structuredContent`. The server reads the same config and credentials as the CLI and builds its engine once at startup — restart it after adding a key.

The manifest is available from `translatex manifest --format json`.

### 3. JSON Schema

```bash
translatex schema --format json
```

Draft 2020-12 definitions for `TranslateRequest`, `TranslateResponse`, `ErrorResponse`, `ProviderInfo`, `HealthRecord`, `AttemptRecord`.

## Error handling

| `code` | Retryable | Agent action |
| --- | --- | --- |
| `RATE_LIMITED` | yes | Wait `retryAfterSecs`, retry |
| `QUOTA_EXHAUSTED` | yes | Rotate credential or wait |
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

Free web endpoints occupy priorities 1-12 and are always available. Official APIs occupy 20-27 and are registered only when credentials exist. LLM providers occupy 40-42. Lower is tried first.

Inspect the live set rather than assuming:

```bash
translatex providers --format json
translatex health --format json
```

`reverso` is registered but excluded from the automatic chain: its endpoint rejects this client's TLS fingerprint, so an automatic request would always pay for a failed attempt. Pin it with `--provider reverso`, list it in `--fallback`, or name it in `chain` to try it anyway.

## Credential resolution

Later wins:

1. Built-in free endpoints.
2. `~/.config/translatex/credentials.toml` (or `--credentials`, `TRANSLATEX_CREDENTIALS`).
3. `TRANSLATEX_<PROVIDER>_<FIELD>` environment variables.
4. OS keyring when a field is set to `@keyring`.

Cooldown state lives in `~/.config/translatex/cache/keys-<provider>.json` and is shared across processes. `translatex keys --reset` clears it.

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

Full list: `translatex languages --format json`.

## Preserve patterns

`preserve` takes Rust `regex` patterns. Matched spans are masked before translation and restored afterwards. A provider that drops or duplicates a token fails the attempt and the chain moves on — that is deliberate, so corrupted output never reaches the caller.

## Installation

```bash
cargo install translatex
# or: https://github.com/w21180239/translatex/releases/latest
```

Single binary, no runtime dependencies.

## Testing an integration

```bash
echo '{"text":"hi","targetLang":"zh"}' | translatex batch --format json
translatex schema --format json | sha256sum    # schema stability
translatex health --format json
```

## Versioning

`translatex --version` returns semver. Field names and error codes are the public contract; adding optional fields is not a breaking change, removing or renaming one is.

## License

MIT OR Apache-2.0
