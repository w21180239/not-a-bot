//! Retry backoff shared by engine and providers.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

const BASE_MS: u64 = 300;
const CAP_MS: u64 = 3_000;

/// Exponential backoff with jitter. Upstream `Retry-After` wins, clamped so a
/// hostile header cannot stall the caller for minutes.
pub fn backoff(attempt: u32, retry_after_secs: Option<u64>) -> Duration {
    if let Some(secs) = retry_after_secs {
        return Duration::from_secs(secs.min(30));
    }
    let exp = BASE_MS.saturating_mul(1u64 << attempt.min(5));
    let capped = exp.min(CAP_MS);
    let jitter = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 % (capped / 2 + 1))
        .unwrap_or(0);
    Duration::from_millis((capped + jitter).min(CAP_MS + capped / 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_then_caps() {
        let a = backoff(0, None).as_millis();
        let b = backoff(3, None).as_millis();
        let c = backoff(9, None).as_millis();
        assert!(a <= (CAP_MS + 200) as u128);
        assert!(b > a);
        assert!(c <= (CAP_MS + 1500) as u128, "capped: {c}");
    }

    #[test]
    fn retry_after_wins_but_is_clamped() {
        assert_eq!(backoff(0, Some(4)), Duration::from_secs(4));
        assert_eq!(backoff(0, Some(3600)), Duration::from_secs(30));
    }
}
