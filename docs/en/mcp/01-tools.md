**English** · [简体中文](../../zh-cn/mcp/01-tools.md)

# MCP — Tool reference

## `translate`

Input schema is `TranslateRequest` from the generated JSON Schema:

| Argument | Type | Required | Notes |
| --- | --- | --- | --- |
| `text` | string | yes | Empty text is `INVALID_INPUT` |
| `targetLang` | string | yes | Canonical language code |
| `sourceLang` | string | no | Defaults to `auto` |
| `provider` | string | no | Pin one provider |
| `fallbackChain` | string[] | no | Tried after the pinned provider |
| `preserve` | string[] | no | Regex spans to keep verbatim |
| `timeoutMs` | integer | no | Whole-call budget |
| `idempotencyKey` | string | no | Echoed back as `requestId` |

Returns `TranslateResponse`:

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

Failure sets `isError` with an `error` payload that follows [the error contract](../api/02-errors.md).

## `roundtrip`

Input is `RoundtripRequest`:

| Argument | Type | Required | Notes |
| --- | --- | --- | --- |
| `text` | string | yes | Empty text is `INVALID_INPUT` |
| `sourceLang` | string | no | Defaults to `auto` |
| `pivot` | string | no | Bridge language; `auto` picks the recommended pair |
| `provider` | string | no | Pin one provider for both hops |
| `fallbackChain` | string[] | no | Tried after the pinned provider |
| `preserve` | string[] | no | Regex spans kept verbatim |
| `timeoutMs` | integer | no | Per-hop budget |
| `idempotencyKey` | string | no | Echoed back as `requestId` |

Returns `RoundtripResponse`:

```json
{
  "text": "…",
  "detectedSource": "en",
  "pivotLang": "zh",
  "pivotText": "…",
  "provider": "google",
  "providers": ["google", "edge"],
  "latencyMs": 612,
  "requestId": "…",
  "chars": 126
}
```

Two provider calls, sometimes three for Chinese input under `pivot: "auto"`. Use
it for prose that reads machine-generated; do not use it on text where numbers,
names or exact wording must survive. See [roundtrip/OVERVIEW.md](../roundtrip/OVERVIEW.md).

## `providers`

No arguments. Returns the same catalogue as `translatex providers --format json`, filtered to what is registered on this machine.

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

| Argument | Type | Default | Notes |
| --- | --- | --- | --- |
| `provider` | string | `all` | Provider id or `all` |

`*` in a provider's `languages` expands to the canonical list. Unknown ids return `PROVIDER_NOT_FOUND`; unconfigured ones return `PROVIDER_NOT_CONFIGURED`.

## `health`

No arguments. Probes every registered provider with its real endpoint:

```json
{
  "healthy": false,
  "providers": [
    { "provider": "google", "configured": true, "healthy": true, "latencyMs": 380 },
    { "provider": "sogou", "configured": true, "healthy": false, "error": "rate limited by sogou" }
  ]
}
```

`healthy` is false when any configured provider fails. Keyless providers are always `configured: true`; keyed ones report `configured: false` without credentials and are not probed.

## Notes for tool authors

- Prefer `structuredContent`; the `text` block is the same JSON for clients that only render text.
- `tools/list` never changes during a session; `listChanged` is `false`.
- The server builds its engine once at startup. Credential files are read then, so restart the server after adding a key.

## Navigation

- ← [MCP overview](OVERVIEW.md)
- → [Error contract](../api/02-errors.md)
