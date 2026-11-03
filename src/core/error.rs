//! Structured error taxonomy. `code` and `retryable` drive fallback, retry and agent decisions.

use serde::{{Deserialize, Serialize}};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("provider not found: {name}")]
    ProviderNotFound { name: String },

    #[error("provider {name} is implemented but not configured; add credentials or a base_url")]
    ProviderNotConfigured { name: String },

    #[error("{provider} cannot detect the source language; pass an explicit source")]
    AutoDetectUnsupported { provider: String },

    #[error("language pair not supported by {provider}: {from_lang} -> {target}")]
    LanguagePairNotSupported {
        from_lang: String,
        target: String,
        provider: String,
    },

    #[error("quota exhausted for {provider}")]
    QuotaExhausted {
        provider: String,
        retry_after_secs: Option<u64>,
    },

    #[error("rate limited by {provider}")]
    RateLimited {
        provider: String,
        retry_after_secs: Option<u64>,
    },

    #[error("authentication failed for {provider}: {reason}")]
    AuthFailed { provider: String, reason: String },

    #[error("upstream {provider} returned {status}")]
    Upstream {
        provider: String,
        status: u16,
        body: String,
    },

    #[error("network error: {0}")]
    Network(String),

    #[error("timed out after {timeout_ms} ms")]
    Timeout { timeout_ms: u64 },

    #[error("no provider configured; set a key or install a spec")]
    NoProvider,

    #[error("every provider in the chain failed")]
    ChainExhausted,

    #[error("every provider in the chain failed; last error from {provider}: {detail}")]
    ChainExhaustedWith { provider: String, detail: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("bad response from {provider}: {reason}")]
    BadResponse { provider: String, reason: String },

    #[error("invalid preserve pattern: {0}")]
    BadPattern(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::ProviderNotFound { .. } => "PROVIDER_NOT_FOUND",
            Error::ProviderNotConfigured { .. } => "PROVIDER_NOT_CONFIGURED",
            Error::AutoDetectUnsupported { .. } => "AUTO_DETECT_UNSUPPORTED",
            Error::LanguagePairNotSupported { .. } => "LANGUAGE_PAIR_NOT_SUPPORTED",
            Error::QuotaExhausted { .. } => "QUOTA_EXHAUSTED",
            Error::RateLimited { .. } => "RATE_LIMITED",
            Error::AuthFailed { .. } => "AUTH_FAILED",
            Error::Upstream { .. } => "UPSTREAM_ERROR",
            Error::Network(_) => "NETWORK_ERROR",
            Error::Timeout { .. } => "TIMEOUT",
            Error::NoProvider => "NO_PROVIDER",
            Error::ChainExhausted => "CHAIN_EXHAUSTED",
            Error::ChainExhaustedWith { .. } => "CHAIN_EXHAUSTED",
            Error::InvalidInput(_) => "INVALID_INPUT",
            Error::BadResponse { .. } => "BAD_RESPONSE",
            Error::BadPattern(_) => "INVALID_PRESERVE_PATTERN",
            Error::Config(_) => "CONFIG_ERROR",
            Error::Internal(_) => "INTERNAL",
        }
    }

    /// Retryable means the same provider may serve a later attempt.
    pub fn retryable(&self) -> bool {
        match self {
            Error::RateLimited { .. }
            | Error::Network(_)
            | Error::Timeout { .. }
            | Error::Upstream { status: 500..=599, .. } => true,
            Error::Upstream { status, .. } => !matches!(status, 400..=499),
            _ => false,
        }
    }

    /// Auth and quota failures cool the current key, then fall through to the next hop.
    pub fn cools_key(&self) -> bool {
        matches!(
            self,
            Error::AuthFailed { .. } | Error::QuotaExhausted { .. } | Error::RateLimited { .. }
        )
    }

    /// A chain skip, not a permanent failure: try the next provider.
    pub fn skip_to_next_provider(&self) -> bool {
        !matches!(self, Error::InvalidInput(_) | Error::BadPattern(_))
    }

    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Error::QuotaExhausted { retry_after_secs, .. }
            | Error::RateLimited { retry_after_secs, .. } => *retry_after_secs,
            _ => None,
        }
    }

    pub fn suggested_action(&self) -> &'static str {
        match self {
            Error::ProviderNotFound { .. } => "check_provider_name",
            Error::ProviderNotConfigured { .. } => "configure_a_provider",
            Error::AutoDetectUnsupported { .. } => "pass_an_explicit_source",
            Error::LanguagePairNotSupported { .. } => "change_provider_or_language",
            Error::QuotaExhausted { .. } => "switch_key_or_upgrade_plan",
            Error::RateLimited { .. } => "backoff_and_retry",
            Error::AuthFailed { .. } => "refresh_credentials",
            Error::Upstream { status, .. } => {
                if *status >= 500 {
                    "retry_later"
                } else {
                    "fix_request"
                }
            }
            Error::Network(_) => "retry_with_backoff",
            Error::Timeout { .. } => "increase_timeout_and_retry",
            Error::NoProvider => "configure_a_provider",
            Error::ChainExhausted => "check_credentials_or_network",
            Error::ChainExhaustedWith { .. } => "check_credentials_or_network",
            Error::InvalidInput(_) => "fix_input",
            Error::BadResponse { .. } => "report_bug_or_upgrade",
            Error::BadPattern(_) => "fix_preserve_pattern",
            Error::Config(_) => "fix_config_file",
            Error::Internal(_) => "report_bug",
        }
    }

    pub fn wire(&self) -> WireError {
        WireError {
            code: self.code().to_string(),
            message: self.to_string(),
            retryable: self.retryable(),
            retry_after_secs: self.retry_after_secs(),
            suggested_action: self.suggested_action().to_string(),
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Error::Timeout { timeout_ms: 0 }
        } else {
            Error::Network(e.to_string())
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Internal(format!("json: {e}"))
    }
}

