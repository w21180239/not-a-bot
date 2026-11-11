**English** · [简体中文](../../zh-cn/troubleshooting/OVERVIEW.md)

# Troubleshooting — Overview

[← Documentation index](../README.md)

Start with two commands. They answer most questions before you read any further:

```bash
not-a-bot providers --format json | jq '.providers[] | {name, kind, priority, configured}'
not-a-bot health --format json
```

## File index

| File | Scope |
| --- | --- |
| [01-common-issues.md](01-common-issues.md) | Symptom → cause → fix, per error class |

## Fast triage

| Symptom | First thing to check |
| --- | --- |
| `NO_PROVIDER` | Is any free endpoint registered? Is the chain empty for this pair? |
| `PROVIDER_NOT_CONFIGURED` | Credential fields complete? `not-a-bot keys <provider>` |
| `PROVIDER_NOT_FOUND` | Typo, or a keyed provider you never configured |
| `AUTO_DETECT_UNSUPPORTED` | Pass `-s en` instead of relying on `auto` |
| `CHAIN_EXHAUSTED` | `--verbose` for the per-hop trail |
| `TIMEOUT` everywhere | Network egress, proxy, or a too-small `timeoutMs` |
| Translation equals input | Endpoint changed shape; treat as a bug |

## Navigation

- ← [Documentation index](../README.md)
- → [Common issues](01-common-issues.md)
