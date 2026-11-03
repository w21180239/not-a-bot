//! Shared HTTP client and response helpers.

use crate::core::error::{Error, Result};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use std::time::Duration;

pub const USER_AGENT: &str = concat!("translatex/", env!("CARGO_PKG_VERSION"));

pub fn client(timeout: Duration) -> Result<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(timeout)
        .gzip(true)
        .brotli(true)
        .build()
        .map_err(|e| Error::Internal(format!("http client: {e}")))
}

/// Convert a response into text, mapping non-success statuses to typed errors.
pub async fn body_text(provider: &str, resp: Response) -> Result<String> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| Error::Network(format!("read body: {e}")))?;
    if status != StatusCode::OK {
        return Err(crate::core::error::map_status(
            provider,
            status.as_u16(),
            &text,
        ));
    }
    Ok(text)
}

pub async fn json<T: serde::de::DeserializeOwned>(provider: &str, resp: Response) -> Result<T> {
    let text = body_text(provider, resp).await?;
    serde_json::from_str(&text).map_err(|e| Error::BadResponse {
        provider: provider.to_string(),
        reason: format!("{}: {}", e, truncate(&text, 160)),
    })
}

/// Attach a per-request timeout override when the caller supplied one.
pub fn with_timeout(req: RequestBuilder, timeout_ms: Option<u64>) -> RequestBuilder {
    match timeout_ms {
        Some(ms) if ms > 0 => req.timeout(Duration::from_millis(ms)),
        _ => req,
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// UTC now in ISO-8601 with milliseconds, as providers expect.
pub fn now_iso8601() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S.000Z")
        .to_string()
}

pub use crate::core::error::map_status;

/// Canonical query string: keys sorted, values percent-encoded (RFC 3986).
pub fn canonical_query(form: &std::collections::BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (i, (k, v)) in form.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        out.push_str(&percent_encode(k));
        out.push('=');
        out.push_str(&percent_encode(v));
    }
    out
}

/// Unreserved set per RFC 3986: A-Z a-z 0-9 - _ . ~. Everything else escaped.
pub fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// HMAC-SHA1(key, message), base64.
pub fn hmac_sha1(key: &str, message: &str) -> String {
    use base64::Engine;
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    type H = Hmac<Sha1>;
    let mut mac = H::new_from_slice(key.as_bytes()).expect("hmac accepts any key length");
    mac.update(message.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

/// UTC now as unix seconds. Signing schemes use this for freshness.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// UTC now in unix milliseconds; several web endpoints use this as a cache-buster.
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// MD5 hex digest. Baidu and Youdao sign with MD5 over a concatenated secret.
/// Crates.io package `md-5` imports as `md5`.
pub fn md5_hex(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

/// SHA-256 hex digest.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

/// Raw HMAC-SHA256 bytes (for key-derivation chains like Tencent TC3).
pub fn hmac_sha256_bytes(key: &[u8], message: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type H = Hmac<Sha256>;
    let mut mac = H::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(message);
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn percent_encode_follows_rfc3986() {
        assert_eq!(percent_encode("a b"), "a%20b");
        assert_eq!(percent_encode("a/b"), "a%2Fb");
        assert_eq!(percent_encode("~_-."), "~_-.");
        assert_eq!(percent_encode("="), "%3D");
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // RFC 4231-style sanity: SHA256 hash of known string.
        let got = sha256_hex(b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            got,
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    #[test]
    fn canonical_query_is_sorted_and_encoded() {
        let mut m = BTreeMap::new();
        m.insert("b".into(), "2".into());
        m.insert("a".into(), "1 1".into());
        assert_eq!(canonical_query(&m), "a=1%201&b=2");
    }

    #[test]
    fn hmac_sha1_is_deterministic() {
        let a = hmac_sha1("secret", "GET&%2F&a%3D1");
        let b = hmac_sha1("secret", "GET&%2F&a%3D1");
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn now_iso8601_has_required_shape() {
        let t = now_iso8601();
        assert!(t.ends_with('Z'));
        assert!(t.contains('T'));
    }
}