impl From<config::ConfigError> for Error {
    fn from(e: config::ConfigError) -> Self {
        Error::Config(e.to_string())
    }
}

impl From<regex::Error> for Error {
    fn from(e: regex::Error) -> Self {
        Error::BadPattern(e.to_string())
    }
}

/// Error as it appears on stdout. Tagged by `code`, stable across releases.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WireError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
    pub suggested_action: String,
}

/// Map an HTTP status to the right variant. Providers call this after the request.
pub fn map_status(provider: &str, status: u16, body: &str) -> Error {
    let body: String = body.chars().take(400).collect();
    match status {
        401 | 403 => Error::AuthFailed {
            provider: provider.to_string(),
            reason: body,
        },
        402 => Error::QuotaExhausted {
            provider: provider.to_string(),
            retry_after_secs: None,
        },
        429 => Error::RateLimited {
            provider: provider.to_string(),
            retry_after_secs: None,
        },
        _ => Error::Upstream {
            provider: provider.to_string(),
            status,
            body,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_mapping() {
        assert!(matches!(
            map_status("p", 401, "bad key"),
            Error::AuthFailed { .. }
        ));
        assert!(matches!(map_status("p", 429, ""), Error::RateLimited { .. }));
        assert!(matches!(map_status("p", 503, ""), Error::Upstream { .. }));
        assert!(matches!(map_status("p", 400, ""), Error::Upstream { .. }));
    }

    #[test]
    fn retryable_split() {
        assert!(Error::RateLimited { provider: "p".into(), retry_after_secs: None }.retryable());
        assert!(Error::Network("x".into()).retryable());
        assert!(Error::Upstream { provider: "p".into(), status: 500, body: String::new() }.retryable());
        assert!(!Error::Upstream { provider: "p".into(), status: 400, body: String::new() }.retryable());
        assert!(!Error::AuthFailed { provider: "p".into(), reason: String::new() }.retryable());
        assert!(!Error::InvalidInput("x".into()).retryable());
    }

    #[test]
    fn key_cooling_and_skip_rules() {
        let auth = Error::AuthFailed { provider: "p".into(), reason: String::new() };
        assert!(auth.cools_key());
        assert!(!auth.retryable());
        assert!(auth.skip_to_next_provider());
        assert!(!Error::InvalidInput("x".into()).skip_to_next_provider());
    }

    #[test]
    fn wire_shape_is_stable() {
        let e = Error::RateLimited { provider: "deepl".into(), retry_after_secs: Some(7) }.wire();
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["code"], "RATE_LIMITED");
        assert_eq!(json["retryAfterSecs"], 7);
        assert_eq!(json["suggestedAction"], "backoff_and_retry");
        assert_eq!(json["retryable"], true);
    }

    #[test]
    fn body_is_truncated() {
        let big = "x".repeat(5000);
        let e = map_status("p", 500, &big);
        match e {
            Error::Upstream { body, .. } => assert_eq!(body.chars().count(), 400),
            other => panic!("wrong variant: {other:?}"),
        }
    }
}
