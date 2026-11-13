# CLI reference for the not-a-bot skill

`nab` is the short name of `not-a-bot`; both behave the same. Every answer is JSON with a `text` field and a typed error unless `--format text` is given. Confirm the binary once with `nab --version` (missing: `npm install -g not-a-bot-cli`, or `cargo install --git https://github.com/w21180239/not-a-bot`).

## Operations

| The user wants | Run |
| --- | --- |
| A draft audited against its source, offline | `nab lint draft.txt --source source.txt --format text` |
| Text in another language | `nab translate "<text>" -t <lang> --preserve @code --format json` |
| The same language, rewritten by a round trip | `nab rewrite "<text>" --from <lang> --preset standard --preserve @code --format json` |
| The same, quickly and with no subscription | `nab rewrite "<text>" --from <lang> --preset light --format json` |
| Many strings | `nab batch --file jobs.ndjson --format json` (`--kind rewrite` or `--kind lint` for many rewrites or lints, `--parallel N`, optional `id` per line) |
| What can serve a language pair | `nab providers --format json` |
| Whether anything is broken | `nab health --format json` |

Translate into language L: `translate -t L`. Translate and humanise: `translate -t L` first, then humanise the result by the method in `SKILL.md` or with `rewrite --from L`. `rewrite` returns text in the language it started from.

## `nab lint`

`nab lint <DRAFT|-> [--source FILE] [--format text|json] [--strict]`. `-` reads the draft from stdin. Output is JSON (`tells`, `missingFromDraft`, `newInDraft`) by default; `--format text` prints one hit per line. No network and no model.

- Without `--source`: lists word, phrase, structure, spelling and punctuation tells with their line numbers.
- With `--source`: also lists fact tokens (code spans, URLs, paths, ticket IDs, mentions, versions, numbers, names). `missing-from-draft` are tokens the draft lost; `new-in-draft` are tokens it gained. Confirm each by meaning: a number written as a word shows as missing.
- `--strict` exits non-zero when there is any tell or any dropped fact, for scripts and CI.
- Hits are prompts to look, not errors. Spelling hits matter only when the register is `uk` or `nz` (US spellings) or `us`.

## `nab rewrite`

A round trip through another language that rewrites the sentences on the way. The result also returns `chain`, `hops`, `missingFacts` and `warnings`.

| Goal | Preset | Cost |
| --- | --- | --- |
| Humanise AI-sounding prose (default) | `--preset standard` | LLM out, free LLM back, then a meaning-repair hop, on the user's Claude or OpenCode subscription; several LLM calls, tens of seconds; lowest detector score |
| Lightest rewrite, highest fidelity | `--preset light` | Google out, repair, one LLM rewrite back, repair; several LLM calls, tens of seconds |

Both presets need an LLM `rewrite_llm` (default `opencode-cli`, alternative `claude-cli`), else `INVALID_INPUT`; `claude-cli` and `opencode-cli` run the local CLI, need no API key and never join the automatic chain. Every preset ends with a fix hop: an LLM that sees the original and returns small span edits restoring lost or changed meaning (names, numbers, dates, amounts, IDs, code, negations, qualifiers, actor, cause, dropped or invented claims) and never restores deliberately cut feeling, pledge or pleasantry sentences. Measured on 30 texts x 2 samples (desklib, source 0.999): `standard` mean P(AI) 0.364, `light` 0.489; both lose almost no fact tokens (0.5 and 0 texts of 30) and 98% and 97% of texts have no major meaning error. Machine translation keeps sentence structure, so machine-only chains stay near the source score. `strong` is removed and fails with `INVALID_INPUT`. No preset makes text undetectable by AI detectors; do not promise that.

Other options: `--depth 1-3` (machine translation only, no repair hop; a third hop loses more facts than it gains), `--chain "<[engine:]lang,...>"`, `--pivot <lang>` (default: Chinese for non-Chinese text, English for Chinese text; `--pivot ja` keeps more grammar, better for precise technical English). First match wins in the order preset, depth, chain, pivot.

