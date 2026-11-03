//! Fallback engine: chain providers, rotate keys, mask preserve spans, retry.

use crate::core::config::Settings;
use crate::core::creds::CredentialStore;
use crate::core::error::{Error, Result};
use crate::core::keys::KeyRing;
use crate::core::preserve::Preserver;
use crate::core::types::{AttemptRecord, TranslateRequest, TranslateResponse};
use crate::providers::Registry;
use crate::retry::backoff;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct Engine {
    registry: Arc<Registry>,
    creds: CredentialStore,
    cache_dir: Option<std::path::PathBuf>,
    settings: Settings,
    /// Resolved once: the configured chain changes only when the process restarts.
    chain: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub response: TranslateResponse,
    pub attempts: Vec<AttemptRecord>,
}

impl Engine {
    pub fn new(
        registry: Arc<Registry>,
        creds: CredentialStore,
        cache_dir: Option<std::path::PathBuf>,
        settings: Settings,
    ) -> Self {
        let chain = settings.chain();
        Self { registry, creds, cache_dir, settings, chain }
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn creds(&self) -> &CredentialStore {
        &self.creds
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub async fn translate(&self, req: &TranslateRequest) -> Result<Outcome> {
        let request_id = req
            .idempotency_key
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if req.text.trim().is_empty() {
            return Err(Error::InvalidInput("text is empty".into()));
        }
        let target = crate::core::lang::normalize(&req.target_lang);
        let source = crate::core::lang::normalize(&req.source_lang);
        if !crate::core::lang::is_known(&target) {
            return Err(Error::InvalidInput(format!("unknown target language {}", req.target_lang)));
        }
        if target == source {
            return Err(Error::InvalidInput(format!(
                "source and target are both {target}"
            )));
        }
        let mut inner = req.clone();
        inner.target_lang = target.clone();
        inner.source_lang = source.clone();

        let mut preserver = Preserver::new(&req.preserve)?;
        let payload = match &mut preserver {
            Some(p) => p.mask(&inner.text),
            None => inner.text.clone(),
        };
        inner.text = payload;

        let chain = self.registry.chain(
            req.provider.as_deref(),
            &req.fallback_chain,
            &self.chain,
            &req.source_lang,
            &target,
        )?;
        let pinned = req.provider.is_some();
        // Per-request timeout wins over the configured default.
        let budget_ms = req.timeout_ms.or(self.settings.timeout_ms).unwrap_or(20_000);
        let budget = Duration::from_millis(budget_ms);

        let mut attempts = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        let mut last: Option<Error> = None;
        let mut any_tried = false;
        // Providers skipped because every configured key is on cooldown.
        let mut cooling: Vec<(String, u64)> = Vec::new();

        for provider in chain {
            let info = provider.info();
            // Key labels come from the credential store so providers stay
            // stateless about how many keys are configured.
            let labels = if info.needs_key {
                self.creds.labels(&info.name)
            } else {
                vec!["default".to_string()]
            };
            if info.needs_key && labels.is_empty() {
                continue;
            }
            // Cooldown only helps when there is another key to rotate to. A
            // keyless provider has one pseudo-credential, so cooling it would
            // take the channel offline for an hour instead of rotating.
            let ring = if info.needs_key {
                let path = crate::core::keys::state_path(self.cache_dir.as_deref(), &info.name);
                KeyRing::new(labels.clone(), path)
            } else {
                KeyRing::unmanaged(labels.clone())
            };
            let rounds = labels.len().max(1);

            for _ in 0..rounds {
                let now = chrono::Utc::now();
                let Some(idx) = ring.pick(now) else {
                    if ring.all_cooling(now) {
                        cooling.push((
                            info.name.clone(),
                            ring.next_available(now).unwrap_or(0),
                        ));
                    }
                    break;
                };
                let label = labels[idx].clone();
                let Some(cred) = self.creds.keys(&info.name).get(idx).cloned().or_else(|| {
                    (!info.needs_key).then(|| crate::core::creds::Credential {
                        provider: info.name.clone(),
                        label: label.clone(),
                        fields: Default::default(),
                    })
                }) else {
                    break;
                };

                any_tried = true;
                let mut attempt = 0u32;
                loop {
                    let started = Instant::now();
                    let result = match tokio::time::timeout(
                        budget,
                        provider.translate_once(&inner, &cred),
                    )
                    .await
                    {
                        Ok(r) => r,
                        Err(_) => Err(Error::Timeout { timeout_ms: budget_ms }),
                    };
                    let latency_ms = started.elapsed().as_millis() as u64;

                    match result {
                        Ok(mut resp) => {
                            if let Some(p) = &preserver {
                                resp.text = match p.restore(&resp.text) {
                                    Ok(text) => text,
                                    Err(err) => {
                                        attempts.push(AttemptRecord {
                                            provider: info.name.clone(),
                                            key_label: Some(label.clone()),
                                            ok: false,
                                            latency_ms,
                                            error_code: Some(err.code().to_string()),
                                        });
                                        failures.push(format!("{}: {err}", info.name));
                                        last = Some(err);
                                        break;
                                    }
                                };
                            }
                            ring.record_success(&label);
                            attempts.push(AttemptRecord {
                                provider: info.name.clone(),
                                key_label: Some(label.clone()),
                                ok: true,
                                latency_ms,
                                error_code: None,
                            });
                            resp.provider = info.name.clone();
                            resp.engine = if resp.engine.is_empty() {
                                info.name.clone()
                            } else {
                                resp.engine
                            };
                            resp.request_id = request_id.clone();
                            resp.target_lang = target.clone();
                            // Providers report their own dialect of the source
                            // code (`zh-CN`, `chi`); the wire contract promises
                            // the canonical one.
                            resp.detected_source = crate::core::lang::normalize(&resp.detected_source);
                            if resp.chars.is_none() {
                                resp.chars = Some(req.text.chars().count());
                            }
                            return Ok(Outcome { response: resp, attempts });
                        }
                        Err(err) => {
                            ring.record_failure(&label, &err, chrono::Utc::now());
                            attempts.push(AttemptRecord {
                                provider: info.name.clone(),
                                key_label: Some(label.clone()),
                                ok: false,
                                latency_ms,
                                error_code: Some(err.code().to_string()),
                            });
                            failures.push(format!("{}: {err}", info.name));
                            last = Some(err);

                            // A timeout means this endpoint exceeded the budget;
                            // retrying it just burns another full budget. Move on.
                            let retry_same = last.as_ref().is_some_and(|e| e.retryable())
                                && !matches!(last, Some(Error::Timeout { .. }))
                                && attempt < self.settings.retries();
                            if retry_same {
                                let secs = last.as_ref().and_then(|e| e.retry_after_secs());
                                attempt += 1;
                                tokio::time::sleep(backoff(attempt, secs)).await;
                                continue;
                            }
                            if !last.as_ref().is_some_and(|e| e.skip_to_next_provider()) {
                                return Err(last.unwrap());
                            }
                            break;
                        }
                    }
                }
            }
        }

        Err(if !any_tried {
            // Keys exist but every one is on cooldown: that is a rate limit with
            // a known recovery time, not a missing provider.
            match cooling.iter().map(|(_, secs)| *secs).max() {
                Some(secs) => Error::RateLimited {
                    provider: cooling[0].0.clone(),
                    retry_after_secs: Some(secs),
                },
                None => Error::NoProvider,
            }
        } else if pinned {
            // A pinned provider has no chain to exhaust; report its own error.
            last.unwrap_or(Error::ChainExhausted)
        } else {
            match last {
                Some(Error::Upstream { provider, .. }) => Error::ChainExhaustedWith {
                    provider,
                    detail: failures.join("; "),
                },
                Some(other) => other,
                None => Error::ChainExhausted,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::creds::Credential;
    use crate::core::error::Result as R;
    use crate::core::types::{ProviderInfo, ProviderKind};
    use crate::providers::Provider;
    use crate::core::TranslateRequest as TR;
    use async_trait::async_trait;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Stub {
        name: &'static str,
        calls: Arc<AtomicUsize>,
        /// Codes to return per call; `None` means succeed.
        script: Vec<Option<&'static str>>,
    }

    #[async_trait]
    impl Provider for Stub {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                display_name: self.name.into(),
                kind: if self.name == "free" { ProviderKind::Web } else { ProviderKind::Official },
                priority: if self.name == "free" { 30 } else { 1 },
                needs_key: self.name != "free",
                max_chars: 5000,
                auto_detect: true,
                languages: vec!["*".into()],
                free_tier: None,
                configured: true,
            }
        }
        fn credential_labels(&self) -> Vec<String> {
            if self.name == "free" {
                vec!["default".into()]
            } else {
                vec!["k1".into(), "k2".into()]
            }
        }
        async fn translate_once(&self, req: &TR, _cred: &Credential) -> R<TranslateResponse> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            match self.script.get(n).copied().flatten() {
                Some("auth") => Err(Error::AuthFailed { provider: self.name.into(), reason: "bad".into() }),
                Some("rate") => Err(Error::RateLimited { provider: self.name.into(), retry_after_secs: Some(1) }),
                Some(_) => Err(Error::Upstream {
                    provider: self.name.into(),
                    status: 500,
                    body: "boom".into(),
                }),
                None => Ok(TranslateResponse {
                    text: if self.info().kind == ProviderKind::Web {
                        req.text.clone()
                    } else {
                        format!("{}:{}", self.name, req.text)
                    },
                    detected_source: req.source_lang.clone(),
                    target_lang: req.target_lang.clone(),
                    provider: self.name.into(),
                    engine: self.name.into(),
                    latency_ms: 1,
                    request_id: String::new(),
                    chars: None,
                    warnings: Vec::new(),
                }),
            }
        }
    }

    fn stub(
        name: &'static str,
        script: Vec<Option<&'static str>>,
    ) -> (Arc<Stub>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Arc::new(Stub { name, calls: calls.clone(), script }),
            calls,
        )
    }

    fn request(provider: Option<&str>) -> TR {
        TR {
            text: "hello".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: provider.map(String::from),
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: None,
            metadata: BTreeMap::new(),
        }
    }

    fn creds_with(provider: &str, labels: &[&str]) -> CredentialStore {
        let mut store = CredentialStore::default();
        for label in labels {
            store.add(Credential {
                provider: provider.into(),
                label: (*label).into(),
                fields: [("api_key".to_string(), "x".to_string())]
                    .into_iter()
                    .collect(),
            });
        }
        store
    }

    /// Retries off: these tests count calls, not backoff behaviour.
    fn test_settings() -> Settings {
        Settings { retries: Some(0), ..Default::default() }
    }

    fn dir() -> tempfile::TempDir {
        tempfile::TempDir::new().unwrap()
    }

    #[tokio::test]
    async fn falls_through_to_next_provider() {
        let home = dir();
        let (bad, bad_calls) = stub("deepl", vec![Some("up")]);
        let (good, _) = stub("free", vec![]);
        let mut reg = Registry::new();
        reg.register(bad);
        reg.register(good);
        let engine = Engine::new(
            Arc::new(reg),
            creds_with("deepl", &["k1"]),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let out = engine.translate(&request(None)).await.unwrap();
        assert_eq!(out.response.provider, "free");
        assert_eq!(bad_calls.load(Ordering::SeqCst), 1);
        assert_eq!(out.attempts.len(), 2);
        assert!(!out.attempts[0].ok);
        assert!(out.attempts[1].ok);
    }

    #[tokio::test]
    async fn auth_failure_rotates_to_next_key() {
        let home = dir();
        let (p, calls) = stub("deepl", vec![Some("auth"), None]);
        let mut reg = Registry::new();
        reg.register(p);
        let engine = Engine::new(
            Arc::new(reg),
            creds_with("deepl", &["k1", "k2"]),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let out = engine.translate(&request(None)).await.unwrap();
        assert_eq!(out.response.provider, "deepl");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(out.attempts[0].key_label.as_deref(), Some("k1"));
        assert_eq!(out.attempts[1].key_label.as_deref(), Some("k2"));
    }

    #[tokio::test]
    async fn cooling_key_is_skipped() {
        let home = dir();
        let (p, calls) = stub("deepl", vec![Some("auth"), None]);
        let mut reg = Registry::new();
        reg.register(p);
        let engine = Engine::new(
            Arc::new(reg),
            creds_with("deepl", &["k1", "k2"]),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        engine.translate(&request(None)).await.unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);

        // k1 cooled by auth, k2 succeeded; the next call must reuse k2 only.
        let out = engine.translate(&request(None)).await.unwrap();
        assert!(out.response.text.starts_with("deepl:"));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn preserve_round_trip_masks_and_restores() {
        let home = dir();
        let (p, calls) = stub("free", vec![]);
        let mut reg = Registry::new();
        reg.register(p);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let mut req = request(None);
        req.text = "run make test now".into();
        req.preserve = vec!["make test".into()];
        let out = engine.translate(&req).await.unwrap();
        // Stub echoes the masked payload; restore must put the span back.
        assert_eq!(out.response.text, "run make test now");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn unknown_provider_is_reported() {
        let home = dir();
        let (p, _) = stub("free", vec![]);
        let mut reg = Registry::new();
        reg.register(p);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let err = engine.translate(&request(Some("nope"))).await.unwrap_err();
        assert_eq!(err.code(), "PROVIDER_NOT_FOUND");
    }

    #[tokio::test]
    async fn empty_chain_is_chain_exhausted() {
        let home = dir();
        let engine = Engine::new(
            Arc::new(Registry::new()),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let err = engine.translate(&request(None)).await.unwrap_err();
        assert_eq!(err.code(), "NO_PROVIDER");
    }

    #[tokio::test]
    async fn unconfigured_keyed_provider_is_skipped() {
        let home = dir();
        let (p, calls) = stub("first", vec![]);
        let mut reg = Registry::new();
        reg.register(p);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let err = engine.translate(&request(None)).await.unwrap_err();
        assert_eq!(err.code(), "NO_PROVIDER");
        assert_eq!(calls.load(Ordering::SeqCst), 0, "no request for missing keys");
    }

    #[tokio::test]
    async fn configured_chain_overrides_registration_order() {
        let home = dir();
        let (api, _) = stub("deepl", vec![]);
        let (web, _) = stub("free", vec![]);
        let mut reg = Registry::new();
        // Registered first and highest priority, but the config chain names the
        // other provider, so `free` must serve the request.
        reg.register(api);
        reg.register(web);
        let settings = Settings {
            chain: Some(vec!["free".to_string()]),
            retries: Some(0),
            ..Default::default()
        };
        let engine = Engine::new(
            Arc::new(reg),
            creds_with("deepl", &["k1"]),
            Some(home.path().to_path_buf()),
            settings,
        );
        let out = engine.translate(&request(None)).await.unwrap();
        assert_eq!(out.response.provider, "free");
    }

    /// Slow provider used to prove the engine enforces the request budget.
    struct Slow;

    #[async_trait]
    impl Provider for Slow {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "slow".into(),
                display_name: "slow".into(),
                kind: ProviderKind::Web,
                priority: 1,
                needs_key: false,
                max_chars: 5000,
                auto_detect: true,
                languages: vec!["*".into()],
                free_tier: None,
                configured: true,
            }
        }
        async fn translate_once(&self, req: &TR, _cred: &Credential) -> R<TranslateResponse> {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            Ok(TranslateResponse {
                text: req.text.clone(),
                detected_source: "en".into(),
                target_lang: req.target_lang.clone(),
                provider: "slow".into(),
                engine: "slow".into(),
                latency_ms: 300,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn request_timeout_is_enforced() {
        let home = dir();
        let mut reg = Registry::new();
        reg.register(Arc::new(Slow));
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let mut req = request(None);
        req.provider = Some("slow".into());
        req.timeout_ms = Some(30);
        let started = Instant::now();
        let err = engine.translate(&req).await.unwrap_err();
        assert_eq!(err.code(), "TIMEOUT");
        assert!(
            started.elapsed() < std::time::Duration::from_millis(250),
            "engine must cut the call at the budget, not at the endpoint timeout"
        );
    }

    /// Reports `zh-CN` the way several web endpoints do.
    struct Dialect;

    #[async_trait]
    impl Provider for Dialect {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "dialect".into(),
                display_name: "dialect".into(),
                kind: ProviderKind::Web,
                priority: 1,
                needs_key: false,
                max_chars: 5000,
                auto_detect: true,
                languages: vec!["*".into()],
                free_tier: None,
                configured: true,
            }
        }
        async fn translate_once(&self, req: &TR, _cred: &Credential) -> R<TranslateResponse> {
            Ok(TranslateResponse {
                text: "x".into(),
                detected_source: "zh-CN".into(),
                target_lang: req.target_lang.clone(),
                provider: "dialect".into(),
                engine: "dialect".into(),
                latency_ms: 0,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn detected_source_is_canonicalised() {
        let home = dir();
        let mut reg = Registry::new();
        reg.register(Arc::new(Dialect));
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let mut req = request(None);
        req.provider = Some("dialect".into());
        let out = engine.translate(&req).await.unwrap();
        assert_eq!(out.response.detected_source, "zh");
    }
}
