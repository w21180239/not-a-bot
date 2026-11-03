**English** · [简体中文](../../zh-cn/env/OVERVIEW.md)

# Environment — Overview

Environment variables are the lowest-friction way to configure a container or a CI job. Every variable has a file-based equivalent; the environment wins.

## File index

| File | Scope |
| --- | --- |
| [01-variables.md](01-variables.md) | Full variable list, precedence, examples |

## Quick list

| Variable | Purpose |
| --- | --- |
| `TRANSLATEX_HOME` | Base directory for config, credentials and cache |
| `TRANSLATEX_CONFIG` | Explicit config file path |
| `TRANSLATEX_CREDENTIALS` | Explicit credentials file path |
| `TRANSLATEX_CHAIN` | Comma-separated chain override |
| `TRANSLATEX_TIMEOUT_MS` | Default per-request budget |
| `TRANSLATEX_LOG` | stderr log level |
| `TRANSLATEX_<PROVIDER>_<FIELD>` | One credential field |

## Navigation

- ← [Documentation index](../README.md)
- → [Variable reference](01-variables.md)
