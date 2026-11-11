**简体中文** · [English](../../en/env/01-variables.md)

# 环境变量 — 参考

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

## 路径

| 变量 | 作用 | 默认 |
| --- | --- | --- |
| `NOT_A_BOT_HOME` | 根目录；配置、凭据、缓存都在其下 | `~/.config/not-a-bot` |
| `NOT_A_BOT_CONFIG` | 配置文件路径，优先于 `NOT_A_BOT_HOME` | `<home>/config.toml` |
| `NOT_A_BOT_CREDENTIALS` | 凭据文件路径 | `<home>/credentials.toml` |
| `XDG_CONFIG_HOME` | `NOT_A_BOT_HOME` 未设置时生效 | `~/.config` |

缓存与冷却文件位于 `<home>/cache/`。设置 `NOT_A_BOT_HOME` 可以隔离整套环境（含冷却状态），并行 CI 任务很有用。

## 行为

| 变量 | 作用 | 示例 |
| --- | --- | --- |
| `NOT_A_BOT_CHAIN` | 替换内置链。未列出的 Provider 之后按优先级追加 | `edge,google,deepl` |
| `NOT_A_BOT_TIMEOUT_MS` | 默认单次预算；请求里的 `timeoutMs` 覆盖它 | `8000` |
| `NOT_A_BOT_REWRITE_PIVOT` | 请求写 `auto` 时 `rewrite` 的默认中转语言；`--pivot` 优先 | `ja` |
| `NOT_A_BOT_REWRITE_PRESET` | 请求没给 chain、preset、depth、pivot 时 `rewrite` 的默认预设 | `standard` |
| `NOT_A_BOT_REWRITE_LLM` | `light`、`standard` 预设里 LLM 跳所用的引擎 | `claude-cli` |
| `NOT_A_BOT_LOG` | stderr 日志级别 | `debug` |

## 凭据

格式：`NOT_A_BOT_<PROVIDER>_<FIELD>`，全大写蛇形，Provider 与字段之间用 `_`。

| Provider | 变量 |
| --- | --- |
| DeepL | `NOT_A_BOT_DEEPL_API_KEY` |
| Azure | `NOT_A_BOT_AZURE_API_KEY`、`NOT_A_BOT_AZURE_REGION` |
| Google Cloud | `NOT_A_BOT_GOOGLECLOUD_API_KEY` |
| 百度 | `NOT_A_BOT_BAIDU_APPID`、`NOT_A_BOT_BAIDU_SECRET_KEY` |
| 腾讯云 | `NOT_A_BOT_TENCENT_SECRET_ID`、`NOT_A_BOT_TENCENT_SECRET_KEY` |
| 有道 | `NOT_A_BOT_YOUDAO_APP_KEY`、`NOT_A_BOT_YOUDAO_APP_SECRET` |
| 彩云 | `NOT_A_BOT_CAIYUN_TOKEN` |
| 小牛 | `NOT_A_BOT_NIUTRANS_APIKEY` |
| 阿里云 | `NOT_A_BOT_ALIYUN_ACCESS_KEY_ID`、`NOT_A_BOT_ALIYUN_ACCESS_KEY_SECRET` |
| OpenAI | `NOT_A_BOT_OPENAI_API_KEY` |
| Anthropic | `NOT_A_BOT_ANTHROPIC_API_KEY` |
| DeepSeek | `NOT_A_BOT_DEEPSEEK_API_KEY` |

环境变量凭据的标签是 `env`，因此无法按标签轮换。需要单 Provider 多 Key 时请用 `credentials.toml`。

## 优先级

没有单一赢家：几种来源往同一份轮换表里加 Key。

```text
credentials.toml 的标签  ──▶  env 标签（排在最后）  ──▶  免费网页接口（完全不需要凭据）
```

`NOT_A_BOT_<PROVIDER>_<FIELD>` 会在文件标签之后追加一个 `env` 凭据，所以文件里的 Key 先试，环境变量是该 Provider 的最后一路。`credentials.toml` 里写成 `@keyring` 的字段在加载时从系统钥匙串读取。

变量只设置了一部分（例如只有 `NOT_A_BOT_BAIDU_APPID`）时凭据不完整，该 Provider 不会注册。`not-a-bot providers --format json` 能看到最终的 `configured` 状态。

## CI 示例

```bash
export NOT_A_BOT_HOME="$PWD/.tx"
export NOT_A_BOT_CHAIN="edge,google,deepl"
export NOT_A_BOT_DEEPL_API_KEY="$DEEPL_TOKEN"
not-a-bot batch --file jobs.ndjson --format json > out.ndjson
```

## 导航

- ← [环境变量概览](OVERVIEW.md)
- → [配置文件](../cli/02-config-files.md)
