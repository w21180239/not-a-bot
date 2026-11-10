**简体中文** · [English](../../en/integrations/02-supported-harnesses.md)

# 支持的 harness

[← 集成](OVERVIEW.md) · [文档索引](../README.md)

harness 指承载 Agent 的宿主：终端里的编码 Agent、编辑器助手，或能执行命令的聊天客户端。`translatex` 用两份文档覆盖 24 家：会加载 skill 的宿主拿到 `SKILL.md`，读项目规则的宿主拿到一段指令。每家都能直接调用 CLI，那才是真正的接入点。

<div align="center">
<table>
<tr>
<td align="center" width="150"><img src="../../../repo-assets/agents/claude-code.png" width="44" alt="Claude Code"><br/><b>Claude Code</b><br/><sub>skill · plugin</sub></td>
<td align="center" width="150"><img src="../../../repo-assets/agents/codex.png" width="44" alt="Codex CLI"><br/><b>Codex CLI</b><br/><sub>skill · AGENTS.md</sub></td>
<td align="center" width="150"><img src="../../../repo-assets/agents/gemini-cli.png" width="44" alt="Gemini CLI"><br/><b>Gemini CLI</b><br/><sub>skill · GEMINI.md</sub></td>
<td align="center" width="150"><img src="../../../repo-assets/agents/opencode.png" width="44" alt="OpenCode"><br/><b>OpenCode</b><br/><sub>skill · AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../../../repo-assets/agents/cursor.png" width="44" alt="Cursor"><br/><b>Cursor</b><br/><sub>.cursor/rules</sub></td>
<td align="center"><img src="../../../repo-assets/agents/cline.png" width="44" alt="Cline"><br/><b>Cline</b><br/><sub>.clinerules</sub></td>
<td align="center"><img src="../../../repo-assets/agents/roo-code.png" width="44" alt="Roo Code"><br/><b>Roo Code</b><br/><sub>.roo/rules</sub></td>
<td align="center"><img src="../../../repo-assets/agents/kilo-code.png" width="44" alt="Kilo Code"><br/><b>Kilo Code</b><br/><sub>.kilocode/rules</sub></td>
</tr>
<tr>
<td align="center"><img src="../../../repo-assets/agents/continue.png" width="44" alt="Continue"><br/><b>Continue</b><br/><sub>.continue/rules</sub></td>
<td align="center"><img src="../../../repo-assets/agents/jetbrains.png" width="44" alt="JetBrains AI"><br/><b>JetBrains AI</b><br/><sub>.aiassistant/rules</sub></td>
<td align="center"><img src="../../../repo-assets/agents/zed.png" width="44" alt="Zed"><br/><b>Zed</b><br/><sub>.rules</sub></td>
<td align="center"><img src="../../../repo-assets/agents/aider.png" width="44" alt="Aider"><br/><b>Aider</b><br/><sub>CONVENTIONS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../../../repo-assets/agents/qwen-code.png" width="44" alt="Qwen Code"><br/><b>Qwen Code</b><br/><sub>QWEN.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/crush.png" width="44" alt="Crush"><br/><b>Crush</b><br/><sub>CRUSH.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/goose.png" width="44" alt="Goose"><br/><b>Goose</b><br/><sub>instructions</sub></td>
<td align="center"><img src="../../../repo-assets/agents/deepseek-harness.png" width="44" alt="DeepSeek Harness"><br/><b>DeepSeek Harness</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../../../repo-assets/agents/mimo-code.svg" width="44" alt="MiMo Code"><br/><b>MiMo Code</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/atomcode.png" width="44" alt="AtomCode"><br/><b>AtomCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/openclaw.png" width="44" alt="OpenClaw"><br/><b>OpenClaw</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/hermes-agent.png" width="44" alt="Hermes Agent"><br/><b>Hermes Agent</b><br/><sub>AGENTS.md</sub></td>
</tr>
<tr>
<td align="center"><img src="../../../repo-assets/agents/pi.png" width="44" alt="Pi"><br/><b>Pi</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/reasonix.png" width="44" alt="Reasonix"><br/><b>Reasonix</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/zcode.png" width="44" alt="ZCode"><br/><b>ZCode</b><br/><sub>AGENTS.md</sub></td>
<td align="center"><img src="../../../repo-assets/agents/qwenpaw.svg" width="44" alt="QwenPaw"><br/><b>QwenPaw</b><br/><sub>instructions</sub></td>
</tr>
</table>
</div>

## 四条接入通道

| 通道 | 宿主读什么 | 何时加载 | 覆盖的 harness |
| --- | --- | --- | --- |
| Skill | skills 目录里的 `SKILL.md` | 按需：description 匹配任务时才加载正文 | Claude Code、Codex CLI、Gemini CLI、OpenCode |
| 规则 | 项目规则目录里的 Markdown | 随项目，通常常驻或匹配时加载 | Cursor、Cline、Roo Code、Kilo Code、Continue、JetBrains AI、Zed |
| 指令 | 项目指令文件，追加写入 | 整个会话常驻 | Aider、Qwen Code、Crush、Goose、DeepSeek Harness、MiMo Code、AtomCode、OpenClaw、Hermes Agent、Pi、Reasonix、ZCode、QwenPaw |
| CLI | 不读文档 | — | 任何能执行 shell 命令的宿主 |

宿主支持 skill 时优先用 skill：只有 description 常驻上下文，正文按需加载，不做翻译的会话大约只花 90 个 token，而不是一整页。

## 路径与命令

路径相对于你要让 Agent 工作的项目；源文件来自 `translatex` 仓库。

