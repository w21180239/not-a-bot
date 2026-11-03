//! Fallback engine: chain providers, rotate keys, mask preserve spans, retry.

use crate::core::config::Settings;
use crate::core::creds::CredentialStore;
use crate::core::error::{Error, Result};
use crate::core::keys::KeyRing;
use crate::core::preserve::Preserver;
use crate::core::types::{
    AttemptRecord, RoundtripRequest, TranslateRequest, TranslateResponse,
};
use crate::providers::{Provider, Registry};
use crate::retry::backoff;
use futures::StreamExt;
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

/// Everything one provider's attempt loop produced, merged by the driver.
struct ProviderRun {
    name: String,
    /// `None` when no credential could be tried (no keys, or all cooling).
    result: Option<Result<TranslateResponse>>,
    attempts: Vec<AttemptRecord>,
    failures: Vec<String>,
    tried: bool,
    cooling: Option<(String, u64)>,
}

/// A round trip plus the per-hop trail, so `--verbose` can explain both legs.
#[derive(Debug, Clone)]
pub struct RoundtripOutcome {
    pub response: crate::core::types::RoundtripResponse,
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
        // Pinning means "this one first": racing its fallback against it could
        // return the fallback while the pinned provider was about to answer.
        let width = if pinned { 1 } else { self.settings.race_width() };

        let mut attempts = Vec::new();
        let mut failures: Vec<String> = Vec::new();
        let mut last: Option<Error> = None;
        let mut any_tried = false;
        // Providers skipped because every configured key is on cooldown.
        let mut cooling: Vec<(String, u64)> = Vec::new();

        // A sliding window of providers. One in flight is the strict sequential
        // chain; more cut tail latency because a hung provider no longer blocks
        // the ones behind it. The first success wins and the rest are cancelled.
        let runs = futures::stream::iter(chain.into_iter().map(|provider| {
            self.run_provider(provider, &inner, preserver.as_ref(), budget, budget_ms)
        }))
        .buffer_unordered(width);

