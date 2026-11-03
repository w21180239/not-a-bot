**English** · [简体中文](../../zh-cn/roundtrip/OVERVIEW.md)

# Round trip — Overview

`roundtrip` translates text out through a pivot language and back into the
source language:

```
source text ──▶ pivot language ──▶ source language
             hop 1              hop 2
```

The second hop is the point. A translation into a language with different
syntax cannot reuse the original phrasing, so translating back produces new
wording. That is how machine cadence gets removed without asking a model to
"rewrite this": the rewrite is a side effect of the round trip.

## When it helps

- Marketing and landing-page copy that reads like a template.
- Chat replies, emails and docs prose that sound generated.
- Any text the user calls "AI-sounding", "stiff" or "too polished".

It rewrites wording and keeps the sentences. An opener ("In today's fast-paced
landscape"), a hedge ("it's important to note") or a scaffold ("not just X but
Y") is content: it survives translation into every language, so a round trip
preserves it. Deleting those clauses is an editorial step; do it before the
translation, and say which sentences you removed.

## Ordering with translation

`roundtrip` returns text in the language it started from, so translate first when
the target differs:

```bash
translatex translate "<text>" -t zh --format json          # into Chinese
translatex roundtrip "<Chinese text>" --from zh --format json   # de-AI the Chinese
```

## When it hurts

The round trip paraphrases, so it can drift on anything precise:

| Content | Use round trip? |
| --- | --- |
| Marketing copy, announcements, blog prose | yes |
| Legal text, contracts, policy | no |
| API reference, config examples, error text | no |
| Numbers, dates, names, versions | no |
| Code, identifiers, paths | never — pass them through `preserve` |

If the caller needs both (prose with numbers), keep the numbers out of the text
or verify them afterwards. The response carries `pivotText` so the intermediate
can be audited.

## Interface

```bash
translatex roundtrip "In today's fast-paced landscape, …" --from en --format json
translatex roundtrip draft.md --pivot ja          # alias: translatex polish
```

```json
{
  "text": "…",
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

| Field | Meaning |
| --- | --- |
| `text` | Final result, back in the source language |
| `detectedSource` | Language the first hop detected |
| `pivotLang` | Bridge language actually used |
| `pivotText` | Intermediate translation, for auditing |
| `provider` | Provider that served the final hop |
| `providers` | One entry per hop, in order |

`translatex polish` is an alias for `roundtrip`.

## Pivot selection

`pivot: "auto"` (the default) uses Chinese for non-Chinese input and English for
Chinese input. [01-pivot-choice.md](01-pivot-choice.md) has the measurements
behind that choice and the cases where Japanese is better.

Override per request with `--pivot` / `"pivot"`, or for every request with
`roundtrip_pivot` in `config.toml`.

## Cost

One round trip is two translations, so it costs two provider calls (sometimes
three: see the auto-pivot retry in [01-pivot-choice.md](01-pivot-choice.md)).
Latency is roughly the sum of the two hops, not the maximum.

## Navigation

- ← [Documentation index](../README.md)
- → [Choosing the pivot](01-pivot-choice.md)
