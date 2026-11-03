<div align="center">

# translatex

**给 Agent 的免费翻译：一条命令，不用 API Key，不用注册，不用配置。**

一个 6 MB 的静态二进制，内置 26 个翻译引擎，并发竞速。输出 JSON，错误带类型，24 家编码 Agent 复制一份 skill 就能用。

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#许可)
[![Tests](https://img.shields.io/badge/tests-245-brightgreen.svg)](#开发)
[![Providers](https://img.shields.io/badge/providers-26-blueviolet.svg)](#provider-一览)
[![Binary](https://img.shields.io/badge/binary-6%20MB-lightgrey.svg)](#安装)
[![Agent-native](https://img.shields.io/badge/agent--native-JSON-purple.svg)](#支持的-harness)

`translatex translate "Hello world" -t zh`

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [文档](./docs/README.md)

</div>

---

## 目录

- [为什么选 translatex](#为什么选-translatex)
- [能力](#能力)
- [安装](#安装)
- [快速开始](#快速开始)
- [支持的 harness](#支持的-harness)
- [往返改写：去 AI 味](#往返改写去-ai-味)
- [Provider 一览](#provider-一览)
- [实测状态](#实测状态)
- [配置](#配置)
- [CLI 参考](#cli-参考)
- [错误契约](#错误契约)
- [文档](#文档)
- [开发](#开发)

## 为什么选 translatex

| 为什么会被选 | 意味着什么 |
| --- | --- |
| **免费，不是试用** | 14 个公开接口 —— Google 翻译、必应/Edge、Papago、Reverso、搜狗等，无需 Key、无需账号、没有配额表。 |
| **零配置** | 装完就能翻。配置文件是用来固定 Provider、加 Key 的，不是启动门槛。 |
| **天生快** | 最多三个 Provider 同时竞速，先答者胜出，其余取消；慢接口拖不住请求。 |
| **Agent 原生** | stdout 出 JSON/NDJSON，日志走 stderr，退出码可分支，错误带 `retryable` 与 `suggestedAction`，`manifest` 列出全部操作。 |
| **不走 MCP** | 接入就是 CLI 加一份 skill 文档：没有进程、没有端口、没有协议握手，也没有要同步的配置。 |
| **去 AI 味** | `roundtrip` 经中转语言翻出再翻回，套话在往返里活不下来。 |
| **Key 越多越稳** | 需要 SLA 时接 DeepL、Azure、Google Cloud、百度、腾讯等 9 家官方 API；Key 自动轮换、自动冷却。 |
| **不用部署** | 一个 6 MB 静态二进制，无运行时、无 Docker、无数据库。笔记本、CI、容器都能跑。 |
| **代码原样保留** | 代码、标识符、URL、版本号按正则打码再还原；还原失败直接判定该跳失败。 |

脚本、CI、编码 Agent 都能直接用。本来准备去注册一个翻译 API 的话，从这里开始。

### 链路怎么排

```
请求 ─▶ 优先级 1-12   免费公开接口      （无 Key、无需注册）
     ─▶ 优先级 20-27  官方 API          （仅在已配置凭据时）
     ─▶ 优先级 40-42  LLM              （最后手段，需显式配置）
```

数字越小越先试。每一跳都有记录，失败时能看出查过哪些引擎、各自为什么退场。

## 能力

| 能力 | 说明 |
| --- | --- |
| **26 个 Provider** | 14 个免费网页接口、9 个官方 API、3 个 LLM。LibreTranslate 通过 `base_url` 按需启用。 |
| **回退链** | 按优先级排序；默认最多三个 Provider 同时在飞，慢接口拖不住后面健康的通道。`--fallback` 顺序原样保留。 |
| **往返改写** | `roundtrip` 经中转语言翻出再翻回，逼出改写，去掉机器味。中转语言经实测选定：非中文用 `zh`，中文用 `en`。 |
| **Key 轮换** | 单 Provider 多 Key 轮询，冷却状态跨进程共享在 `~/.config/translatex/cache/`。 |
| **结构化输出** | stdout 只输出 JSON / NDJSON，日志走 stderr，退出码可直接分支。 |
| **保护片段** | 正则匹配的代码、变量、URL 在翻译前打码，翻译后还原；还原失败会触发回退。 |
| **24 家 harness** | Claude Code、Codex、Cursor、Zed 等 24 家共用同一份 skill 或指令块，无需启动服务。路径见 [integrations/](./integrations/README.md)。 |
| **真实超时** | 请求里的 `timeoutMs`（或配置里的 `timeout_ms`）限制整个 Provider 调用，不只是一次读。 |
| **诚实的错误** | 每个失败都带 `code`、`retryable`、`retryAfterSecs`、`suggestedAction`。 |

## 安装

```bash
npm install -g translatex                 # 按平台下载预编译二进制
cargo install translatex                 # 或源码构建，Rust 1.89+
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

## 支持的 harness

接入方式是一条命令加一份文档：宿主会加载 skill 就给 skill，读项目规则就给规则文件，读项目指令就追加一段。没有服务要启动，也没有协议要协商。

<div align="center">
<table>
<tr>
<td align="center" width="150"><img src="repo-assets/agents/claude-code.png" width="44" alt="Claude Code"><br/><b>Claude Code</b><br/><sub>skill · 插件</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/codex.png" width="44" alt="Codex CLI"><br/><b>Codex CLI</b><br/><sub>skill · AGENTS.md</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/gemini-cli.png" width="44" alt="Gemini CLI"><br/><b>Gemini CLI</b><br/><sub>skill · GEMINI.md</sub></td>
<td align="center" width="150"><img src="repo-assets/agents/opencode.png" width="44" alt="OpenCode"><br/><b>OpenCode</b><br/><sub>skill · AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/cursor.png" width="44" alt="Cursor"><br/><b>Cursor</b><br/><sub>.cursor/rules</sub></td>
<td align="center"><img src="repo-assets/agents/cline.png" width="44" alt="Cline"><br/><b>Cline</b><br/><sub>.clinerules</sub></td>
<td align="center"><img src="repo-assets/agents/roo-code.png" width="44" alt="Roo Code"><br/><b>Roo Code</b><br/><sub>.roo/rules</sub></td>
<td align="center"><img src="repo-assets/agents/kilo-code.png" width="44" alt="Kilo Code"><br/><b>Kilo Code</b><br/><sub>.kilocode/rules</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/continue.png" width="44" alt="Continue"><br/><b>Continue</b><br/><sub>.continue/rules</sub></td>
<td align="center"><img src="repo-assets/agents/jetbrains.png" width="44" alt="JetBrains AI"><br/><b>JetBrains AI</b><br/><sub>.aiassistant/rules</sub></td>
<td align="center"><img src="repo-assets/agents/zed.png" width="44" alt="Zed"><br/><b>Zed</b><br/><sub>.rules</sub></td>
<td align="center"><img src="repo-assets/agents/aider.png" width="44" alt="Aider"><br/><b>Aider</b><br/><sub>CONVENTIONS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/qwen-code.png" width="44" alt="Qwen Code"><br/><b>Qwen Code</b><br/><sub>QWEN.md</sub></td>
<td align="center"><img src="repo-assets/agents/crush.png" width="44" alt="Crush"><br/><b>Crush</b><br/><sub>CRUSH.md</sub></td>
<td align="center"><img src="repo-assets/agents/goose.png" width="44" alt="Goose"><br/><b>Goose</b><br/><sub>指令</sub></td>
<td align="center"><img src="repo-assets/agents/deepseek-harness.png" width="44" alt="DeepSeek Harness"><br/><b>DeepSeek Harness</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/mimo-code.svg" width="44" alt="MiMo Code"><br/><b>MiMo Code</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/atomcode.png" width="44" alt="AtomCode"><br/><b>AtomCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/openclaw.png" width="44" alt="OpenClaw"><br/><b>OpenClaw</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/hermes-agent.png" width="44" alt="Hermes Agent"><br/><b>Hermes Agent</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="repo-assets/agents/pi.png" width="44" alt="Pi"><br/><b>Pi</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/reasonix.png" width="44" alt="Reasonix"><br/><b>Reasonix</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/zcode.png" width="44" alt="ZCode"><br/><b>ZCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="repo-assets/agents/qwenpaw.svg" width="44" alt="QwenPaw"><br/><b>QwenPaw</b><br/><sub>指令</sub></td>
</tr>
</table>

<i>24 家 harness，两份文档。其它只要能执行 shell，就直接走 CLI。</i>
</div>

```bash
# 会加载 skill 的宿主
mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/
mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/
mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/   # Codex、Gemini CLI

# 其余宿主按自己的文件名读同一段内容
cat integrations/guidance/AGENTS.md >> AGENTS.md
```

| 想了解 | 阅读 |
| --- | --- |
| 每家的确切路径与命令 | [integrations/README.md](./integrations/README.md) |
| 四条通道的区别与验证方法 | [docs/zh-cn/integrations/02-supported-harnesses.md](./docs/zh-cn/integrations/02-supported-harnesses.md) |
| Claude Code 的一条命令插件安装 | [integrations/claude-code/README.md](./integrations/claude-code/README.md) |

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
roundtrip_pivot = "auto"
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

解析顺序：`credentials.toml` 里的 Key 标签按文件顺序轮换，`TRANSLATEX_<PROVIDER>_<FIELD>` 在其后追加一个 `env` 标签，字段写成 `@keyring` 时从系统钥匙串读取。免费接口三者都不需要。

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
| `QUOTA_EXHAUSTED` | 否 | 换 Key 或等配额窗口恢复 |
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
cargo test                # 245 个测试
cargo build --release     # 开 LTO 约 5 分钟
cargo clippy --all-targets
```

新增 Provider：在 `src/providers/<name>.rs` 实现 `Provider`，网页接口把 `priority` 放在 1-12、官方 API 放在 20-27，在 `src/cli.rs::build_registry` 注册，并把名字加进 `KNOWN_PROVIDERS`。

## 许可

MIT OR Apache-2.0

任选 [Apache License 2.0](LICENSE-APACHE) 或 [MIT 许可](LICENSE-MIT) 其一使用。
