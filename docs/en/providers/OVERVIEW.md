**English** · [简体中文](../../zh-cn/providers/OVERVIEW.md)

# Providers — Overview

A provider is one translation backend. Every provider reports a stable id, a kind, a priority, a language set, an optional character limit, and whether it can detect the source language.

## File index

| File | Scope |
| --- | --- |
| [01-supported-providers.md](01-supported-providers.md) | Full catalogue: id, kind, priority, auth, limits, verified status |
| [02-priorities-and-fallback.md](02-priorities-and-fallback.md) | How the chain is assembled, matching rules, error-driven hop decisions |
| [03-adding-a-provider.md](03-adding-a-provider.md) | Implementing and registering a new backend |

## Kinds

| Kind | Meaning | Authenticated |
| --- | --- | --- |
| `web` | Public endpoint of a translation website, reverse-engineered | No |
| `official` | Vendor API with documented authentication | Yes |
| `llm` | Chat model prompted to translate | Yes |

## The short version

- Free web endpoints occupy priorities 1-12 and are always registered.
- Official APIs occupy 20-27 and are registered only when a complete credential set exists.
- LLM providers occupy 40-42 and require an explicit `api_key`.
- LibreTranslate is registered only when `[provider.libretranslate] base_url` is set, because no public instance is alive.
- `iflytek` is retired upstream and aliased to `iflyrec`.

## Inspecting at runtime

```bash
translatex providers --format json
translatex languages --provider edge --format json
translatex health --format json
```

`providers` is the authoritative list for your machine: registration depends on credentials and config, so it can differ from the catalogue.

## Navigation

- ← [Documentation index](../README.md)
- → [Supported providers](01-supported-providers.md)
