**English** · [简体中文](../../zh-cn/api/01-cli-json.md)

# API — Request and response shapes

[← Domain overview](OVERVIEW.md) · [Documentation index](../README.md)

## Request

```json
{
  "text": "Run `make test` before pushing.",
  "sourceLang": "auto",
  "targetLang": "zh",
  "provider": "google",
  "fallbackChain": ["edge", "iciba"],
  "preserve": ["`[^`]*`"],
  "timeoutMs": 15000,
  "idempotencyKey": "job-42",
  "metadata": { "traceId": "abc123" }
}
```

| Field | Required | Default | Notes |
| --- | --- | --- | --- |
| `text` | yes | — | Empty or whitespace-only is `INVALID_INPUT` |
| `targetLang` | yes | — | Canonical code; unknown codes are rejected |
| `sourceLang` | no | `auto` | `auto` requires provider detection support |
| `provider` | no | unset | Pins one provider |
| `fallbackChain` | no | `[]` | Providers tried after the pinned one, or first when unpinned |
| `preserve` | no | `[]` | Rust `regex` patterns |
| `timeoutMs` | no | config | Caps the whole provider call |
| `idempotencyKey` | no | generated | Echoed in `requestId` |
| `metadata` | no | `{}` | Reserved for caller bookkeeping, never read by the engine |

Unknown fields are rejected (`deny_unknown_fields`), which turns typo'd option names into immediate errors instead of silent defaults.

## Success

```json
{
  "text": "推送前先跑 `make test`。",
  "detectedSource": "en",
  "targetLang": "zh",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "c506d3bb-8eb2-451a-9e84-f2be63833f79",
  "chars": 33
}
```

| Field | Meaning |
| --- | --- |
| `provider` | Provider that produced the result |
| `engine` | Backend name the provider reported; equals `provider` unless the vendor names it differently |
| `detectedSource` | Detected or normalised source code |
| `latencyMs` | Wall time for the successful attempt |
| `requestId` | `idempotencyKey` when supplied, otherwise a UUID |
| `chars` | Source character count |
| `warnings` | Present only when non-empty |

With `--verbose`, an `attempts` array is added:

```json
"attempts": [
  { "provider": "deepl", "keyLabel": "default", "ok": false, "latencyMs": 120, "errorCode": "AUTH_FAILED" },
  { "provider": "google", "keyLabel": "default", "ok": true, "latencyMs": 412 }
]
```

## Batch NDJSON

One request per line; one result per line, in input order:

```json
{"index":0,"ok":true,"result":{"text":"你好世界","provider":"google", …}}
{"index":1,"ok":false,"error":{"code":"LANGUAGE_PAIR_NOT_SUPPORTED", …}}
```

`index` is the 0-based line number of the request. Invalid JSON lines are skipped with a stderr warning and do not appear in the output.

## Text format

`--format text` prints the translation and nothing else, so shell pipelines stay clean:

```bash
translatex translate "Hello" -t zh --format text
```

Errors still go to stderr as JSON when `--format json` is active.

## Navigation

- ← [API overview](OVERVIEW.md)
- → [Error contract](02-errors.md)
