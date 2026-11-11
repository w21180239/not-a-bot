# Changelog

Documentation history. Application changes are recorded here as well while the project is pre-1.0.

## Unreleased

### Added

- `roundtrip --scorer "<command>"` (config `roundtrip_scorer`): an external detector gets each result on stdin and prints a score from 0 to 1 (higher = more AI-like) as its last stdout line. Run without a shell, bounded by `--timeout-ms`; a failing scorer adds a warning and leaves the run unscored.
- `roundtrip --best-of 1-5`: runs the chain N times and keeps the run with no missing facts (else the fewest), then the lowest score. Above 1 it needs a scorer.
- `roundtrip --target-score 0-1`: after the requested chain, tries each preset of `roundtrip_ladder` (default `llm,lynote-fix`) until the best run scores at or below the target, else returns the best run with a `target score not met` warning.
- Response fields `score`, `scorer` (program name only) and `candidates` (per run: `presetOrChain`, `score`, `missingFacts` count). Request fields `scorer`, `bestOf`, `targetScore`.
- `benches/roundtrip-depth/measure.py --scorer CMD` adds "detector src" and "detector out" columns, and configurations `llm-best3` and `llm-target`.

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
