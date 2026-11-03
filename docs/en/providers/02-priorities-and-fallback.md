**English** · [简体中文](../../zh-cn/providers/02-priorities-and-fallback.md)

# Providers — Priorities and fallback

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## Building the chain

`Registry::chain` assembles the attempt list in this order:

1. **Fallback list** — `--fallback` / `fallbackChain`, in the order written.
2. **Configured chain** — `chain` in `config.toml` or `TRANSLATEX_CHAIN`.
3. **Everything else** registered, sorted by ascending `priority`.

Duplicates are removed on the resolved provider id, so an alias and its target cannot both appear. When a provider is pinned with `--provider`:

- If it can serve the request, it goes first and the fallback list follows.
- If it cannot (no auto-detection, unsupported pair) and a fallback list exists, the fallback list takes over.
- If it cannot and there is no fallback, the request fails with `AUTO_DETECT_UNSUPPORTED` or `LANGUAGE_PAIR_NOT_SUPPORTED`, naming the provider.

## Filtering

A provider is dropped from an unpinned chain when either check fails:

| Check | Applied when | Result |
| --- | --- | --- |
| Target language supported | Always, unless `target` is `auto` | Provider cannot translate into that language |
| Source detection supported | `sourceLang` is `auto` | Provider would need an explicit source |

`supports_pair` is the hook for endpoints with asymmetric support. `iflyrec` overrides it to require Chinese on one side.

## Priority bands

| Band | Kind | Rationale |
| --- | --- | --- |
| 1-3 | Fast, high-quality web endpoints | Google, Edge, Iciba cover most traffic |
| 4-8 | Specialised web endpoints | iFlyrec (zh), ModernMT, Volcengine, Lara, Lingvanex, Mirai, Papago |
| 9-12 | Fallback web endpoints | Reverso, Sogou, MyMemory, SysTran |
| 20-27 | Official APIs | Deterministic and SLA-backed, but require keys |
| 40-42 | LLM | Highest cost, most latency, least predictable |

Lower is earlier. The bands are a contract, not an accident: a web endpoint must never outrank an official API in the default chain unless it is genuinely more reliable for that language pair.

## Default-chain exclusions

One provider is registered but never selected automatically:

| Provider | Why | How to use it |
| --- | --- | --- |
| `reverso` | Cloudflare rejects this client's TLS fingerprint, so the attempt fails at the handshake every time | `--provider reverso`, `--fallback reverso`, or listing it in `chain` |

`DEFAULT_EXCLUDED` in `src/providers/mod.rs` is the skip list. The registry never appends those providers by priority; naming one anywhere — pinned, fallback or configured chain — still attempts it, and `translatex providers` still lists it.

## Attempt behaviour

For each provider the engine can make up to `retries + 1` calls:

| Error class | Inline retry | Key cooldown | Next provider |
| --- | --- | --- | --- |
| `RATE_LIMITED` | yes, after backoff | yes | yes |
| `QUOTA_EXHAUSTED` | no | yes | yes |
| `AUTH_FAILED` | no | yes | yes |
| `UPSTREAM_ERROR` 5xx | yes | no | yes |
| `UPSTREAM_ERROR` 4xx | no | no | yes |
| `NETWORK_ERROR` | yes | no | yes |
| `TIMEOUT` | no | no | yes |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | no | no | yes |
| `INVALID_INPUT` | no | no | no — returned immediately |

A timeout is deliberately not retried inline: the provider already consumed its whole budget, and trying again just doubles the wait.

## Cooldown

Cooldowns exist to rotate between credentials, not to disable a provider:

- Keyed providers persist cooldown state to `~/.config/translatex/cache/keys-<provider>.json`, shared by every process.
- Keyless providers use an unmanaged ring. Their single pseudo-credential never cools, because there is no second key to rotate to and a cooldown would silently remove the channel.
- If every key of every provider is cooling, the error is `RATE_LIMITED` with the longest remaining cooldown, not `NO_PROVIDER`.

## Navigation

- ← [Supported providers](01-supported-providers.md)
- → [Adding a provider](03-adding-a-provider.md)
