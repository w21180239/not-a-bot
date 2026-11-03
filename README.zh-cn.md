# translatex

**面向 Agent 的网页优先翻译工具。** 一个静态二进制：先打免费网页接口，有 Key 时再回退到官方 API，所有请求都以结构化 JSON 返回。

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [文档](./docs/README.md)

---

## 为什么网页优先

官方翻译 API 要付费、要注册，账单晚一天就整体不可用。而 Google 翻译、必应、讯飞、Papago、Reverso 背后的公开网页接口，本来就在免费回答同样的问题，质量相同，也没有配额表。`translatex` 把这些接口当主路径，把付费 API 当兜底：需要 SLA 时、需要免费接口不支持的语言时，才轮到它们。

```
请求 ─▶ 优先级 1-12   免费网页接口      （无 Key、无需注册）
     ─▶ 优先级 20-27  官方 API         （仅在已配置凭据时）
     ─▶ 优先级 40-42  LLM             （最后手段，需显式配置）
```

数字越小越先尝试。每一跳都会记录，失败时能直接看出查询过哪些引擎、各自为什么拒绝。

## 能力

| 能力 | 说明 |
| --- | --- |
| **26 个 Provider** | 14 个免费网页接口、9 个官方 API、3 个 LLM。LibreTranslate 通过 `base_url` 按需启用。 |
| **回退链** | 按优先级排序；默认最多三个 Provider 同时在飞，慢接口拖不住后面健康的通道。`--fallback` 顺序原样保留。 |
| **往返改写** | `roundtrip` 经中转语言翻出再翻回，逼出改写，去掉机器味。中转语言经实测选定：非中文用 `zh`，中文用 `en`。 |
| **密钥轮换** | 单 Provider 多密钥轮询，冷却状态跨进程共享于 `~/.config/translatex/cache/`。 |
| **结构化输出** | stdout 只输出 JSON / NDJSON，日志走 stderr，退出码可直接分支。 |
| **保护片段** | 正则匹配的代码、变量、URL 在翻译前打码，翻译后还原；还原失败会触发回退。 |
| **面向 Agent 的 CLI** | 每条命令都在 stdout 输出 JSON、stderr 输出日志，退出码可直接分支。推荐的 harness 接入方式是 CLI + skill：[integrations/](./integrations/README.md)。 |
| **真实超时** | 请求里的 `timeoutMs`（或配置里的 `timeout_ms`）限制整个 Provider 调用，不只是一次读。 |
| **诚实的错误** | 每个失败都带 `code`、`retryable`、`retryAfterSecs`、`suggestedAction`。 |

## 安装

```bash
npm install -g translatex                 # 按平台下载预编译二进制
cargo install translatex                 # 或源码构建，Rust 1.80+
# 或直接取预编译二进制
# https://github.com/w21180239/translatex/releases/latest
```

验证：

```bash
translatex --version
translatex providers --format text
```

## 快速开始

