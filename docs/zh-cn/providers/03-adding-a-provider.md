**简体中文** · [English](../../en/providers/03-adding-a-provider.md)

# Provider — 新增后端

[← 领域概览](OVERVIEW.md) · [文档索引](../README.md)

## 1. 实现 trait

新建 `src/providers/<name>.rs`：

```rust
use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
use crate::core::http::{body_text, map_status};
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use crate::providers::{Provider, ProviderKind};
use async_trait::async_trait;

pub struct Example {
    client: reqwest::Client,
}

#[async_trait]
impl Provider for Example {
    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "example".into(),
            display_name: "Example Translate (web)".into(),
            kind: ProviderKind::Web,
            priority: 6,
            needs_key: false,
            max_chars: 5_000,
            auto_detect: true,
            languages: vec!["*".into()],
            free_tier: Some("unlimited (web endpoint)".into()),
            configured: true,
        }
    }

    async fn translate_once(
        &self,
        req: &TranslateRequest,
        _cred: &Credential,
    ) -> Result<TranslateResponse> {
        // 返回强类型错误，冷却与回退由引擎负责。
        Ok(TranslateResponse {
            text: String::new(),
            detected_source: req.source_lang.clone(),
            target_lang: req.target_lang.clone(),
            provider: "example".into(),
            engine: "example".into(),
            latency_ms: 0,
            request_id: String::new(),
            chars: Some(req.text.chars().count()),
            warnings: Vec::new(),
        })
    }
}
```

## 2. 选择优先级

| 区间 | 用途 |
| --- | --- |
| 1-3 | 顶级网页接口，质量与可用性最好 |
| 4-12 | 其他网页接口 |
| 20-27 | 官方 API |
| 40-50 | LLM |

## 3. 如实声明能力

- `languages`：接口接受的规范语言码；接受站点所懂全部语言时用 `["*"]`。
- `auto_detect`：接口需要显式源语言时设为 false，链会在 `auto` 请求中跳过它。
- `max_chars`：适配器一次真正能发送的上限。
- `needs_key`：为 true 时，注册依赖已配置的凭据。

## 4. 注册

在 `src/cli.rs::build_registry` 中，无 Key 的 Provider：

```rust
reg.register(Arc::new(example::Example::new()));
```

有 Key 的放进 `build_keyed`：

```rust
if has("example") {
    reg.register(Arc::new(example::Example::new(&opts("example"))));
}
```

然后把 id 加进 `src/providers/mod.rs` 的 `KNOWN_PROVIDERS`，并与其他 `pub mod` 一起声明模块。

## 5. 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_contract() {
        let info = Example::new().info();
        assert_eq!(info.name, "example");
        assert_eq!(info.kind, ProviderKind::Web);
        assert_eq!(info.priority, 6);
    }
}
```

上线前做实测：

```bash
translatex translate "Hello world" -t zh --provider example --format json
```

返回与源文一致的 Provider 属于静默失败。解析响应体，缺少译文时返回 `Error::BadResponse`。

## 错误映射

| 情况 | 返回 |
| --- | --- |
| 401/403 | `Error::AuthFailed { provider, reason }` |
| 402 | `Error::QuotaExhausted { provider, retry_after_secs }` |
| 429 | `Error::RateLimited { provider, retry_after_secs }` |
| 5xx | `Error::Upstream { provider, status, body }` |
| 无法解析的响应体 | `Error::BadResponse { provider, reason }` |
| 不支持的语言对 | `Error::LanguagePairNotSupported { … }` |

HTTP 情况用 `map_status` 覆盖，不要重复写匹配。

## 导航

- ← [优先级与回退](02-priorities-and-fallback.md)
- ↑ [Provider 概览](OVERVIEW.md)
