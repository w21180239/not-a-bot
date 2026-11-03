# 变更记录

文档修订历史。1.0 之前，应用变更也记录在此。

## 0.1.0 — 2026-11-04

### 新增

- 中英文完整文档树：概览、安装、CLI、Provider、API、MCP、环境变量、架构、故障排查。
- 根 README 按“网页优先”架构重写，覆盖中、英、俄三种语言。
- 三套测试：`tests/contract`（线格式与 Provider 目录不变量）、`tests/integration`（对 mock 服务器的 Provider 往返）、`tests/mcp`（对真实二进制的 stdio JSON-RPC）。合计 243 个。

### 调整

- 所有示例统一为 camelCase（`targetLang`、`retryAfterSecs`、`suggestedAction`）。
- 冷却状态路径按代码实际情况写为 `~/.config/translatex/cache/`。
- 错误表补充 `PROVIDER_NOT_CONFIGURED` 与 `AUTO_DETECT_UNSUPPORTED`。
- `volcengine` 与 `papago` 只对外声明规范语言码；接口方言（`zh-Hant`、`zh-CN`）留在适配器内部。
- `reverso` 从自动链中排除：它在 TLS 握手阶段就失败，自动请求每次都白付一次尝试。显式指定或写进 `--fallback` 仍会尝试。

### 修复

- `config.toml` 与 `TRANSLATEX_CHAIN` 中的 `chain` 现在生效；此前引擎始终使用编译期默认链。
- `--provider X --fallback A,B` 现在会真正降级到 fallback 列表。
- MCP `tools/list` 输出对象 schema，`tools/call` 返回带 `isError` 的 `content`。
- MCP 服务读取真实配置与凭据，并只构建一次引擎。
- 保护片段打码不再把字节偏移当字符偏移，此前会破坏中文输入甚至 panic。
- 无 Key 的 Provider 不再进入冷却而从链中悄悄消失。

### 移除

- 未使用的 `iflytek` 适配器模块。该名字仍是 `KNOWN_PROVIDERS` 中指向 `iflyrec` 的别名，旧配置继续可用。
