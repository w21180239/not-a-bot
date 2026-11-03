# Changelog

Documentation history. Application changes are recorded here as well while the project is pre-1.0.

## 0.1.0 — 2026-11-04

### Added

- Full documentation tree for English and Chinese: overview, install, CLI, providers, API, environment, architecture, troubleshooting, round trips, integrations.
- Root README around the web-first architecture in English, Chinese and Russian.
- Two test suites: `tests/contract` (wire shapes and provider catalogue invariants) and `tests/integration` (provider round trips against a mock server). 243 tests in total.
- Parallel provider racing: `race_width` (default 3) keeps that many attempts in flight, the first success wins, failed slots refill from the chain. `--race N` and `race_width = 1` restore strict sequencing.
- `translatex roundtrip` (alias `polish`): translate out through a pivot language and back to strip machine-flavoured phrasing. Default pivot is Chinese for non-Chinese text and English for Chinese text, chosen by measurement; `roundtrip_pivot` and `--pivot` override it.
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
