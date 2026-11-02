# translatex

**Agent-first translation CLI & MCP server.** Single binary, zero config, structured JSON/NDJSON output, official APIs only.

## Features

- **Provider fallback chain** — walks DeepL → Azure → Google Cloud → Baidu → Tencent → Youdao → Caiyun → Niutrans → LibreTranslate → LLM until one succeeds
- **Key rotation** — multiple API keys per provider, automatic cooldown on quota/auth/rate-limit errors, shared across processes via cache dir
- **Structured output** — JSON, NDJSON streaming, or bare text; every error carries `code`, `retryable`, `retry_after_secs`, `suggested_action`
- **Preserve patterns** — regex spans (code blocks, variables, etc.) are masked before translation and restored afterwards; corruption is detected and triggers fallback
- **Agent-native** — CLI emits machine-readable output; MCP server exposes `translate`, `providers`, `languages`, `health` tools
- **Zero-config defaults** — works out of the box with free endpoints; keyed providers auto-detected from `~/.config/translatex/credentials.toml`, environment variables, or OS keyring

## Quick Start

```bash
# Install (Linux/macOS/Windows via cargo)
cargo install translatex

# Or download a prebuilt binary from GitHub Releases
```

```bash
# Translate once (JSON output)
translatex translate "Hello world" -t zh

# NDJSON streaming for batch
translatex batch --file jobs.ndjson --parallel 4

# Explicit provider with fallback chain
translatex translate "..." -t ja --provider deepl --fallback openai,google

# Preserve code blocks, variables, etc.
translatex translate "Run `make test` first." -t zh --preserve '`[^`]*`'

# MCP server (stdio transport)
translatex mcp
```

## Configuration

### Credentials (`~/.config/translatex/credentials.toml`)

```toml
# DeepL (api_key is required; `:fx` suffix uses free endpoint)
[deepl]
api_key = "@keyring"   # reads from OS keyring (macOS Keychain, Windows Credential Manager, libsecret)

# Azure AI Translator (region is optional)
[azure]
api_key = "@keyring"
region = "eastus"

# Google Cloud Translation
[googlecloud]
api_key = "@keyring"

# Baidu Fanyi
[baidu]
appid = "@keyring"
secret_key = "@keyring"

# Tencent Cloud MT
[tencent]
secret_id = "@keyring"
secret_key = "@keyring"

# Youdao
[youdao]
app_key = "@keyring"
app_secret = "@keyring"

# Caiyun
[caiyun]
token = "@keyring"

# Niutrans
[niutrans]
api_key = "@keyring"

# OpenAI-compatible LLM (DeepSeek, Moonshot, SiliconFlow, etc.)
[openai]
api_key = "@keyring"
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
```

### Environment variables (override file)

All fields can also be set via `TRANSLATEX_<PROVIDER>_<FIELD>`, e.g.:

```bash
export TRANSLATEX_DEEPL_API_KEY="your-key:fx"
export TRANSLATEX_OPENAI_BASE_URL="https://api.deepseek.com/v1"
```

### Settings (`~/.config/translatex/config.toml`)

```toml
# Override the default auto chain
chain = ["deepl", "azure", "googlecloud", "baidu", "libretranslate", "openai"]

default_target = "zh"
timeout_ms = 20000
retries = 2

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
```

## CLI Reference

```
translatex <COMMAND>

COMMANDS:
    translate       Translate a single text (reads stdin if no argument)
    batch           Translate many requests from NDJSON file/stdin
    providers       List providers with capabilities and credential status
    languages       List supported language codes (optionally per provider)
    keys            Show key rotation state (cooldowns, failure counts)
    health          Probe all configured providers
    schema          Print JSON Schema for request/response types
    manifest        Print MCP tool manifest
    mcp             Run as MCP server over stdio
    init            Print or write default config template
    completions     Generate shell completions

GLOBAL FLAGS:
    --format json|text   Output format (default: json)
    --quiet              Compact single-line JSON
    --verbose            Include attempt trail in response
    --config <path>      Override config file
    --credentials <path> Override credentials file
    --log <level>        error|warn|info|debug (default: warn)
```

## Error Codes

| Code | Retryable | Retry After | Suggested Action |
|------|-----------|-------------|------------------|
| `QUOTA_EXHAUSTED` | Yes | `retry_after_secs` | `switch_provider_or_upgrade_plan` |
| `RATE_LIMITED` | Yes | `retry_after_secs` | `backoff_and_retry` |
| `AUTH_FAILED` | No | - | `refresh_credentials` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | No | - | `change_provider_or_language` |
| `NETWORK_ERROR` | Yes | - | `retry_with_backoff` |
| `TIMEOUT` | Yes | - | `increase_timeout_and_retry` |
| `UPSTREAM_ERROR` (5xx) | Yes | - | `retry_later` |
| `UPSTREAM_ERROR` (4xx) | No | - | `fix_request` |
| `CHAIN_EXHAUSTED` | No | - | `check_credentials_or_network` |

## MCP Integration

```bash
# Install as MCP server (once)
translatex mcp
# or
mcp install translatex
```

Available tools:
- `translate` — single translation with full fallback chain
- `providers` — list providers and their capabilities
- `languages` — supported language codes
- `health` — provider health check with latency

## Provider Matrix (2026)

| Provider | Free Tier | Quality | Latency | Streaming | Regions |
|----------|-----------|---------|---------|-----------|---------|
| DeepL | 1M chars total (dev) / 500k/mo (free) | ★★★★★ | ~300ms | No | Global |
| Azure | 2M chars/mo (F0) | ★★★★☆ | ~350ms | No | Global |
| Google Cloud | 500k chars/mo | ★★★★☆ | ~400ms | No | Global |
| Baidu | 50k chars/mo | ★★★☆☆ | ~300ms | No | China |
| Tencent | Varies | ★★★☆☆ | ~300ms | No | China |
| Youdao | Trial quota | ★★★☆☆ | ~300ms | No | China |
| Caiyun | 1M chars one-time | ★★★★☆ | ~400ms | No | China |
| Niutrans | 500k on signup | ★★★☆☆ | ~300ms | No | China |
| LibreTranslate | Unlimited (self-host) | ★★★☆☆ | ~800ms | No | Self-host |
| OpenAI-compatible LLM | Pay-as-you-go | ★★★★☆ | ~500ms | Yes | Global |

**Default auto chain:** `deepl > azure > googlecloud > baidu > tencent > youdao > caiyun > niutrans > libretranslate > openai`

## License

MIT OR Apache-2.0