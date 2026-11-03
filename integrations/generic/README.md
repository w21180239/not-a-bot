# Anything else

`translatex mcp` speaks MCP 2024-11-05 over stdio: one JSON-RPC 2.0 object per
line on stdout, logs on stderr, nothing else.

## Handshake

```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"client","version":"0"}}}
{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
```

The server answers `initialize` with `protocolVersion`, `capabilities.tools` and
`serverInfo`; notifications are never answered.

## Call a tool

```json
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"translate","arguments":{"text":"Hello world","targetLang":"zh"}}}
```

The result follows the MCP tool contract:

```json
{
  "content": [{ "type": "text", "text": "{\"text\":\"你好世界\", …}" }],
  "structuredContent": { "text": "你好世界", "provider": "google", … },
  "isError": false
}
```

Read `structuredContent`. On failure `isError` is `true` and
`structuredContent.error` carries the typed error.

## Without MCP

The CLI is the same engine with a simpler interface:

```bash
translatex translate "Hello world" -t zh --format json
translatex schema --format json     # JSON Schema for every wire type
translatex manifest --format json   # tool catalogue for discovery
```

## Tool catalogue

| Tool | Arguments | Returns |
| --- | --- | --- |
| `translate` | `TranslateRequest` | `TranslateResponse` |
| `roundtrip` | `RoundtripRequest` | `RoundtripResponse` |
| `providers` | `{}` | `{providers: ProviderInfo[]}` |
| `languages` | `{provider?: string}` | `{provider, languages: string[]}` |
| `health` | `{}` | `{healthy, providers: HealthRecord[]}` |

Generate the authoritative schemas from the binary rather than copying them:

```bash
translatex schema --format json
```
