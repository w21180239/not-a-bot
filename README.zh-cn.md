<div align="center">

# translatex

**命令行 AI 人性化改写：把 AI 味的文字改得像人写的。同时也是给 Agent 用的免费翻译，一个二进制，不用 API Key。**

用你自己的 Claude 或 OpenCode 订阅，对 AI 生成的文字做人性化改写和润色。人名、数字、代码原样保留，丢了什么会明确告诉你。同一个 6 MB 二进制还能通过 28 个翻译引擎翻译，其中 14 个是免费网页接口、不需要 Key，并带一份 skill，24 家编码 Agent 复制即用。

[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#许可)
[![Tests](https://img.shields.io/badge/tests-319-brightgreen.svg)](#开发)
[![Providers](https://img.shields.io/badge/providers-28-blueviolet.svg)](#provider-一览)
[![Binary](https://img.shields.io/badge/binary-6%20MB-lightgrey.svg)](#安装)
[![Agent-native](https://img.shields.io/badge/agent--native-JSON-purple.svg)](#支持的-harness)

`translatex roundtrip "<AI 味的文字>" --from en --preset standard`

[English](./README.md) · [简体中文](./README.zh-cn.md) · [Русский](./README.ru.md) · [文档](./docs/README.md)

</div>

---

## 目录

- [你能得到什么](#你能得到什么)
- [改写前后](#改写前后)
- [选哪个预设](#选哪个预设)
- [实测结果](#实测结果)
- [为什么选 translatex](#为什么选-translatex)
- [能力](#能力)
- [安装](#安装)
- [快速开始](#快速开始)
- [支持的 harness](#支持的-harness)
- [往返改写：原理](#往返改写原理)
- [Provider 一览](#provider-一览)
- [实测状态](#实测状态)
- [配置](#配置)
- [CLI 参考](#cli-参考)
- [错误契约](#错误契约)
- [文档](#文档)
- [开发](#开发)

## 你能得到什么

一个 AI 文本人性化改写工具，加一个免费翻译 CLI，装在同一个二进制里。

- **AI 文字人性化改写。** `translatex roundtrip` 改写带 AI 味的文字（套话开头、"不仅……更是……"、"值得注意的是"），让它读起来像人写的。它用你自己的 Claude Code 或 OpenCode 订阅，不用 API Key，也没有额外账单。原理是由 LLM 来做的回译：翻到另一种语言再翻回来，两跳都在改写。
- **免费的往返。** 默认预设走免费网页引擎，零成本。它改用词、保留句子结构，速度快，改写力度较轻。
- **事实不丢。** 每次往返后都拿原文核对人名、数字、ID、URL 和代码。丢了什么，会列在 `missingFacts` 里并给出警告。
- **修复一跳。** `strong` 在最后加一跳，让 LLM 提出小编辑。translatex 只应用精确且很小的编辑，所以改写结果保留，丢掉的事实补回来。
- **给 Agent 的翻译。** `translatex translate` 最多让三个免费引擎（Google、Edge、Papago、搜狗等）竞速，输出 JSON，错误带类型。不用注册，不用配置。
- **不承诺骗过 AI 检测器。** 目标是读起来像人写的文字，同时事实不丢。没有任何方法能让文本稳定地检测不出，translatex 也不做这个承诺。

如果你在找 AI 人性化改写工具、去 AI 腔的办法、想在脚本或编码 Agent 里给 AI 生成的文字去 AI 味和润色：这是一个 CLI，Claude Code、Codex、OpenCode、Cursor 等 24 家 harness 都能调用，改写由 DeepSeek、Haiku 或你订阅里的任何模型完成。

## 改写前后

输入：一段带 AI 味、含人名和数字的英文。

```
In today's fast-paced landscape, Aroha's team isn't just shipping a feature, they're fundamentally transforming how 4,200 customers manage their invoices. It's worth noting that this comprehensive solution leverages cutting-edge automation to empower finance teams and unlock their full potential.
```

```bash
translatex roundtrip "<上面这段>" --from en --preset standard --format json
```

`--preset standard`（OpenCode CLI，DeepSeek，两跳共 31 秒）的输出，2026-11-11 跑一次，只保留主要字段：

```json
{
  "text": "Aroha's team is doing more than shipping one feature: they're changing how 4,200 customers manage their invoices. Automation takes the manual work off finance teams, so their time goes to work that's worth more.",
  "chain": ["zh", "en"],
  "hops": [
    {"from": "en", "to": "zh", "provider": "opencode-cli", "text": "Aroha 的团队正在做的，是让 4,200 家客户换一种方式管理发票，这次不只是上线一个功能。方案用自动化替财务团队省下手工活，让他们的时间用在更有价值的地方。", "latencyMs": 11027},
    {"from": "zh", "to": "en", "provider": "opencode-cli", "text": "Aroha's team is doing more than shipping one feature: they're changing how 4,200 customers manage their invoices. Automation takes the manual work off finance teams, so their time goes to work that's worth more.", "latencyMs": 20508}
  ],
  "missingFacts": [],
  "latencyMs": 31544
}
```

开头套话、模糊限定和大词都没了，Aroha 和 4,200 都在。有一处对比句式（"more than shipping one feature"）留了下来，所以结果更适合当作改得不错的初稿，人工再润色一遍还能更好。

同一段文字走默认的 `light` 预设（两跳免费机器翻译，1.0 秒），2026-11-11 跑一次：

```json
{
  "text": "In today's fast-paced environment, the Araha team is not just releasing features—they are fundamentally changing how their 4,200 customers manage their invoices. Notably, this integrated solution leverages cutting-edge automation technologies to empower finance teams and maximize their potential.",
  "chain": ["zh", "ja", "en"],
  "missingFacts": ["Aroha"],
  "warnings": ["fact check: missing Aroha"]
}
```

机器翻译改了用词，保留了每个句子："not just X" 和 "in today's" 都还在，人名变成了 Araha。事实检查把它抓了出来。

## 选哪个预设

| 预设 | 做什么 | 改写力度 | 速度 | 丢失事实（10 段文本） | 需要 |
| --- | --- | --- | --- | --- | --- |
| `light`（默认） | 两跳机器翻译（en 到 zh 到 ja 再到 en） | 轻：用词变，句子不变 | 约 1 秒 | 5 | 无 |
| `standard` | LLM 出去再回来，两跳都"像人一样翻译并改写" | 去掉结构性 AI 腔 | 8 到 14 秒 | 0 | Claude 或 OpenCode 订阅 |
| `strong` | LLM 跳到 zh 和 ja，机器翻译跳经 fi 回来，最后一跳修复 | 改写最强，且补回丢掉的事实 | 约 25 秒 | 1 | 订阅 |

润色英文正文选 `standard`，要改写得最多选 `strong`，更看重速度或要零成本时选 `light`。文本里有代码或链接时加 `--preserve @code --preserve @url`。`--depth 1-3` 和 `--chain` 可以手动搭出其他机器翻译链或混合链。

## 实测结果

2026-11-11 运行：每种配置 10 段文本，只跑一次（5 段 AI 味英文、3 段事实密集的职场英文、2 段中文）。脚本与语料见 [`benches/roundtrip-depth/`](./benches/roundtrip-depth/README.md)。

| 配置 | 与原文的词重合度 | 剩余 AI 腔（共 10） | 丢失事实 | 延迟 |
| --- | --- | --- | --- | --- |
| light，1 跳（`--depth 1`，0.1.x 默认） | 0.556 | 7 | 5 | 0.5 s |
| light，2 跳（0.3 默认） | 0.520 | 5 | 5 | 0.9 s |
| 3 跳机器翻译（`--depth 3`） | 0.480 | 6 | 8 | 1.1 s |
| standard，经 opencode-cli（DeepSeek） | 0.446 | 0 | 0 | 14 s |
| standard，经 claude-cli（Haiku） | 0.535 | 0 | 0 | 8 s |
| strong，claude-cli / opencode-cli | 0.369 / 0.396 | 0 / 1 | 1 / 1 | 24 / 28 s |

重合度是实词 Jaccard，越低说明改写越多。AI 腔指结构性套路（"not just/only X"、"more than just"、"it's worth noting"、"in today's"、"at its core"、不仅、更是、值得注意、当今）。丢失事实是事实检查报告缺失的人名、数字、ID 和代码片段数。

数据说明了什么：

- 机器翻译改用词、保留句子结构，所以 "not just X, it's Y" 在任何机器翻译深度下都会留下。
- 哪怕只有一跳机器翻译，也会改拼人名（Aroha 变 Aloha，Tom 变 Tomi）、展开缩写（NZD 变 "New Zealand dollars"）。
- 第三跳机器翻译丢的事实比多改写的部分更多，所以默认是两跳。
- LLM 跳去掉了结构性 AI 腔，在这批语料上事实全部保留。
- 放在 LLM 跳之后的机器翻译跳可能把对比句式带回来（"ei ainoastaan ... vaan myös" 回成 "not only ... but also"），所以没有修复跳的同一条链仍留几处。
- 修复跳把这条链的事实丢失从 3 到 5 个降到 1 个，并保留了大部分改写，这就是 `strong`。
- 早先的结论（2026-11-04）：德语、法语、西班牙语中转几乎不改写，中文和日文改写最多。见 [docs/zh-cn/roundtrip/01-pivot-choice.md](./docs/zh-cn/roundtrip/01-pivot-choice.md)。

局限：10 段文本、只跑一次，样本很小；AI 腔清单和事实检查都是启发式；重合度奖励的是"改得多"，不是"改得好"。这里没有测任何 AI 检测器。

## 为什么选 translatex

| 为什么会被选 | 意味着什么 |
| --- | --- |
| **用你的订阅做人性化改写** | `claude-cli` 和 `opencode-cli` 在你已付费的套餐上运行 `claude -p` 和 `opencode run`。不用 API Key，不用新账号。 |
| **事实有人核对** | 每次往返都会列出没能回来的人名、数字、ID、URL 和代码片段。 |
| **免费，不是试用** | 14 个公开接口（Google 翻译、必应/Edge、Papago、Reverso、搜狗等），无需 Key、无需账号、没有配额表。 |
| **零配置** | 装完就能翻。配置文件是用来固定 Provider、加 Key 的，不是启动门槛。 |
| **天生快** | 最多三个 Provider 同时竞速，先答者胜出，其余取消；慢接口拖不住请求。 |
| **Agent 原生** | stdout 出 JSON/NDJSON，日志走 stderr，退出码可分支，错误带 `retryable` 与 `suggestedAction`，`manifest` 列出全部操作。 |
| **不走 MCP** | 接入就是 CLI 加一份 skill 文档：没有进程、没有端口、没有协议握手，也没有要同步的配置。 |
| **Key 越多越稳** | 需要 SLA 时接 DeepL、Azure、Google Cloud、百度、腾讯等 9 家官方 API；Key 自动轮换、自动冷却。 |
| **不用部署** | 一个 6 MB 静态二进制，无运行时、无 Docker、无数据库。笔记本、CI、容器都能跑。 |
| **代码原样保留** | 正则片段，或具名模式 `@code`、`@url`，在机器翻译前打码、之后还原；还原失败直接判定该跳失败。 |

脚本、CI、编码 Agent 都能直接用。本来准备去注册一个翻译 API，或者把 AI 文字贴进某个改写网站的话，从这里开始。

### 链路怎么排

```
请求 ─▶ 优先级 1-12   免费公开接口      （无 Key、无需注册）
     ─▶ 优先级 20-27  官方 API          （仅在已配置凭据时）
     ─▶ 优先级 40-42  LLM              （最后手段，需显式配置）
     ─▶ 优先级 60-61  claude-cli、opencode-cli（本机 agent CLI，只在点名时运行）
```

数字越小越先试。每一跳都有记录，失败时能看出查过哪些引擎、各自为什么退场。

## 能力

| 能力 | 说明 |
| --- | --- |
| **人性化改写 / 去 AI 腔** | `roundtrip` 经一跳或多跳翻出再翻回。机器翻译跳免费；LLM 跳会重建句子。预设 `light`、`standard`、`strong`，或自己写 `--chain`。 |
| **事实检查** | 每次往返后给出 `missingFacts` 和警告：结果里丢掉的人名、数字、ID、URL 和代码。 |
| **修复跳** | `<llm>:fix` 让 LLM 提出小编辑，只应用精确的；丢失的事实一定补回，其余编辑共用一份预算，所以抄不回原文。 |
| **28 个 Provider** | 14 个免费网页接口、9 个官方 API、5 个 LLM（OpenAI、Anthropic、DeepSeek，以及本机的 `claude-cli` 和 `opencode-cli`）。LibreTranslate 通过 `base_url` 按需启用。 |
| **回退链** | 按优先级排序；默认最多三个 Provider 同时在飞，慢接口拖不住后面健康的通道。`--fallback` 顺序原样保留。 |
| **Key 轮换** | 单 Provider 多 Key 轮询，冷却状态跨进程共享在 `~/.config/translatex/cache/`。 |
| **结构化输出** | stdout 只输出 JSON / NDJSON，日志走 stderr，退出码可直接分支。 |
| **保护片段** | 正则或 `@code`、`@url` 匹配的片段在机器翻译前打码、翻译后还原；LLM 跳会拿到片段清单，事后检查。 |
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
translatex translate 'Run `make test` before pushing.' -t zh --preserve @code

# 人性化改写 AI 味文字：LLM 经中文翻出再翻回（用你的 Claude 或 OpenCode 订阅）
translatex roundtrip "In today's fast-paced landscape, it's important to note that …" --from en --preset standard --preserve @code

# 免费且快：两跳机器翻译，之后读 missingFacts
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
| `translatex roundtrip "…" --from en --preset standard --format json` | 人性化改写：经一跳或多跳翻出再翻回 |
| `translatex providers --format json` | 本机能用哪些 Provider |
| `translatex health --format json` | 探测所有已配置 Provider |

## 往返改写：原理

`roundtrip` 把文本经一跳或多跳翻译出去，再翻回原语言。翻到语法结构不同的语言，文本必须被重建，生成式文字的套话也就不会原样回来。机器翻译跳是顺带做到这一点；LLM 跳则被明确要求"像人一样翻译并改写"。

```bash
translatex roundtrip "<text>" --from en --preset standard --preserve @code --format json
translatex roundtrip "<text>" --from en --depth 3                    # zh、ja、fi 再回来，只用机器翻译
translatex roundtrip "<text>" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
translatex roundtrip --from en --pivot ja < draft.md                 # 别名：translatex polish
```

- **选跳。** 先匹配者生效：`--chain "<[engine:]lang,...>"`、`--preset <name>`、`--depth 1-3`、`--pivot <lang>`，最后是配置里的 `roundtrip_preset`（默认 `light`）。链末尾没回到源语言时，自动补一跳。
- **LLM Provider。** `claude-cli` 在你的 Claude 订阅上运行 `claude -p`（默认模型 haiku）；`opencode-cli` 在你的 OpenCode 额度上运行 `opencode run`（默认模型 `opencode-go/deepseek-v4.1-flash`）。它们不需要 Key，每跳几秒，只在被点名时运行：`auto` 从不选它们。`roundtrip_llm`（默认 `opencode-cli`）决定 `standard` 和 `strong` 预设用哪个引擎。
- **以原文为锚。** LLM 跳也会收到源文本，在改写之前先用最小的改动修复前面机器翻译跳丢掉的语义。
- **修复跳。** `<llm>:fix` 返回 JSON 小编辑列表（`from`、`to`、`why`）。translatex 只应用精确且很小的编辑。补回缺失事实的编辑一律应用；其余编辑共用一份小预算，所以这一跳抄不回原文。每条编辑记录在 `edits` 里，带 `kind`（`fact` 或 `meaning`）、`applied` 和 `reason`。
- **事实检查。** `missingFacts` 列出源文本里有、结果里没有的人名、数字、ID、代码片段和 URL（数字按数位比较，人名按整词比较），并加一条 `fact check:` 警告。与常见句首词同形的人名可能漏检。
- **优先引擎。** 点名的机器翻译引擎是"优先"而非"钉死"。遇到可重试错误（限流、超时、网络、额度），这一跳改用另一个引擎，并避开相邻两跳的引擎，同时加警告。LLM 跳从不回退。
- **占位符。** 机器翻译跳对受保护片段打码。LLM 跳原样接收片段，提示词里列出清单，片段丢了会加警告。

保护代码和链接请用 `--preserve @code` 和 `--preserve @url`，不要写 `` `[^`]*` `` 这类正则：在 Windows 上，npm 的 `.cmd` 外壳经 cmd.exe 运行，Python 或 Node 不经 shell 启动 translatex 时会吞掉其中的 `^`。

响应字段：`text`、`detectedSource`、`chain`（依次经过的语言）、`hops`（`from`、`to`、`provider`、`text`、`latencyMs`，修复跳多一个 `edits`）、`provider`（最后一跳）、`missingFacts`、`warnings`、`latencyMs`、`requestId`、`chars`。

合同、API 文档、配置和读者要核对的数字不要往返：每一跳都会改写，事实检查只是启发式。链、预设与编辑规则见 [docs/zh-cn/roundtrip/02-chains-presets-and-llm.md](./docs/zh-cn/roundtrip/02-chains-presets-and-llm.md)；测量与方法见 [docs/zh-cn/roundtrip/01-pivot-choice.md](./docs/zh-cn/roundtrip/01-pivot-choice.md)。

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
| 60 | Claude Code CLI | `claude-cli` | 无；在你的 Claude 订阅上运行 `claude -p`，可选 `model`（默认 haiku）和 `bin` |
| 61 | OpenCode CLI | `opencode-cli` | 无；在你的 OpenCode 额度上运行 `opencode run`，可选 `model`（默认 `opencode-go/deepseek-v4.1-flash`）和 `bin` |

`claude-cli` 和 `opencode-cli` 从不加入自动链，只在被点名时运行：`roundtrip` 的某一跳、预设、`--provider` 或 `--fallback`。

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
roundtrip_preset = "light"          # light、standard、strong
roundtrip_llm = "opencode-cli"   # 或 "claude-cli"；standard 和 strong 预设所用的引擎
log = "warn"

[provider.libretranslate]
base_url = "http://localhost:5000"

[provider.openai]
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[provider.claude-cli]
model = "haiku"
bin = "claude"

[provider.opencode-cli]
model = "opencode-go/deepseek-v4.1-flash"
bin = "opencode"
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
| `roundtrip [TEXT]` | 经一跳或多跳往返，人性化改写 AI 味文字（`--chain`、`--preset`、`--depth`、`--pivot`） |
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
| 往返改写：概览、链与预设、实测结果 | [docs/zh-cn/roundtrip/OVERVIEW.md](./docs/zh-cn/roundtrip/OVERVIEW.md) |
| 测量脚本与结果 | [benches/roundtrip-depth/README.md](./benches/roundtrip-depth/README.md) |
| Harness 接入 | [integrations/README.md](./integrations/README.md) |
| Agent 集成 | [AGENTS.md](./AGENTS.md) |

## 开发

```bash
cargo test                # 319 个测试
cargo build --release     # 开 LTO 约 5 分钟
cargo clippy --all-targets
```

新增 Provider：在 `src/providers/<name>.rs` 实现 `Provider`，网页接口把 `priority` 放在 1-12、官方 API 放在 20-27，在 `src/cli.rs::build_registry` 注册，并把名字加进 `KNOWN_PROVIDERS`。

## 许可

MIT OR Apache-2.0

任选 [Apache License 2.0](LICENSE-APACHE) 或 [MIT 许可](LICENSE-MIT) 其一使用。
