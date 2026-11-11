# Changelog

Documentation history. Application changes are recorded here as well while the project is pre-1.0.

## 0.2.0 — 2026-11-12

### Breaking

- `strong` is removed and fails with `INVALID_INPUT` as an unknown preset. The presets are `light` and `standard`, and the default `rewrite_ladder` is `["light", "standard"]`.
- `light` is Google machine translation out, a fix hop, one LLM rewrite back and a final fix hop (chain `["zh","fix","en","fix"]`). It needs an LLM `rewrite_llm`, else `INVALID_INPUT`.

### Changed

- Every preset ends with an LLM minimal meaning-repair hop (`<llm>:fix`). The fix prompt covers meaning, not only facts: negations, qualifiers and certainty, actor and cause, order, dropped or invented claims. It still never restores cut feeling, pledge or pleasantry sentences. Fact edits always apply; other edits share one copy-back budget.
- The `standard` return hop rewrites freely without the original (chain `["zh","en","fix"]`).
- Results (`benches/prompt-lab`, 30 English texts x 2 samples, desklib detector, source texts 0.999): `standard` mean P(AI) 0.364, 0.5 texts with a lost fact token, 98% of texts with no major meaning error; `light` 0.489, 0, 97%. No chain that costs more than `standard` scored lower.

## 0.1.1 — 2026-11-11

### Fixed

- npm package renamed to `not-a-bot-cli`: the registry rejected `not-a-bot` as too similar to `notabot`. Install with `npm install -g not-a-bot-cli`; the commands are still `not-a-bot` and `nab`.

## 0.1.0 — 2026-11-11

not-a-bot continues translatex 0.3.0 under a new name. Nothing else changed in behaviour except the default preset.

### Added

- `lint` subcommand (`nab lint DRAFT [--source FILE] [--strict]`): offline check for AI tells (vocabulary, phrases, structure, spelling, punctuation) and, with `--source`, the facts a draft dropped or added. Ports the humanizer `lint.py`; JSON output is the new `LintResponse` schema type.

### Changed

- The binary is `not-a-bot`, with `nab` as a short alias that behaves the same. The crate is `not-a-bot` (library `not_a_bot`) and the npm package is `not-a-bot-cli` (npm rejected `not-a-bot` as too close to `notabot`; platform packages `not-a-bot-<os>-<arch>`); both npm bins run the same launcher.
- The `roundtrip` subcommand is now `rewrite`, and its default preset is `standard` (LLM out and back, then a fact repair); `light` stays available with `--preset light`. The schema types `RoundtripRequest` and `RoundtripResponse` are now `RewriteRequest` and `RewriteResponse`.
- Config keys `roundtrip_preset`, `roundtrip_pivot`, `roundtrip_llm`, `roundtrip_scorer` and `roundtrip_ladder` are now `rewrite_*`.
- Environment variables use the `NOT_A_BOT_` prefix instead of `TRANSLATEX_` (for example `NOT_A_BOT_REWRITE_PRESET`, `NOT_A_BOT_CONFIG`, `NOT_A_BOT_HOME`).
- The config and cache directory is `~/.config/not-a-bot`. Old names, variables and directories are not read.
- Provider metadata keys use the `not-a-bot.` prefix (`not-a-bot.style`, `not-a-bot.history`, `not-a-bot.original`), and the HTTP User-Agent is `not-a-bot/<version>`.
- The `translatex` integration skill is now `not-a-bot`.
- Presets: `standard` (default) is an LLM hop out, an LLM hop back and a fact-only repair hop (`<llm>:fix`); `strong` is an LLM hop out and back with no repair (furthest rewrite, lowest detector score, may drop a fact, always listed in `missingFacts`); `light` is unchanged. `standard` and `strong` reject a non-LLM `rewrite_llm` with `INVALID_INPUT`. Humanize hops delete sentences that only express feelings, pledges or pleasantries.
- The fix hop (`engine:fix`) restores hard facts only (names, places, products, numbers, dates, amounts, currencies, IDs, versions, paths, code spans, negations, qualifiers) against the original, is given the fact check's list of missing tokens, and never restores cut feeling or pledge sentences. Before, it repaired meaning.
- Install from source with `cargo install --git https://github.com/w21180239/not-a-bot`; the crate is not published to crates.io.

### Added

- Added: not-a-bot agent skill that merges a writing method (fact invariants, audience profiles, AI-tell patterns, optional synthetic-anchor voice profiles overridable from `~/.config/not-a-bot/profiles/`) with the CLI guide, and the skill-eval and detector benches under `benches/skill-evals` and `benches/detector`.

### Removed

- The Russian README and `docs/ru`.

## translatex history

Entries below predate the rename and describe translatex, including its `roundtrip` command and `TRANSLATEX_*` variables.

## Unreleased

### Changed

- LLM humanize hops delete sentences that only express feelings or pledges (thanks, excitement, pride, "we remain committed") and closing pleasantries, moving any name, place, number, date or amount in them into a kept sentence. On the 30-text English set in `benches/prompt-lab` this lowered the mean desklib score from 0.530 to 0.410 on the default opencode model.

