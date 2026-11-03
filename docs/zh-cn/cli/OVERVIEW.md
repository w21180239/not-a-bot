**简体中文** · [English](../../en/cli/OVERVIEW.md)

# CLI — 概览

stdout 只承载机器可读输出；日志、警告、进度都走 stderr，所以 `translatex … | jq` 永远可用。

## 命令索引

| 命令 | 作用 | 详情 |
| --- | --- | --- |
| `translate` | 单条翻译，参数或 stdin | [01-commands.md](01-commands.md) |
| `batch` | NDJSON 进、NDJSON 出 | [01-commands.md](01-commands.md) |
| `providers` | 能力与配置状态 | [01-commands.md](01-commands.md) |
| `languages` | 规范语言码或单个 Provider 的集合 | [01-commands.md](01-commands.md) |
| `keys` | 冷却状态，`--reset` 清空 | [01-commands.md](01-commands.md) |
| `health` | 探测已配置 Provider | [01-commands.md](01-commands.md) |
| `schema` | 全部线格式类型的 JSON Schema | [01-commands.md](01-commands.md) |
| `manifest` | MCP 工具清单 | [01-commands.md](01-commands.md) |
| `mcp` | stdio MCP 服务 | [../mcp/OVERVIEW.md](../mcp/OVERVIEW.md) |
| `init` | 配置模板 | [02-config-files.md](02-config-files.md) |
| `completions` | Shell 补全脚本 | [01-commands.md](01-commands.md) |

## 全局参数

| 参数 | 作用 |
| --- | --- |
| `--format json\|text` | JSON 对象（默认）或纯译文 |
| `--quiet` | 输出紧凑单行 JSON |
| `--verbose` | 结果中附带逐跳尝试轨迹 |
| `--config PATH` | 指定配置路径（环境变量 `TRANSLATEX_CONFIG`） |
| `--credentials PATH` | 指定凭据路径（环境变量 `TRANSLATEX_CREDENTIALS`） |
| `--log LEVEL` | stderr 日志级别：`error`、`warn`、`info`、`debug` |

## 退出码

| 码 | 含义 | 常见原因 |
| --- | --- | --- |
| 0 | 成功 | — |
| 1 | 请求错误 | 输入非法、语言对不支持、认证失败 |
| 2 | 配置错误 | TOML 写错、未知字段 |
| 3 | 链耗尽 | 全部 Provider 失败或都未配置 |
| 4 | 网络错误 | DNS、TLS、传输层超时 |

Agent 应先按退出码分支，再看 `error.code`。

## 配置文件

- [02-config-files.md](02-config-files.md) —— `config.toml`、`credentials.toml`、优先级、钥匙串占位符。

## 导航

- ← [文档索引](../README.md)
- → [API 与错误契约](../api/OVERVIEW.md)
