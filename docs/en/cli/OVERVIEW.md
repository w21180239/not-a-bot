**English** · [简体中文](../../zh-cn/cli/OVERVIEW.md)

# CLI — Overview

[← Documentation index](../README.md)

stdout carries machine-readable output only. Logs, warnings, and progress go to stderr, so `not-a-bot … | jq` always works.

## Command index

| Command | Scope | Detail |
| --- | --- | --- |
| `translate` | One request, text argument or stdin | [01-commands.md](01-commands.md) |
| `rewrite` | Out through a pivot language and back; strips machine cadence | [01-commands.md](01-commands.md) |
| `batch` | NDJSON in, NDJSON out | [01-commands.md](01-commands.md) |
| `providers` | Capability and configuration state | [01-commands.md](01-commands.md) |
| `languages` | Canonical codes or one provider's set | [01-commands.md](01-commands.md) |
| `keys` | Cooldown state, `--reset` | [01-commands.md](01-commands.md) |
| `health` | Probe configured providers | [01-commands.md](01-commands.md) |
| `schema` | JSON Schema for all wire types | [01-commands.md](01-commands.md) |
| `manifest` | Machine-readable list of CLI operations | [01-commands.md](01-commands.md) |
| `init` | Write a config template | [01-commands.md](01-commands.md) |
| `completions` | Shell completion script | [01-commands.md](01-commands.md) |

## Global flags

| Flag | Effect |
| --- | --- |
| `--format json\|text` | JSON object (default) or bare translation |
| `--quiet` | Compact single-line JSON instead of pretty output |
| `--verbose` | Include the per-attempt trail in the result |
| `--race N` | Providers kept in flight for `auto` (default 3, `1` = sequential) |
| `--config PATH` | Override the config path (env `NOT_A_BOT_CONFIG`) |
| `--credentials PATH` | Override the credentials path (env `NOT_A_BOT_CREDENTIALS`) |
| `--log LEVEL` | stderr log level: `error`, `warn`, `info`, `debug` |

## Exit codes

| Code | Meaning | Typical cause |
| --- | --- | --- |
| 0 | Success | — |
| 1 | Request error | Invalid input, unsupported pair, auth failure |
| 2 | Config error | Malformed TOML, unknown key |
| 3 | Chain exhausted | Every provider failed or none configured |
| 4 | Network error | DNS, TLS, timeout at the transport layer |

Agents should branch on the exit code first and on `error.code` second.

## Configuration files

- [02-config-files.md](02-config-files.md) — `config.toml`, `credentials.toml`, precedence, keyring placeholders.

## Navigation

- ← [Documentation index](../README.md)
- → [API and error contract](../api/OVERVIEW.md)
