# Changelog

Documentation history. Application changes are recorded here as well while the project is pre-1.0.

## 0.1.0 — 2026-11-04

### Added

- Full documentation tree for English and Chinese: overview, install, CLI, providers, API, MCP, environment, architecture, troubleshooting.
- Root README around the web-first architecture in English, Chinese and Russian.
- Three test suites: `tests/contract` (wire shapes and provider catalogue invariants), `tests/integration` (provider round trips against a mock server), `tests/mcp` (JSON-RPC over stdio against the real binary). 243 tests in total.

### Changed

- Corrected every wire example to camelCase (`targetLang`, `retryAfterSecs`, `suggestedAction`).
- Cooldown state path documented as `~/.config/translatex/cache/`, matching the code.
- Error tables now include `PROVIDER_NOT_CONFIGURED` and `AUTO_DETECT_UNSUPPORTED`.
- `volcengine` and `papago` advertise canonical language codes; the endpoint dialect (`zh-Hant`, `zh-CN`) stays inside the adapters.
- `reverso` is excluded from the automatic chain: it fails at the TLS handshake, so every automatic request paid for an attempt that could not succeed. Pinning it or listing it in `--fallback` still works.

### Fixed

- `chain` from `config.toml` and `TRANSLATEX_CHAIN` is now honoured; previously the engine always used the compiled default.
- `--provider X --fallback A,B` now degrades to the fallback list instead of ignoring it.
- MCP `tools/list` emits object schemas and `tools/call` returns `content` with `isError`.
- MCP server loads real configuration and credentials, and builds its engine once.
- Preserve masking no longer indexes byte offsets as character offsets, which corrupted or panicked on CJK input.
- Keyless providers no longer enter cooldown and silently drop out of the chain.

### Removed

- The unused `iflytek` adapter module. The name remains a `KNOWN_PROVIDERS` alias for `iflyrec`, so existing configuration keeps working.
