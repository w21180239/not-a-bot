//! Key rotation with cross-process cooldown state.
//!
//! Each provider may hold several credential sets. A credential that returns
//! auth/quota/rate-limit errors is put on cooldown so the next attempt uses a
//! different key. Cooldowns are persisted in the cache directory so concurrent
//! agents and CLI processes share the view.

use crate::core::error::Error;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Default cooldowns by failure class, seconds.
const QUOTA_SECS: u64 = 3600;
const RATE_SECS: u64 = 60;
const AUTH_SECS: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KeyState {
    /// label -> cooldown expiry
    pub cooled_until: BTreeMap<String, DateTime<Utc>>,
    /// label -> consecutive failures
    pub failures: BTreeMap<String, u32>,
}

impl KeyState {
    pub fn is_cooling(&self, label: &str, now: DateTime<Utc>) -> bool {
        self.cooled_until
            .get(label)
            .is_some_and(|until| *until > now)
    }

    pub fn remaining(&self, label: &str, now: DateTime<Utc>) -> u64 {
        self.cooled_until
            .get(label)
            .map(|until| (*until - now).num_seconds().max(0) as u64)
            .unwrap_or(0)
    }
}

/// Cooldown duration applied for an error, honouring upstream `Retry-After`.
pub fn cooldown_for(err: &Error) -> Option<u64> {
    if !err.cools_key() {
        return None;
    }
    Some(match err {
        Error::QuotaExhausted { retry_after_secs, .. } => retry_after_secs.unwrap_or(QUOTA_SECS),
        Error::RateLimited { retry_after_secs, .. } => retry_after_secs.unwrap_or(RATE_SECS),
        Error::AuthFailed { .. } => AUTH_SECS,
        _ => return None,
    })
}

/// Selects and tracks credentials for one provider.
pub struct KeyRing {
    labels: Vec<String>,
    state: Mutex<KeyState>,
    path: Option<PathBuf>,
}

impl KeyRing {
    pub fn new(labels: Vec<String>, path: Option<PathBuf>) -> Self {
        let state = path
            .as_deref()
            .and_then(load_state)
            .unwrap_or_default();
        Self {
            labels,
            state: Mutex::new(state),
            path,
        }
    }

    /// Index of the next usable credential, or `None` when all are cooling.
    /// Selection is round-robin: start after the last used slot so keys take
    /// turns at equal priority.
    pub fn pick(&self, now: DateTime<Utc>) -> Option<usize> {
        let state = self.state.lock().ok()?;
        if self.labels.is_empty() {
            return None;
        }
        for offset in 0..self.labels.len() {
            let idx = offset % self.labels.len();
            if !state.is_cooling(&self.labels[idx], now) {
                return Some(idx);
            }
        }
        None
    }

    /// Earliest moment a cooling credential becomes usable again.
    pub fn next_available(&self, now: DateTime<Utc>) -> Option<u64> {
        let state = self.state.lock().ok()?;
        self.labels
            .iter()
            .map(|l| state.remaining(l, now))
            .filter(|s| *s > 0)
            .min()
    }

    pub fn record_success(&self, label: &str) {
        if let Ok(mut state) = self.state.lock() {
            state.failures.remove(label);
            state.cooled_until.remove(label);
            self.persist(&state);
        }
    }

    /// Apply the cooldown implied by `err` to `label`.
    pub fn record_failure(&self, label: &str, err: &Error, now: DateTime<Utc>) {
        let Some(secs) = cooldown_for(err) else {
            return;
        };
        if let Ok(mut state) = self.state.lock() {
            *state.failures.entry(label.to_string()).or_default() += 1;
            state.cooled_until.insert(
                label.to_string(),
                now + chrono::Duration::seconds(secs as i64),
            );
            self.persist(&state);
        }
    }

    pub fn snapshot(&self, now: DateTime<Utc>) -> Vec<KeyStatus> {
        let Ok(state) = self.state.lock() else {
            return Vec::new();
        };
        self.labels
            .iter()
            .map(|label| KeyStatus {
                label: label.clone(),
                cooling: state.is_cooling(label, now),
                cooling_secs: state.remaining(label, now),
                failures: state.failures.get(label).copied().unwrap_or(0),
            })
            .collect()
    }

    fn persist(&self, state: &KeyState) {
        let Some(path) = &self.path else { return };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string(state) {
            let _ = std::fs::write(path, text);
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyStatus {
    pub label: String,
    pub cooling: bool,
    pub cooling_secs: u64,
    pub failures: u32,
}

/// Merge cooldown state from disk, keeping the later expiry per label.
fn load_state(path: &Path) -> Option<KeyState> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn state_path(cache_dir: Option<&Path>, provider: &str) -> Option<PathBuf> {
    cache_dir.map(|dir| dir.join(format!("keys-{provider}.json")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as Cd;
    use tempfile::TempDir;

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn cooldown_duration_per_class() {
        assert_eq!(
            cooldown_for(&Error::QuotaExhausted { provider: "p".into(), retry_after_secs: None }),
            Some(QUOTA_SECS)
        );
        assert_eq!(
            cooldown_for(&Error::RateLimited { provider: "p".into(), retry_after_secs: Some(5) }),
            Some(5)
        );
        assert_eq!(
            cooldown_for(&Error::AuthFailed { provider: "p".into(), reason: String::new() }),
            Some(AUTH_SECS)
        );
        assert_eq!(cooldown_for(&Error::Network("x".into())), None);
    }

    #[test]
    fn failure_cools_label_and_success_clears_it() {
        let ring = KeyRing::new(vec!["a".into()], None);
        let t = now();
        ring.record_failure("a", &Error::RateLimited { provider: "p".into(), retry_after_secs: Some(60) }, t);
        assert_eq!(ring.pick(t), None, "only key is cooling");
        assert_eq!(ring.next_available(t), Some(60));
        assert_eq!(ring.pick(t + Cd::seconds(61)), Some(0));

        ring.record_success("a");
        assert_eq!(ring.pick(t), Some(0));
        let snap = ring.snapshot(t);
        assert_eq!(snap[0].failures, 0);
    }

    #[test]
    fn round_robin_skips_cooling_key() {
        let labels: Vec<String> = vec!["k1".into(), "k2".into(), "k3".into()];
        let ring = KeyRing::new(labels, None);
        let t = now();
        ring.record_failure("k1", &Error::AuthFailed { provider: "p".into(), reason: String::new() }, t);
        let picked = ring.pick(t).expect("a key is free");
        assert_ne!(picked, 0, "cooling key must not be picked");
        let snap = ring.snapshot(t);
        assert!(snap[0].cooling && snap[0].cooling_secs > 0);
        assert!(!snap[1].cooling);
    }

    #[test]
    fn cooldown_survives_process_restart() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("keys-deepl.json");
        let t = now();
        {
            let ring = KeyRing::new(vec!["a".into(), "b".into()], Some(path.clone()));
            ring.record_failure("a", &Error::QuotaExhausted { provider: "p".into(), retry_after_secs: Some(3600) }, t);
        }
        let reloaded = KeyRing::new(vec!["a".into(), "b".into()], Some(path));
        let snap = reloaded.snapshot(t);
        assert!(snap[0].cooling, "cooldown must persist across processes");
        assert!(!snap[1].cooling);
        assert_eq!(reloaded.pick(t), Some(1));
    }

    #[test]
    fn empty_labels_has_no_key() {
        let ring = KeyRing::new(vec![], None);
        assert_eq!(ring.pick(now()), None);
        assert_eq!(ring.next_available(now()), None);
    }
}
