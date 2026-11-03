**English** · [简体中文](../../zh-cn/roundtrip/01-pivot-choice.md)

# Round trip — Choosing the pivot

The pivot has to be far enough from the source language to force a rewrite, and
well enough supported that both hops succeed. Those pull in opposite directions,
so the choice was measured rather than guessed.

## Method

Five deliberately AI-flavoured English sentences (stock openers, hedge phrases,
"not just X but Y" constructions) were round-tripped through six pivots on
2026-11-04, using the default auto chain. For each pair:

- **Word overlap** — Jaccard similarity between content words of the input and
  the result, stop words removed. Lower means more rewriting.
- **Length delta** — mean absolute change in length.
- **Latency** — wall clock for both hops.

Run it yourself with the script in the repository history, or with one command:

```bash
translatex roundtrip "In today's fast-paced digital landscape, …" --from en --pivot ja --format json
```

## Results

| Pivot | Word overlap | Length change | Latency | What happened |
| --- | --- | --- | --- | --- |
| `ru` | 0.561 | 6.9% | 634 ms | Most rewriting, but the most semantic drift |
| `ja` | 0.582 | 5.3% | 512 ms | Strong rewrite, keeps more structure than Chinese |
| `zh` | 0.618 | 7.1% | 600 ms | Strong rewrite, best provider coverage |
| `fr` | 0.675 | 2.2% | 586 ms | Mild; some sentences came back unchanged |
| `de` | 0.695 | 2.0% | 633 ms | Mild |
| `es` | 0.791 | 1.6% | 565 ms | Barely changes anything |

Representative outputs:

```
source  Our comprehensive solution leverages cutting-edge technology to empower
        teams to unlock their full potential and drive meaningful outcomes.
zh      Our integrated solutions leverage cutting-edge technology to empower
        teams to unlock their full potential and drive meaningful results.
ja      Our comprehensive solutions leverage cutting-edge technology to help
        teams maximize their potential and deliver meaningful results.
de      Our comprehensive solution leverages cutting-edge technology to empower
        teams to reach their full potential and achieve meaningful results.
```

German, French and Spanish preserve English sentence structure, so the round trip
returns almost the input. They are the wrong tool for this job.

## Recommendation

| Source | Default pivot | Why |
| --- | --- | --- |
| Anything non-Chinese | `zh` | Rewrites as much as Japanese, and is served by every provider including the Chinese-anchored ones |
| Chinese (`zh`, `zh-TW`, `yue`) | `en` | The mirror of the same rule |

`auto` implements exactly that table.

**Japanese is the documented alternative.** It rewrites slightly more and keeps
more grammatical structure, which is an advantage for technical English where
Chinese drops articles and plurals. Set it when the text must stay precise:

```bash
translatex roundtrip api-notes.md --from en --pivot ja
```

**Russian rewrites most but drifts most.** It is not the default for a reason:
in the sample it replaced "comprehensive" with "end-to-end" and dropped
qualifiers. Use it only when maximal rewriting matters more than fidelity.

## The auto-pivot retry

With `sourceLang: "auto"` the language is unknown when the pivot must be chosen,
so `auto` guesses Chinese first and inspects `detectedSource` afterwards. If the
text turns out to be Chinese, the guessed pivot *was* the source language, and
the first hop is redone into English. That costs one extra call only for Chinese
input; every other language takes two calls.

## Configuring it

Per request:

```json
{ "text": "…", "pivot": "ja" }
```

Every request:

```toml
# ~/.config/translatex/config.toml
roundtrip_pivot = "zh"
```

Navigation: ← [Round trip overview](OVERVIEW.md) · ↑ [Documentation index](../README.md)
