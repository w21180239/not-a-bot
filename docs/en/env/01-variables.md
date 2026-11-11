**English** · [简体中文](../../zh-cn/env/01-variables.md)

# Environment — Variable reference

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## Paths

| Variable | Effect | Default |
| --- | --- | --- |
| `NOT_A_BOT_HOME` | Base directory; config, credentials and cache live under it | `~/.config/not-a-bot` |
| `NOT_A_BOT_CONFIG` | Config file path; overrides `NOT_A_BOT_HOME` | `<home>/config.toml` |
| `NOT_A_BOT_CREDENTIALS` | Credentials file path | `<home>/credentials.toml` |
| `XDG_CONFIG_HOME` | Respected when `NOT_A_BOT_HOME` is unset | `~/.config` |

Cache and cooldown files live under `<home>/cache/`. Setting `NOT_A_BOT_HOME` isolates a whole environment, including cooldown state — useful for parallel CI jobs.

## Behaviour

| Variable | Effect | Example |
| --- | --- | --- |
| `NOT_A_BOT_CHAIN` | Replaces the built-in chain. Providers not listed are appended afterwards by priority. | `edge,google,deepl` |
| `NOT_A_BOT_TIMEOUT_MS` | Default per-attempt budget. `timeoutMs` in a request overrides it. | `8000` |
| `NOT_A_BOT_REWRITE_PIVOT` | Default `rewrite` pivot when the request says `auto`. `--pivot` wins. | `ja` |
| `NOT_A_BOT_REWRITE_PRESET` | Default `rewrite` preset when the request names no chain, preset, depth or pivot. | `standard` |
| `NOT_A_BOT_REWRITE_LLM` | Engine behind the LLM hops of the `standard` and `strong` presets. | `claude-cli` |
| `NOT_A_BOT_LOG` | stderr log level | `debug` |

## Credentials

Pattern: `NOT_A_BOT_<PROVIDER>_<FIELD>`, upper-snake, provider and field separated by `_`.

| Provider | Variables |
| --- | --- |
| DeepL | `NOT_A_BOT_DEEPL_API_KEY` |
| Azure | `NOT_A_BOT_AZURE_API_KEY`, `NOT_A_BOT_AZURE_REGION` |
| Google Cloud | `NOT_A_BOT_GOOGLECLOUD_API_KEY` |
| Baidu | `NOT_A_BOT_BAIDU_APPID`, `NOT_A_BOT_BAIDU_SECRET_KEY` |
| Tencent | `NOT_A_BOT_TENCENT_SECRET_ID`, `NOT_A_BOT_TENCENT_SECRET_KEY` |
| Youdao | `NOT_A_BOT_YOUDAO_APP_KEY`, `NOT_A_BOT_YOUDAO_APP_SECRET` |
| Caiyun | `NOT_A_BOT_CAIYUN_TOKEN` |
| Niutrans | `NOT_A_BOT_NIUTRANS_APIKEY` |
| Alibaba Cloud | `NOT_A_BOT_ALIYUN_ACCESS_KEY_ID`, `NOT_A_BOT_ALIYUN_ACCESS_KEY_SECRET` |
| OpenAI | `NOT_A_BOT_OPENAI_API_KEY` |
| Anthropic | `NOT_A_BOT_ANTHROPIC_API_KEY` |
| DeepSeek | `NOT_A_BOT_DEEPSEEK_API_KEY` |

Environment credentials appear with the label `env` and therefore cannot be rotated per-label. Use `credentials.toml` when you need several keys per provider.

## Precedence

There is no single winner; each source contributes keys to the same rotation:

```text
credentials.toml labels  ──▶  env label (last)  ──▶  free web endpoints (no credential at all)
```

`NOT_A_BOT_<PROVIDER>_<FIELD>` adds an `env` credential after the file's labels, so file keys are tried first and the environment key is the last resort for that provider. A field set to `@keyring` in `credentials.toml` is read from the OS keyring at load time.

A partially set variable (for example only `NOT_A_BOT_BAIDU_APPID`) leaves the credential incomplete, and the provider stays unregistered. `not-a-bot providers --format json` shows the resulting `configured` state.

## CI example

```bash
export NOT_A_BOT_HOME="$PWD/.tx"
export NOT_A_BOT_CHAIN="edge,google,deepl"
export NOT_A_BOT_DEEPL_API_KEY="$DEEPL_TOKEN"
not-a-bot batch --file jobs.ndjson --format json > out.ndjson
```

## Navigation

- ← [Environment overview](OVERVIEW.md)
- → [Config files](../cli/02-config-files.md)
