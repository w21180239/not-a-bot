**简体中文** · [English](../../en/env/OVERVIEW.md)

# 环境变量 — 概览

[← 文档索引](../README.md)

环境变量是容器和 CI 中最省事的配置方式。每个变量都有对应的文件配置，环境变量优先。

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-variables.md](01-variables.md) | 完整变量表、优先级、示例 |

## 快速列表

| 变量 | 作用 |
| --- | --- |
| `NOT_A_BOT_HOME` | 配置、凭据、缓存的根目录 |
| `NOT_A_BOT_CONFIG` | 显式指定配置文件 |
| `NOT_A_BOT_CREDENTIALS` | 显式指定凭据文件 |
| `NOT_A_BOT_CHAIN` | 逗号分隔的链覆盖 |
| `NOT_A_BOT_TIMEOUT_MS` | 默认单次预算 |
| `NOT_A_BOT_LOG` | stderr 日志级别 |
| `NOT_A_BOT_<PROVIDER>_<FIELD>` | 单个凭据字段 |

## 导航

- ← [文档索引](../README.md)
- → [变量参考](01-variables.md)
