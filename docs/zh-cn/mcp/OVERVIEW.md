**简体中文** · [English](../../en/mcp/OVERVIEW.md)

# MCP — 概览

`translatex mcp` 通过 stdio 运行 MCP 服务。它读取与 CLI 相同的配置和凭据文件，CLI 配好的东西 Agent 直接可用，无需二次配置。

## 传输

- 帧格式：一行一个 JSON-RPC 2.0 消息。
- 仅 stdio：stdout 只放协议消息，stderr 放日志。
- 通知（无 `id`，或 `notifications/*`）永不回复。

## 方法

| 方法 | 行为 |
| --- | --- |
| `initialize` | 返回协议版本 `2024-11-05`、`tools` 能力、服务名与版本 |
| `notifications/initialized` | 忽略 |
| `ping` | 空结果 |
| `tools/list` | 工具清单，`inputSchema` 为对象 |
| `tools/call` | 执行一个工具 |
| 其他 | JSON-RPC 错误 `-32601` |

## 客户端配置

```json
{
  "mcpServers": {
    "translatex": {
      "command": "translatex",
      "args": ["mcp"]
    }
  }
}
```

若二进制不在 PATH 上，把 `command` 换成绝对路径。

## 结果结构

工具结果遵循 MCP 契约：

```json
{
  "content": [{ "type": "text", "text": "{\"text\":\"你好\"}" }],
  "structuredContent": { "text": "你好", "provider": "google" },
  "isError": false
}
```

失败时 `isError` 为 `true`，`structuredContent` 携带与 CLI 相同的 `error` 对象。Agent 可以直接读 `structuredContent`，不必解析文本块。

## 文件索引

| 文件 | 范围 |
| --- | --- |
| [01-tools.md](01-tools.md) | 工具 schema、参数、返回值、失败模式 |

## 导航

- ← [文档索引](../README.md)
- → [工具参考](01-tools.md)