| Harness | 通道 | 写到 | 命令 |
| --- | --- | --- | --- |
| Claude Code | Skill | `.claude/skills/translatex/SKILL.md` | `mkdir -p .claude/skills/translatex && cp integrations/skill/translatex/SKILL.md .claude/skills/translatex/` |
| Claude Code | Skill（个人级） | `~/.claude/skills/translatex/SKILL.md` | 同上，放到 `$HOME` 下 |
| Claude Code | 插件 | 由 CLI 安装 | `/plugin marketplace add w21180239/translatex`，再 `/plugin install translatex@translatex` |
| Codex CLI | Skill | `~/.agents/skills/translatex/SKILL.md` | `mkdir -p ~/.agents/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.agents/skills/translatex/` |
| Codex CLI | 指令 | `AGENTS.md`、`~/.codex/AGENTS.md` | `cat integrations/guidance/AGENTS.md >> AGENTS.md` |
| Gemini CLI | Skill | `~/.agents/skills/translatex/SKILL.md` | 与 Codex 共用 |
| Gemini CLI | 指令 | `GEMINI.md` | `cat integrations/guidance/AGENTS.md >> GEMINI.md` |
| OpenCode | Skill | `~/.config/opencode/skills/translatex/SKILL.md` | `mkdir -p ~/.config/opencode/skills/translatex && cp integrations/skill/translatex/SKILL.md ~/.config/opencode/skills/translatex/` |
| OpenCode | 指令 | `AGENTS.md` | `cat integrations/guidance/AGENTS.md >> AGENTS.md` |
| Cursor | 规则 | `.cursor/rules/translatex.mdc` | `mkdir -p .cursor/rules && cp integrations/cursor/translatex.mdc .cursor/rules/` |
| Cline | 规则 | `.clinerules/translatex.md` | `mkdir -p .clinerules && cat integrations/guidance/AGENTS.md > .clinerules/translatex.md` |
| Roo Code | 规则 | `.roo/rules/translatex.md` | `mkdir -p .roo/rules && cat integrations/guidance/AGENTS.md > .roo/rules/translatex.md` |
| Kilo Code | 规则 | `.kilocode/rules/translatex.md` | `mkdir -p .kilocode/rules && cat integrations/guidance/AGENTS.md > .kilocode/rules/translatex.md` |
| Continue | 规则 | `.continue/rules/translatex.md` | `mkdir -p .continue/rules && cat integrations/guidance/AGENTS.md > .continue/rules/translatex.md` |
| JetBrains AI | 规则 | `.aiassistant/rules/translatex.md` | `mkdir -p .aiassistant/rules && cat integrations/guidance/AGENTS.md > .aiassistant/rules/translatex.md` |
| Zed | 规则 | `.rules` | `cat integrations/guidance/AGENTS.md >> .rules` |
| Aider | 指令 | `CONVENTIONS.md` | `cat integrations/guidance/AGENTS.md >> CONVENTIONS.md`，再 `aider --read CONVENTIONS.md` |
| Qwen Code | 指令 | `QWEN.md` | `cat integrations/guidance/AGENTS.md >> QWEN.md` |
| Crush | 指令 | `CRUSH.md` | `cat integrations/guidance/AGENTS.md >> CRUSH.md` |
| Goose | 指令 | 项目指令文件 | 粘贴该段 |
| DeepSeek Harness | 指令 | `AGENTS.md` | `cat integrations/guidance/AGENTS.md >> AGENTS.md` |
| MiMo Code | 指令 | `AGENTS.md` | 同上 |
| AtomCode | 指令 | `AGENTS.md` | 同上 |
| OpenClaw | 指令 | `AGENTS.md` | 同上 |
| Hermes Agent | 指令 | `AGENTS.md` | 同上 |
| Pi | 指令 | `AGENTS.md` | 同上 |
| Reasonix | 指令 | `AGENTS.md` | 同上 |
| ZCode | 指令 | `AGENTS.md` | 同上 |
| QwenPaw | 指令 | 项目指令文件 | 粘贴该段 |
| 其它 | CLI | — | Agent 直接执行 `translatex … --format json` |

Windows PowerShell：`New-Item -ItemType Directory -Force <dir>`，再用 `Get-Content <src> | Add-Content <dst>`。

## 文档里写了什么

两份文档都只讲四条规则，其它都不需要：

1. 调用 `translatex`，不要自己翻译。
2. 读 JSON 里的 `text`；`provider` 与 `detectedSource` 由工具给出。
3. 代码、路径、标识符、URL 走 `--preserve`。
4. 处理 AI 味文字用 `roundtrip --preset llm --preserve @code`，LLM 跳会重建句子；机器翻译预设保留句子，用它们之前要自己先删 AI 套话。检查 `missingFacts` 和 `warnings`，并如实报告。

完整 skill 还包括错误表和重试规则：[`integrations/skill/translatex/SKILL.md`](../../../integrations/skill/translatex/SKILL.md)。

## 验证接入

让 harness 翻一句，然后看回复里有没有 provider 名：

```bash
translatex translate "Hello world" -t zh --format json
```

有译文但没有 `provider` 字段，说明是模型自己翻的，文档没被读到。确认上面的路径、确认 harness 所在 shell 里 `translatex --version` 能跑，然后重启会话让它重新读取。

图形界面宿主（Cursor、VS Code）启动时的 `PATH` 常和终端不同。二进制装在 `~/.cargo/bin` 下时，把该目录加进宿主的运行环境，或者改用 npm 安装，让 shim 落在全局路径里。

## 导航

- ← [集成](OVERVIEW.md)
- → [往返翻译](../roundtrip/OVERVIEW.md)
- ↑ [文档索引](../README.md)
