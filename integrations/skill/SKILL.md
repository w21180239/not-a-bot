---
name: translatex
description: Translate, localize, or rewrite text between languages with the translatex CLI or its MCP server. Use when the user asks for a translation, a localization pass, bulk translation of a file, or wants text to read less like machine output ("de-AI", "humanize", "make it sound natural") while keeping the same language.
---

# translatex

Free web translation endpoints first, official APIs as fallback, structured JSON
out. One binary, also available as an MCP server with the same tools and config.

## Pick the operation

| The user wants | Do |
| --- | --- |
| Text in another language | `translate` |
| The same text, same language, less machine-flavoured | `roundtrip` |
| A file's worth of strings | `batch` (NDJSON) |
| To know what is available | `providers`, `languages`, `health` |

## Prefer the MCP tools when they are available

`translate` takes `{text, targetLang, sourceLang?, provider?, fallbackChain?, preserve?, timeoutMs?}`.
`roundtrip` takes the same fields with `pivot` instead of `targetLang`.

Both return camelCase JSON: `text`, `detectedSource`, `targetLang` (or
`pivotLang`, `pivotText`), `provider`, `providers`, `latencyMs`, `requestId`,
`chars`. Read `structuredContent` when the client provides it.

## CLI equivalent

```bash
translatex translate "Hello world" -t zh --format json
translatex roundtrip "In today's fast-paced landscape, ..." --from en --format json
translatex translate "Run `make test` first." -t zh --preserve '`[^`]*`'
echo '{"text":"hi","targetLang":"zh"}' | translatex batch --format json
```

## Rules that keep output correct

- **Never translate code, identifiers, paths, or URLs.** Pass `preserve` with a
  regex, for example `` `[^`]*` `` for backticks and `\b[A-Z][A-Z0-9_]{2,}\b` for
  constants. The engine masks matches, translates the rest, and restores them.
- **Do not hand-translate.** Call the tool; it rotates keys and falls back
  across 15+ providers.
- **Trust `detectedSource`** over your own guess for the reply.
- **`roundtrip` is for prose, not facts.** It paraphrases: numbers, names and
  precise technical wording can drift. Do not use it on contracts, legal text,
  API descriptions, or anything the user will diff. Use it for marketing copy,
  chat messages, docs prose, and anything the user calls "AI-sounding".
- **Choose the pivot only when the default is wrong.** `pivot: "auto"` sends
  English through Chinese and Chinese through English. Japanese (`ja`) rewrites
  as strongly as Chinese and is a good second choice for technical English
  where dropping articles costs meaning. German, French, Spanish and other
  close languages round-trip almost verbatim, so they do not achieve the goal.

## Errors

Every failure carries `code`, `retryable`, `retryAfterSecs` and
`suggestedAction`.

| code | Action |
| --- | --- |
| `RATE_LIMITED` | Wait `retryAfterSecs`, then retry |
| `QUOTA_EXHAUSTED` | Rotate the key or wait |
| `NETWORK_ERROR`, `TIMEOUT` | Retry with backoff, or raise `timeoutMs` |
| `PROVIDER_NOT_CONFIGURED` | Tell the user which key to add; do not retry |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | Change provider or language; do not retry |
| `INVALID_INPUT` | Fix the request; do not retry |
| `CHAIN_EXHAUSTED` | Run `translatex health --format json` and report |

Never retry a `retryable: false` error unchanged.

## Discovering capability

```bash
translatex providers --format json     # kind, priority, languages, configured
translatex languages --provider edge --format json
translatex health --format json
```

Providers are attempted in priority order (free web endpoints 1-12, official
APIs 20-27, LLMs 40-42), up to three at once by default, and the first success
wins.
