# 变更记录

文档修订历史。1.0 之前，应用变更也记录在此。

## 0.2.0 — 2026-11-11

### 新增

- `roundtrip` 支持 N 跳链。`--chain "<[engine:]lang,...>"` 接受任意顺序的引擎和语言，末尾没回到原语言时自动补一跳；`--depth 1-3` 只用机器翻译，依次经过 zh、ja、fi（中文原文为 en、ja、fi）；`--preset <name>` 选择 `mt`、`mt-deep`、`llm`、`lynote` 或 `lynote-fix`。优先级：chain、preset、depth、pivot，最后是配置里的 `roundtrip_preset`。
- 两个调用本机 agent CLI、不需要 API key 的 Provider：`claude-cli`（`claude -p`，用你的 Claude 订阅，默认模型 haiku）和 `opencode-cli`（`opencode run`，用你的 OpenCode 额度，默认模型 `opencode-go/deepseek-v4.1-flash`）。在 `[provider.claude-cli]`、`[provider.opencode-cli]` 下配置 `model` 和 `bin`。它们只在被点名时运行，`auto` 从不选它们。
- 配置项 `roundtrip_preset`（默认 `mt`）、`roundtrip_llm`（默认 `opencode-cli`，可选 `claude-cli`），以及环境变量 `TRANSLATEX_ROUNDTRIP_PRESET`、`TRANSLATEX_ROUNDTRIP_LLM`。
- 往返里的 LLM 跳会收到原文，在改写之前先用最小的改动修复前面机器翻译跳丢掉的语义。
- 修复跳 `<llm>:fix`：LLM 返回 JSON 小编辑列表，translatex 只应用精确且很小的编辑。补回缺失事实的编辑一律应用，其余编辑共用一份小预算，所以这一跳无法把原文抄回来。每条编辑都记录在该跳的 `edits` 里，带 `kind`（`fact` 或 `meaning`）、`applied` 和 `reason`。
- 事实检查：每次往返都报告 `missingFacts`（结果里缺失的人名、数字、ID、代码片段和 URL；数字按数位比较，人名按整词比较），列表非空时加一条 `fact check:` 警告。
- 响应字段 `chain`、`hops`、`missingFacts`、`warnings`。
- 具名保护模式 `@code`（行内和围栏代码）与 `@url`。在 Windows 上，npm 的 `.cmd` 外壳经 cmd.exe 运行；Python 或 Node 不经 shell 启动 translatex 时，cmd.exe 会吞掉 `` `[^`]*` `` 这类正则里的 `^`，具名模式避开这个问题。
- 链里点名的机器翻译引擎是"优先"而非"钉死"：遇到可重试错误，这一跳改用另一个引擎，并避开相邻两跳的引擎，同时加警告。LLM 跳从不回退。
- `benches/roundtrip-depth/`：10 段文本的语料和 `measure.py`，实测结果写进往返文档（10 段文本，只跑一次，2026-11-11）。

### 调整（破坏性）

- `roundtrip` 响应移除 `pivotLang`、`pivotText`、`providers`。改读 `chain`、`hops[].text` 和 `hops[].provider`。

### 调整

- 默认往返是两跳机器翻译的 `mt`（例如 en 到 zh 到 ja 再到 en），此前是一跳。在语料上，剩余 AI 腔从 10 段里的 7 段降到 5 段，事实丢失相同。
- LLM 跳原样接收受保护片段，提示词里列出这些片段；片段丢了会加 `preserve:` 警告。机器翻译跳仍然用占位符遮蔽。
- Provider 总数为 28：14 个免费网页端点、9 个官方 API、5 个 LLM Provider。

### 修复

- `roundtrip/01-pivot-choice.md` 不再说测量脚本"在仓库历史里"，改为指向 `benches/roundtrip-depth/measure.py`。
- Skill 里写过往返无法去掉 "not just X but Y"。这只对机器翻译预设成立，LLM 预设可以去掉。

## 0.1.2 — 2026-11-04

### 修复

- Windows 平台包以 `translatex-windows-x64` 发布：`translatex-win32-x64` 每次都会触发 npm 的垃圾过滤并返回 403，重试无效。shim 里把 `win32` 映射到这个包名。
- npm 发布可以重跑了：某个版本已在 registry 上就跳过，发布中断后重跑会补齐剩下的，而不是对已发出的包清一色返回 409。

## 0.1.1 — 2026-11-04

### 调整

- npm 分发改为按平台的二进制包（`translatex-windows-x64`、`translatex-linux-arm64` 等）作为 optional 依赖，安装期不再执行任何脚本 —— npm 12 默认禁止依赖执行安装脚本，原来的 `postinstall` 下载方式会装出空壳。
- 声明的最低 Rust 版本改为 1.89（依赖图里最高的 `rust-version`），原来的 1.80 实际从未编译通过。
- CI 覆盖 `cargo fmt --check`、`clippy -D warnings`、三大平台测试、MSRV 构建与 npm 打包检查；Dependabot 跟踪 actions、Cargo 与 npm 清单。

### 修复

- `cargo clippy --all-targets -- -D warnings` 已清零，此前 `preserve`、`google`、`modernmt`、`volcengine`、`providers` 与引擎测试中共有 7 条告警。
- 仓库声明 `MIT OR Apache-2.0` 却没有许可证正文，现补上 `LICENSE-MIT` 与 `LICENSE-APACHE`。

## 0.1.0 — 2026-11-04

### 新增

- 中英文完整文档树：概览、安装、CLI、Provider、API、环境变量、架构、故障排查、往返翻译、集成。
- 根 README 覆盖中、英、俄三种语言，首屏按收益与读者会搜的词来写，Provider 链路放到后面解释。
- 两套测试：`tests/contract`（线格式与 Provider 目录不变量）、`tests/integration`（对 mock 服务器的 Provider 往返）。合计 245 个。
- Provider 并发竞速：`race_width`（默认 3）让最多三个尝试同时在飞，第一个成功者胜出，失败的空位由链中下一个补上。`--race N` 与 `race_width = 1` 恢复严格串行。
- `translatex roundtrip`（别名 `polish`）：经中转语言翻出再翻回，去掉机器味。默认中转为非中文用中文、中文用英文，经实测选定；`roundtrip_pivot`（配置）、`TRANSLATEX_ROUNDTRIP_PIVOT` 与 `--pivot` 可覆盖。
- 失败响应带上 `attempts` 数组，`CHAIN_EXHAUSTED` 不用重跑就能看到每一跳试了什么。
- npm 包（`npm install -g translatex`）按平台下载预编译二进制、覆盖五个目标的 tag 发布流水线，以及 Claude Code、Codex CLI、Gemini CLI、OpenCode、Cursor、Cline、Roo Code、Kilo Code、Continue、JetBrains AI、Zed、Aider、Qwen Code、Crush、Goose、DeepSeek Harness、MiMo Code、AtomCode、OpenClaw、Hermes Agent、Pi、Reasonix、ZCode、QwenPaw 的 CLI + skill 接入文档。

### 调整

- 所有示例统一为 camelCase（`targetLang`、`retryAfterSecs`、`suggestedAction`）。
- 冷却状态路径按代码实际情况写为 `~/.config/translatex/cache/`。
- 错误表补充 `PROVIDER_NOT_CONFIGURED` 与 `AUTO_DETECT_UNSUPPORTED`。
- `volcengine` 与 `papago` 只对外声明规范语言码；接口方言（`zh-Hant`、`zh-CN`）留在适配器内部。
- `reverso` 从自动链中排除：它在 TLS 握手阶段就失败，自动请求每次都白付一次尝试。显式指定或写进 `--fallback` 仍会尝试。
- Agent 接入统一为 CLI + skill；MCP 服务在 0.1.0 周期内移除。

### 修复

- `config.toml` 与 `TRANSLATEX_CHAIN` 中的 `chain` 现在生效；此前引擎始终使用编译期默认链。
- `--provider X --fallback A,B` 现在会真正回退到 fallback 列表。
- 保护片段打码不再把字节偏移当字符偏移，此前会破坏中文输入甚至 panic。
- 无 Key 的 Provider 不再进入冷却而从链中悄悄消失。

### 移除

- 未使用的 `iflytek` 适配器模块。该名字仍是 `KNOWN_PROVIDERS` 中指向 `iflyrec` 的别名，旧配置继续可用。
