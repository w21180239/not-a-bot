**English** · [简体中文](../../zh-cn/cli/01-commands.md)

# CLI — Command reference

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## `translate`

```bash
not-a-bot translate "Hello world" -t zh
not-a-bot translate -t zh --json '{"text":"Hello","sourceLang":"en"}'
echo "Hello" | not-a-bot translate -t ja
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `TEXT` | stdin | Text to translate |
| `-t, --to` | required | Target language code |
| `-s, --from` | `auto` | Source language or `auto` |
| `-p, --provider` | `auto` | Pin one provider, or walk the chain |
| `--fallback` | empty | Comma-separated providers tried first; with `--provider`, tried after it |
| `--preserve` | empty | Regex for spans to keep verbatim; repeatable |
| `--timeout-ms` | config | Per-request budget in milliseconds |
| `--request-id` | generated | Correlation id, echoed and used for cooldowns |
| `--json` | — | Full request object; replaces the flags above |

Semantics worth knowing:

- `--provider deepl --fallback google,edge` tries DeepL, then Google, then Edge. A pinned provider that cannot serve the request (no auto-detection, unsupported pair) hands over to the fallback list instead of failing.
- `--fallback google,edge` without `--provider` puts those two at the front of the chain, in the order written, before the configured chain.
- `--preserve` runs before the chain. A restore failure counts as a failed attempt and moves to the next provider.
- `--from auto` requires the provider to support detection. Providers with `autoDetect: false` are skipped; pinning one returns `AUTO_DETECT_UNSUPPORTED`.

## `rewrite`

```bash
not-a-bot rewrite "In today's fast-paced landscape, …" --from en --format json
not-a-bot rewrite --from en --pivot ja < draft.md
not-a-bot rewrite "…" --from en --preset standard --preserve @code
not-a-bot rewrite "…" --from en --chain "opencode-cli:zh,edge:en,opencode-cli:fix"
```

| Flag | Default | Meaning |
| --- | --- | --- |
| `TEXT` | stdin | Text to round-trip |
| `-s, --from` | `auto` | Source language |
| `--pivot` | `auto` | Bridge language |
| `--chain` | — | Hops `[engine:]lang`, comma-separated; a final hop back to the source is appended when missing; `<llm>:fix` repairs |
| `--preset` | config `rewrite_preset` (`standard`) | `light`, `standard`, `strong` |
| `--depth` | — | 1-3 machine-translation hops through zh, ja, fi (Chinese sources: en, ja, fi) |
| `-p, --provider` | `auto` | Pin one provider for both hops |
| `--fallback` | empty | Providers tried after the pinned one |
| `--preserve` | empty | Regex spans kept verbatim, or the named patterns `@code` and `@url`; repeatable |
| `--timeout-ms` | config | Per-hop budget |
| `--request-id` | generated | Correlation id |
| `--json` | — | Full `RewriteRequest`; replaces the other flags |

Hop selection, first match wins: `--chain`, `--preset`, `--depth`, `--pivot`,
then `rewrite_preset` from the config (default `standard`; `light` is two machine-translation
hops). `claude-cli` and `opencode-cli` run only when a hop, a preset or
`--provider` names them. The response carries `chain`, `hops`, `missingFacts`
and `warnings`; read `missingFacts` after every run. Chains, presets and the
repair hop: [rewrite/02-chains-presets-and-llm.md](../rewrite/02-chains-presets-and-llm.md).

The result is always in the source language. `pivot: auto` uses Chinese for
non-Chinese input and English for Chinese input; `--pivot ja` is the documented
alternative for technical English. See
[rewrite/01-pivot-choice.md](../rewrite/01-pivot-choice.md).

A pivot equal to the source language is replaced with the other half of the
pair, so `--from en --pivot en` becomes `en -> zh -> en` instead of a no-op.

## `lint`

```bash
not-a-bot lint draft.md
not-a-bot lint draft.md --source original.md --format json
cat draft.md | not-a-bot lint - --source original.md --strict
```

Offline check for AI tells and dropped facts: no network, no model, no config or credentials. Hits are prompts to look, so the exit code is `0` unless `--strict` is set.

| Flag | Default | Meaning |
| --- | --- | --- |
| `DRAFT` | required | Draft file, or `-` for stdin |
| `--source FILE` | — | Original text; adds the fact diff |
| `--strict` | off | Exit `1` when there is any tell or any `missingFromDraft` entry (CI, pre-commit) |

Tells carry a `kind`: `vocab` (word with its plain replacement), a phrase family (`deep saying`, `run-up`, `arguing with no one`, `dramatic closer`, `about the document`, `chatbot residue`, `meta wrapper`, `knowledge-limit`), `staged-contrast`, `us-spelling`, `bold-label-bullet`, `horizontal-rule`, `repeated-openings` (line 0), `punctuation` (em dash, spaced en dash, spaced `--`, curly double quotes) and, with `--source`, `introduced-script`. The fact diff compares code spans, URLs, paths, ticket IDs, `@mentions`, numbers and names: `missingFromDraft` holds `kind:value` tokens in the source but not the draft, `newInDraft` the reverse (names are not reported as new). A number written as a word (`3` to `three`) counts as missing.

`--format json` (the global default) prints `LintResponse`:

```json
{"tells":[{"kind":"vocab","line":1,"match":"delve","hint":"look at / dig into"}],"missingFromDraft":["number:5"],"newInDraft":[]}
```

`--format text` prints the lines of the humanizer `lint.py`: `N: kind: detail` per tell, then `missing-from-draft:` and `new-in-draft:` lines, a `-- source ...` summary (word recall, length ratio) and a `-- N words, ... N hits` footer. A missing draft or source file returns `INVALID_INPUT` (exit `1`).

## `batch`

```bash
not-a-bot batch --file jobs.ndjson --parallel 4
cat jobs.ndjson | not-a-bot batch --parallel 8
```

Input is one `TranslateRequest` per line. Output is one object per line, sorted back into input order even though execution is concurrent:

```json
{"index":0,"ok":true,"result":{…}}
{"index":1,"ok":false,"error":{"code":"RATE_LIMITED",…}}
```

Malformed lines are skipped with a stderr warning; the batch continues.

## `providers`

```bash
not-a-bot providers --format json
not-a-bot providers --format text
```

JSON form is `{providers: ProviderInfo[]}`. Text form is `name<TAB>status`, useful in a shell.

## `languages`

```bash
not-a-bot languages                       # every canonical code
not-a-bot languages --provider google     # that provider's advertised set
```

`*` in a provider's advertised set means it accepts any canonical code. An unknown name returns `PROVIDER_NOT_FOUND`; an implemented but unconfigured one returns `PROVIDER_NOT_CONFIGURED`.

## `keys`

```bash
not-a-bot keys                    # cooldown and failure state for configured providers
not-a-bot keys deepl              # one provider
not-a-bot keys --reset            # clear cooldown state for configured providers
```

## `health`

```bash
not-a-bot health --format json
```

Probes each registered provider with its real endpoint. Returns `{healthy, providers: HealthRecord[]}` and exits non-zero when a configured provider is unhealthy.

## `schema`, `manifest`, `init`, `completions`

```bash
not-a-bot schema      > schema.json
not-a-bot manifest    > manifest.json
not-a-bot init --path ~/.config/not-a-bot/config.toml
not-a-bot completions bash
```

`schema` emits draft 2020-12 definitions for `TranslateRequest`, `TranslateResponse`, `ErrorResponse`, `ProviderInfo`, `HealthRecord`, `AttemptRecord`, `RewriteRequest`, `RewriteResponse` and `LintResponse`.

`manifest` emits `{interface: "cli", commands: [{name, command, description, inputSchema?}], skill, install}`, so an agent can discover the operations without parsing `--help`.

## Navigation

- ← [CLI overview](OVERVIEW.md)
- → [Config files](02-config-files.md)
