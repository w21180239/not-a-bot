**简体中文** · [English](../../en/mcp/01-tools.md)

# MCP — 工具参考

## `translate`

入参 schema 即生成的 `TranslateRequest`：

| 参数 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `text` | string | 是 | 空文本返回 `INVALID_INPUT` |
| `targetLang` | string | 是 | 规范语言码 |
| `sourceLang` | string | 否 | 默认 `auto` |
| `provider` | string | 否 | 指定一个 Provider |
| `fallbackChain` | string[] | 否 | 指定 Provider 之后尝试 |
| `preserve` | string[] | 否 | 原样保留的正则片段 |
| `timeoutMs` | integer | 否 | 整体调用预算 |
| `idempotencyKey` | string | 否 | 作为 `requestId` 回显 |

返回 `TranslateResponse`：

```json
{
  "text": "你好，世界",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "…",
  "chars": 11
}
```

失败时置 `isError`，`error` 载荷遵循[错误契约](../api/02-errors.md)。

## `providers`

无参数。返回与 `translatex providers --format json` 相同的目录，仅限本机已注册的 Provider。

```json
{
  "providers": [
    {
      "name": "google",
      "displayName": "Google Translate (web)",
      "kind": "web",
      "priority": 1,
      "needsKey": false,
      "maxChars": 5000,
      "autoDetect": true,
      "languages": ["*"],
      "configured": true
    }
  ]
}
```

## `languages`

| 参数 | 类型 | 默认 | 说明 |
| --- | --- | --- | --- |
| `provider` | string | `all` | Provider id 或 `all` |

Provider 语言集合里的 `*` 展开为全部规范语言码。未知 id 返回 `PROVIDER_NOT_FOUND`；未配置返回 `PROVIDER_NOT_CONFIGURED`。

## `health`

无参数。用真实接口探测每个已注册 Provider：

```json
{
  "healthy": false,
  "providers": [
    { "provider": "google", "configured": true, "healthy": true, "latencyMs": 380 },
    { "provider": "sogou", "configured": true, "healthy": false, "error": "rate limited by sogou" }
  ]
}
```

任一已配置 Provider 失败时 `healthy` 为 false。无 Key 的 Provider 永远是 `configured: true`；有 Key 但缺凭据的报 `configured: false`，不会被探测。

## 给工具作者的说明

- 优先读 `structuredContent`；`text` 块是同一份 JSON，供只渲染文本的客户端使用。
- 会话期间 `tools/list` 不变，`listChanged` 为 `false`。
- 服务启动时只构建一次引擎并读取凭据文件，新增 Key 后需重启服务。

## 导航

- ← [MCP 概览](OVERVIEW.md)
- → [错误契约](../api/02-errors.md)
