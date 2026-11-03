**简体中文** · [English](../../en/integrations/OVERVIEW.md)

# 集成 — 概览

`translatex` 的接入方式是一条命令加一份 skill：Agent 执行命令、读 JSON，文档告诉它何时执行。没有服务要启动，也没有协议要协商。

## 分发渠道

| 渠道 | 命令 | 说明 |
| --- | --- | --- |
| npm | `npm install -g translatex` | 安装时按平台下载预编译二进制 |
| Cargo | `cargo install translatex --locked` | 源码构建；Rust 1.80+ 即可 |
| 发布包 | [Releases](https://github.com/w21180239/translatex/releases/latest) | `translatex-<target>.tar.gz`，二进制在根目录 |

npm 包本身只是壳：`postinstall` 从对应 GitHub Release 下载压缩包，`bin/translatex.js` 负责 exec。离线环境可以用 `TRANSLATEX_BINARY_PATH` 指向本地构建产物。

## 两份文档，覆盖所有 harness

| 文档 | 适用 |
| --- | --- |
| [`integrations/skill/translatex/SKILL.md`](../../../integrations/skill/translatex/SKILL.md) | 会加载 skill 的 harness：Claude Code、Codex、Gemini CLI、OpenCode |
| [`integrations/guidance/AGENTS.md`](../../../integrations/guidance/AGENTS.md) | 任何读取项目指令或规则文件的 harness |

完整矩阵（Claude Code、Codex CLI、Gemini CLI、OpenCode、Cursor、Cline、Roo Code、Kilo Code、Continue、JetBrains AI、Zed、Aider、Qwen Code、Crush、Goose、DeepSeek Harness、MiMo Code、AtomCode、OpenClaw、Hermes Agent、Pi、Reasonix、ZCode、QwenPaw）及各自的文件路径见 [`integrations/README.md`](../../../integrations/README.md)。

## 用哪个入口

| 需求 | 命令 |
| --- | --- |
| 模型在对话中翻译 | `translatex translate "…" -t <lang> --format json` |
| 去掉机器味、改写 | `translatex roundtrip "…" --from <lang> --format json` |
| 脚本、CI、pre-commit | 同一条 CLI，读 `text` |
| 批量 | `translatex batch`（NDJSON 进、NDJSON 出） |
| 查能力 | `translatex providers`、`translatex languages`、`translatex health` |

## Agent 环境里的凭据

从图形界面启动的 Agent（Cursor、VS Code）经常拿不到 shell 环境变量。把凭据写进 `~/.config/translatex/credentials.toml`，任何进程都能读。只用免费网页接口时完全不需要凭据，装好二进制就能用。

## 导航

- ← [文档索引](../README.md)
- → [往返翻译](../roundtrip/OVERVIEW.md)
