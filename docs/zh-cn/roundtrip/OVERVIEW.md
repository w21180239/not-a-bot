**简体中文** · [English](../../en/roundtrip/OVERVIEW.md)

# 往返翻译 — 概览

`roundtrip` 先把文本翻译到中转语言，再翻回原语言：

```
原文 ──▶ 中转语言 ──▶ 原语言
        第一跳        第二跳
```

起作用的是第二跳。翻译到一个语法结构完全不同的语言后，原文的措辞无法照搬，翻回来必然是新句子。机器味就是这样被洗掉的 —— 不需要让模型"改写一遍"，改写是往返过程的副产品。

## 适合

- 像模板写出来的营销文案、落地页。
- 聊天回复、邮件、文档里一眼看出是生成的段落。
- 用户说"AI 味太重""太端着""太顺了"的文本。

## 不适合

往返会改写，任何需要精确的地方都可能漂移：

| 内容 | 是否适用 |
| --- | --- |
| 营销文案、公告、博客正文 | 适合 |
| 合同、法律、政策 | 不适合 |
| API 文档、配置示例、报错文本 | 不适合 |
| 数字、日期、人名、版本号 | 不适合 |
| 代码、标识符、路径 | 永远不适合 —— 用 `preserve` 保护 |

两者混在一起时（正文里带数字），要么把数字单独抽出来，要么事后核对。响应里有 `pivotText`，可以核对中间结果。

## 接口

```bash
translatex roundtrip "In today's fast-paced landscape, …" --from en --format json
translatex roundtrip draft.md --pivot ja          # 别名：translatex polish
```

```json
{
  "text": "……",
  "detectedSource": "en",
  "pivotLang": "zh",
  "pivotText": "……",
  "provider": "google",
  "providers": ["google", "edge"],
  "latencyMs": 612,
  "requestId": "…",
  "chars": 126
}
```

| 字段 | 含义 |
| --- | --- |
| `text` | 最终结果，已回到原语言 |
| `detectedSource` | 第一跳检测到的语言 |
| `pivotLang` | 实际使用的中转语言 |
| `pivotText` | 中间译文，便于核对 |
| `provider` | 完成最后一跳的 Provider |
| `providers` | 逐跳的 Provider，按顺序 |

MCP 中对应 `roundtrip` 工具。

## 中转语言怎么选

`pivot: "auto"`（默认）对非中文用中文、对中文用英文。依据与例外见 [01-pivot-choice.md](01-pivot-choice.md)。

单次覆盖用 `--pivot` / `"pivot"`，全局默认用 `config.toml` 的 `roundtrip_pivot`。

## 成本

一次往返 = 两次翻译，也就是两次 Provider 调用（中文输入在 auto 下偶尔是三次，见 [01-pivot-choice.md](01-pivot-choice.md)）。延迟约等于两跳之和，不是最大值。

## 导航

- ← [文档索引](../README.md)
- → [选择中转语言](01-pivot-choice.md)
