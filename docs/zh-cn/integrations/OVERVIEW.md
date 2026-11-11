**简体中文** · [English](../../en/integrations/OVERVIEW.md)

# 集成 — 概览

[← 文档索引](../README.md)

`not-a-bot` 的接入方式是一条命令加一份 skill：Agent 执行命令、读 JSON，文档告诉它何时执行。没有服务要启动，也没有协议要协商。

## 分发渠道

| 渠道 | 命令 | 说明 |
| --- | --- | --- |
| npm | `npm install -g not-a-bot` | 拉取对应平台的二进制包，安装期不执行任何脚本 |
| Cargo | `cargo install not-a-bot --locked` | 源码构建；Rust 1.89+ 即可 |
| 发布包 | [Releases](https://github.com/w21180239/not-a-bot/releases/latest) | `not-a-bot-<target>.tar.gz`，二进制在根目录 |

npm 包本身只是壳：二进制放在按平台的包里（`not-a-bot-linux-x64`、`not-a-bot-darwin-arm64`、`not-a-bot-windows-x64` 等），以 optional 依赖声明，npm 会装对应平台那一个，安装期不跑任何脚本。`bin/not-a-bot.js` 负责 exec；离线环境可以用 `NOT_A_BOT_BINARY_PATH` 指向本地构建产物。

## 两份文档，覆盖所有 harness

| 文档 | 适用 |
| --- | --- |
| [`integrations/skill/not-a-bot/SKILL.md`](../../../integrations/skill/not-a-bot/SKILL.md) | 会加载 skill 的 harness：Claude Code、Codex、Gemini CLI、OpenCode |
| [`integrations/guidance/AGENTS.md`](../../../integrations/guidance/AGENTS.md) | 任何读取项目指令或规则文件的 harness |

24 家 harness 的名单、图标与各自的文件路径见 [02-supported-harnesses.md](02-supported-harnesses.md)；仓库内的安装清单与逐家页面见 [`integrations/README.md`](../../../integrations/README.md)。

| 页面 | 内容 |
| --- | --- |
| [02-supported-harnesses.md](02-supported-harnesses.md) | 图标网格、四条接入通道、每家的路径与命令、验证方法 |

## 用哪个入口

| 需求 | 命令 |
| --- | --- |
| 模型在对话中翻译 | `not-a-bot translate "…" -t <lang> --format json` |
| 人性化改写 AI 味文字 | `not-a-bot rewrite "…" --from <lang> --preset standard --preserve @code --format json` |
| 脚本、CI、pre-commit | 同一条 CLI，读 `text` |
| 批量 | `not-a-bot batch`（NDJSON 进、NDJSON 出） |
| 查能力 | `not-a-bot providers`、`not-a-bot languages`、`not-a-bot health` |

## Agent 环境里的凭据

从图形界面启动的 Agent（Cursor、VS Code）经常拿不到 shell 环境变量。把凭据写进 `~/.config/not-a-bot/credentials.toml`，任何进程都能读。只用免费网页接口时完全不需要凭据，装好二进制就能用。

## 导航

- ← [文档索引](../README.md)
- → [往返翻译](../rewrite/OVERVIEW.md)
