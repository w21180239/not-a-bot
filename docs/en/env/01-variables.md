**English** · [简体中文](../../zh-cn/env/01-variables.md)

# Environment — Variable reference

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## Paths

| Variable | Effect | Default |
| --- | --- | --- |
| `TRANSLATEX_HOME` | Base directory; config, credentials and cache live under it | `~/.config/translatex` |
| `TRANSLATEX_CONFIG` | Config file path; overrides `TRANSLATEX_HOME` | `<home>/config.toml` |
| `TRANSLATEX_CREDENTIALS` | Credentials file path | `<home>/credentials.toml` |
| `XDG_CONFIG_HOME` | Respected when `TRANSLATEX_HOME` is unset | `~/.config` |

Cache and cooldown files live under `<home>/cache/`. Setting `TRANSLATEX_HOME` isolates a whole environment, including cooldown state — useful for parallel CI jobs.

## Behaviour

| Variable | Effect | Example |
| --- | --- | --- |
| `TRANSLATEX_CHAIN` | Replaces the built-in chain. Providers not listed are appended afterwards by priority. | `edge,google,deepl` |
| `TRANSLATEX_TIMEOUT_MS` | Default per-attempt budget. `timeoutMs` in a request overrides it. | `8000` |
| `TRANSLATEX_ROUNDTRIP_PIVOT` | Default `roundtrip` pivot when the request says `auto`. `--pivot` wins. | `ja` |
| `TRANSLATEX_LOG` | stderr log level | `debug` |

## Credentials

Pattern: `TRANSLATEX_<PROVIDER>_<FIELD>`, upper-snake, provider and field separated by `_`.

| Provider | Variables |
| --- | --- |
| DeepL | `TRANSLATEX_DEEPL_API_KEY` |
| Azure | `TRANSLATEX_AZURE_API_KEY`, `TRANSLATEX_AZURE_REGION` |
| Google Cloud | `TRANSLATEX_GOOGLECLOUD_API_KEY` |
| Baidu | `TRANSLATEX_BAIDU_APPID`, `TRANSLATEX_BAIDU_SECRET_KEY` |
| Tencent | `TRANSLATEX_TENCENT_SECRET_ID`, `TRANSLATEX_TENCENT_SECRET_KEY` |
| Youdao | `TRANSLATEX_YOUDAO_APP_KEY`, `TRANSLATEX_YOUDAO_APP_SECRET` |
| Caiyun | `TRANSLATEX_CAIYUN_TOKEN` |
| Niutrans | `TRANSLATEX_NIUTRANS_APIKEY` |
| Alibaba Cloud | `TRANSLATEX_ALIYUN_ACCESS_KEY_ID`, `TRANSLATEX_ALIYUN_ACCESS_KEY_SECRET` |
| OpenAI | `TRANSLATEX_OPENAI_API_KEY` |
| Anthropic | `TRANSLATEX_ANTHROPIC_API_KEY` |
| DeepSeek | `TRANSLATEX_DEEPSEEK_API_KEY` |

Environment credentials appear with the label `env` and therefore cannot be rotated per-label. Use `credentials.toml` when you need several keys per provider.

## Precedence

There is no single winner; each source contributes keys to the same rotation:

```text
credentials.toml labels  ──▶  env label (last)  ──▶  free web endpoints (no credential at all)
```

`TRANSLATEX_<PROVIDER>_<FIELD>` adds an `env` credential after the file's labels, so file keys are tried first and the environment key is the last resort for that provider. A field set to `@keyring` in `credentials.toml` is read from the OS keyring at load time.

A partially set variable (for example only `TRANSLATEX_BAIDU_APPID`) leaves the credential incomplete, and the provider stays unregistered. `translatex providers --format json` shows the resulting `configured` state.

## CI example

```bash
export TRANSLATEX_HOME="$PWD/.tx"
export TRANSLATEX_CHAIN="edge,google,deepl"
export TRANSLATEX_DEEPL_API_KEY="$DEEPL_TOKEN"
translatex batch --file jobs.ndjson --format json > out.ndjson
```

## Navigation

- ← [Environment overview](OVERVIEW.md)
- → [Config files](../cli/02-config-files.md)
