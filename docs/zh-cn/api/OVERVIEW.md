**简体中文** · [English](../../en/api/OVERVIEW.md)

# API — 概览

[← 文档索引](../README.md)

CLI 的每条命令共用同一套线格式契约：一个请求对象进，一个响应或一个强类型错误出。字段名一律 camelCase。

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-cli-json.md](01-cli-json.md) | 请求/响应结构、NDJSON 批次格式 |
| [02-errors.md](02-errors.md) | 错误码、重试语义、Agent 决策规则 |

## 类型

| 类型 | 使用方 | 用途 |
| --- | --- | --- |
| `TranslateRequest` | `translate`、`batch` | 翻译什么、怎么翻 |
| `TranslateResponse` | 成功路径 | 结果与来源信息 |
| `RoundtripRequest` | `roundtrip` | 文本，加 `chain`、`preset`、`depth` 或 `pivot` |
| `RoundtripResponse` | `roundtrip` 成功 | 最终文本、`chain`、每跳 `hops`（修复跳带 `edits`）、`missingFacts`、`warnings` |
| `ErrorResponse` | CLI 失败 | 强类型错误，外加已经试过的跳 |
| `WireError` | `ErrorResponse` 内部 | 错误码、消息、重试提示 |
| `ProviderInfo` | `providers` | 能力与配置状态 |
| `HealthRecord` | `health` | 单个 Provider 的探测结果 |
| `AttemptRecord` | `--verbose`、失败的链 | 一跳尝试记录 |

Schema 直接从二进制生成，不要手抄：

```bash
translatex schema --format json
```

## 稳定性

- 字段名与错误码是公开契约。
- 可以新增可选字段；解析方必须忽略未知字段。
- 删除或重命名字段会提升主版本号。

## 导航

- ← [文档索引](../README.md)
- → [请求与响应结构](01-cli-json.md)
