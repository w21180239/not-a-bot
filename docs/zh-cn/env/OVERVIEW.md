**简体中文** · [English](../../en/env/OVERVIEW.md)

# 环境变量 — 概览

环境变量是容器和 CI 中最省事的配置方式。每个变量都有对应的文件配置，环境变量优先。

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-variables.md](01-variables.md) | 完整变量表、优先级、示例 |

## 快速列表

| 变量 | 作用 |
| --- | --- |
| `TRANSLATEX_HOME` | 配置、凭据、缓存的根目录 |
| `TRANSLATEX_CONFIG` | 显式指定配置文件 |
| `TRANSLATEX_CREDENTIALS` | 显式指定凭据文件 |
| `TRANSLATEX_CHAIN` | 逗号分隔的链覆盖 |
| `TRANSLATEX_TIMEOUT_MS` | 默认单次预算 |
| `TRANSLATEX_LOG` | stderr 日志级别 |
| `TRANSLATEX_<PROVIDER>_<FIELD>` | 单个凭据字段 |

## 导航

- ← [文档索引](../README.md)
- → [变量参考](01-variables.md)
