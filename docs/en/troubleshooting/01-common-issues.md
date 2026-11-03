**English** · [简体中文](../../zh-cn/troubleshooting/01-common-issues.md)

# Troubleshooting — Common issues

## `NO_PROVIDER`

Nothing in the chain can serve this request.

| Cause | Check | Fix |
| --- | --- | --- |
| Every provider filtered out for the pair | `translatex providers --format json` | Choose another target, or a provider that covers it |
| `chain` in config lists only unregistered providers | `translatex providers` | List registered ids, or delete `chain` |
| Keyed-only chain and no credentials | `translatex keys` | Add credentials |

## `PROVIDER_NOT_CONFIGURED`

The id exists in the binary but is not registered. For keyed providers this means an incomplete credential set: for example Baidu needs both `appid` and `secret_key`. Check the field names against the provider table, then:

```bash
translatex providers --format json | jq '.providers[] | select(.name=="baidu")'
```

## `AUTO_DETECT_UNSUPPORTED`

You pinned a provider that cannot detect the source: `lingvanex`, `mymemory`, `systran`, `reverso`. Either pass `-s <code>` or drop `--provider` so the chain can pick a detecting provider.

## `RATE_LIMITED` on a free endpoint

Several web endpoints throttle by IP:

| Provider | Typical behaviour |
| --- | --- |
| `sogou` | `errorCode 20`, reported as `RATE_LIMITED` with a 300 s hint |
| `mymemory` | Daily character cap; resets at midnight UTC |
| `lara` | Daily credit per IP |

Wait out `retryAfterSecs`. If it persists, reorder the chain so the throttled provider runs later:

```toml
chain = ["edge", "google", "iciba", "sogou"]
```

## `reverso` always fails with HTML in the message

Cloudflare rejects the TLS fingerprint of a rustls client. The endpoint itself is alive and works from browsers, so this only happens when you ask for `reverso` explicitly — it is excluded from the automatic chain. There is no header workaround; use another provider.

## `TIMEOUT` for every provider

1. Confirm the budget: `translatex translate "hi" -t zh --timeout-ms 30000`.
2. Confirm egress: `translatex health --format json`.
3. Behind a proxy, set `HTTPS_PROXY`; reqwest honours it.

Remember that a timeout is not retried inline by design, so a slow provider costs exactly one budget before the chain moves on.

## A provider returns the source unchanged

Silent failures look like success. Two defences:

- The engine verifies preserve-token restoration and treats corruption as a failed attempt.
- Every adapter parses the response body and returns `BAD_RESPONSE` when the translation field is missing.

If you still see an echo, the endpoint changed its response shape. Capture the body and update the adapter.

## Batch output is shorter than the input

Invalid JSON lines are skipped with a stderr warning. Run the batch again without redirecting stderr to see which line numbers failed.

## Cooldowns will not clear

```bash
translatex keys --reset
ls ~/.config/translatex/cache/
```

If the directory is not writable, cooldowns are memory-only for that process and every call starts clean.

## The agent translates by hand instead of calling the CLI

1. Confirm the skill or instructions file is in the path the harness documents.
2. `translatex --version` must succeed in the same shell the agent uses; a GUI-launched agent often has a different `PATH`.
3. Ask for a translation and check the reply for a `provider` name. A reply without one means the agent never ran the command.

## Navigation

- ← [Troubleshooting overview](OVERVIEW.md)
- → [Error contract](../api/02-errors.md)