## 0.3.0 — 2026-11-11

### Added

- `roundtrip --scorer "<command>"` (config `roundtrip_scorer`): an external detector gets each result on stdin and prints a score from 0 to 1 (higher = more AI-like) as its last stdout line. Run without a shell, bounded by `--timeout-ms`; a failing scorer adds a warning and leaves the run unscored.
- `roundtrip --best-of 1-5`: runs the chain N times and keeps the run with no missing facts (else the fewest), then the lowest score. Above 1 it needs a scorer.
- `roundtrip --target-score 0-1`: after the requested chain, tries each preset of `roundtrip_ladder` (default `standard,strong`) until the best run scores at or below the target, else returns the best run with a `target score not met` warning.
- Response fields `score`, `scorer` (program name only) and `candidates` (per run: `presetOrChain`, `score`, `missingFacts` count). Request fields `scorer`, `bestOf`, `targetScore`.
- `benches/roundtrip-depth/measure.py --scorer CMD` adds "detector src" and "detector out" columns, and configurations `standard-best3` and `standard-target`.

### Changed (breaking)

- Roundtrip presets are named by strength: `mt` is now `light` (two machine-translation hops, the default), `llm` is now `standard` (LLM out and back), and the preset that ended in a repair hop is now `strong` (LLM hops into zh and ja, machine-translation hops through fi and back, then a repair hop). Old names are rejected with `INVALID_INPUT` listing `light, standard, strong`; no aliases are kept.
- Config `roundtrip_preset` defaults to `light` and `roundtrip_ladder` to `["standard", "strong"]`. Update any config or `TRANSLATEX_ROUNDTRIP_PRESET` that holds an old name.

### Removed

- The three-hop machine-translation preset `mt-deep` and the preset that ran the strong chain without its repair hop. In the benchmark a third machine-translation hop lost more facts than two, and the chain without the repair hop lost 3 to 5 facts against 1. `--depth 1-3` and `--chain` still build such chains by hand.

### Changed

- `benches/roundtrip-depth/measure.py` names configurations after the new presets (`light-d1`, `light`, `standard-claude`, `standard-opencode`, `strong-claude`, `strong-opencode`) and gains `--langs` (default `en`) to filter corpus rows. Rows already in `results.jsonl` under old names stay readable and are not migrated.

## 0.2.0 — 2026-11-11

### Added

