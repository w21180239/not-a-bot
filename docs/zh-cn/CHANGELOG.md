# 变更记录

文档修订历史。1.0 之前，应用变更也记录在此。

## 0.1.0 — 未发布

not-a-bot 以新名字延续 translatex 0.3.0。除默认预设外，行为不变。

### 新增

- `lint` 子命令（`nab lint DRAFT [--source FILE] [--strict]`）：离线检查 AI 味（词汇、短语、结构、拼写、标点），加 `--source` 时比对草稿丢失或新增的事实。移植自 humanizer 的 `lint.py`；JSON 输出对应新增的 schema 类型 `LintResponse`。

### 变更

- 二进制名为 `not-a-bot`，另有行为一致的短别名 `nab`。crate 为 `not-a-bot`（库名 `not_a_bot`），npm 包为 `not-a-bot`（平台包 `not-a-bot-<os>-<arch>`），两个 npm bin 共用同一个启动器。
- 子命令 `roundtrip` 改为 `rewrite`，默认预设改为 `standard`（LLM 出去再回来，再做事实修复）；`light` 仍可用 `--preset light` 选择。schema 类型 `RoundtripRequest`、`RoundtripResponse` 改为 `RewriteRequest`、`RewriteResponse`。
- 配置项 `roundtrip_preset`、`roundtrip_pivot`、`roundtrip_llm`、`roundtrip_scorer`、`roundtrip_ladder` 改为 `rewrite_*`。
- 环境变量前缀由 `TRANSLATEX_` 改为 `NOT_A_BOT_`（如 `NOT_A_BOT_REWRITE_PRESET`、`NOT_A_BOT_CONFIG`、`NOT_A_BOT_HOME`）。
- 配置和缓存目录改为 `~/.config/not-a-bot`。旧名字、旧变量和旧目录不再读取。
- Provider 元数据键改用 `not-a-bot.` 前缀（`not-a-bot.style`、`not-a-bot.history`、`not-a-bot.original`），HTTP User-Agent 为 `not-a-bot/<version>`。
- 集成 skill `translatex` 改名为 `not-a-bot`。
- 预设：`standard`（默认）为 LLM 出去、LLM 回来，再加一次只修事实的修复跳（`<llm>:fix`）；`strong` 为 LLM 出去再回来、不带修复（改写最远、检测器分数最低，可能丢事实，一定列在 `missingFacts`）；`light` 不变。`standard`、`strong` 遇到非 LLM 的 `rewrite_llm` 返回 `INVALID_INPUT`。人性化跳会删掉只表达感受、承诺或客套的句子。
- 修复跳（`engine:fix`）改为对照原文只修硬事实（名字、地点、产品、数字、日期、金额、币种、ID、版本、路径、代码片段、否定、限定词），会拿到事实检查列出的缺失片段，且从不恢复被删的感受或承诺句。此前修复的是语义。
- 从源码安装用 `cargo install --git https://github.com/w21180239/not-a-bot`；该 crate 没有发布到 crates.io。

### 新增

- 新增：not-a-bot agent skill，把写作方法（事实不变量、受众 profile、AI 腔对照表，可由 `~/.config/not-a-bot/profiles/` 覆盖的 profile）与 CLI 使用指南合并为一份；评测工具和检测器放在 `benches/skill-evals` 与 `benches/detector`。

### 移除

- 俄文 README 和 `docs/ru`。

## translatex 历史

以下条目早于改名，描述的是 translatex，包括它的 `roundtrip` 命令和 `TRANSLATEX_*` 变量。

## 未发布

### 变更

- LLM 去 AI 味改写会删掉只表达情绪或表态的句子（感谢、兴奋、自豪、"we remain committed"）和结尾客套，句中的名字、地名、数字、日期、金额先挪进保留的句子。在 `benches/prompt-lab` 的 30 段英文上，默认 opencode 模型的 desklib 平均分从 0.530 降到 0.410。

## 0.3.0 — 2026-11-11

### 新增

- `roundtrip --scorer "<command>"`（配置项 `roundtrip_scorer`）：外部检测器从 stdin 读取每个结果，在 stdout 最后一行输出 0 到 1 的分数（越高越像 AI）。不经过 shell，受 `--timeout-ms` 限制；打分器失败时加一条警告，该次运行记为未打分。
- `roundtrip --best-of 1-5`：把链跑 N 次，先选没有丢事实的（都丢了就选丢得最少的），再选分数最低的。N 大于 1 时需要打分器。
- `roundtrip --target-score 0-1`：先跑请求的链，再依次尝试 `roundtrip_ladder`（默认 `standard,strong`）里的预设，直到最好的运行分数不高于目标；否则返回最好的运行，并加一条 `target score not met` 警告。
- 响应字段 `score`、`scorer`（只有程序名）和 `candidates`（每次运行一条：`presetOrChain`、`score`、`missingFacts` 个数）。请求字段 `scorer`、`bestOf`、`targetScore`。
- `benches/roundtrip-depth/measure.py --scorer CMD` 增加 "detector src" 和 "detector out" 两列，以及 `standard-best3`、`standard-target` 两个配置。

### 调整（破坏性）

- 往返预设按强度重新命名：`mt` 改为 `light`（两跳机器翻译，默认），`llm` 改为 `standard`（LLM 出去再回来），以修复跳收尾的那个预设改为 `strong`（LLM 跳到 zh 和 ja，再经机器翻译跳过 fi 回来，最后一跳修复）。旧名字会被拒绝，返回 `INVALID_INPUT` 并列出 `light, standard, strong`，不保留别名。
- 配置项 `roundtrip_preset` 默认 `light`，`roundtrip_ladder` 默认 `["standard", "strong"]`。配置文件或 `TRANSLATEX_ROUNDTRIP_PRESET` 里写着旧名字的要改掉。

### 移除

- 三跳机器翻译预设 `mt-deep`，以及不带修复跳、直接运行 strong 链的预设。基准里第三跳机器翻译比两跳丢更多事实，没有修复跳的链丢 3 到 5 条事实，加修复跳后只丢 1 条。`--depth 1-3` 和 `--chain` 仍可手动搭出这类链。

### 调整

- `benches/roundtrip-depth/measure.py` 的配置名改用新预设名（`light-d1`、`light`、`standard-claude`、`standard-opencode`、`strong-claude`、`strong-opencode`），并新增 `--langs`（默认 `en`）按语言过滤语料行。`results.jsonl` 里用旧名字写下的行仍可读取，不做迁移。

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
