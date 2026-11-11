**简体中文** · [English](../../en/providers/01-supported-providers.md)

# Provider — 支持的后端

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

`字符数` 是适配器在切分前的单次请求上限。`自动` 表示接口能否检测源语言。状态来自实测，而非代码路径。

## 免费网页接口

| # | Provider | Id | 自动 | 字符数 | 接口 / 机制 | 状态 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Google 翻译 | `google` | 是 | 5 000 | `translate.googleapis.com/translate_a/single`，`client=gtx` | 实测可用 |
| 2 | 微软 Edge 翻译 | `edge` | 是 | 50 000 | `edge.microsoft.com/translate/auth` + `api-edge.cognitive.microsofttranslator.com` | 实测可用 |
| 3 | 爱词霸 | `iciba` | 是 | 3 000 | `dict.iciba.com`，AES-ECB 签名，响应加密 | 实测可用 |
| 4 | 讯飞 iFlyrec | `iflyrec` | 是 | 2 000 | `fanyi.iflyrec.com/TranslationService/v1/textAutoTranslation`，数字语言 id | 仅中文相关语言对 |
| 4 | ModernMT | `modernmt` | 是 | 5 000 | `webapi.modernmt.com/translate`，md5 校验 + 时间戳 | 实测可用 |
| 5 | 火山引擎 | `volcengine` | 是 | 5 000 | `translate.volcengine.com/crx/translate/v1/` | 实测可用 |
| 5 | Lara Translate | `lara` | 是 | 500 | `laratranslate.com`，从页面抓 Bearer token | 实测可用，有每日额度 |
| 6 | Lingvanex | `lingvanex` | 否 | 1 000 | `lingvanex.com/translation/api`，页面里的 `API_BEARER_TOKEN` | 实测可用 |
| 7 | Mirai Translate | `mirai` | 是 | 500 | `trial.miraitranslate.com`，每次调用抓 `var tran` 并打 trace | 实测可用，仅日文相关语言对 |
| 8 | Papago | `papago` | 是 | 1 000 | `papago.naver.com`，先取 Cookie 再表单 POST | 实测可用 |
| 9 | Reverso Context | `reverso` | 否 | 2 000 | `api.reverso.net/translate/v1/translation`，三字母语言码 | Cloudflare 拦截 rustls JA3；不在自动链中 |
| 10 | 搜狗翻译 | `sogou` | 是 | 5 000 | `fanyi.sogou.com/api/transpc/text/result`，md5 签名 | 按 IP 限流 → `RATE_LIMITED` |
| 11 | MyMemory | `mymemory` | 否 | 5 000 | `api.mymemory.translated.net/get`，RFC 3066 语言码 | 实测可用，有每日上限 |
| 12 | SysTran | `systran` | 否 | 5 000 | `translate.systran.net`，从 bundle 抓 OAuth client credentials | 实测可用 |
| — | LibreTranslate | `libretranslate` | 是 | 5 000 | 任意自建 `/translate` | 必须配置 `base_url` |

## 官方 API

| # | Provider | Id | 凭据字段 | 备注 |
| --- | --- | --- | --- | --- |
| 20 | DeepL | `deepl` | `api_key` | Free 与 Pro Key 结构相同 |
| 20 | 百度 | `baidu` | `appid`、`secret_key` | MD5 签名，独立语言码表 |
| 21 | Google Cloud 翻译 | `googlecloud` | `api_key` | `v2` 接口，按段重复 `q` |
| 22 | Azure 翻译 | `azure` | `api_key`、`region` | 订阅 Key 加 `Ocp-Apim-Subscription-Region` |
| 23 | 阿里云 | `aliyun` | `access_key_id`、`access_key_secret` | RPC 签名 v1 |
| 24 | 彩云小译 | `caiyun` | `token` | `api.interpreter.caiyunai.com` |
| 25 | 小牛翻译 | `niutrans` | `apikey` | https API v2 |
| 26 | 腾讯云 | `tencent` | `secret_id`、`secret_key` | TC3-HMAC-SHA256 |
| 27 | 有道 | `youdao` | `app_key`、`app_secret` | v3 签名，SHA-256 |

官方 Provider 只在凭据完整时注册，`not-a-bot providers` 会如实反映。

## LLM

| # | Provider | Id | 字段 |
| --- | --- | --- | --- |
| 40 | OpenAI | `openai` | `api_key`，可选 `base_url`、`model` |
| 41 | Anthropic | `anthropic` | `api_key`，可选 `model` |
| 42 | DeepSeek | `deepseek` | `api_key`，可选 `model` |

LLM 通过对话补全提示翻译。放在最后是因为按 token 计费，且可能改写而不是翻译。

### 本机 agent CLI

| # | Provider | Id | 执行 | 额度 | 默认模型 |
| --- | --- | --- | --- | --- | --- |
| 60 | Claude Code CLI | `claude-cli` | `claude -p` | 你的 Claude 订阅 | haiku |
| 61 | OpenCode CLI | `opencode-cli` | `opencode run` | 你的 OpenCode 额度 | `opencode-go/deepseek-v4.1-flash` |

不需要 API key。在 `[provider.claude-cli]` 或 `[provider.opencode-cli]` 下设置 `model` 和 `bin`。它们从不加入自动链：只在被点名时运行，即 `rewrite` 的某一跳、预设、`--provider` 或 `--fallback`。见 [../rewrite/02-chains-presets-and-llm.md](../rewrite/02-chains-presets-and-llm.md)。

## 已退役名称

| 名称 | 处理 |
| --- | --- |
| `iflytek` | 别名到 `iflyrec`。`fanyi.xfyun.cn` 需要登录账户，旧接口返回 404。 |

别名在查找时解析，因此 `--provider iflytek` 可用，且不会在链中重复出现。

## 导航

- ← [Provider 概览](OVERVIEW.md)
- → [优先级与回退](02-priorities-and-fallback.md)
