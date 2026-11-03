**简体中文** · [English](../../en/api/02-errors.md)

# API — 错误契约

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

每个失败都携带足够信息，让 Agent 不必猜测就能决策。

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "rate limited by sogou",
    "retryable": true,
    "retryAfterSecs": 300,
    "suggestedAction": "backoff_and_retry"
  },
  "requestId": ""
}
```

## 错误码

| 错误码 | 可重试 | `suggestedAction` | 原因 |
| --- | --- | --- | --- |
| `RATE_LIMITED` | 是 | `backoff_and_retry` | 按 IP 或按 Key 限流 |
| `QUOTA_EXHAUSTED` | 否 | `switch_key_or_upgrade_plan` | 套餐额度用尽 |
| `AUTH_FAILED` | 否 | `refresh_credentials` | 凭据错误或过期 |
| `NETWORK_ERROR` | 是 | `retry_with_backoff` | DNS、TLS、连接重置 |
| `TIMEOUT` | 是 | `increase_timeout_and_retry` | 预算耗尽 |
| `UPSTREAM_ERROR` | 仅 5xx | `retry_later` / `fix_request` | Provider 返回错误状态 |
| `BAD_RESPONSE` | 否 | `report_bug_or_upgrade` | 响应体不符合预期结构 |
| `CHAIN_EXHAUSTED` | 否 | `check_credentials_or_network` | 每一跳都失败 |
| `NO_PROVIDER` | 否 | `configure_a_provider` | 已注册的 Provider 没有一个能服务这次请求 |
| `PROVIDER_NOT_FOUND` | 否 | `check_provider_name` | 未知 Provider id |
| `PROVIDER_NOT_CONFIGURED` | 否 | `configure_a_provider` | 已实现但缺少凭据 |
| `AUTO_DETECT_UNSUPPORTED` | 否 | `pass_an_explicit_source` | 指定 Provider 不支持检测 |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | 否 | `change_provider_or_language` | 语言对超出范围 |
| `INVALID_INPUT` | 否 | `fix_input` | 空文本、错误语言码、源目标相同 |
| `INVALID_PRESERVE_PATTERN` | 否 | `fix_preserve_pattern` | 正则无法编译 |
| `CONFIG_ERROR` | 否 | `fix_config_file` | TOML 格式错误或未知键 |
| `INTERNAL` | 否 | `report_bug` | translatex 自身的缺陷 |

## 决策表

| `retryable` | `suggestedAction` | Agent 应采取 |
| --- | --- | --- |
| true | `backoff_and_retry` | 等待 `retryAfterSecs` 后重试同一请求 |
| true | `retry_with_backoff` | 指数退避（300ms → 600ms → 1.2s）后重试 |
| true | `switch_key_or_upgrade_plan` | 换凭据或等额度重置 |
| false | `refresh_credentials` | 重新读取凭据，再重试一次 |
| false | `pass_an_explicit_source` | 设置 `sourceLang` 后重试 |
| false | `change_provider_or_language` | 换 `provider` 或 `targetLang` |
| false | `configure_a_provider` | 补凭据或 `base_url` |
| false | `check_credentials_or_network` | 用 `health` 探测后告警 |
| false | `fix_input` / `fix_config_file` | 修请求或文件，不要重试 |

## 尝试记录

`--verbose` 会附带逐跳记录，成功路径靠它看明细；失败的链本身就会带上已经试过的跳，`CHAIN_EXHAUSTED` 因此可直接诊断：

```json
{
  "error": { "code": "CHAIN_EXHAUSTED", … },
  "attempts": [
    { "provider": "deepl", "keyLabel": "default", "ok": false, "latencyMs": 95, "errorCode": "AUTH_FAILED" },
    { "provider": "google", "keyLabel": "default", "ok": false, "latencyMs": 2100, "errorCode": "NETWORK_ERROR" }
  ]
}
```

数组在没有任何 Provider 被触达时省略：请求非法、Provider 名不存在或凭据缺失，都在链开始前就失败了。成功响应只有加 `--verbose` 才带 `attempts`。

## 退出码

| 错误类别 | 退出码 |
| --- | --- |
| 无错误 | 0 |
| `CONFIG_ERROR` | 2 |
| `CHAIN_EXHAUSTED`、`NO_PROVIDER` | 3 |
| `NETWORK_ERROR`、`TIMEOUT` | 4 |
| 其他 | 1 |

## 导航

- ← [请求与响应结构](01-cli-json.md)
- → [故障排查](../troubleshooting/01-common-issues.md)
