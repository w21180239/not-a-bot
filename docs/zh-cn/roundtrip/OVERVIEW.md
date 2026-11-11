**简体中文** · [English](../../en/roundtrip/OVERVIEW.md)

# 往返翻译 — 概览

[← 文档索引](../README.md)

`roundtrip` 把文本经一跳或多跳翻译出去，再翻回原语言：

```
原文 ──▶ 第 1 跳 ──▶ 第 2 跳 ──▶ … ──▶ 原语言
```

起作用的是翻回来这一步。翻译到语法结构不同的语言后，原文的措辞无法照搬，翻回来必然是新句子。机器翻译跳的改写是副产品；LLM 跳的改写就是指令：每一跳都按"像人一样翻译并改写"来做。结果读起来更不像生成的文本，人名、数字和代码会拿原文逐项核对。

## 各类跳的作用

| 跳 | 改变 | 保留 | 成本 |
| --- | --- | --- | --- |
| 机器翻译（默认 `light`） | 用词 | 句子结构，"not just X, it's Y" 之类句式会留下 | 免费，约 1 秒 |
| LLM（`standard`、`strong`） | 用词和句子结构，去掉结构性的 AI 腔 | 事实，由事实检查核对 | 用你自己的 Claude 或 OpenCode 订阅，8 到 28 秒 |
| 修复（`<llm>:fix`） | 少量精确编辑，补回丢掉的事实或语义 | 改写结果 | 多一次 LLM 调用 |

实测 10 段文本、各跑一次：默认 `light` 还剩 5 段带 AI 腔句式、丢了 5 个事实；`standard` 剩 0 段、丢 0 个事实。完整表格见 [01-pivot-choice.md](01-pivot-choice.md#选择跳数)。

目标是让文字读起来像人写的，同时事实不丢。translatex 对 AI 检测器不做任何承诺，也不保证文本"检测不出"。

## 与翻译的先后

`roundtrip` 返回的是起始语言的文本，目标语言不同时要先翻译：

```bash
translatex translate "<text>" -t zh --format json          # 先译成中文
translatex roundtrip "<中文文本>" --from zh --format json   # 再对中文做人性化改写
```

## 不适合

往返会改写，任何需要精确的地方都可能漂移：

| 内容 | 是否适用 |
| --- | --- |
| 营销文案、公告、博客正文 | 适合 |
| 带少量人名和数字的正文 | 适合，并检查 `missingFacts` |
| 合同、法律、政策 | 不适合 |
| API 文档、配置示例、报错文本 | 不适合 |
| 读者会逐项核对的数字、日期、人名、版本号 | 不适合 |
| 代码、标识符、路径 | 永远不适合，用 `--preserve @code` 保护 |

事实检查列出结果里丢了什么，但它不能让读者要核对的文本变得可以往返。响应里的 `hops` 带每一跳的输出，可以核对中间结果。

## 接口

```bash
translatex roundtrip "In today's fast-paced landscape, …" --from en --format json
translatex roundtrip "…" --from en --preset standard              # LLM 出去再回来（需要 claude-cli 或 opencode-cli）
translatex roundtrip "…" --from en --preset strong       # 改写最强，外加一次修复
translatex roundtrip "…" --from en --depth 3                 # zh -> ja -> fi -> en，只用机器翻译
translatex roundtrip "…" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
translatex roundtrip --from en --pivot ja < draft.md         # 别名：translatex polish
translatex roundtrip "Run \`make test\`" --from en --preset standard --preserve @code
```

选跳规则，先匹配者生效：`--chain`、`--preset`、`--depth`、`--pivot`，最后是配置里的 `roundtrip_preset`（默认 `light`）。

| 参数 | 含义 |
| --- | --- |
| `--chain "<[engine:]lang,...>"` | 显式指定每一跳；末尾没回到原语言时自动补一跳 |
| `--preset <name>` | `light`（默认）、`standard`、`strong` |
| `--depth 1-3` | 只用机器翻译，依次经过 zh、ja、fi（中文原文为 en、ja、fi） |
| `--pivot <lang>` | 单个中转语言，或 `auto` |
| `--preserve <pattern>` | 正则，或具名模式 `@code`、`@url` |

链、预设、LLM Provider、修复跳和事实检查见 [02-chains-presets-and-llm.md](02-chains-presets-and-llm.md)。

```json
{
  "text": "…",
  "detectedSource": "en",
  "chain": ["zh", "en"],
  "hops": [
    {"from": "en", "to": "zh", "provider": "opencode-cli", "text": "……", "latencyMs": 11027},
    {"from": "zh", "to": "en", "provider": "opencode-cli", "text": "…", "latencyMs": 20508}
  ],
  "provider": "opencode-cli",
  "missingFacts": [],
  "latencyMs": 31544,
  "requestId": "…",
  "chars": 297
}
```

| 字段 | 含义 |
| --- | --- |
| `text` | 最终结果，已回到原语言 |
| `detectedSource` | 第一跳检测到的语言 |
| `chain` | 依次经过的语言（不含源语言）；修复跳记为 `fix` |
| `hops` | 每一跳的 `from`、`to`、`provider`、输出 `text`、`latencyMs`；修复跳多一个 `edits` |
| `hops[].edits` | 每条编辑的 `from`、`to`、`why`、`kind`（`fact` 或 `meaning`）、`applied`，被跳过时有 `reason` |
| `provider` | 完成最后一跳的 Provider |
| `missingFacts` | 输出中缺失的原文事实（人名、数字、ID、URL、代码） |
| `warnings` | 非空时出现：`fact check:`、`preserve:` 和引擎回退提示 |
| `latencyMs`、`requestId`、`chars` | 总耗时、关联 id、输入长度 |

`translatex polish` 是 `roundtrip` 的别名。

## 中转语言怎么选

`pivot: "auto"` 对非中文用中文、对中文用英文，是默认 `light` 预设的第一跳。依据与例外见 [01-pivot-choice.md](01-pivot-choice.md)。

单次覆盖用 `--pivot` / `"pivot"`，全局默认用 `config.toml` 的 `roundtrip_pivot`。

## 成本

一次往返的 Provider 调用数等于跳数：默认 `light` 是三次（中文输入在 auto 下偶尔更多，见 [01-pivot-choice.md](01-pivot-choice.md#auto-中转的重试逻辑)）。延迟约等于各跳之和，不是最大值。LLM 跳每跳几秒，并消耗订阅额度。

## 导航

- ← [文档索引](../README.md)
- → [选择中转语言和跳数](01-pivot-choice.md)
- → [链、预设与 LLM 跳](02-chains-presets-and-llm.md)
