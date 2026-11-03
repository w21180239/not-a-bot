---
name: translatex
description: Use when the user asks to translate text, localize UI strings or documentation, batch-translate a file, or make prose read less like machine output ("de-AI", "humanize", "too polished", "sounds generated"), and when a translation must work without configuring an API key.
---

# translatex

## Overview

One binary, no configuration needed for the free web endpoints. It rotates keys,
races providers and falls back on its own, and every answer is JSON with a
`text` field and a typed error.

**Core principle:** the tool produces the translation; you decide what to
delete. Typing the translation yourself loses provider fallback and the audit
trail. Editing out filler the user asked you to remove is a different act, and
it is allowed.

## Confirm the binary once

```bash
translatex --version        # missing? npm install -g translatex, or cargo install translatex
```

## Operations

| The user wants | Run |
| --- | --- |
| Text in another language | `translatex translate "<text>" -t <lang> --format json` |
| The same language, less machine cadence | `translatex roundtrip "<text>" --from <lang> --format json` |
| Many strings | `translatex batch --file jobs.ndjson --format json` |
| To know what can serve a pair | `translatex providers --format json` |
| To know whether anything is broken | `translatex health --format json` |

Read `text` from the JSON. `provider` and `detectedSource` tell you what actually
happened.

## Choosing the order

| Task | Do |
| --- | --- |
| Translate into language L | `translate -t L` |
| De-AI text that is already in L | `roundtrip --from L` |
| Translate into L **and** de-AI it | `translate -t L` first, then `roundtrip --from L` on the result |

`roundtrip` returns text in the language it started from, so it can only de-AI
the target language once the text is already in it.

## The boundary: wording versus content

`roundtrip` rewrites wording. It keeps the sentences. An opener like "In
today's fast-paced digital landscape", a hedge like "it's important to note",
and a scaffold like "not just X but Y" are content: they survive translation
into every language, so a round trip preserves them.

When the user asks for that filler to be gone:

1. Delete those clauses yourself.
2. Run the tool on what remains.
3. Say which sentences you removed.

Presenting an edited text as a straight translation hides a decision the user
did not see.

## Workflow

1. **Identify the operation and the order** from the table above.
2. **Protect what must not change.** Code, identifiers, paths, URLs and version
   strings go through `--preserve`:

   ```bash
   translatex translate "Run \`make test\` before pushing to v2.4.0." -t zh \
     --preserve '`[^`]*`' --preserve 'v[0-9]+\.[0-9]+\.[0-9]+' --format json
   ```

3. **Run it.** One command. Do not pre-split the text unless it exceeds the
   provider limit shown by `providers`.
4. **Report.** Give the final text, name the provider when it matters, and
   disclose any sentences you removed. Add `--verbose` only when debugging a
   failure.

## Round trip options

- Default pivot: Chinese for non-Chinese text, English for Chinese text.
- `--pivot ja` rewrites as strongly and keeps more grammar; the better choice
  for technical English that must stay precise.
- **Never round-trip** legal text, contracts, API reference, configuration,
  error messages, or numbers and names the user will check: the second hop
  paraphrases by design.

## Errors

Every failure carries `code`, `retryable`, `retryAfterSecs`, `suggestedAction`.

| Code | Do |
| --- | --- |
| `RATE_LIMITED` | Wait `retryAfterSecs`, retry once |
| `QUOTA_EXHAUSTED` | Switch key or wait; tell the user |
| `NETWORK_ERROR`, `TIMEOUT` | Retry with backoff, or raise `--timeout-ms` |
| `CHAIN_EXHAUSTED` | Run `translatex health --format json`, report which providers failed |
| `PROVIDER_NOT_CONFIGURED` | Name the missing credential; do not retry |
| `PROVIDER_NOT_FOUND` | Fix the provider name; check `translatex providers` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | Change provider or target; do not retry |
| `AUTO_DETECT_UNSUPPORTED` | Pass `--from <code>` |
| `INVALID_INPUT` | Fix the request; do not retry |

Retry only when `retryable` is `true`.

## Rationalizations to reject

| Thought | Reality |
| --- | --- |
| "It's one short sentence, faster to translate myself" | The tool answers in ~300 ms and the user gets a provider they can audit. |
| "I'll paraphrase to remove the AI tone" | Deleting filler *before* translating is editing; inventing the translation is not. Do the first, never the second. |
| "Roundtrip will remove the AI opener" | It rewrites wording and keeps sentences. Delete the opener or report that it is still there. |
| "The user didn't ask for a command" | They asked for a translation. The command is how this workspace produces one. |

## Red flags

- You are about to type the translated sentence yourself.
- You are claiming `roundtrip` removed a sentence it cannot remove.
- You are deleting clauses and presenting the result as a straight translation.
- You are sending code, paths or identifiers without `--preserve`.
- You are retrying an error whose `retryable` is `false`.

## Example

```bash
$ translatex translate "Run \`make test\` before pushing." -t zh --preserve '`[^`]*`' --format json
{
  "text": "推送前先跑 `make test`。",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 312,
  "requestId": "…",
  "chars": 34
}
```

The backtick span survived verbatim; provider and latency came from the tool.

## Reference

Full CLI, provider and error documentation: https://github.com/w21180239/translatex
