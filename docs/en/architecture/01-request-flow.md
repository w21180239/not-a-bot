**English** · [简体中文](../../zh-cn/architecture/01-request-flow.md)

# Architecture — Request flow

## Steps

1. **Normalise** — `source_lang` and `target_lang` go through `lang::normalize`. Unknown targets are `INVALID_INPUT`; equal source and target is also rejected.
2. **Mask** — every `preserve` pattern is compiled once. Matches are replaced with opaque placeholders before any provider sees the text.
3. **Build the chain** — `Registry::chain` applies the fallback list, the configured chain, then the remaining registry entries by priority. Providers that cannot serve the pair, or cannot detect the source, are dropped.
4. **Attempt** — for each provider, the engine picks a credential, applies the budget, calls `translate_once`, records an `AttemptRecord`.
5. **Restore** — on success, the preserver puts the masked spans back. A restore failure is treated as a failed attempt.
6. **Return** — `Outcome { response, attempts }`, or the most specific error the chain produced.

## Timeouts

The budget is `timeoutMs` from the request, else `timeout_ms` from config, else 20 s. It wraps the whole provider call with `tokio::time::timeout`, so a provider that ignores its own socket timeout still cannot exceed the budget.

A timeout is not retried inline: the provider already spent the full budget, and a retry would double the wait. The chain moves to the next provider.

## Retries

Within one provider, retryable errors (rate limit, quota, network, 5xx) are retried up to `retries` times with jittered exponential backoff. `Retry-After` from the error wins when present, clamped so a hostile header cannot stall the caller.

## Error selection

| Situation | Returned error |
| --- | --- |
| Nothing was attempted and a provider exists | `PROVIDER_NOT_CONFIGURED` / `PROVIDER_NOT_FOUND` / `AUTO_DETECT_UNSUPPORTED` |
| Every key of a keyed provider is cooling | `RATE_LIMITED` with the longest remaining cooldown |
| A pinned provider failed | Its own error, not `CHAIN_EXHAUSTED` |
| Several providers failed | The last error, or `CHAIN_EXHAUSTED` when only upstream failures occurred |

## Preserve engine

Placeholders are derived from the whole match rather than a fixed token, so a translation that reorders text cannot accidentally restore a span into the wrong sentence. Restoration is verified: if a placeholder is missing or duplicated, `restore` returns an error and the attempt counts as failed.

## Navigation

- ← [Architecture overview](OVERVIEW.md)
- → [Credentials and rotation](02-credentials-and-rotation.md)
