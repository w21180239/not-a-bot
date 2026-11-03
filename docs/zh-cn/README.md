**简体中文** · [English](../en/README.md)

# translatex — 文档

`translatex` 是面向 Agent 的翻译 CLI：先走免费网页接口，有凭据时回退到官方 API，密钥自动轮换，输出结构化 JSON。

## 从这里开始

| 你想…… | 阅读 |
| --- | --- |
| 一分钟装好并翻译第一句 | [install/OVERVIEW.md](install/OVERVIEW.md) |
| 查所有命令与参数 | [cli/OVERVIEW.md](cli/OVERVIEW.md) |
| 知道有哪些 Provider、顺序如何 | [providers/OVERVIEW.md](providers/OVERVIEW.md) |
| 接入 Claude Code、Codex、OpenCode 等 Agent | [integrations/OVERVIEW.md](integrations/OVERVIEW.md) |
| 看 24 家 harness 的图标与各自路径 | [integrations/02-supported-harnesses.md](integrations/02-supported-harnesses.md) |
| 安全地解析输出 | [api/OVERVIEW.md](api/OVERVIEW.md) |
| 排查某个通道失败 | [troubleshooting/OVERVIEW.md](troubleshooting/OVERVIEW.md) |

## 领域

| 领域 | 范围 |
| --- | --- |
| [install/](install/OVERVIEW.md) | Cargo、预编译包、首次请求、命令补全 |
| [cli/](cli/OVERVIEW.md) | 命令、参数、退出码、配置与凭据文件 |
| [providers/](providers/OVERVIEW.md) | Provider 目录、优先级方案、回退语义、新增 Provider |
| [roundtrip/](roundtrip/OVERVIEW.md) | 经中转语言的去 AI 味改写；中转语言实测选型 |
| [integrations/](integrations/OVERVIEW.md) | npm、发布包、各 harness 的 CLI 与 skill 接入 |
| [api/](api/OVERVIEW.md) | JSON 与 NDJSON 结构、错误分类、Agent 决策规则 |
| [env/](env/OVERVIEW.md) | 环境变量与优先级 |
| [architecture/](architecture/OVERVIEW.md) | 请求流程、密钥轮换、冷却状态、保护片段引擎 |
| [troubleshooting/](troubleshooting/OVERVIEW.md) | 症状 → 原因 → 处理 |

## 项目信息

- **版本：** 0.1.0
- **仓库：** [github.com/w21180239/translatex](https://github.com/w21180239/translatex)
- **许可：** MIT OR Apache-2.0
- **Agent 集成：** [AGENTS.md](../../AGENTS.md)
- **变更记录：** [CHANGELOG.md](CHANGELOG.md)