        futures::pin_mut!(runs);
        while let Some(run) = runs.next().await {
            any_tried |= run.tried;
            if let Some(cooling_provider) = run.cooling {
                cooling.push(cooling_provider);
            }
            attempts.extend(run.attempts);
            failures.extend(run.failures);

            match run.result {
                None => {}
                Some(Ok(mut resp)) => {
                    resp.provider = run.name.clone();
                    resp.engine = if resp.engine.is_empty() {
                        run.name
                    } else {
                        resp.engine
                    };
                    resp.request_id = request_id.clone();
                    resp.target_lang = target.clone();
                    // Providers report their own dialect of the source code
                    // (`zh-CN`, `chi`); the wire contract promises the
                    // canonical one.
                    resp.detected_source = crate::core::lang::normalize(&resp.detected_source);
                    if resp.chars.is_none() {
                        resp.chars = Some(req.text.chars().count());
                    }
                    return Ok(Outcome { response: resp, attempts });
                }
                Some(Err(err)) => {
                    // Invalid input or a bad pattern: another provider cannot
                    // help, so stop the window instead of burning more calls.
                    if !err.skip_to_next_provider() {
                        return Err(err);
                    }
                    last = Some(err);
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

    /// Round trip: source -> pivot -> source.
    ///
    /// The second hop does the work. Sending the text through a language with a
    /// different syntax forces a rewrite; translating back cannot reuse the
    /// original phrasing, so the result loses the stock cadence that makes
    /// model-written text recognisable.
    pub async fn roundtrip(&self, req: &RoundtripRequest) -> Result<RoundtripOutcome> {
        let started = Instant::now();
        let request_id = req
            .idempotency_key
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if req.text.trim().is_empty() {
            return Err(Error::InvalidInput("text is empty".into()));
        }

        let requested_pivot = crate::core::lang::normalize(&req.pivot);
        let source = crate::core::lang::normalize(&req.source_lang);
        let mut pivot = if requested_pivot == "auto" {
            crate::core::lang::default_pivot(&source).to_string()
        } else {
            requested_pivot.clone()
        };
        if !crate::core::lang::is_known(&pivot) || pivot == "auto" {
            return Err(Error::InvalidInput(format!(
                "unknown pivot language {}",
                req.pivot
            )));
        }
        // Sending text to its own language is not a round trip.
        if pivot == source {
            pivot = crate::core::lang::alternate_pivot(&pivot).to_string();
        }

        let hop = |text: String, from: String, to: String| TranslateRequest {
            text,
            source_lang: from,
            target_lang: to,
            provider: req.provider.clone(),
            fallback_chain: req.fallback_chain.clone(),
            preserve: req.preserve.clone(),
            timeout_ms: req.timeout_ms,
            idempotency_key: Some(request_id.clone()),
            metadata: req.metadata.clone(),
        };

        let mut first = self
            .translate(&hop(req.text.clone(), source.clone(), pivot.clone()))
            .await?;
        let mut detected = crate::core::lang::normalize(&first.response.detected_source);

        // `auto` guessed Chinese because the source was unknown. If the text is
        // Chinese the guess was the source itself, so redo into English.
        if requested_pivot == "auto" && detected == pivot {
            pivot = crate::core::lang::alternate_pivot(&pivot).to_string();
            first = self
                .translate(&hop(req.text.clone(), source.clone(), pivot.clone()))
                .await?;
            detected = crate::core::lang::normalize(&first.response.detected_source);
        }

        let mut back = if detected == "auto" || !crate::core::lang::is_known(&detected) {
            crate::core::lang::default_pivot(&pivot).to_string()
        } else {
            detected.clone()
        };
        if back == pivot {
            back = crate::core::lang::alternate_pivot(&pivot).to_string();
        }

        let second = self
            .translate(&hop(first.response.text.clone(), pivot.clone(), back))
            .await?;

        let mut warnings = first.response.warnings.clone();
        warnings.extend(second.response.warnings.clone());
        let mut attempts = first.attempts;
        attempts.extend(second.attempts);

        let pivot_text = first.response.text;
        let first_provider = first.response.provider;
        let final_text = second.response.text;
        let final_provider = second.response.provider;

        Ok(RoundtripOutcome {
            response: crate::core::types::RoundtripResponse {
                text: final_text,
                detected_source: detected,
                pivot_lang: pivot,
                pivot_text,
                provider: final_provider.clone(),
                providers: vec![first_provider, final_provider],
                latency_ms: started.elapsed().as_millis() as u64,
                request_id,
                chars: Some(req.text.chars().count()),
                warnings,
            },
            attempts,
        })
    }

    /// One provider, every key, every retry: the body of the fallback chain,
    /// callable concurrently. Owns its cooldown bookkeeping so the driver only
    /// merges results.
    async fn run_provider(
        &self,
        provider: Arc<dyn Provider>,
        req: &TranslateRequest,
        preserver: Option<&Preserver>,
        budget: Duration,
        budget_ms: u64,
    ) -> ProviderRun {
        let info = provider.info();
        let mut run = ProviderRun {
            name: info.name.clone(),
            result: None,
            attempts: Vec::new(),
            failures: Vec::new(),
            tried: false,
            cooling: None,
        };

        // Key labels come from the credential store so providers stay
        // stateless about how many keys are configured.
        let labels = if info.needs_key {
            self.creds.labels(&info.name)
        } else {
            vec!["default".to_string()]
        };
        if info.needs_key && labels.is_empty() {
            return run;
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
                    run.cooling = Some((info.name.clone(), ring.next_available(now).unwrap_or(0)));
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

            run.tried = true;
            let mut attempt = 0u32;
            loop {
                let started = Instant::now();
                let result = match tokio::time::timeout(
                    budget,
                    provider.translate_once(req, &cred),
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => Err(Error::Timeout { timeout_ms: budget_ms }),
                };
                let latency_ms = started.elapsed().as_millis() as u64;

                match result {
                    Ok(mut resp) => {
                        if let Some(p) = preserver {
                            resp.text = match p.restore(&resp.text) {
                                Ok(text) => text,
                                Err(err) => {
                                    run.attempts.push(AttemptRecord {
                                        provider: info.name.clone(),
                                        key_label: Some(label.clone()),
                                        ok: false,
                                        latency_ms,
                                        error_code: Some(err.code().to_string()),
                                    });
                                    run.failures.push(format!("{}: {err}", info.name));
                                    run.result = Some(Err(err));
                                    break;
                                }
                            };
                        }
                        ring.record_success(&label);
                        run.attempts.push(AttemptRecord {
                            provider: info.name.clone(),
                            key_label: Some(label.clone()),
                            ok: true,
                            latency_ms,
                            error_code: None,
                        });
                        run.result = Some(Ok(resp));
                        return run;
                    }
                    Err(err) => {
                        ring.record_failure(&label, &err, chrono::Utc::now());
                        run.attempts.push(AttemptRecord {
                            provider: info.name.clone(),
                            key_label: Some(label.clone()),
                            ok: false,
                            latency_ms,
                            error_code: Some(err.code().to_string()),
                        });
                        run.failures.push(format!("{}: {err}", info.name));

                        // A timeout means this endpoint exceeded the budget;
                        // retrying it just burns another full budget. Move on.
                        let retry_same = err.retryable()
                            && !matches!(err, Error::Timeout { .. })
                            && attempt < self.settings.retries();
                        let fatal = !err.skip_to_next_provider();
                        let retry_after = err.retry_after_secs();
                        run.result = Some(Err(err));

                        if retry_same {
                            attempt += 1;
                            tokio::time::sleep(backoff(attempt, retry_after)).await;
                            continue;
                        }
                        if fatal {
                            return run;
                        }
                        break;
                    }
                }
            }
        }
        run
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
    /// Sequential by default in tests: these cases assert chain order.
    fn test_settings() -> Settings {
        Settings {
            retries: Some(0),
            race_width: Some(1),
            ..Default::default()
        }
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

    /// Answers after `delay`, so a race is observable on the wall clock.
    struct Delayed {
        name: &'static str,
        delay: Duration,
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl Provider for Delayed {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                display_name: self.name.into(),
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
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(self.delay).await;
            Ok(TranslateResponse {
                text: format!("{}:{}", self.name, req.text),
                detected_source: req.source_lang.clone(),
                target_lang: req.target_lang.clone(),
                provider: self.name.into(),
                engine: self.name.into(),
                latency_ms: self.delay.as_millis() as u64,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    fn delayed(name: &'static str, delay_ms: u64) -> (Arc<Delayed>, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Arc::new(Delayed {
                name,
                delay: Duration::from_millis(delay_ms),
                calls: calls.clone(),
            }),
            calls,
        )
    }

    fn raced(settings: Settings) -> Settings {
        Settings { chain: Some(vec!["slow".into(), "fast".into()]), ..settings }
    }

    #[tokio::test]
    async fn race_returns_the_first_success() {
        let home = dir();
        let (slow, slow_calls) = delayed("slow", 300);
        let (fast, fast_calls) = delayed("fast", 5);
        let mut reg = Registry::new();
        reg.register(slow);
        reg.register(fast);
        let settings = raced(Settings {
            retries: Some(0),
            race_width: Some(2),
            ..Default::default()
        });
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            settings,
        );

        let started = Instant::now();
        let out = engine.translate(&request(None)).await.expect("translate");
        let elapsed = started.elapsed();

        assert_eq!(out.response.provider, "fast");
        assert!(
            elapsed < Duration::from_millis(250),
            "racing must not wait for the slow provider, took {elapsed:?}"
        );
        assert_eq!(fast_calls.load(Ordering::SeqCst), 1);
        assert_eq!(slow_calls.load(Ordering::SeqCst), 1, "the loser was started");
    }

    #[tokio::test]
    async fn race_width_one_walks_the_chain_in_order() {
        let home = dir();
        let (slow, _) = delayed("slow", 300);
        let (fast, fast_calls) = delayed("fast", 5);
        let mut reg = Registry::new();
        reg.register(slow);
        reg.register(fast);
        let settings = raced(Settings {
            retries: Some(0),
            race_width: Some(1),
            ..Default::default()
        });
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            settings,
        );

        let started = Instant::now();
        let out = engine.translate(&request(None)).await.expect("translate");

        assert_eq!(out.response.provider, "slow", "first in the chain wins");
        assert_eq!(fast_calls.load(Ordering::SeqCst), 0, "never reached");
        assert!(started.elapsed() >= Duration::from_millis(250));
    }

    #[tokio::test]
    async fn pinned_provider_is_not_raced_against_its_fallback() {
        let home = dir();
        let (slow, _) = delayed("slow", 60);
        let (fast, fast_calls) = delayed("fast", 5);
        let mut reg = Registry::new();
        reg.register(slow);
        reg.register(fast);
        let settings = raced(Settings {
            retries: Some(0),
            race_width: Some(3),
            ..Default::default()
        });
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            settings,
        );

        let mut req = request(Some("slow"));
        req.fallback_chain = vec!["fast".into()];
        let out = engine.translate(&req).await.expect("translate");

        assert_eq!(out.response.provider, "slow", "pinning wins over speed");
        assert_eq!(fast_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn race_refills_the_window_after_a_failure() {
        let home = dir();
        // Keyless: one attempt, then the provider is done.
        let (bad, bad_calls) = stub("free", vec![Some("up")]);
        let (good, good_calls) = delayed("slow", 50);
        let mut reg = Registry::new();
        reg.register(bad);
        reg.register(good);
        let settings = Settings {
            chain: Some(vec!["free".into(), "slow".into()]),
            retries: Some(0),
            race_width: Some(1),
            ..Default::default()
        };
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            settings,
        );

        let out = engine.translate(&request(None)).await.expect("translate");

        assert_eq!(out.response.provider, "slow");
        assert_eq!(bad_calls.load(Ordering::SeqCst), 1);
        assert_eq!(good_calls.load(Ordering::SeqCst), 1);
        assert_eq!(out.attempts.len(), 2, "the failed hop is still recorded");
        assert!(!out.attempts[0].ok);
        assert!(out.attempts[1].ok);
    }

    /// Records every hop and rewrites the text, so the round trip is visible.
    struct Recorder {
        calls: Arc<std::sync::Mutex<Vec<(String, String, String)>>>,
        detected: &'static str,
    }

    #[async_trait]
    impl Provider for Recorder {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "recorder".into(),
                display_name: "recorder".into(),
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
            self.calls.lock().unwrap().push((
                req.source_lang.clone(),
                req.target_lang.clone(),
                req.text.clone(),
            ));
            Ok(TranslateResponse {
                text: format!("{}({})", req.text, req.target_lang),
                detected_source: self.detected.into(),
                target_lang: req.target_lang.clone(),
                provider: "recorder".into(),
                engine: "recorder".into(),
                latency_ms: 0,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    fn recorder(
        detected: &'static str,
    ) -> (Arc<Recorder>, Arc<std::sync::Mutex<Vec<(String, String, String)>>>) {
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        (
            Arc::new(Recorder {
                calls: calls.clone(),
                detected,
            }),
            calls,
        )
    }

    fn roundtrip_req(source: &str, pivot: &str) -> crate::core::types::RoundtripRequest {
        crate::core::types::RoundtripRequest {
            text: "hello".into(),
            source_lang: source.into(),
            pivot: pivot.into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: Some("rt-1".into()),
            metadata: Default::default(),
        }
    }

    #[tokio::test]
    async fn roundtrip_goes_out_through_the_pivot_and_back() {
        let home = dir();
        let (rec, calls) = recorder("en");
        let mut reg = Registry::new();
        reg.register(rec);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let out = engine
            .roundtrip(&roundtrip_req("en", "auto"))
            .await
            .expect("roundtrip");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2, "one hop out, one hop back");
        assert_eq!(calls[0].0, "en");
        assert_eq!(calls[0].1, "zh", "English goes out through Chinese");
        assert_eq!(calls[1].0, "zh");
        assert_eq!(calls[1].1, "en");

        assert_eq!(out.response.pivot_lang, "zh");
        assert_eq!(out.response.detected_source, "en");
        assert_eq!(out.response.pivot_text, "hello(zh)");
        assert_eq!(out.response.text, "hello(zh)(en)");
        assert_eq!(out.response.providers, vec!["recorder", "recorder"]);
        assert_eq!(out.response.request_id, "rt-1");
        assert_eq!(out.attempts.len(), 2);
    }

    #[tokio::test]
    async fn roundtrip_auto_pivot_uses_english_for_chinese() {
        let home = dir();
        let (rec, calls) = recorder("zh");
        let mut reg = Registry::new();
        reg.register(rec);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let out = engine
            .roundtrip(&roundtrip_req("zh", "auto"))
            .await
            .expect("roundtrip");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1, "en", "Chinese goes out through English");
        assert_eq!(calls[1].1, "zh");
        assert_eq!(out.response.pivot_lang, "en");
    }

    #[tokio::test]
    async fn roundtrip_retries_through_english_when_auto_guesses_the_source() {
        let home = dir();
        // `auto` starts with zh; the provider reports the text is Chinese, so
        // the guessed pivot was the source and the hop is redone into English.
        let (rec, calls) = recorder("zh");
        let mut reg = Registry::new();
        reg.register(rec);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let out = engine
            .roundtrip(&roundtrip_req("auto", "auto"))
            .await
            .expect("roundtrip");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 3, "zh guess, redo into en, then back");
        assert_eq!(calls[0].1, "zh");
        assert_eq!(calls[1].1, "en");
        assert_eq!(calls[2].1, "zh");
        assert_eq!(out.response.pivot_lang, "en");
        assert_eq!(out.response.detected_source, "zh");
    }

    #[tokio::test]
    async fn roundtrip_moves_a_pivot_that_equals_the_source() {
        let home = dir();
        let (rec, calls) = recorder("en");
        let mut reg = Registry::new();
        reg.register(rec);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        engine
            .roundtrip(&roundtrip_req("en", "en"))
            .await
            .expect("roundtrip");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls[0].1, "zh", "a source-language pivot is unusable");
    }

    #[tokio::test]
    async fn roundtrip_rejects_an_unknown_pivot() {
        let home = dir();
        let (rec, _) = recorder("en");
        let mut reg = Registry::new();
        reg.register(rec);
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );

        let err = engine
            .roundtrip(&roundtrip_req("en", "klingon"))
            .await
            .expect_err("unknown pivot");
        assert_eq!(err.code(), "INVALID_INPUT");
    }
}
