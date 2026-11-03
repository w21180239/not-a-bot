# translatex skill

An agent skill for the `translatex` CLI and MCP server: web-first translation with provider fallback, key rotation, and structured JSON output.

## Install

```bash
cargo install translatex
# or: https://github.com/w21180239/translatex/releases/latest

# Link this skill into your agent's skills directory
mkdir -p ~/.config/opencode/skills
ln -s /path/to/translatex-skill ~/.config/opencode/skills/translatex
```

Verify the binary before relying on it:

```bash
translatex --version
translatex providers --format text
```

## CLI

```bash
translatex translate "Hello world" -t zh --format json
translatex translate "Deploy failed" -t ja --provider deepl --fallback google,edge
translatex translate 'Run `make test` first.' -t zh --preserve '`[^`]*`'
translatex batch --file jobs.ndjson --parallel 4
translatex providers --format json
translatex health --format json
```

All wire fields are camelCase. Unknown request fields are rejected, so a typo fails immediately instead of silently using a default.

## MCP

```bash
translatex mcp
```

Client configuration:

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

The server loads the same `config.toml` and `credentials.toml` as the CLI and builds its engine once at startup. Restart it after adding a key.

Tools: `translate`, `providers`, `languages`, `health`. Results carry `content` (one text block) and `structuredContent` (the same JSON); failures set `isError: true`.

## Tool schemas

### `translate`

Input:

```json
{
  "text": "string",
  "targetLang": "string (ISO 639-1)",
  "sourceLang": "string (default: auto)",
  "provider": "string (omit to walk the chain)",
  "fallbackChain": ["string"],
  "preserve": ["regex"],
  "timeoutMs": 20000,
  "idempotencyKey": "string"
}
```

Output:

```json
{
  "text": "translated text",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "uuid",
  "chars": 33
}
```

### `providers`

Returns the registered set with `name`, `displayName`, `kind` (`web` | `official` | `llm`), `priority`, `needsKey`, `maxChars`, `autoDetect`, `languages`, `freeTier`, `configured`.

### `languages`

`{provider?}` → `{provider, languages[]}`. A provider that advertises `*` expands to the canonical list. Unknown ids return `PROVIDER_NOT_FOUND`; unconfigured ones return `PROVIDER_NOT_CONFIGURED`.

### `health`

`{}` → `{healthy, providers[]}` with per-provider `configured`, `healthy`, `latencyMs`, `error`.

## Configuration

Resolution order, later wins:

1. Built-in free endpoints.
2. `~/.config/translatex/credentials.toml`.
3. `TRANSLATEX_<PROVIDER>_<FIELD>` environment variables.
4. OS keyring when a field is `@keyring`.

`credentials.toml` uses `[provider.label]` sections:

```toml
[deepl.default]
api_key = "@keyring"

[azure.default]
api_key = "@keyring"
region = "eastasia"

[openai.default]
api_key = "@keyring"
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
```

The label is the rotation slot: `[deepl.backup]` is used when `default` is cooling.

## Errors

```json
{
  "code": "RATE_LIMITED",
  "message": "rate limited by sogou",
  "retryable": true,
  "retryAfterSecs": 300,
  "suggestedAction": "backoff_and_retry"
}
```

Act on `suggestedAction`:

| Action | Do |
| --- | --- |
| `backoff_and_retry` | Sleep `retryAfterSecs`, retry |
| `retry_with_backoff` | Exponential backoff, retry |
| `switch_key_or_upgrade_plan` | Rotate credential or wait |
| `refresh_credentials` | Re-read credentials, retry once |
| `pass_an_explicit_source` | Set `sourceLang` |
| `change_provider_or_language` | Change `provider` or `targetLang` |
| `configure_a_provider` | Add credentials |
| `fix_input` / `fix_config_file` | Fix and resubmit; do not retry |

## Provider order

Free web endpoints (priorities 1-12) are always available; official APIs (20-27) only when credentials exist; LLM (40-42) last. Inspect the live set with `translatex providers --format json`.

## License

MIT OR Apache-2.0
