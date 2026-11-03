**简体中文** · [English](../../en/api/01-cli-json.md)

# API — 请求与响应结构

## 请求

```json
{
  "text": "推送前先跑 `make test`。",
  "sourceLang": "auto",
  "targetLang": "en",
  "provider": "google",
  "fallbackChain": ["edge", "iciba"],
  "preserve": ["`[^`]*`"],
  "timeoutMs": 15000,
  "idempotencyKey": "job-42",
  "metadata": { "traceId": "abc123" }
}
```

| 字段 | 必填 | 默认 | 说明 |
| --- | --- | --- | --- |
| `text` | 是 | — | 空或全空白为 `INVALID_INPUT` |
| `targetLang` | 是 | — | 规范语言码，未知会被拒绝 |
| `sourceLang` | 否 | `auto` | `auto` 需要 Provider 支持检测 |
| `provider` | 否 | 不设置 | 指定一个 Provider |
| `fallbackChain` | 否 | `[]` | 指定 Provider 之后尝试，或未指定时的最前顺序 |
| `preserve` | 否 | `[]` | Rust `regex` 表达式 |
| `timeoutMs` | 否 | 配置值 | 限制整个 Provider 调用 |
| `idempotencyKey` | 否 | 自动生成 | 作为 `requestId` 回显 |
| `metadata` | 否 | `{}` | 留给调用方记账，引擎不读 |

未知字段会被拒绝（`deny_unknown_fields`），拼错的参数名会立刻暴露，而不是被静默忽略。

## 成功

```json
{
  "text": "Run `make test` before pushing.",
  "detectedSource": "zh",
  "targetLang": "en",
  "provider": "google",
  "engine": "google",
  "latencyMs": 412,
  "requestId": "c506d3bb-8eb2-451a-9e84-f2be63833f79",
  "chars": 21
}
```

| 字段 | 含义 |
| --- | --- |
| `provider` | 产出结果的 Provider |
| `engine` | Provider 上报的后端名；除非厂商另有命名，否则与 `provider` 相同 |
| `detectedSource` | 检测到或规范化后的源语言 |
| `latencyMs` | 成功那一次尝试的耗时 |
| `requestId` | 有 `idempotencyKey` 时用它，否则是 UUID |
| `chars` | 源文本字符数 |
| `warnings` | 仅非空时出现 |

加 `--verbose` 会多出 `attempts`：

```json
"attempts": [
  { "provider": "deepl", "keyLabel": "default", "ok": false, "latencyMs": 120, "errorCode": "AUTH_FAILED" },
  { "provider": "google", "keyLabel": "default", "ok": true, "latencyMs": 412 }
]
```

## 批次 NDJSON

每行一个请求，每行一个结果，顺序与输入一致：

```json
{"index":0,"ok":true,"result":{"text":"你好世界","provider":"google",…}}
{"index":1,"ok":false,"error":{"code":"LANGUAGE_PAIR_NOT_SUPPORTED",…}}
```

`index` 是请求的 0 基行号。非法 JSON 行会被跳过并在 stderr 警告，不会出现在输出里。

## 纯文本格式

`--format text` 只打印译文，便于 shell 管道：

```bash
translatex translate "Hello" -t zh --format text
```

`--format json` 下，错误仍以 JSON 打到 stderr。

## 导航

- ← [API 概览](OVERVIEW.md)
- → [错误契约](02-errors.md)
