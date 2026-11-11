**简体中文** · [English](../../en/roundtrip/01-pivot-choice.md)

# 往返翻译 — 中转语言与跳数的选择

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

本页前半讲中转语言（2026-11-04），后半讲该走几跳、是否交给 LLM（2026-11-11）。

## 选择中转语言

中转语言要离原文足够远，才能逼出改写；同时又要有足够多的 Provider 支持，两跳都能成功。这两个要求方向相反，所以是实测出来的，不是拍的。

## 方法

2026-11-04，用五句刻意写成"AI 味"的英文（套话开头、模糊限定、"not just X but Y" 句式），经六个中转语言各跑一次往返，走默认自动链。对每组统计：

- **词重合度** —— 输入与结果实词（去停用词）的 Jaccard 相似度，越低说明改写越多。
- **长度变化** —— 长度绝对变化的平均值。
- **延迟** —— 两跳总耗时。

2026-11-04 的脚本没有保留。想在单段文本上快速验证，用一条命令：

```bash
translatex roundtrip "In today's fast-paced digital landscape, …" --from en --pivot ja --format json
```

## 结果

| 中转语言 | 词重合度 | 长度变化 | 延迟 | 实际表现 |
| --- | --- | --- | --- | --- |
| `ru` | 0.561 | 6.9% | 634 ms | 改写最多，语义漂移也最大 |
| `ja` | 0.582 | 5.3% | 512 ms | 改写强，比中文保留更多结构 |
| `zh` | 0.618 | 7.1% | 600 ms | 改写强，Provider 覆盖最好 |
| `fr` | 0.675 | 2.2% | 586 ms | 轻微，部分句子原样返回 |
| `de` | 0.695 | 2.0% | 633 ms | 轻微 |
| `es` | 0.791 | 1.6% | 565 ms | 基本没变 |

典型输出：

```
原文   Our comprehensive solution leverages cutting-edge technology to empower
       teams to unlock their full potential and drive meaningful outcomes.
zh     Our integrated solutions leverage cutting-edge technology to empower
       teams to unlock their full potential and drive meaningful results.
ja     Our comprehensive solutions leverage cutting-edge technology to help
       teams maximize their potential and deliver meaningful results.
de     Our comprehensive solution leverages cutting-edge technology to empower
       teams to reach their full potential and achieve meaningful results.
```

德语、法语、西班牙语与英语结构相近，往返之后几乎原样返回，达不到目的。

## 推荐

| 原文语言 | 默认中转 | 理由 |
| --- | --- | --- |
| 非中文 | `zh` | 改写强度与日文相当，且所有 Provider 都支持（含只做中文对的） |
| 中文（`zh`、`zh-TW`、`yue`） | `en` | 同一规则的镜像 |

`auto` 就是这张表。它是默认 `light` 预设的第一跳，也是 `--pivot` 的唯一一跳；`--depth` 在它之后再加 ja 和 fi。

**日文是文档给出的备选。** 它改写略多、保留的语法结构更多，因此更适合技术类英文——以中文为中转会丢掉冠词和单复数：

```bash
translatex roundtrip --from en --pivot ja < api-notes.md
```

**俄文改写最多，但漂移最明显**，所以不是默认：样本里它把 "comprehensive" 换成了 "end-to-end" 并丢了限定词。只有在"改写优先于保真"时才用。

## auto 中转的重试逻辑

`sourceLang: "auto"` 时选中转语言还不知道原文是什么，所以 `auto` 先押中文，拿到 `detectedSource` 后再判断。如果原文就是中文，说明押的中转等于原文，第一跳会用英文重做一次。只有中文输入会多花一次调用，其它语言每跳一次调用。

## 配置

单次：

```json
{ "text": "…", "pivot": "ja" }
```

全局：

```toml
# ~/.config/translatex/config.toml
roundtrip_pivot = "zh"
```

## 选择跳数

第二跳、第三跳机器翻译还能多改写多少？换成 LLM 跳又会怎样？[`benches/roundtrip-depth/`](../../../benches/roundtrip-depth/README.md) 用每种配置 10 段文本来回答：5 段 AI 味英文、3 段事实密集的职场英文（含人名、数字、ID、代码）、2 段中文。

对每段文本、每种配置的统计方法：

- **重合度**：与原文的实词 Jaccard，越低说明改写越多。
- **剩余 AI 腔**：10 段里仍带结构性 AI 腔的段数："not just/only X"、"more than just"、"it's worth noting"、"in today's"、"at its core"、不仅、更是、值得注意、当今。
- **丢失事实**：事实检查报告缺失的人名、数字、ID 和代码片段，全语料求和。
- **延迟**：整条链的平均耗时。

```bash
python benches/roundtrip-depth/measure.py --bin target/release/translatex.exe --out out --configs light,standard-opencode
```

结果（2026-11-11 运行；每种配置 10 段文本，只跑一次）：

| 配置 | 重合度 | 剩余 AI 腔（共 10） | 丢失事实 | 延迟 |
| --- | --- | --- | --- | --- |
| light，1 跳（`--depth 1`，0.1.x 默认） | 0.556 | 7 | 5 | 0.5 s |
| light，2 跳（0.3 默认） | 0.520 | 5 | 5 | 0.9 s |
| 3 跳机器翻译（`--depth 3`） | 0.480 | 6 | 8 | 1.1 s |
| standard，经 opencode-cli（DeepSeek） | 0.446 | 0 | 0 | 14 s |
| standard，经 claude-cli（Haiku） | 0.535 | 0 | 0 | 8 s |
| strong，claude-cli / opencode-cli | 0.369 / 0.396 | 0 / 1 | 1 / 1 | 24 / 28 s |

数字说明了什么：

- 机器翻译改用词、保留句子结构，所以 "not just X, it's Y" 在任何深度都会留下。
- 哪怕只有一跳机器翻译，也会改拼人名（Aroha 变成 Aloha，Tom 变成 Tomi）、展开缩写代码（NZD 变成 "New Zealand dollars"）。
- 第三跳机器翻译丢的事实（8 对 5）比多改写的部分（重合度 0.480 对 0.520）更多。所以默认从一跳改成两跳，三跳仍可用 `--depth 3`。
- LLM 跳去掉了结构性 AI 腔，在这批语料上事实全部保留。
- 放在 LLM 跳之后的机器翻译跳可能把对比句式带回来：芬兰语 "ei ainoastaan ... vaan myös" 回成 "not only ... but also"。所以没有修复跳的同一条链在部分运行里仍剩一处。
- 修复跳把这条链的事实丢失从 3 到 5 个降到 1 个，并保留了大部分改写（重合度 0.369 和 0.396，没有修复跳时是 0.292 和 0.307）。带修复跳的这条链就是 `strong`。

局限：每种配置只有 10 段文本、只跑一次，样本很小；AI 腔清单和事实检查都是启发式；重合度奖励的是"改得多"，不是"改得好"；这里没有测任何 AI 检测器，也不能把任何结果读成"检测不出"。

## 导航

- ← [往返翻译概览](OVERVIEW.md)
- → [链、预设与 LLM 跳](02-chains-presets-and-llm.md)
- ↑ [文档索引](../README.md)
