**English** · [简体中文](../../zh-cn/api/02-errors.md)

# API — Error contract

Every failure carries enough information for an agent to decide without guessing.

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

## Codes

| Code | Retryable | `suggestedAction` | Cause |
| --- | --- | --- | --- |
| `RATE_LIMITED` | yes | `backoff_and_retry` | Per-IP or per-key throttle |
| `QUOTA_EXHAUSTED` | yes | `switch_key_or_upgrade_plan` | Plan limit reached |
| `AUTH_FAILED` | no | `refresh_credentials` | Bad or expired credentials |
| `NETWORK_ERROR` | yes | `retry_with_backoff` | DNS, TLS, connection reset |
| `TIMEOUT` | yes | `increase_timeout_and_retry` | Budget exhausted |
| `UPSTREAM_ERROR` | 5xx only | `retry_later` / `fix_request` | Provider returned an error status |
| `BAD_RESPONSE` | no | `report_bug_or_upgrade` | Body did not match the expected shape |
| `CHAIN_EXHAUSTED` | no | `check_credentials_or_network` | Every hop failed |
| `NO_PROVIDER` | no | `configure_a_provider` | Nothing registered could serve this request |
| `PROVIDER_NOT_FOUND` | no | `check_provider_name` | Unknown provider id |
| `PROVIDER_NOT_CONFIGURED` | no | `configure_a_provider` | Implemented, but no credentials |
| `AUTO_DETECT_UNSUPPORTED` | no | `pass_an_explicit_source` | Pinned provider cannot detect |
| `LANGUAGE_PAIR_NOT_SUPPORTED` | no | `change_provider_or_language` | Pair outside the provider's range |
| `INVALID_INPUT` | no | `fix_input` | Empty text, bad code, same source and target |
| `INVALID_PRESERVE_PATTERN` | no | `fix_preserve_pattern` | Regex did not compile |
| `CONFIG_ERROR` | no | `fix_config_file` | Malformed TOML or unknown key |
| `INTERNAL` | no | `report_bug` | Bug in translatex itself |

## Decision table

| `retryable` | `suggestedAction` | Agent should |
| --- | --- | --- |
| true | `backoff_and_retry` | Sleep `retryAfterSecs`, retry the same request |
| true | `retry_with_backoff` | Exponential backoff (300 ms → 600 ms → 1.2 s), retry |
| true | `switch_key_or_upgrade_plan` | Rotate the credential or wait for quota reset |
| false | `refresh_credentials` | Re-read credentials, then retry once |
| false | `pass_an_explicit_source` | Set `sourceLang` and retry |
| false | `change_provider_or_language` | Change `provider` or `targetLang` |
| false | `configure_a_provider` | Add credentials or a `base_url` |
| false | `check_credentials_or_network` | Probe with `health`, then alert |
| false | `fix_input` / `fix_config_file` | Fix the request or file; do not retry |

## Attempts

`--verbose` adds per-hop records so a `CHAIN_EXHAUSTED` result is diagnosable:

```json
{
  "error": { "code": "CHAIN_EXHAUSTED", … },
  "attempts": [
    { "provider": "deepl", "keyLabel": "default", "ok": false, "latencyMs": 95, "errorCode": "AUTH_FAILED" },
    { "provider": "google", "keyLabel": "default", "ok": false, "latencyMs": 2100, "errorCode": "NETWORK_ERROR" }
  ]
}
```

## Exit codes

| Error classes | Exit |
| --- | --- |
| none | 0 |
| `CONFIG_ERROR` | 2 |
| `CHAIN_EXHAUSTED`, `NO_PROVIDER` | 3 |
| `NETWORK_ERROR`, `TIMEOUT` | 4 |
| everything else | 1 |

## Navigation

- ← [Request and response shapes](01-cli-json.md)
- → [Troubleshooting](../troubleshooting/01-common-issues.md)