- `roundtrip` runs an N-hop chain. `--chain "<[engine:]lang,...>"` takes any order of engines and languages and appends a final hop back to the source when it is missing; `--depth 1-3` runs a machine-translation-only chain through zh, ja and fi (Chinese sources go en, ja, fi); `--preset <name>` selects `mt`, `mt-deep`, `llm`, `lynote` or `lynote-fix`. Precedence: chain, preset, depth, pivot, then config `roundtrip_preset`.
- Two providers that run a local agent CLI and need no API key: `claude-cli` (`claude -p`, your Claude subscription, default model haiku) and `opencode-cli` (`opencode run`, your OpenCode quota, default model `opencode-go/deepseek-v4.1-flash`). Configure `model` and `bin` under `[provider.claude-cli]` and `[provider.opencode-cli]`. They run only when named and are never picked by `auto`.
- Config `roundtrip_preset` (default `mt`), `roundtrip_llm` (default `opencode-cli`; alternative `claude-cli`), and the environment variables `TRANSLATEX_ROUNDTRIP_PRESET` and `TRANSLATEX_ROUNDTRIP_LLM`.
- LLM hops in a round trip receive the original text and repair meaning that earlier machine-translation hops lost, with the smallest change, before rewriting.
- Repair hop `<llm>:fix`: the LLM returns a JSON list of small edits and translatex applies only exact, small ones. Edits that restore a missing fact always apply; other edits share a small budget so the hop cannot copy the original back. Each edit is reported in the hop's `edits` with `kind` (`fact` or `meaning`), `applied` and `reason`.
- Fact guard: every round trip reports `missingFacts` (names, numbers, IDs, code spans and URLs absent from the result; numbers compared by digits, names as whole words) and adds a `fact check:` warning when the list is not empty.
- Response fields `chain`, `hops`, `missingFacts` and `warnings`.
- Named preserve patterns `@code` (inline and fenced code) and `@url`. On Windows the npm `.cmd` shim runs through cmd.exe, which strips `^` from a regex such as `` `[^`]*` `` when Python or Node spawns translatex without a shell; the named patterns avoid that.
- A named machine-translation engine in a chain is preferred, not pinned: on a retryable error the hop falls back to another engine, avoiding the engines of the neighbouring hops, with a warning. LLM hops never fall back.
- `benches/roundtrip-depth/` with a 10-text corpus and `measure.py`, plus the measured results in the round-trip docs (10 texts, one run, 2026-11-11).

### Changed (breaking)

- `roundtrip` responses drop `pivotLang`, `pivotText` and `providers`. Read `chain`, `hops[].text` and `hops[].provider` instead.

### Changed

- The default round trip is `mt` with two machine-translation hops (for example en to zh to ja to en); it was one hop. On the corpus it left 5 of 10 AI tells instead of 7 at the same fact loss.
- LLM hops receive protected spans as-is and the prompt lists them; a dropped span adds a `preserve:` warning. Machine-translation hops still mask.
- Provider count is 28: 14 free web endpoints, 9 official APIs, 5 LLM providers.

### Fixed

- `roundtrip/01-pivot-choice.md` no longer says the measurement script is in the repository history; it points at `benches/roundtrip-depth/measure.py`.
- The skill said a round trip cannot remove "not just X but Y". That holds for the machine-translation presets only; the LLM presets remove it.

## 0.1.2 — 2026-11-04

### Fixed

- The Windows platform package is published as `translatex-windows-x64`. npm's spam filter answers every attempt on `translatex-win32-x64` with a 403 that a retry does not clear; the shim maps `win32` to that name.
- npm publishing is re-runnable. Each package is skipped when its exact version is already on the registry, so a release that fails halfway is finished by a re-run instead of answering 409 for everything it had already shipped.

## 0.1.1 — 2026-11-04

### Changed

- npm distribution ships the binary in per-platform packages (`translatex-windows-x64`, `translatex-linux-arm64`, …) pulled in as optional dependencies. Installing runs no lifecycle script, which npm 12 blocks by default, so `npm install -g translatex` no longer depends on `postinstall` being allowed.
- Declared MSRV is 1.89, the highest `rust-version` in the dependency graph; the previous 1.80 claim never compiled.
- CI covers `cargo fmt --check`, `clippy -D warnings`, tests on Linux/macOS/Windows, an MSRV build, and an npm packaging check. Dependabot tracks the actions, Cargo and npm manifests.

### Fixed

- `cargo clippy --all-targets -- -D warnings` is clean; seven warnings were outstanding in `preserve`, `google`, `modernmt`, `volcengine`, `providers` and the engine tests.
- The repository declared `MIT OR Apache-2.0` but shipped no license text; `LICENSE-MIT` and `LICENSE-APACHE` are now in the tree.

## 0.1.0 — 2026-11-04

### Added

- Full documentation tree for English and Chinese: overview, install, CLI, providers, API, environment, architecture, troubleshooting, round trips, integrations.
- Root README in English, Chinese and Russian, leading with the benefit and the search terms a reader would use; the provider chain is explained further down.
- Two test suites: `tests/contract` (wire shapes and provider catalogue invariants) and `tests/integration` (provider round trips against a mock server). 245 tests in total.
- Parallel provider racing: `race_width` (default 3) keeps that many attempts in flight, the first success wins, failed slots refill from the chain. `--race N` and `race_width = 1` restore strict sequencing.
- `translatex roundtrip` (alias `polish`): translate out through a pivot language and back to strip machine-flavoured phrasing. Default pivot is Chinese for non-Chinese text and English for Chinese text, chosen by measurement; `roundtrip_pivot` (config), `TRANSLATEX_ROUNDTRIP_PIVOT` and `--pivot` override it.
- Failed translations carry the `attempts` array, so a `CHAIN_EXHAUSTED` result names every hop that was tried without a second run.
- npm package (`npm install -g translatex`) that downloads the prebuilt binary, a tagged release workflow for five targets, and per-harness CLI + skill setup for Claude Code, Codex CLI, Gemini CLI, OpenCode, Cursor, Cline, Roo Code, Kilo Code, Continue, JetBrains AI, Zed, Aider, Qwen Code, Crush, Goose, DeepSeek Harness, MiMo Code, AtomCode, OpenClaw, Hermes Agent, Pi, Reasonix, ZCode and QwenPaw.

### Changed

- Corrected every wire example to camelCase (`targetLang`, `retryAfterSecs`, `suggestedAction`).
- Cooldown state path documented as `~/.config/translatex/cache/`, matching the code.
- Error tables now include `PROVIDER_NOT_CONFIGURED` and `AUTO_DETECT_UNSUPPORTED`.
- `volcengine` and `papago` advertise canonical language codes; the endpoint dialect (`zh-Hant`, `zh-CN`) stays inside the adapters.
- `reverso` is excluded from the automatic chain: it fails at the TLS handshake, so every automatic request paid for an attempt that could not succeed. Pinning it or listing it in `--fallback` still works.
- Agent integration is CLI + skill only; the MCP server was removed during the 0.1.0 cycle.

### Fixed

- `chain` from `config.toml` and `TRANSLATEX_CHAIN` is now honoured; previously the engine always used the compiled default.
- `--provider X --fallback A,B` now degrades to the fallback list instead of ignoring it.
- Preserve masking no longer indexes byte offsets as character offsets, which corrupted or panicked on CJK input.
- Keyless providers no longer enter cooldown and silently drop out of the chain.

### Removed

- The unused `iflytek` adapter module. The name remains a `KNOWN_PROVIDERS` alias for `iflyrec`, so existing configuration keeps working.
