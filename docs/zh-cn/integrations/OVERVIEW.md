**简体中文** · [English](../../en/integrations/OVERVIEW.md)

# 集成 — 概览

`translatex` 是一个二进制、两种入口：CLI 与 MCP 服务。不同 harness 能消费的入口不同，所以两边都给了可直接复制的配置。

## 分发渠道

| 渠道 | 命令 | 说明 |
| --- | --- | --- |
| npm | `npm install -g translatex` | 安装时按平台下载预编译二进制 |
| Cargo | `cargo install translatex --locked` | 源码构建；Rust 1.80+ 即可 |
| 发布包 | [Releases](https://github.com/w21180239/translatex/releases/latest) | `translatex-<target>.tar.gz`，二进制在根目录 |

npm 包本身只是壳：`postinstall` 从对应 GitHub Release 下载压缩包，`bin/translatex.js` 负责 exec。离线环境可以用 `TRANSLATEX_BINARY_PATH` 指向本地构建产物。

## Harness 对照表

| Harness | 推荐方式 | 文件 |
| --- | --- | --- |
| Claude Code | 插件（MCP + skill） | [`integrations/claude-code/`](../../../integrations/claude-code/README.md) |
| OpenCode | MCP 服务 + skill | [`integrations/opencode/`](../../../integrations/opencode/README.md) |
| Codex | MCP 服务 + `AGENTS.md` | [`integrations/codex/`](../../../integrations/codex/README.md) |
| Cursor | 项目级 MCP 服务 | [`integrations/cursor/`](../../../integrations/cursor/README.md) |
| VS Code / Copilot | 工作区 MCP 服务 | [`integrations/vscode/`](../../../integrations/vscode/README.md) |
| 其它 MCP 客户端 | stdio JSON-RPC | [`integrations/generic/`](../../../integrations/generic/README.md) |

读取 `SKILL.md` 的 harness（Claude Code、OpenCode）：[`integrations/skill/SKILL.md`](../../../integrations/skill/SKILL.md)。

## 用哪个入口

| 需求 | 入口 |
| --- | --- |
| 模型在对话中翻译 | MCP `translate` |
| 去掉机器味、改写 | MCP `roundtrip` |
| 脚本、CI、pre-commit | CLI `--format json` |
| 批量 | CLI `batch`（NDJSON 进、NDJSON 出） |
| 查能力 | MCP `providers` / `languages` / `health` |

两种入口读同一份 `config.toml` 和 `credentials.toml`，CLI 配好的 Key 重启 MCP 服务后即可用。

## Agent 环境里的凭据

从图形界面启动的 Agent（Cursor、VS Code）经常拿不到 shell 环境变量。两个可靠做法：

1. 写 `~/.config/translatex/credentials.toml`，两种入口都会读。
2. 在 MCP 服务配置里加 `env` 块，支持 `mcpServers` 的客户端都认。

只用免费网页接口时两者都不需要，只有官方 API 才要求配置。

## 导航

- ← [文档索引](../README.md)
- → [往返翻译](../roundtrip/OVERVIEW.md)
