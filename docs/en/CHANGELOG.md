# Changelog

Documentation history. Application changes are recorded here as well while the project is pre-1.0.

## 0.1.0 — 2026-11-03

### Added

- Full documentation tree for English and Chinese: overview, install, CLI, providers, API, MCP, environment, architecture, troubleshooting.
- Root README rewritten around the web-first architecture in English, Chinese and Russian.

### Changed

- Corrected every wire example to camelCase (`targetLang`, `retryAfterSecs`, `suggestedAction`).
- Cooldown state path documented as `~/.config/translatex/cache/`, matching the code.
- Error tables now include `PROVIDER_NOT_CONFIGURED` and `AUTO_DETECT_UNSUPPORTED`.

### Fixed

- `chain` from `config.toml` and `TRANSLATEX_CHAIN` is now honoured; previously the engine always used the compiled default.
- `--provider X --fallback A,B` now degrades to the fallback list instead of ignoring it.
- MCP `tools/list` emits object schemas and `tools/call` returns `content` with `isError`.
- MCP server loads real configuration and credentials, and builds its engine once.
- Preserve masking no longer indexes byte offsets as character offsets, which corrupted or panicked on CJK input.
- Keyless providers no longer enter cooldown and silently drop out of the chain.
