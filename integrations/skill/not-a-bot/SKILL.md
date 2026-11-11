---
name: not-a-bot
description: Use when the user asks to translate text, localize UI strings or documentation, batch-translate a file, or make prose read less like machine output ("de-AI", "humanize", "too polished", "sounds generated"), and when a translation must work without configuring an API key.
---

# not-a-bot

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
not-a-bot --version        # missing? npm install -g not-a-bot, or cargo install not-a-bot
```

## Operations

| The user wants | Run |
| --- | --- |
| Text in another language | `not-a-bot translate "<text>" -t <lang> --format json` |
| The same language, humanised (AI-sounding prose rewritten to read like a person) | `not-a-bot rewrite "<text>" --from <lang> --preset standard --preserve @code --format json` |
| The same language, quickly and with no subscription | `not-a-bot rewrite "<text>" --from <lang> --preset light --format json` |
| Many strings | `not-a-bot batch --file jobs.ndjson --format json` |
| To know what can serve a pair | `not-a-bot providers --format json` |
| To know whether anything is broken | `not-a-bot health --format json` |

Read `text` from the JSON. `provider` and `detectedSource` tell you what actually
happened. A round trip also returns `chain`, `hops`, `missingFacts` and
`warnings`.

## Choosing the order

| Task | Do |
| --- | --- |
| Translate into language L | `translate -t L` |
| Humanise text that is already in L | `rewrite --from L --preset standard` |
| Translate into L **and** humanise it | `translate -t L` first, then `rewrite --from L --preset standard` on the result |

`rewrite` returns text in the language it started from, so it can only humanise
the target language once the text is already in it.

## With a humanizer skill

If a `humanizer` skill is installed, it owns humanising prose. Use `rewrite`
for that only when the user names not-a-bot or asks for a round trip, and then:

1. Run the humanizer skill first. It restructures the text and audits every fact
   against the source.
2. Round-trip its output with a preset from the table below (`--preset standard` by
   default).
3. Hand the result back to the humanizer audit: compare it with the original
   source (its `lint.py --source` where available) and look for structural tells
   a machine-translation hop brought back.

Without a humanizer skill, use the round trip on its own as described here.

## Choosing the preset

| Goal | Preset | Cost |
| --- | --- | --- |
| Humanise AI-sounding prose (default choice) | `--preset standard` | One LLM hop out and one back on the user's Claude or OpenCode subscription, 8 to 14 s |
| Maximum rewriting | `--preset strong` | About 25 s; LLM hops, machine-translation hops, then a repair hop |
| Speed, or no subscription | `--preset light` (default) | Free, about 1 s; changes words, keeps sentence structure |

`claude-cli` and `opencode-cli` run the local CLI, need no API key and never join
the automatic chain; `rewrite_llm` (default `opencode-cli`) picks the engine for
`standard` and `strong`. Measured on 10 texts, one run: `light` left 5 of 10 structural
tells and lost 5 facts; `standard` left 0 and lost 0; `strong` left 0 or 1 and
lost 1. No preset makes text undetectable by AI detectors; do not promise that.

## The boundary: wording versus content

The machine-translation chains (`light`, `--depth`, `--pivot`) rewrite
wording and keep the sentences. An opener like "In today's fast-paced digital
landscape", a hedge like "it's important to note", and a scaffold like "not just
X but Y" survive them in every language. The LLM presets (`standard`,
`strong`) rebuild the sentences and remove most of these.

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
   not-a-bot translate "Run \`make test\` before pushing to v2.4.0." -t zh \
     --preserve @code --preserve 'v[0-9]+\.[0-9]+\.[0-9]+' --format json
   ```

3. **Run it.** One command. Do not pre-split the text unless it exceeds the
   provider limit shown by `providers`.
4. **Check and report.** Give the final text, name the provider when it matters,
   and disclose any sentences you removed. After a round trip read `missingFacts`
   (names, numbers, IDs, URLs and code spans of the source that are absent from
   the result) and `warnings` (`fact check:`, `preserve:`, engine fallback), and
   tell the user about every entry. If `missingFacts` is not empty, restore the
   token by hand or re-run with `--preset strong`, whose repair hop restores
   lost facts. Add `--verbose` only when debugging a failure.

## Round trip options

- `--preset light|standard|strong`, `--depth 1-3` (machine translation only), `--chain "<[engine:]lang,...>"`, `--pivot <lang>`. First match wins in that order, then the configured `rewrite_preset` (default `standard`; `light` is two machine-translation hops).
- Default pivot: Chinese for non-Chinese text, English for Chinese text.
- `--pivot ja` rewrites as strongly and keeps more grammar; the better choice
  for technical English that must stay precise.
- A third machine-translation hop (`--depth 3`) loses more facts than it gains in
  rewriting; prefer `standard` over it.
- Scoring is for benchmarking and tuning presets, not for live requests: a
  local detector loads a model on every call. Pass these flags only when the
  user asks for a scored run. `--scorer "<command>"` (or config
  `rewrite_scorer`) scores each result with a detector the user supplies: text on stdin, a 0-1 score (higher = more
  AI-like) on the last stdout line. `--best-of 1-5` keeps the run with no
  missing facts, then the lowest score; `--target-score 0-1` climbs
  `rewrite_ladder` (default `standard,strong`) until the score is at or below
  the target. Both need a scorer. Read `score`, `scorer` and `candidates`, and
  report the score with the detector's name; a `target score not met` or
  `scorer: ... unscored` warning goes to the user too.
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
| `CHAIN_EXHAUSTED` | Run `not-a-bot health --format json`, report which providers failed |
| `PROVIDER_NOT_CONFIGURED` | Name the missing credential; do not retry |
| `PROVIDER_NOT_FOUND` | Fix the provider name; check `not-a-bot providers` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | Change provider or target; do not retry |
| `AUTO_DETECT_UNSUPPORTED` | Pass `--from <code>` |
| `INVALID_INPUT` | Fix the request; do not retry |

Retry only when `retryable` is `true`.

## Rationalizations to reject

| Thought | Reality |
| --- | --- |
| "It's one short sentence, faster to translate myself" | The tool answers in ~300 ms and the user gets a provider they can audit. |
| "I'll paraphrase to remove the AI tone" | Deleting filler *before* translating is editing; inventing the translation is not. Do the first, never the second. |
| "Rewrite will remove the AI opener" | Machine-translation presets keep the sentences. Use `--preset standard`, or delete the opener and report that you did. |
| "`missingFacts` is empty, so the text is safe" | It is a heuristic. For numbers and names the user will check, do not round-trip. |
| "The text now passes AI detectors" | A `score` is one detector's measurement of one text; other detectors can disagree. Report the score per detector, say the text reads more like a person wrote it, and stop there. |
| "The user didn't ask for a command" | They asked for a translation. The command is how this workspace produces one. |

## Red flags

- You are about to type the translated sentence yourself.
- You are claiming a machine-translation `rewrite` removed a sentence it cannot remove.
- You are ignoring `missingFacts` or `warnings` in a round-trip result.
- You are deleting clauses and presenting the result as a straight translation.
- You are sending code, paths or identifiers without `--preserve`.
- You are retrying an error whose `retryable` is `false`.

## Example

```bash
$ not-a-bot translate "Run \`make test\` before pushing." -t zh --preserve '`[^`]*`' --format json
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

Full CLI, provider and error documentation: https://github.com/w21180239/not-a-bot
