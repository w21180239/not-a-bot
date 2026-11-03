**简体中文** · [English](../../en/roundtrip/01-pivot-choice.md)

# 往返翻译 — 中转语言的选择

中转语言要离原文足够远，才能逼出改写；同时又要有足够多的 Provider 支持，两跳都能成功。这两个要求方向相反，所以是实测出来的，不是拍的。

## 方法

2026-11-04，用五句刻意写成"AI 味"的英文（套话开头、模糊限定、"not just X but Y" 句式），经六个中转语言各跑一次往返，走默认自动链。对每组统计：

- **词重合度** —— 输入与结果实词（去停用词）的 Jaccard 相似度，越低说明改写越多。
- **长度变化** —— 长度绝对变化的平均值。
- **延迟** —— 两跳总耗时。

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

`auto` 就是这张表。

**日文是官方备选。** 它改写略多、保留的语法结构更多，对中文会丢掉冠词和单复数的技术类文本更合适：

```bash
translatex roundtrip api-notes.md --from en --pivot ja
```

**俄文改写最多，但漂移最明显**，所以不是默认：样本里它把 "comprehensive" 换成了 "end-to-end" 并丢了限定词。只有在"改写优先于保真"时才用。

## auto 的回退逻辑

`sourceLang: "auto"` 时选中转语言还不知道原文是什么，所以 `auto` 先押中文，拿到 `detectedSource` 后再判断。如果原文就是中文，说明押的中转等于原文，第一跳会用英文重做一次。只有中文输入会多花一次调用，其它语言都是两跳。

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

导航：← [往返翻译概览](OVERVIEW.md) · ↑ [文档索引](../README.md)
