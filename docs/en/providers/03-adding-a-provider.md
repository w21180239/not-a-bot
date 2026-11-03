**English** · [简体中文](../../zh-cn/providers/03-adding-a-provider.md)

# Providers — Adding a provider

## 1. Implement the trait

Create `src/providers/<name>.rs`:

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
        // Return a typed error; the engine handles cooldown and fallback.
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

## 2. Pick the priority

| Range | Use for |
| --- | --- |
| 1-3 | Top web endpoints, best quality and uptime |
| 4-12 | Other web endpoints |
| 20-27 | Official APIs |
| 40-50 | LLM providers |

## 3. Declare capabilities honestly

- `languages`: list the canonical codes the endpoint accepts, or `["*"]` when it accepts anything the site understands.
- `auto_detect`: false when the endpoint needs an explicit source. The chain then skips it for `auto` requests.
- `max_chars`: the limit your adapter can actually send in one request.
- `needs_key`: true makes registration depend on configured credentials.

## 4. Register it

In `src/cli.rs::build_registry`, for a keyless provider:

```rust
reg.register(Arc::new(example::Example::new()));
```

For a keyed one, add it inside `build_keyed`:

```rust
if has("example") {
    reg.register(Arc::new(example::Example::new(&opts("example"))));
}
```

Then add the id to `KNOWN_PROVIDERS` in `src/providers/mod.rs`, and declare the module with the other `pub mod` lines.

## 5. Test

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

Live-check it before shipping:

```bash
translatex translate "Hello world" -t zh --provider example --format json
```

A provider that returns the source unchanged is a silent failure. Parse the response body and return `Error::BadResponse` when the translation is missing.

## Error mapping

| Condition | Return |
| --- | --- |
| 401/403 | `Error::AuthFailed { provider, reason }` |
| 402 | `Error::QuotaExhausted { provider, retry_after_secs }` |
| 429 | `Error::RateLimited { provider, retry_after_secs }` |
| 5xx | `Error::Upstream { provider, status, body }` |
| Unparseable body | `Error::BadResponse { provider, reason }` |
| Unsupported pair | `Error::LanguagePairNotSupported { … }` |

`map_status` covers the HTTP cases; use it rather than duplicating the match.

## Navigation

- ← [Priorities and fallback](02-priorities-and-fallback.md)
- ↑ [Providers overview](OVERVIEW.md)
