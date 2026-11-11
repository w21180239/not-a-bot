**English** · [简体中文](../../zh-cn/env/OVERVIEW.md)

# Environment — Overview

[← Documentation index](../README.md)

Environment variables are the lowest-friction way to configure a container or a CI job. Every variable has a file-based equivalent; the environment wins.

## File index

| File | Scope |
| --- | --- |
| [01-variables.md](01-variables.md) | Full variable list, precedence, examples |

## Quick list

| Variable | Purpose |
| --- | --- |
| `NOT_A_BOT_HOME` | Base directory for config, credentials and cache |
| `NOT_A_BOT_CONFIG` | Explicit config file path |
| `NOT_A_BOT_CREDENTIALS` | Explicit credentials file path |
| `NOT_A_BOT_CHAIN` | Comma-separated chain override |
| `NOT_A_BOT_TIMEOUT_MS` | Default per-request budget |
| `NOT_A_BOT_LOG` | stderr log level |
| `NOT_A_BOT_<PROVIDER>_<FIELD>` | One credential field |

## Navigation

- ← [Documentation index](../README.md)
- → [Variable reference](01-variables.md)
