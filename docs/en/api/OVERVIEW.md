**English** · [简体中文](../../zh-cn/api/OVERVIEW.md)

# API — Overview

[← Documentation index](../README.md)

The wire contract is the same for every command: one request object in, one response or one typed error out. All field names are camelCase.

## File index

| File | Scope |
| --- | --- |
| [01-cli-json.md](01-cli-json.md) | Request/response shapes, NDJSON batch framing |
| [02-errors.md](02-errors.md) | Error codes, retry semantics, agent decision rules |

## Types

| Type | Used by | Purpose |
| --- | --- | --- |
| `TranslateRequest` | `translate`, `batch` | What to translate and how |
| `TranslateResponse` | success paths | Result plus provenance |
| `RoundtripRequest` | `roundtrip` | Text plus the pivot language |
| `RoundtripResponse` | `roundtrip` success | Final text, pivot text, both hops |
| `ErrorResponse` | CLI failures | Typed error, plus the hops already tried |
| `WireError` | inside `ErrorResponse` | Code, message, retry hints |
| `ProviderInfo` | `providers` | Capability and configuration |
| `HealthRecord` | `health` | Probe result per provider |
| `AttemptRecord` | `--verbose`, failed chains | One provider hop |

Generate the schema from the binary rather than copying it:

```bash
translatex schema --format json
```

## Stability

- Field names and error codes are the public contract.
- New optional fields may appear; parsers must ignore unknown ones.
- Removing or renaming a field bumps the major version.

## Navigation

- ← [Documentation index](../README.md)
- → [Request and response shapes](01-cli-json.md)
