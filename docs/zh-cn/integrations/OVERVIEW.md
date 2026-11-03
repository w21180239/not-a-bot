**简体中文** · [English](../../en/integrations/OVERVIEW.md)

# 集成 — 概览

[← 文档索引](../README.md)

`translatex` 的接入方式是一条命令加一份 skill：Agent 执行命令、读 JSON，文档告诉它何时执行。没有服务要启动，也没有协议要协商。

## 分发渠道

| 渠道 | 命令 | 说明 |
| --- | --- | --- |
| npm | `npm install -g translatex` | 拉取对应平台的二进制包，安装期不执行任何脚本 |
| Cargo | `cargo install translatex --locked` | 源码构建；Rust 1.89+ 即可 |
| 发布包 | [Releases](https://github.com/w21180239/translatex/releases/latest) | `translatex-<target>.tar.gz`，二进制在根目录 |

npm 包本身只是壳：二进制放在按平台的包里（`translatex-linux-x64`、`translatex-darwin-arm64`、`translatex-windows-x64` 等），以 optional 依赖声明，npm 会装对应平台那一个，安装期不跑任何脚本。`bin/translatex.js` 负责 exec；离线环境可以用 `TRANSLATEX_BINARY_PATH` 指向本地构建产物。

## 两份文档，覆盖所有 harness

| 文档 | 适用 |
| --- | --- |
| [`integrations/skill/translatex/SKILL.md`](../../../integrations/skill/translatex/SKILL.md) | 会加载 skill 的 harness：Claude Code、Codex、Gemini CLI、OpenCode |
| [`integrations/guidance/AGENTS.md`](../../../integrations/guidance/AGENTS.md) | 任何读取项目指令或规则文件的 harness |

24 家 harness 的名单、图标与各自的文件路径见 [02-supported-harnesses.md](02-supported-harnesses.md)；仓库内的安装清单与逐家页面见 [`integrations/README.md`](../../../integrations/README.md)。

| 页面 | 内容 |
| --- | --- |
| [02-supported-harnesses.md](02-supported-harnesses.md) | 图标网格、四条接入通道、每家的路径与命令、验证方法 |

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
