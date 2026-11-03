# 变更记录

文档修订历史。1.0 之前，应用变更也记录在此。

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
