**English** · [简体中文](../../zh-cn/rewrite/OVERVIEW.md)

# Round trip — Overview

[← Documentation index](../README.md)

`rewrite` translates text out through one or more hops and back into the
source language:

```
source text ──▶ hop 1 ──▶ hop 2 ──▶ … ──▶ source language
```

The way back is the point. A translation into a language with different syntax
cannot reuse the original phrasing, so translating back produces new wording.
With machine-translation hops that rewrite is a side effect. With LLM hops it is
the instruction: each hop translates and rewrites the way a person would. The
result reads less like generated text, with names, numbers and code checked
against the source. The main use is English copy: AI-sounding English goes out
through Chinese and back, and the `light` and `standard` presets are tuned and
measured on it. Chinese text works with the same presets.

## What each kind of hop does

| Hop | Changes | Keeps | Cost |
| --- | --- | --- | --- |
| Machine translation (first hop of `light`) | Words | Sentence structure, so "not just X, it's Y" survives | Free |
| LLM (`light` return hop, `standard`) | Words and sentence structure; removes the structural tells | Facts, checked by the fact guard | Your Claude or OpenCode subscription, several seconds per hop |
| Repair (`<llm>:fix`, last hop of every preset) | A few exact edits that restore lost meaning (names, numbers, dates, IDs, code, negations, certainty, actor, cause, dropped or invented claims); never cut feeling or pledge sentences | The rewrite | One more LLM call |

Measured on 10 texts, one run: a two-hop machine-translation chain left 5 of 10 tells and lost 5
facts; `standard` left 0 tells and lost 0 facts. Details and the full table:
[01-pivot-choice.md](01-pivot-choice.md#choosing-the-depth).

The goal is prose that reads like a person wrote it, with the facts intact.
not-a-bot makes no claim about AI detectors and does not promise undetectable
text.

## Ordering with translation

`rewrite` returns text in the language it started from, so translate first when
the target differs:

```bash
not-a-bot translate "<text>" -t zh --format json          # into Chinese
not-a-bot rewrite "<Chinese text>" --from zh --format json   # humanise the Chinese
```

## When it hurts

The round trip paraphrases, so it can drift on anything precise:

| Content | Use round trip? |
| --- | --- |
| Marketing copy, announcements, blog prose | yes |
| Prose with a few names and numbers | yes, and read `missingFacts` |
| Legal text, contracts, policy | no |
| API reference, config examples, error text | no |
| Numbers, dates, names, versions the reader will check | no |
| Code, identifiers, paths | never; pass them through `--preserve @code` |

The fact guard lists what the result lost; it does not make a round trip safe for
text the reader will verify. The response carries every hop's output in `hops`,
so the intermediates can be audited.

## Interface

```bash
not-a-bot rewrite "In today's fast-paced landscape, …" --from en --format json
not-a-bot rewrite "…" --from en --preset standard              # LLM out, free LLM back, then a meaning repair (needs claude-cli or opencode-cli)
not-a-bot rewrite "…" --from en --preset light         # Google out, repair, LLM back, repair: lightest rewrite, highest fidelity
not-a-bot rewrite "…" --from en --depth 3                 # zh -> ja -> fi -> en, machine translation only
not-a-bot rewrite "…" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
not-a-bot rewrite --from en --pivot ja < draft.md         # alias: not-a-bot polish
not-a-bot rewrite "Run \`make test\`" --from en --preset standard --preserve @code
```

Hop selection, first match wins: `--chain`, `--preset`, `--depth`, `--pivot`, then
the configured `rewrite_preset` (default `standard`).

| Flag | Meaning |
| --- | --- |
| `--chain "<[engine:]lang,...>"` | Explicit hops; a final hop back to the source is appended when missing |
| `--preset <name>` | `light`, `standard` |
| `--depth 1-3` | Machine translation only, through zh, ja, fi (Chinese sources: en, ja, fi) |
| `--pivot <lang>` | One bridge language, or `auto` |
| `--preserve <pattern>` | Regex, or the named patterns `@code` and `@url` |

Chains, presets, the LLM providers, the repair hop and the fact guard are
described in [02-chains-presets-and-llm.md](02-chains-presets-and-llm.md).

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

| Field | Meaning |
| --- | --- |
| `text` | Final result, back in the source language |
| `detectedSource` | Language the first hop detected |
| `chain` | Languages visited after the source; a repair hop shows as `fix` |
| `hops` | Per hop: `from`, `to`, `provider`, output `text`, `latencyMs`; a repair hop adds `edits` |
| `hops[].edits` | Per edit: `from`, `to`, `why`, `kind` (`fact` or `meaning`), `applied`, and `reason` when skipped |
| `provider` | Provider that served the final hop |
| `missingFacts` | Input tokens (names, numbers, IDs, URLs, code) absent from the output |
| `warnings` | Present when non-empty: `fact check:`, `preserve:` and engine fallback notes |
| `latencyMs`, `requestId`, `chars` | Total wall clock, correlation id, input length |

`not-a-bot polish` is an alias for `rewrite`.

## Pivot selection

`pivot: "auto"` uses Chinese for non-Chinese input and English for Chinese
input, and is the first hop of the `light` preset.
[01-pivot-choice.md](01-pivot-choice.md) has the measurements behind that
choice and the cases where Japanese is better.

Override per request with `--pivot` / `"pivot"`, or for every request with
`rewrite_pivot` in `config.toml`.

## Cost

One round trip costs one provider call per hop: three for the `light` preset
(more for Chinese input detected under `auto`, see
[01-pivot-choice.md](01-pivot-choice.md#the-auto-pivot-retry)). Latency is
roughly the sum of the hops, not the maximum. LLM hops take seconds each and
spend subscription quota.

## Navigation

- ← [Documentation index](../README.md)
- → [Choosing the pivot and the depth](01-pivot-choice.md)
- → [Chains, presets and LLM hops](02-chains-presets-and-llm.md)
