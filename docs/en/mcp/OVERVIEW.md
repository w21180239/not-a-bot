**English** · [简体中文](../../zh-cn/mcp/OVERVIEW.md)

# MCP — Overview

`translatex mcp` runs an MCP server over stdio. It uses the same config and credential files as the CLI, so anything configured for `translatex translate` is available to your agent without a second setup.

## Transport

- Framing: one JSON-RPC 2.0 message per line.
- stdio only: stdout carries protocol messages, stderr carries logs.
- Notifications (no `id`, or `notifications/*`) are never answered.

## Methods

| Method | Behaviour |
| --- | --- |
| `initialize` | Returns protocol version `2024-11-05`, `tools` capability, server name and version |
| `notifications/initialized` | Ignored |
| `ping` | Empty result |
| `tools/list` | Tool catalogue with object-shaped `inputSchema` |
| `tools/call` | Executes one tool |
| anything else | JSON-RPC error `-32601` |

## Client configuration

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

For a workspace-scoped binary, point `command` at the absolute path.

## Result shape

Tool results follow the MCP contract:

```json
{
  "content": [{ "type": "text", "text": "{\"text\":\"你好\"}" }],
  "structuredContent": { "text": "你好", "provider": "google" },
  "isError": false
}
```

On failure, `isError` is `true` and `structuredContent` carries the same `error` object the CLI prints. An agent can read `structuredContent` directly and skip parsing the text block.

## File index

| File | Scope |
| --- | --- |
| [01-tools.md](01-tools.md) | Tool schemas, arguments, return values, failure modes |

## Navigation

- ← [Documentation index](../README.md)
- → [Tool reference](01-tools.md)
