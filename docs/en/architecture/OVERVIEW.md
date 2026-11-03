**English** · [简体中文](../../zh-cn/architecture/OVERVIEW.md)

# Architecture — Overview

```
src/
├── main.rs              entry point, hands off to cli::run
├── cli.rs               argument parsing, dispatch, registry construction
├── retry.rs             exponential backoff with jitter
├── core/
│   ├── config.rs        Settings, default chain, paths
│   ├── creds.rs         credential loading, keyring expansion, env overlay
│   ├── crypto.rs        AES-ECB/CBC helpers used by the Iciba adapter
│   ├── engine.rs        fallback loop, key selection, preserve, timeout
│   ├── http.rs          shared clients, signing helpers, body handling
│   ├── keys.rs          per-credential cooldown state, cross-process
│   ├── lang.rs          canonical codes, aliases, per-provider tables
│   ├── preserve.rs      regex masking and restoration
│   ├── types.rs         wire types shared by the CLI and providers
│   └── error.rs         error taxonomy, wire mapping, HTTP status mapping
└── providers/           one module per backend plus registry and trait
```

## File index

| File | Scope |
| --- | --- |
| [01-request-flow.md](01-request-flow.md) | Validation, masking, chain assembly, attempts, restore |
| [02-credentials-and-rotation.md](02-credentials-and-rotation.md) | Credential resolution, rotation rounds, cooldown tiers |

## Design constraints

1. **stdout is a data channel.** Nothing but the requested payload is written there. Every diagnostic goes to stderr.
2. **Providers are stateless.** They receive a credential and a request; the engine owns retries, cooldowns and fallback.
3. **Errors are typed before they are formatted.** `Error::*` decides retry, cooldown and exit code; `WireError` is the serialised view.
4. **Chain order is data.** The engine never hardcodes provider names; it asks the registry.

## Navigation

- ← [Documentation index](../README.md)
- → [Request flow](01-request-flow.md)
