**English** · [简体中文](../../zh-cn/roundtrip/02-chains-presets-and-llm.md)

# Round trip — Chains, presets and LLM hops

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

A round trip is a list of hops. Machine translation changes the words and keeps
the sentence structure; an LLM hop can rebuild the sentences. A chain mixes the
two, and presets are chains that were measured and named.

## How a chain works

A hop is `[engine:]lang`. A chain is a comma-separated list of hops, in any
order of engines and languages:

```bash
translatex roundtrip "<text>" --from en --chain "opencode-cli:zh,google:ja,edge:en"
```

- No engine in a hop: the automatic provider chain serves it (up to `race_width` providers race).
- A named engine: that engine is preferred for the hop (see [preferred engines](#preferred-engines-and-fallback)).
- The result must end in the source language. When the last hop is not the source, a final hop back to it is appended.
- `<llm>:fix` is a repair hop. It stays in the current language (see [the repair hop](#the-repair-hop)).

Shortcuts that expand to a chain:

| Flag | Expands to |
| --- | --- |
| `--depth 1` | one machine-translation hop out through zh, then back |
| `--depth 2` | zh, ja, back |
| `--depth 3` | zh, ja, fi, back |
| `--pivot ja` | one hop out through ja, then back |
| `--preset <name>` | a named chain from the table below |

Chinese sources use en instead of zh: depth 3 runs en, ja, fi, then back to Chinese.

Precedence, first match wins: `--chain`, `--preset`, `--depth`, `--pivot`, then
`roundtrip_preset` from the config (default `light`).

## Presets

| Preset | Hops (English source) | Needs a subscription | Typical latency |
| --- | --- | --- | --- |
| `light` (default) | zh, ja, en: two machine-translation hops out | no | about 1 s |
| `standard` | LLM to zh, LLM back to en; both hops "translate and rewrite like a person" | yes | 8 to 14 s |
| `strong` | LLM to zh, LLM to ja (previous step as history), google to fi, edge back to en, then a repair hop | yes | about 25 s |

`strong` pushes the text through distant languages with LLM hops, adds
machine-translation hops, and ends with the repair hop that restores facts the
rewrite dropped. No method makes text reliably undetectable, and translatex does
not promise it. The aim is prose that reads like a person wrote it, with the
facts intact. `--depth 1-3` and `--chain` build other chains by hand. Any other
preset name fails with `INVALID_INPUT`, listing `light, standard, strong`.

Set the default in `config.toml`:

```toml
roundtrip_preset = "standard"  # default: light
roundtrip_llm = "claude-cli"   # default: opencode-cli
```

Environment: `TRANSLATEX_ROUNDTRIP_PRESET`, `TRANSLATEX_ROUNDTRIP_LLM`.

## LLM providers and quota

Two providers run a local agent CLI. They need no API key and use the
subscription you already pay for:

| Provider | Runs | Quota | Default model |
| --- | --- | --- | --- |
| `claude-cli` | `claude -p` | your Claude subscription | haiku |
| `opencode-cli` | `opencode run` | your OpenCode quota | `opencode-go/deepseek-v4.1-flash` |

```toml
[provider.claude-cli]
model = "haiku"
bin = "claude"

[provider.opencode-cli]
model = "opencode-go/deepseek-v4.1-flash"
bin = "opencode"
```

They are slower than a web endpoint (seconds per hop) and each hop spends a
little quota. `auto` never picks them: they run only when named, in a chain hop,
a preset or `--provider`.

## What an LLM hop receives

- The hop instruction: translate, and rewrite the way a person would.
- In a round trip, the original text as well. The hop first repairs meaning that earlier machine-translation hops lost, with the smallest change that fixes it, then rewrites.
- For a later LLM hop, the previous LLM hop as history.
- The protected spans, as-is. An LLM hop gets no placeholders; the prompt lists the spans to keep, and translatex checks the output afterwards. A dropped span adds a `preserve: <provider> dropped <span>` warning. Machine-translation hops still mask spans.

Name patterns instead of writing regexes: `--preserve @code` (inline and fenced
code) and `--preserve @url`. On Windows the npm `.cmd` shim runs through
cmd.exe, which strips `^` from a regex such as `` `[^`]*` `` when Python or Node
spawns translatex without a shell. The named patterns avoid that.

## The repair hop

`<llm>:fix` (for example `opencode-cli:fix`, or the last hop of `strong`)
leaves the text in its current language and asks the LLM for a JSON list of
small edits instead of a new text:

```json
[{"from": "Aloha's team", "to": "Aroha's team", "why": "name respelled"}]
```

translatex applies an edit only when it is exact and small:

- `from` must appear exactly once in the current text, must not be empty, and may be at most 40 characters or 25% of the text, whichever is larger.
- `to` may be at most three times the length of `from` plus 80 characters.
- Edits that restore a missing fact (a name, number or other token the guard lists) always apply and spend no budget.
- All other edits (`kind: meaning`) share one budget: content-word overlap with the original may rise by at most 0.10 over the text after the fact edits. Many small edits together cannot copy the original back.

Each hop reports what happened to every edit:

```json
"edits": [
  {"from": "Aloha's team", "to": "Aroha's team", "why": "name respelled", "kind": "fact", "applied": true},
  {"from": "…", "to": "…", "why": "…", "kind": "meaning", "applied": false, "reason": "copies the original back"}
]
```

Skip reasons include `from not found`, `from not unique`, `no change`,
`to too long for from` and `copies the original back`.

The repair hop cut the fact loss of the same chain without it from 3 to 5 facts to 1 on the
corpus below, and kept most of the rewrite.

## Fact guard

Every round trip, whatever the chain, compares the result with the source and
lists in `missingFacts` the tokens that are absent:

- code spans, URLs, paths, ticket IDs, @mentions and numbers: always. Numbers compare by digits, so `18,900` matches `18900` and `4:50pm` matches `4:50 PM`.
- capitalised names, when both texts are in Latin script: as whole words, so `Tomi` does not vouch for `Tom`. Mid-sentence names are case-sensitive. A sentence-initial name is checked case-insensitively unless it is a common sentence-starting word or also appears in lowercase in the source, so a name that doubles as such a word can slip through.

A non-empty list also adds a `fact check: missing <list>` warning. The guard
reports; it does not fix. Read it, then re-run with a repair hop or restore the
token by hand.

## Preferred engines and fallback

A named machine-translation engine in a chain is preferred, not pinned. On a
retryable error (`RATE_LIMITED`, `TIMEOUT`, `NETWORK_ERROR`,
`QUOTA_EXHAUSTED`) the hop falls back to another engine from the automatic chain,
avoiding the engines of the neighbouring hops, and adds a warning:

```
hop 2: edge failed (RATE_LIMITED), served by google
```

LLM hops never fall back: a different model would change the result, so a
failure there is returned as the error.

## Detector score, best of N and target score

LLM hops vary between runs, so a second run can read differently. A scorer lets
translatex pick between runs by a detector's score, under a hard rule: a run that
keeps every fact always beats one that lost a fact, whatever the scores. Use it to benchmark and tune presets; a local detector loads a model on every call, which is too slow and heavy for live requests.

```bash
translatex roundtrip "<text>" --from en --preset standard --best-of 3 --scorer "python score.py" --format json
translatex roundtrip "<text>" --from en --target-score 0.4 --scorer "python score.py" --format json
```

- `--scorer "<command>"` (config `roundtrip_scorer`): translatex writes each result to the command's stdin and reads a score from 0 to 1 (higher = more AI-like) from the last non-empty line of stdout. The command is split into words (quotes group, backslashes stay literal) and run without a shell; programs on `PATH` resolve the same way as the CLI providers, `.cmd` shims included. Each call has the `--timeout-ms` budget, but never less than 120 s, since local detectors load a model per call.
- A scorer that fails, times out or prints something other than a number in [0, 1] adds a `scorer: run <n> (<preset>) unscored: <reason>` warning, and that run is unscored. When no run gets a score, the result is the same as without a scorer.
- `--best-of N` (1-5, default 1) runs the planned chain N times. Selection: no missing facts first (else the fewest), then the lowest score, with unscored runs after scored ones. Above 1 it needs a scorer and fails with `INVALID_INPUT` otherwise.
- `--target-score X` (0-1, needs a scorer) tries the requested chain first, with `--best-of`, then each preset of `roundtrip_ladder` (default `["standard", "strong"]`) not yet tried. It stops at the first rung where the best run so far scores at or below X; otherwise it returns the best run overall with a `target score not met` warning.

The response adds `score` (the selected run's score, `null` without a usable
scorer), `scorer` (program name, no arguments) and, when more than one run
happened, `candidates`: one `{presetOrChain, score, missingFacts}` per run,
`missingFacts` as a count. Only the selected run's text and hops are returned.

```toml
roundtrip_scorer = "python score.py"
roundtrip_ladder = ["standard", "strong"]
```

A score is a measurement from one detector on one text. Report it with the
detector's name; another detector can disagree, and a low score is no claim that
the text is undetectable.

## When to use which

| Situation | Choose |
| --- | --- |
| Humanise AI-sounding prose and keep the facts | `--preset standard` |
| Maximum rewriting, and a few facts may need a repair pass | `--preset strong` |
| No subscription, or speed matters | `--preset light` (default) |
| Machine translation, a little more rewriting | `--depth 3`; fact loss rises faster than rewriting |
| Text with code or URLs | add `--preserve @code --preserve @url` to any preset |
| Legal text, contracts, numbers someone will check | do not round-trip |

Read `missingFacts` and `warnings` after every run, and report them.

## Navigation

- ← [Round trip overview](OVERVIEW.md)
- → [Choosing the pivot and depth](01-pivot-choice.md)
- ↑ [Documentation index](../README.md)