Scoring is for tuning presets, not live requests, because a local detector loads a model on every call. Pass `--scorer "<command>"` (text on stdin, a 0-1 score on the last stdout line, higher = more AI-like), `--best-of 1-5` or `--target-score 0-1` only when the user asks for a scored run, and report the score with the detector's name. `benches/detector/` has one.

### After a round trip

1. Report every `missingFacts` and `warnings` entry (`fact check:`, `preserve:`, engine fallback) to the user. If `missingFacts` is not empty, restore the token by hand (a fact the fix hop could not restore is reported, never silently fixed).
2. Re-run `nab lint --source` against the original source and check the structural tells in `patterns.md` again; machine-translation hops can bring staged contrasts back.
3. Never round-trip legal text, contracts, API reference, configuration, error messages, or numbers and names the reader will check line by line. The fact guard lists what went missing; it does not make such text safe, and it checks names only in Latin-script text.

### Wording versus content

The machine-translation chains (`light`, `--depth`, `--pivot`) rewrite wording and keep the sentences, so a stock opener, a hedge or a "not just X but Y" scaffold survives them in every language. The LLM presets rebuild the sentences. When the user wants that filler gone, prefer an LLM preset; with a machine-translation preset, delete those clauses yourself first, run the tool on the rest, and say which sentences you removed. Presenting an edited text as a straight translation hides a decision the user did not see.

## Protecting what must not change

Code, identifiers, paths, URLs and version strings go through `--preserve`. Use the named patterns `@code` (inline and fenced code) and `@url`; on Windows a regex such as `` `[^`]*` `` loses its `^` when a program spawns the npm `.cmd` shim without a shell. Add a regex for anything else:

```bash
nab translate "Run \`make test\` before pushing to v2.4.0." -t zh \
  --preserve @code --preserve 'v[0-9]+\.[0-9]+\.[0-9]+' --format json
```

Do not pre-split text unless it exceeds the provider limit shown by `nab providers`. Add `--verbose` only when debugging a failure. Typing a translation yourself loses provider fallback and the audit trail; deleting filler the user asked you to remove is editing, and allowed.

## Errors

Every failure carries `code`, `retryable`, `retryAfterSecs`, `suggestedAction`. Retry only when `retryable` is `true`.

| Code | Do |
| --- | --- |
| `RATE_LIMITED` | Wait `retryAfterSecs`, retry once |
| `QUOTA_EXHAUSTED` | Switch key or wait; tell the user |
| `NETWORK_ERROR`, `TIMEOUT` | Retry with backoff, or raise `--timeout-ms` |
| `CHAIN_EXHAUSTED` | Run `nab health --format json`, report which providers failed |
| `PROVIDER_NOT_CONFIGURED` | Name the missing credential; do not retry |
| `PROVIDER_NOT_FOUND` | Fix the provider name; check `nab providers` |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | Change provider or target; do not retry |
| `AUTO_DETECT_UNSUPPORTED` | Pass `--from <code>` |
| `INVALID_INPUT` | Fix the request; do not retry |

## Rationalizations to reject

| Thought | Reality |
| --- | --- |
| "I'll paraphrase to remove the AI tone" | Deleting filler before translating is editing; inventing the translation is not. |
| "Rewrite will remove the AI opener" | Machine-translation presets keep the sentences. Use `--preset standard`, or delete the opener and say so. |
| "`missingFacts` is empty, so the text is safe" | It is a heuristic. For numbers and names the reader will check, do not round-trip. |
| "The text now passes AI detectors" | A score is one detector's measurement of one text. Report it per detector, say the text reads more like a person wrote it, and stop there. |
| "Lint is clean, so the facts are fine" | Lint sees tokens, not meaning. Audit each fact by hand first. |

Full CLI, provider and error documentation: https://github.com/w21180239/not-a-bot
