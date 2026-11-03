**English** · [简体中文](../../zh-cn/architecture/02-credentials-and-rotation.md)

# Architecture — Credentials and rotation

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## Resolution

`CredentialStore::load` reads three sources and layers them:

1. `credentials.toml` — sections `[provider.label]`, each a flat map of fields.
2. Environment — `TRANSLATEX_<PROVIDER>_<FIELD>` becomes a credential labelled `env`.
3. Keyring — any field whose value is `@keyring` is replaced with the secret stored under the account `<provider>.<field>` in the `translators` service.

A provider is `configured` when it has at least one credential with every required field present. `ProviderInfo.configured` reflects that, and `build_keyed` registers official providers only when they qualify.

## Labels and rotation

Every credential carries a label. Rotation is round-robin across labels:

```
[k1 ok] [k2 cooling] [k3 ok]
   ↑ next request starts here, skips k2 while it cools
```

The engine loads the ring once per provider per request, picks the first non-cooling slot, and on failure records the cooldown before moving on. `rounds` equals the number of labels, so a request can try every key once.

## Cooldown tiers

| Failure | Cooldown |
| --- | --- |
| `QUOTA_EXHAUSTED` | `retryAfterSecs`, else 3600 s |
| `RATE_LIMITED` | `retryAfterSecs`, else 60 s |
| `AUTH_FAILED` | 900 s |

State is persisted per provider in `~/.config/translatex/cache/keys-<provider>.json`:

```json
{
  "cooled_until": { "default": "2026-11-03T05:12:44Z" },
  "failures": { "default": 2 }
}
```

Every process reads and writes the same file, so two concurrent calls share one view. `translatex keys --reset` deletes the files.

## Keyless providers

Web endpoints have one pseudo-credential labelled `default` and no second key. They use an unmanaged ring:

- No state file is written.
- `record_failure` is a no-op.
- The channel can never disappear because of a cooldown.

This was a real bug once: cooling a keyless provider removed it from every later request until the cooldown expired, which looked like a silent channel outage.

## All keys cooling

When every key of every provider in the chain is cooling, nothing is attempted. The engine reports `RATE_LIMITED` with the longest remaining cooldown rather than `NO_PROVIDER`, so an agent backs off instead of concluding the setup is broken.

## Navigation

- ← [Request flow](01-request-flow.md)
- ↑ [Architecture overview](OVERVIEW.md)
