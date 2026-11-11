**English** · [简体中文](../../zh-cn/cli/02-config-files.md)

# CLI — Config and credential files

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## Locations

| File | Default path | Override |
| --- | --- | --- |
| Config | `~/.config/not-a-bot/config.toml` | `--config`, `NOT_A_BOT_CONFIG`, `NOT_A_BOT_HOME`, `XDG_CONFIG_HOME` |
| Credentials | `~/.config/not-a-bot/credentials.toml` | `--credentials`, `NOT_A_BOT_CREDENTIALS` |
| Cooldown state | `~/.config/not-a-bot/cache/keys-<provider>.json` | follows the config directory |

A missing file is not an error: defaults apply.

## `config.toml`

```toml
# Replaces the built-in auto chain.
chain = ["edge", "google", "iciba", "deepl", "openai"]

default_target = "zh"
timeout_ms = 20000     # whole-call budget per provider attempt
retries = 1            # extra attempts for retryable errors (not timeouts)
race_width = 3         # providers in flight at once; 1 is strictly sequential
rewrite_pivot = "zh" # default bridge language for `not-a-bot rewrite --pivot auto`
rewrite_preset = "standard" # default round trip when no chain, preset, depth or pivot is given
rewrite_llm = "opencode-cli" # engine behind the LLM hops of `light` and `standard`; or "claude-cli"
concurrency = 4        # reserved for batch sizing
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[provider.azure]
region = "eastasia"

[provider.claude-cli]
model = "haiku"
bin = "claude"

[provider.opencode-cli]
model = "opencode-go/deepseek-v4.1-flash"
bin = "opencode"
```

Rules:

- `deny_unknown_fields` is on. A typo is a `CONFIG_ERROR`, not a silently ignored line.
- Providers named in `chain` run first, in exactly that order, whatever their priority. Everything else registered is appended by ascending priority.
- `timeout_ms` and `retries` set request defaults; `timeoutMs` in a request overrides the timeout.
- `race_width` sets how many providers may be in flight; `1` restores the strict sequential walk. The `--race` flag overrides it for one invocation.
- `rewrite_pivot` is the default bridge for `rewrite` when the request says `"pivot": "auto"`.
- `rewrite_preset` picks the round trip when the request names no `chain`, `preset`, `depth` or `pivot`. `rewrite_llm` names the engine behind the LLM hops of `light` and `standard`.
- `[provider.<name>]` accepts `base_url`, `model`, `region`, `instance`, `tier`, `bin`. Unknown keys there are rejected too.

## `credentials.toml`

```toml
[deepl.default]
api_key = "..."

[deepl.backup]
api_key = "..."

[azure.default]
api_key = "..."
region = "eastasia"

[baidu.default]
appid = "..."
secret_key = "..."

[tencent.default]
secret_id = "..."
secret_key = "..."
```

The section is `<provider>.<label>`. Labels are the rotation order and appear in `keys` output and in `--verbose` attempt records.

## Precedence

Four sources, in this order:

1. Built-in free endpoints, which need no credentials.
2. Labels from `~/.config/not-a-bot/credentials.toml`, tried in file order.
3. `NOT_A_BOT_<PROVIDER>_<FIELD>` environment variables, which become one extra `env` label tried after the file's labels.
4. A field whose value is `@keyring` is read from the OS keyring when the file loads.

Field names are the provider's own: `NOT_A_BOT_DEEPL_API_KEY`, `NOT_A_BOT_AZURE_REGION`, `NOT_A_BOT_BAIDU_SECRET_KEY`.

## Keyring placeholders

Any field may be `"@keyring"`:

```toml
[deepl.default]
api_key = "@keyring"
```

The value is read from the OS keyring entry named `<provider>.<field>` under the service `translators`, which is the same account layout the Python `translators` fork used, so an existing keyring keeps working. A missing entry is a `CONFIG_ERROR` that names the account.

## Validation

```bash
not-a-bot providers --format json   # configured: true/false per provider
not-a-bot keys deepl --format json  # labels plus cooldown state
```

## Navigation

- ← [CLI commands](01-commands.md)
- → [Environment variables](../env/01-variables.md)
