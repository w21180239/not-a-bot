# AGENTS.md

This file documents how `translatex` is intended to be used by agents (LLM-powered tools, AI assistants, autonomous scripts).

## Design Goal

`translatex` is **agent-first**: every interface is machine-readable, every failure is actionable, every behavior is deterministic. Agents should not need to parse human-oriented output or guess retry strategies.

## Programmatic Interfaces

### 1. CLI (stdin/stdout)

```bash
# Single request
translatex translate "Hello world" -t zh --format json

# Batch from NDJSON file
translatex batch --file jobs.ndjson --parallel 4

# Health check
translatex health --format json

# List providers
translatex providers --format json
```

**Input (single request):**

```json
{
  "text": "Hello world",
  "source_lang": "auto",
  "target_lang": "zh",
  "provider": "auto",
  "fallback_chain": ["deepl", "openai"],
  "preserve": ["`[^`]*`"],
  "timeout_ms": 20000,
  "idempotency_key": "optional-correlation-id"
}
```

**Output (success):**

```json
{
  "text": "你好世界",
  "detected_source": "en",
  "target_lang": "zh",
  "provider": "deepl",
  "engine": "deepl",
  "latency_ms": 245,
  "request_id": "uuid",
  "chars": 11,
  "warnings": []
}
```

**Output (error):**

```json
{
  "error": {
    "code": "QUOTA_EXHAUSTED",
    "message": "quota exhausted for deepl",
    "retryable": true,
    "retry_after_secs": 3600,
    "suggested_action": "switch_provider_or_upgrade_plan"
  },
  "attempts": [
    {"provider":"deepl","key_label":"default","ok":false,"latency_ms":120,"error_code":"QUOTA_EXHAUSTED"}
  ],
  "request_id": "uuid"
}
```

**Batch NDJSON:** one request per line, same shape. Output is one JSON object per line (same order as input).

### 2. MCP Server (stdio transport)

```bash
translatex mcp
```

**Tools exposed:**

| Tool | Description | Input Schema | Output Schema |
|------|-------------|--------------|---------------|
| `translate` | Single translation with full fallback | `TranslateRequest` | `TranslateResponse` |
| `providers` | List providers + capabilities | `{}` | `{"providers": ProviderInfo[]}` |
| `languages` | Supported codes (optional `provider` filter) | `{provider?: string}` | `{provider: string, languages: string[]}` |
| `health` | Probe configured providers | `{}` | `{healthy: bool, providers: HealthRecord[]}` |

The MCP manifest is available via `translatex manifest --format json`.

### 3. JSON Schema

```bash
translatex schema --format json
```

Generates a complete draft-2020-12 schema with definitions for:
- `TranslateRequest`
- `TranslateResponse`
- `ErrorResponse`
- `ProviderInfo`
- `HealthRecord`
- `AttemptRecord`

Use this for agent tool generation, validation, or documentation.

## Error Handling Contract

Every error response includes:

```json
{
  "code": "RATE_LIMITED",
  "message": "rate limited by deepl",
  "retryable": true,
  "retry_after_secs": 60,
  "suggested_action": "backoff_and_retry"
}
```

**Agent decision rules:**

| `retryable` | `suggested_action` | Agent should |
|-------------|-------------------|---------------|
| true | `backoff_and_retry` | Wait `retry_after_secs` (or exponential backoff), retry same request |
| true | `retry_with_backoff` | Exponential backoff (300ms → 600ms → 1.2s...), retry |
| true | `retry_later` | Wait at least 60s, retry |
| false | `switch_provider_or_upgrade_plan` | Try a different provider, or alert billing |
| false | `refresh_credentials` | Re-read credentials from keyring/env/config |
| false | `change_provider_or_language` | Change `--provider` or `--to` argument |
| false | `fix_request` | Fix input (bad language code, empty text, etc.) |
| false | `check_credentials_or_network` | All providers failed; check network or all credentials |

## Credential Resolution (agent-transparent)

Priority (later wins):

1. `TRANSLATEX_<PROVIDER>_<FIELD>` environment variables
2. `~/.config/translatex/credentials.toml` (or `TRANSLATEX_CONFIG`)
3. OS keyring (`@keyring` placeholder in TOML)
4. Built-in free endpoints (LibreTranslate, Volcengine web, Google web)

**Key rotation:** Multiple credentials per provider are rotated on auth/quota/rate-limit errors. Cooldown state is shared across processes via a cache file (`~/.cache/translatex/keys-<provider>.json`).

## Language Codes

Canonical ISO 639-1 codes. Aliases accepted:

| Input | Canonical |
|-------|-----------|
| `zh-CN`, `zh-Hans`, `chinese` | `zh` |
| `zh-TW`, `zh-Hant`, `tw` | `zh-TW` |
| `jp`, `japanese` | `ja` |
| `kr`, `korean` | `ko` |
| `iw` | `he` |
| `auto`, `detect`, `auto-detect` | `auto` |

Full list: `translatex languages --format json`

## Provider Capabilities

Query at runtime:

```bash
translatex providers --format json
translatex languages --provider deepl --format json
translatex health --format json
```

## Installation for Agents

```bash
# Linux/macOS (prebuilt)
curl -fsSL https://github.com/w21180239/translatex/releases/latest/download/translatex-x86_64-unknown-linux-gnu.tar.gz | tar xz -C /usr/local/bin

# Windows (Scoop)
scoop install translatex

# Cargo (any platform with Rust)
cargo install translatex
```

The binary is a single statically-linked executable with no runtime dependencies.

## Versioning

`translatex --version` returns semver. JSON schema version is embedded in the schema output. Breaking changes to output shapes will bump the major version.

## Testing Agent Integration

```bash
# Smoke test
echo '{"text":"hi","target_lang":"zh"}' | translatex batch --format json

# Verify schema stability
translatex schema | sha256sum

# Health check (should return healthy=true for configured providers)
translatex health --format json
```

## License

MIT OR Apache-2.0