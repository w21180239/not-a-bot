**English** · [简体中文](../../zh-cn/rewrite/01-pivot-choice.md)

# Round trip — Choosing the pivot and the depth

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

The first half of this page covers the pivot language (2026-11-04). The second
half covers how many hops, and whether an LLM does them (2026-11-11).

## Choosing the pivot

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

The 2026-11-04 script was not kept. For a quick check on one text, run:

```bash
not-a-bot rewrite "In today's fast-paced digital landscape, …" --from en --pivot ja --format json
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

`auto` implements exactly that table. It is the first hop of the `light`
preset and the only hop of `--pivot`; `--depth` adds ja and fi after it.

**Japanese is the documented alternative.** It rewrites slightly more and keeps
more grammatical structure, which is an advantage for technical English where
Chinese drops articles and plurals. Set it when the text must stay precise:

```bash
not-a-bot rewrite api-notes.md --from en --pivot ja
```

**Russian rewrites most but drifts most.** It is not the default for a reason:
in the sample it replaced "comprehensive" with "end-to-end" and dropped
qualifiers. Use it only when maximal rewriting matters more than fidelity.

## The auto-pivot retry

With `sourceLang: "auto"` the language is unknown when the pivot must be chosen,
so `auto` guesses Chinese first and inspects `detectedSource` afterwards. If the
text turns out to be Chinese, the guessed pivot *was* the source language, and
the first hop is redone into English. That costs one extra call only for Chinese
input; every other language takes one call per hop.

## Configuring it

Per request:

```json
{ "text": "…", "pivot": "ja" }
```

Every request:

```toml
# ~/.config/not-a-bot/config.toml
rewrite_pivot = "zh"
```

## Choosing the depth

Does a second or third machine-translation hop rewrite more? Does an LLM hop
change the picture? The bench in
[`benches/rewrite-depth/`](../../../benches/rewrite-depth/README.md) answers
with ten texts per configuration: five AI-flavoured English paragraphs, three
fact-heavy workplace texts (names, numbers, IDs, code) and two Chinese texts.

Method, per text and configuration:

- **Overlap**: content-word Jaccard with the source. Lower means more rewriting.
- **Tells left**: structural AI tells still present, out of the 10 texts: "not just/only X", "more than just", "it's worth noting", "in today's", "at its core", 不仅, 更是, 值得注意, 当今.
- **Facts missing**: names, numbers, IDs and code spans the fact guard reports as absent, summed over the corpus.
- **Latency**: mean wall clock for the whole chain.

```bash
python benches/rewrite-depth/measure.py --bin target/release/not-a-bot.exe --out out --configs light,standard-opencode
```

Results (run on 2026-11-11; 10 texts, one run per configuration):

| Configuration | Overlap | Tells left (of 10) | Facts missing | Latency |
| --- | --- | --- | --- | --- |
| light, 1 hop (`--depth 1`, earliest default) | 0.556 | 7 | 5 | 0.5 s |
| light, 2 hops (previous default) | 0.520 | 5 | 5 | 0.9 s |
| 3 machine-translation hops (`--depth 3`) | 0.480 | 6 | 8 | 1.1 s |
| standard via opencode-cli (DeepSeek) | 0.446 | 0 | 0 | 14 s |
| standard via claude-cli (Haiku) | 0.535 | 0 | 0 | 8 s |
| strong via claude-cli / opencode-cli | 0.369 / 0.396 | 0 / 1 | 1 / 1 | 24 / 28 s |

What the numbers say:

- Machine translation changes words and keeps sentence structure, so "not just X, it's Y" survives every depth.
- Even one machine-translation hop respells names (Aroha became Aloha, Tom became Tomi) and expands codes (NZD became "New Zealand dollars").
- A third machine-translation hop costs more facts (8 against 5) than it gains in rewriting (overlap 0.480 against 0.520). The default moved from one hop to two; three hops stay available as `--depth 3`.
- LLM hops remove the structural tells and kept every fact on this corpus.
- Machine-translation hops placed after an LLM hop can bring a foil back: Finnish "ei ainoastaan ... vaan myös" returned as "not only ... but also". That is why the same chain without a repair hop still left one tell in some runs.
- The repair hop cut the fact loss of that chain from 3 to 5 facts to 1 and kept most of its rewrite (overlap 0.369 and 0.396 against 0.292 and 0.307 without it). That chain with the repair hop is `strong`.

Caveats: ten texts and one run per configuration is a small sample; the tell
list and the fact guard are heuristics; overlap rewards change, not quality;
nothing here measures AI detectors, and no result should be read as
undetectable text.

## Navigation

- ← [Round trip overview](OVERVIEW.md)
- → [Chains, presets and LLM hops](02-chains-presets-and-llm.md)
- ↑ [Documentation index](../README.md)
