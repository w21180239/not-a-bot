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
| The same language, humanised (AI-sounding prose rewritten to read like a person) | `translatex roundtrip "<text>" --from <lang> --preset llm --preserve @code --format json` |
| The same language, quickly and with no subscription | `translatex roundtrip "<text>" --from <lang> --format json` (default `mt`) |
| Many strings | `translatex batch --file jobs.ndjson --format json` |
| To know what can serve a pair | `translatex providers --format json` |
| To know whether anything is broken | `translatex health --format json` |

Read `text` from the JSON. `provider` and `detectedSource` tell you what actually
happened. A round trip also returns `chain`, `hops`, `missingFacts` and
`warnings`.

## Choosing the order

| Task | Do |
| --- | --- |
| Translate into language L | `translate -t L` |
| Humanise text that is already in L | `roundtrip --from L --preset llm` |
| Translate into L **and** humanise it | `translate -t L` first, then `roundtrip --from L --preset llm` on the result |

`roundtrip` returns text in the language it started from, so it can only humanise
the target language once the text is already in it.

## Choosing the preset

| Goal | Preset | Cost |
| --- | --- | --- |
| Humanise AI-sounding prose (default choice) | `--preset llm` | One LLM hop out and one back on the user's Claude or OpenCode subscription, 8 to 14 s |
| Maximum rewriting | `--preset lynote-fix` | About 25 s; LLM hops, machine-translation hops, then a repair hop |
| Speed, or no subscription | `mt` (default), `mt-deep` | Free, about 1 s; changes words, keeps sentence structure |

`claude-cli` and `opencode-cli` run the local CLI, need no API key and never join
the automatic chain; `roundtrip_llm` (default `opencode-cli`) picks the engine for
`llm` and `lynote`. Measured on 10 texts, one run: `mt` left 5 of 10 structural
tells and lost 5 facts; `llm` left 0 and lost 0; `lynote-fix` left 0 or 1 and
lost 1. No preset makes text undetectable by AI detectors; do not promise that.

## The boundary: wording versus content

The machine-translation presets (`mt`, `mt-deep`, `--depth`, `--pivot`) rewrite
wording and keep the sentences. An opener like "In today's fast-paced digital
landscape", a hedge like "it's important to note", and a scaffold like "not just
X but Y" survive them in every language. The LLM presets (`llm`, `lynote`,
`lynote-fix`) rebuild the sentences and remove most of these.

When the user asks for that filler to be gone:

1. Prefer an LLM preset.
2. With a machine-translation preset, delete those clauses yourself first, run the
   tool on what remains, and say which sentences you removed.
3. Check the result for tells that are still there and report them.

Presenting an edited text as a straight translation hides a decision the user
did not see.

## Workflow

1. **Identify the operation and the order** from the table above.
2. **Protect what must not change.** Code, identifiers, paths, URLs and version
   strings go through `--preserve`. Use the named patterns `@code` (inline and
   fenced code) and `@url`; on Windows a regex such as `` `[^`]*` `` loses its `^`
   when a program spawns the npm `.cmd` shim without a shell. Add a regex for
   anything else:

   ```bash
   translatex translate "Run \`make test\` before pushing to v2.4.0." -t zh \
     --preserve @code --preserve 'v[0-9]+\.[0-9]+\.[0-9]+' --format json
   ```

3. **Run it.** One command. Do not pre-split the text unless it exceeds the
   provider limit shown by `providers`.
4. **Check and report.** Give the final text, name the provider when it matters,
   and disclose any sentences you removed. After a round trip read `missingFacts`
   (names, numbers, IDs, URLs and code spans of the source that are absent from
   the result) and `warnings` (`fact check:`, `preserve:`, engine fallback), and
   tell the user about every entry. If `missingFacts` is not empty, restore the
   token by hand or re-run with `--preset lynote-fix`, whose repair hop restores
   lost facts. Add `--verbose` only when debugging a failure.

## Round trip options

- `--preset mt|mt-deep|llm|lynote|lynote-fix`, `--depth 1-3` (machine translation only), `--chain "<[engine:]lang,...>"`, `--pivot <lang>`. First match wins in that order, then the configured `roundtrip_preset` (default `mt`, two machine-translation hops).
- Default pivot: Chinese for non-Chinese text, English for Chinese text.
- `--pivot ja` rewrites as strongly and keeps more grammar; the better choice
  for technical English that must stay precise.
- A third machine-translation hop (`mt-deep`) loses more facts than it gains in
  rewriting; prefer `llm` over it.
- **Never round-trip** legal text, contracts, API reference, configuration,
  error messages, or numbers and names the user will check: every hop
  paraphrases by design. The fact guard lists what went missing; it does not
  make such text safe, and it checks names only in Latin-script text and can
  miss a name that doubles as a common sentence-starting word.

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
| "Roundtrip will remove the AI opener" | Machine-translation presets keep the sentences. Use `--preset llm`, or delete the opener and report that you did. |
| "`missingFacts` is empty, so the text is safe" | It is a heuristic. For numbers and names the user will check, do not round-trip. |
| "The text now passes AI detectors" | Nothing here measures or promises that. Say the text reads more like a person wrote it, and stop there. |
| "The user didn't ask for a command" | They asked for a translation. The command is how this workspace produces one. |

## Red flags

- You are about to type the translated sentence yourself.
- You are claiming a machine-translation `roundtrip` removed a sentence it cannot remove.
- You are ignoring `missingFacts` or `warnings` in a round-trip result.
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
