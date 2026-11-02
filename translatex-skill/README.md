# translatex skill

An OpenCode skill for the `translatex` translation CLI and MCP server.

## Installation

```bash
# Install the CLI
cargo install translatex

# Or download from GitHub Releases
# https://github.com/w21180239/translatex/releases

# Install this skill
mkdir -p ~/.config/opencode/skills
ln -s /path/to/translatex-skill ~/.config/opencode/skills/translatex
```

## Usage

After installation, agents can use `translatex` via the CLI or MCP server.

### CLI Commands

```bash
# Translate text
translatex translate "Hello world" -t zh --format json

# Batch translation
translatex batch --file jobs.ndjson --parallel 4

# List providers
translatex providers --format json

# Health check
translatex health --format json

# MCP server
translatex mcp
```

### MCP Integration

```bash
# Run as MCP server (stdio transport)
translatex mcp

# Or install via mcp CLI
mcp install translatex
```

## Skill Commands

This skill provides the following commands to agents:

### `translatex:translate`
Translate text with automatic provider fallback and key rotation.

**Input:**
```json
{
  "text": "string",
  "target_lang": "string (ISO 639-1)",
  "source_lang": "string (default: auto)",
  "provider": "string (default: auto)",
  "fallback_chain": ["string"],
  "preserve": ["regex patterns"],
  "timeout_ms": "number",
  "idempotency_key": "string"
}
```

**Output:**
```json
{
  "text": "translated text",
  "detected_source": "language code",
  "target_lang": "language code",
  "provider": "provider name",
  "engine": "engine name",
  "latency_ms": "number",
  "request_id": "uuid",
  "chars": "number",
  "warnings": ["string"]
}
```

### `translatex:providers`
List all available providers with their capabilities and configuration status.

### `translatex:languages`
List supported language codes, optionally filtered by provider.

### `translatex:health`
Check health of all configured providers.

## Configuration

The CLI reads configuration from:
1. Environment variables `TRANSLATEX_<PROVIDER>_<FIELD>`
2. `~/.config/translatex/credentials.toml`
3. OS keyring (`@keyring` placeholder in TOML)

Example `credentials.toml`:
```toml
[deepl]
api_key = "@keyring"

[azure]
api_key = "@keyring"
region = "eastus"

[googlecloud]
api_key = "@keyring"

[openai]
api_key = "@keyring"
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
```

## Error Handling

All errors follow a consistent schema:

```json
{
  "code": "RATE_LIMITED",
  "message": "rate limited by deepl",
  "retryable": true,
  "retry_after_secs": 60,
  "suggested_action": "backoff_and_retry"
}
```

Agents should use `suggested_action` to decide how to handle errors.

## MCP Server

The CLI can run as an MCP server over stdio:

```bash
translatex mcp
```

Exposes tools:
- `translate` - Single translation with full fallback
- `providers` - List providers and capabilities
- `languages` - Supported language codes
- `health` - Provider health check

## License

MIT OR Apache-2.0