```bash
# 零配置，只走网页接口
translatex translate "Hello world" -t zh

# 指定 Provider 并保留回退列表
translatex translate "Deploy failed" -t ja --provider deepl --fallback google,edge

# 从 stdin 读取
echo "Good morning" | translatex translate -t fr

# 关闭自动检测，显式给源语言
translatex translate "Guten Tag" -s de -t en

# NDJSON 批量，并发 4
translatex batch --file examples/batch.ndjson --parallel 4

# 不做翻译的代码片段
translatex translate 'Run `make test` before pushing.' -t zh --preserve '`[^`]*`'

# 去机器味：经中文翻出再翻回英文
translatex roundtrip "In today's fast-paced landscape, it's important to note that …" --from en
```

成功响应：

```json
{
  "text": "你好，世界",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "c506d3bb-8eb2-451a-9e84-f2be63833f79",
  "chars": 11
}
```

失败响应携带可机读的原因：

```json
{
  "error": {
    "code": "PROVIDER_NOT_CONFIGURED",
    "message": "provider deepl is implemented but not configured; add credentials or a base_url",
    "retryable": false,
    "suggestedAction": "configure_a_provider"
  },
  "requestId": ""
}
```

## Agent 接入：CLI + skill

接入方式就是这条命令，外加一份告诉 Agent 何时调用的文档。没有服务要启动，也没有协议要协商。

```bash
# Claude Code
mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/

# Codex / Gemini CLI（共享的跨运行时位置）
mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/

# OpenCode
mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/

# 任何读取项目指令文件的 harness
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

同一份 skill 文本适用于所有 harness，只是存放位置不同。Claude Code、Codex、Gemini CLI、OpenCode、Cursor、Cline、Roo Code、Kilo Code、Continue、JetBrains AI、Zed、Aider、Qwen Code、Crush、Goose、DeepSeek Harness、MiMo Code、AtomCode、OpenClaw、Hermes Agent、Pi、Reasonix、ZCode、QwenPaw 以及其它 harness 的具体路径：[integrations/README.md](./integrations/README.md)。

偏好命令参考而非 skill 的 Agent，可以直接调用 CLI：

| 命令 | 用途 |
| --- | --- |
| `translatex translate "…" -t zh --format json` | 单条翻译 |
| `translatex roundtrip "…" --from en --format json` | 经中转语言往返 |
| `translatex providers --format json` | 本机能用哪些 Provider |
| `translatex health --format json` | 探测所有已配置 Provider |

## 往返改写：去 AI 味

翻到一个语法结构完全不同的语言，再翻回来，原文措辞无法照搬，只能重建 —— 机器味就是在这个过程中被洗掉的。一条命令，不用提示词：

```bash
translatex roundtrip "Our comprehensive solution leverages cutting-edge technology to empower teams." --from en --format json
```

```json
{
  "text": "Our integrated solutions use cutting-edge technology to give teams more power.",
  "detectedSource": "en",
  "pivotLang": "zh",
  "pivotText": "我们综合的解决方案利用尖端技术来增强团队的能力。",
  "provider": "google",
  "providers": ["google", "edge"],
  "latencyMs": 612,
  "requestId": "…",
  "chars": 82
}
```

默认中转：非中文用中文，中文用英文。这个组合是实测出来的 —— 德语、法语、西班牙语往返后几乎原样返回；日文改写强度与中文相当；俄文改写最多但漂移也最大。技术类英文要保精确时用 `--pivot ja`；合同、API 文档、带数字且无法核对的内容不要用。方法与数据见 [docs/zh-cn/roundtrip/01-pivot-choice.md](./docs/zh-cn/roundtrip/01-pivot-choice.md)。

它改写措辞但保留句子：AI 套话开头在任何语言里都会保留，所以要先把这些从句删掉再翻译，并告诉用户删了哪些。

## Provider 一览

优先级即回退链排序依据。`字符数` 是适配器在切分前强制限制的单次请求上限。

### 免费网页接口 —— 主路径

| # | Provider | ID | 自动检测 | 字符数 | 备注 |
| --- | --- | --- | --- | --- | --- |
| 1 | Google 翻译（网页） | `google` | 是 | 5 000 | `translate.googleapis.com/translate_a/single`，尽力而为 |
| 2 | 微软 Edge 翻译 | `edge` | 是 | 50 000 | `edge.microsoft.com/translate`，无需 Key |
| 3 | 爱词霸 | `iciba` | 是 | 3 000 | AES-ECB 签名 |
| 4 | 讯飞 iflyrec | `iflyrec` | 是 | 2 000 | 仅支持中文对其它语言 |
| 4 | ModernMT | `modernmt` | 是 | 5 000 | md5 校验 + 时间戳 |
| 5 | 火山引擎 | `volcengine` | 是 | 5 000 | `crx/translate/v1/` |
| 5 | Lara Translate | `lara` | 是 | 500 | 按 IP 的每日额度 |
| 6 | Lingvanex | `lingvanex` | 否 | 1 000 | `API_BEARER_TOKEN` |
| 7 | Mirai Translate | `mirai` | 是 | 500 | 仅日文相关语言对，每次调用抓取 trial key |
| 8 | Papago（Naver） | `papago` | 是 | 1 000 | 先取 Cookie 再表单 POST |
| 9 | Reverso Context | `reverso` | 否 | 2 000 | 三字母语言码；不在自动链中，见状态表 |
| 10 | 搜狗翻译 | `sogou` | 是 | 5 000 | md5 签名；按 IP 限流时返回 `RATE_LIMITED` |
| 11 | MyMemory | `mymemory` | 否 | 5 000 | RFC 3066 语言码，有每日上限 |
| 12 | SysTran | `systran` | 否 | 5 000 | 从前端 bundle 抓 OAuth client credentials |
| — | LibreTranslate | `libretranslate` | 是 | 5 000 | 仅自建：配置 `[provider.libretranslate] base_url` |

### 官方 API —— 有凭据时兜底

| # | Provider | ID | 凭据字段 |
| --- | --- | --- | --- |
| 20 | DeepL | `deepl` | `api_key` |
| 20 | 百度 | `baidu` | `appid`、`secret_key` |
| 21 | Google Cloud 翻译 | `googlecloud` | `api_key` |
| 22 | Azure 翻译 | `azure` | `api_key`、`region` |
| 23 | 阿里云 | `aliyun` | `access_key_id`、`access_key_secret` |
| 24 | 彩云小译 | `caiyun` | `token` |
| 25 | 小牛翻译 | `niutrans` | `apikey` |
| 26 | 腾讯云 | `tencent` | `secret_id`、`secret_key` |
| 27 | 有道 | `youdao` | `app_key`、`app_secret` |

### LLM —— 最后手段

| # | Provider | ID | 凭据字段 |
| --- | --- | --- | --- |
| 40 | OpenAI | `openai` | `api_key`，可选 `base_url`、`model` |
| 41 | Anthropic | `anthropic` | `api_key`，可选 `model` |
| 42 | DeepSeek | `deepseek` | `api_key`，可选 `model` |

## 实测状态

接口会变。以下是 2026-11-03 对免费网页接口做全量实测的结果：

| 状态 | Provider |
| --- | --- |
| 端到端可用 | `google`、`edge`、`iciba`、`modernmt`、`lara`、`volcengine`、`lingvanex`、`papago`、`sogou`、`mymemory`、`systran` |
| 语言对受限 | `iflyrec`（中文 ↔ 其它）、`mirai`（仅日文相关） |
| 高负载时限流 | `sogou` —— 按 IP 限流时报 `RATE_LIMITED` 并带 `retryAfterSecs`，绝不假装翻译成功 |
| 被指纹拦截 | `reverso` —— Cloudflare 拒绝 rustls 的 JA3；仍保留注册以便显式调用，但自动链会跳过它，自动请求不再浪费一次往返 |
| 仅自建 | `libretranslate` —— 已无可用公共实例，自建后配置 `base_url` |
| 上游停用 | `iflytek` —— 已别名到 `iflyrec`，旧配置继续可用 |

在你自己的网络环境里，先跑 `translatex health --format json` 再依赖这张表。

## 配置

`~/.config/translatex/config.toml`（也可用 `--config` 或 `TRANSLATEX_CONFIG`）：

```toml
# 替换内置 auto 链。有凭据的官方 Provider 仍会追加在其后。
chain = ["edge", "google", "iciba", "deepl", "openai"]

default_target = "zh"
timeout_ms = 20000
retries = 1
race_width = 3
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
```

`~/.config/translatex/credentials.toml`：

```toml
[deepl.default]
api_key = "..."

[azure.default]
api_key = "..."
region = "eastasia"

# 第一个 Key 冷却时由第二个接管。
[deepl.backup]
api_key = "..."
```

解析顺序：环境变量（`TRANSLATEX_<PROVIDER>_<FIELD>`）→ 凭据文件 → 系统钥匙串（`@keyring` 占位符）→ 免费接口。

## CLI 参考

| 命令 | 用途 |
| --- | --- |
| `translate [TEXT]` | 单条翻译；省略文本时读 stdin |
| `roundtrip [TEXT]` | 经中转语言往返，去掉机器味 |
| `batch` | NDJSON 进、NDJSON 出，顺序保持不变 |
| `providers` | 各 Provider 的能力与配置状态 |
| `languages` | 规范语言码，或某个 Provider 声明的集合 |
| `keys [PROVIDER]` | 冷却与失败状态；`--reset` 清空 |
| `health` | 探测所有已配置 Provider |
| `schema` | 全部线格式类型的 JSON Schema（draft 2020-12） |
| `manifest` | 机器可读的 CLI 操作清单 |
| `init` | 写出配置模板 |
| `completions SHELL` | Bash/zsh/fish/PowerShell 补全 |

全局参数：`--format json|text`、`--quiet`、`--verbose`、`--race N`、`--config`、`--credentials`、`--log`。

退出码：`0` 成功、`1` 请求错误、`2` 配置错误、`3` 链耗尽、`4` 网络错误。

## 错误契约

| `code` | 可重试 | Agent 应采取 |
| --- | --- | --- |
| `RATE_LIMITED` | 是 | 等待 `retryAfterSecs` 后重试 |
| `QUOTA_EXHAUSTED` | 是 | 换 Key 或等待 |
| `NETWORK_ERROR` | 是 | 指数退避重试 |
| `UPSTREAM_ERROR` | 仅 5xx | 5xx 重试，4xx 修输入 |
| `TIMEOUT` | 是 | 提高 `timeoutMs` 后重试，或换 Provider |
| `AUTH_FAILED` | 否 | 刷新凭据 |
| `PROVIDER_NOT_CONFIGURED` | 否 | 补凭据 |
| `PROVIDER_NOT_FOUND` | 否 | 修正 Provider 名 |
| `AUTO_DETECT_UNSUPPORTED` | 否 | 显式传 `sourceLang` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | 否 | 换 Provider 或目标语言 |
| `INVALID_INPUT` | 否 | 修正请求 |
| `CHAIN_EXHAUSTED` | 否 | 用 `--verbose` 看逐跳记录 |

完整说明：[docs/zh-cn/api/OVERVIEW.md](./docs/zh-cn/api/OVERVIEW.md)。

## 文档

| 语言 | 入口 |
| --- | --- |
| English | [docs/en/OVERVIEW.md](./docs/en/OVERVIEW.md) |
| 简体中文 | [docs/zh-cn/OVERVIEW.md](./docs/zh-cn/OVERVIEW.md) |
| 往返改写与选型 | [docs/zh-cn/roundtrip/OVERVIEW.md](./docs/zh-cn/roundtrip/OVERVIEW.md) |
| Harness 接入 | [integrations/README.md](./integrations/README.md) |
| Agent 集成 | [AGENTS.md](./AGENTS.md) |

## 开发

```bash
cargo test                # 253 个测试
cargo build --release     # 开 LTO 约 5 分钟
cargo clippy --all-targets
```

新增 Provider：在 `src/providers/<name>.rs` 实现 `Provider`，网页接口把 `priority` 放在 1-12、官方 API 放在 20-27，在 `src/cli.rs::build_registry` 注册，并把名字加进 `KNOWN_PROVIDERS`。

## 许可

MIT OR Apache-2.0
