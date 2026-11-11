//! Fallback engine: chain providers, rotate keys, mask preserve spans, retry.

use crate::core::config::Settings;
use crate::core::creds::CredentialStore;
use crate::core::error::{Error, Result};
use crate::core::keys::KeyRing;
use crate::core::preserve::Preserver;
use crate::core::types::{
    AttemptRecord, Hop, ProviderKind, RewriteRequest, TranslateRequest, TranslateResponse,
};
use crate::providers::{Provider, Registry};
use crate::retry::backoff;
use futures::StreamExt;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Minimum time a scorer call gets, whatever the request timeout.
const SCORER_TIMEOUT_FLOOR_MS: u64 = 120_000;

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

/// A chain that produced no translation, with the hops that were tried.
#[derive(Debug)]
pub struct ChainFailure {
    pub error: Error,
    pub attempts: Vec<AttemptRecord>,
}

impl ChainFailure {
    fn new(error: Error) -> Self {
        Self {
            error,
            attempts: Vec::new(),
        }
    }
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

/// Preserve spans of one request: the masked payload for MT providers and the
/// verbatim span list for LLM providers.
struct Preserve {
    preserver: Preserver,
    masked: String,
    spans: Vec<String>,
}

/// A round trip plus the per-hop trail, so `--verbose` can explain both legs.
#[derive(Debug, Clone)]
pub struct RewriteOutcome {
    pub response: crate::core::types::RewriteResponse,
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
        Self {
            registry,
            creds,
            cache_dir,
            settings,
            chain,
        }
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
        self.translate_inner(req)
            .await
            .map_err(|failure| failure.error)
    }

    /// Translate and keep the hop records when the chain fails, so a caller can
    /// report which providers were consulted and why each one declined.
    pub async fn translate_with_attempts(
        &self,
        req: &TranslateRequest,
    ) -> (Result<Outcome>, Vec<AttemptRecord>) {
        match self.translate_inner(req).await {
            Ok(outcome) => (Ok(outcome), Vec::new()),
            Err(failure) => (Err(failure.error), failure.attempts),
        }
    }

    async fn translate_inner(
        &self,
        req: &TranslateRequest,
    ) -> std::result::Result<Outcome, ChainFailure> {
        let request_id = req
            .idempotency_key
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if req.text.trim().is_empty() {
            return Err(ChainFailure::new(Error::InvalidInput(
                "text is empty".into(),
            )));
        }
        let target = crate::core::lang::normalize(&req.target_lang);
        let source = crate::core::lang::normalize(&req.source_lang);
        if !crate::core::lang::is_known(&target) {
            return Err(ChainFailure::new(Error::InvalidInput(format!(
                "unknown target language {}",
                req.target_lang
            ))));
        }
        if target == source {
            return Err(ChainFailure::new(Error::InvalidInput(format!(
                "source and target are both {target}"
            ))));
        }
        let mut inner = req.clone();
        inner.target_lang = target.clone();
        inner.source_lang = source.clone();

        // MT providers get placeholders; LLM providers get the real text plus
        // the span list, because they mangle placeholders but keep real code.
        let preserve = Preserver::new(&req.preserve)
            .map_err(ChainFailure::new)?
            .map(|mut preserver| {
                let masked = preserver.mask(&inner.text);
                let spans = preserver.spans(&inner.text);
                Preserve {
                    preserver,
                    masked,
                    spans,
                }
            });

        let chain = self
            .registry
            .chain(
                req.provider.as_deref(),
                &req.fallback_chain,
                &self.chain,
                &req.source_lang,
                &target,
            )
            .map_err(ChainFailure::new)?;
        let pinned = req.provider.is_some();
        // Per-request timeout wins; otherwise the configured default, raised
        // to the provider's own floor for slow ones (local agent CLIs).
        let base_ms = self.settings.timeout_ms.unwrap_or(20_000);
        let budget_for = |provider: &Arc<dyn Provider>| {
            req.timeout_ms.unwrap_or_else(|| {
                provider
                    .default_timeout_ms()
                    .map_or(base_ms, |floor| floor.max(base_ms))
            })
        };
        // Pinning means "this one first": racing its fallback against it could
        // return the fallback while the pinned provider was about to answer.
        let width = if pinned {
            1
        } else {
            self.settings.race_width()
        };

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
            let budget_ms = budget_for(&provider);
            self.run_provider(
                provider,
                &inner,
                preserve.as_ref(),
                Duration::from_millis(budget_ms),
                budget_ms,
            )
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
                    return Ok(Outcome {
                        response: resp,
                        attempts,
                    });
                }
                Some(Err(err)) => {
                    // Invalid input or a bad pattern: another provider cannot
                    // help, so stop the window instead of burning more calls.
                    if !err.skip_to_next_provider() {
                        return Err(ChainFailure {
                            error: err,
                            attempts,
                        });
                    }
                    last = Some(err);
                }
            }
        }

        let error = if !any_tried {
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
        };
        Err(ChainFailure { error, attempts })
    }

    /// Round trip with the request's scoring options: the scorer command
    /// comes from the request, else the configured `rewrite_scorer`.
    pub async fn rewrite(&self, req: &RewriteRequest) -> Result<RewriteOutcome> {
        use crate::core::scoring::{CommandScorer, Scorer};
        let command = req
            .scorer
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .or_else(|| self.settings.rewrite_scorer());
        // Local detectors load a model per call (10+ s on CPU), so the scorer
        // gets the same kind of floor as the local agent CLIs.
        let timeout_ms = req
            .timeout_ms
            .unwrap_or_else(|| self.settings.timeout().as_millis() as u64)
            .max(SCORER_TIMEOUT_FLOOR_MS);
        let scorer = command
            .map(|c| CommandScorer::parse(&c, timeout_ms))
            .transpose()?;
        self.rewrite_scored(req, scorer.as_ref().map(|s| s as &dyn Scorer))
            .await
    }

    /// Round trip scored by `scorer`: `bestOf` runs per rung, facts first,
    /// then lowest score; `targetScore` climbs the configured ladder.
    pub async fn rewrite_scored(
        &self,
        req: &RewriteRequest,
        scorer: Option<&dyn crate::core::scoring::Scorer>,
    ) -> Result<RewriteOutcome> {
        crate::core::scoring::search(req, &self.settings, scorer, |r| async move {
            self.rewrite_once(&r).await
        })
        .await
    }

    /// One run of the planned chain: source -> hop -> ... -> source.
    ///
    /// Each hop forces the text through a different syntax, so translating
    /// back cannot reuse the original phrasing. Machine hops rewrite words but
    /// keep sentence shapes; LLM hops (humanize style) rewrite the shapes too,
    /// and a later LLM hop sees the previous one as history so it does not
    /// undo it. The output is checked for facts the input had and lost.
    async fn rewrite_once(&self, req: &RewriteRequest) -> Result<RewriteOutcome> {
        use crate::core::chain::{plan, resolve_source, Target};
        use crate::providers::cli_llm::{
            HISTORY_KEY, MISSING_KEY, ORIGINAL_KEY, ORIGINAL_LANG_KEY, STYLE_KEY,
        };

        let started = Instant::now();
        let request_id = req
            .idempotency_key
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if req.text.trim().is_empty() {
            return Err(Error::InvalidInput("text is empty".into()));
        }
        let source = crate::core::lang::normalize(&req.source_lang);
        let is_llm = |name: &str| {
            self.registry
                .get(name)
                .is_some_and(|p| p.info().kind == ProviderKind::Llm)
        };
        let mut plan = plan(req, &self.settings, &source, &is_llm)?;

        let mut hops: Vec<Hop> = Vec::new();
        // Languages visited; a repair pass shows as `fix`.
        let mut chain: Vec<String> = Vec::new();
        let mut attempts = Vec::new();
        let mut warnings = Vec::new();
        let mut text = req.text.clone();
        let mut from = source.clone();
        // Known source language; `auto` until the first hop detects it.
        let mut detected = source.clone();
        // Input and output of the previous hop when it was an LLM hop.
        let mut llm_history: Option<(String, String)> = None;
        let mut i = 0;
        while i < plan.hops.len() {
            let spec = plan.hops[i].clone();
            let first_lang = plan
                .hops
                .iter()
                .find_map(|h| match &h.to {
                    Target::Lang(l) => Some(l.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| source.clone());
            let fix = spec.to == Target::Fix;
            let to = match &spec.to {
                Target::Lang(l) => l.clone(),
                Target::Fix => {
                    if from == "auto" {
                        return Err(Error::InvalidInput(
                            "a fix hop needs a known language; pass sourceLang".into(),
                        ));
                    }
                    from.clone()
                }
                Target::Source => {
                    if plan.appended_back && i > 1 && from == detected {
                        // An explicit chain that already ended in the source.
                        break;
                    }
                    resolve_source(&detected, &first_lang, &from)
                }
            };
            let llm_hop = spec.engine.as_deref().is_some_and(is_llm);
            let mut metadata = req.metadata.clone();
            if llm_hop {
                metadata.insert(
                    STYLE_KEY.into(),
                    if fix { "fix" } else { "humanize" }.into(),
                );
                // The first hop's input is the original already.
                if !hops.is_empty() {
                    metadata.insert(ORIGINAL_KEY.into(), req.text.clone().into());
                    metadata.insert(ORIGINAL_LANG_KEY.into(), detected.clone().into());
                }
                if fix {
                    let missing = crate::core::facts::missing_facts(&req.text, &text);
                    metadata.insert(MISSING_KEY.into(), serde_json::json!(missing));
                }
                if let (false, Some((input, output))) = (fix, &llm_history) {
                    metadata.insert(
                        HISTORY_KEY.into(),
                        serde_json::json!({ "input": input, "output": output }),
                    );
                }
            }
            let hop_req = TranslateRequest {
                text: text.clone(),
                // A repair stays in one language; the engine rejects equal
                // source and target, and the prompt only needs the target.
                source_lang: if fix { "auto".into() } else { from.clone() },
                target_lang: to.clone(),
                provider: spec.engine.clone().or_else(|| req.provider.clone()),
                fallback_chain: req.fallback_chain.clone(),
                preserve: req.preserve.clone(),
                timeout_ms: req.timeout_ms,
                idempotency_key: Some(request_id.clone()),
                metadata,
            };
            let hop_started = Instant::now();
            // A named MT engine is preferred, not pinned: a retryable failure
            // hands the hop to the auto chain. LLM hops stay strict.
            let mut fallback_note = None;
            let out = match self.translate_with_attempts(&hop_req).await {
                (Ok(out), _) => out,
                (Err(err), failed)
                    if !llm_hop
                        && spec.engine.is_some()
                        && (err.retryable() || matches!(err, Error::QuotaExhausted { .. })) =>
                {
                    let engine = spec.engine.clone().unwrap_or_default();
                    let prev = hops.last().map(|h| h.provider.as_str());
                    let next = plan.hops.get(i + 1).and_then(|h| h.engine.as_deref());
                    let names = self.hop_fallback(&hop_req, &engine, prev, next);
                    let Some((first, rest)) = names.split_first() else {
                        return Err(err);
                    };
                    let mut retry = hop_req.clone();
                    retry.provider = Some(first.clone());
                    retry.fallback_chain = rest.to_vec();
                    let out = self.translate(&retry).await?;
                    attempts.extend(failed);
                    fallback_note = Some(format!(
                        "hop {}: {engine} failed ({}), served by {}",
                        i + 1,
                        err.code(),
                        out.response.provider
                    ));
                    out
                }
                (Err(err), _) => return Err(err),
            };
            // An LLM sometimes answers a hop back to a Latin-script language in
            // the pivot's script; retry once, then fail rather than pass it on.
            let out = if llm_hop && !fix && cjk_for_latin_target(&to, &out.response.text) {
                let again = self.translate(&hop_req).await?;
                if cjk_for_latin_target(&to, &again.response.text) {
                    return Err(Error::Upstream {
                        provider: again.response.provider.clone(),
                        status: 502,
                        body: format!("hop {}: output is not in {to}", i + 1),
                    });
                }
                warnings.push(format!("hop {}: output was not in {to}; retried", i + 1));
                again
            } else {
                out
            };
            let latency_ms = hop_started.elapsed().as_millis() as u64;

            if i == 0 && source == "auto" {
                let mut found = crate::core::lang::normalize(&out.response.detected_source);
                // LLM CLIs echo `auto`; the script of the input is enough to
                // pick the return language and spot a Chinese source.
                if found == "auto" {
                    found = crate::core::lang::detect_script(&text)
                        .unwrap_or("auto")
                        .to_string();
                }
                // The plan guessed a non-Chinese source and went out through
                // the source itself: rebuild for the detected language and redo.
                if plan.guessed && found == to {
                    plan = crate::core::chain::plan(req, &self.settings, &found, &is_llm)?;
                    plan.guessed = false;
                    detected = found;
                    attempts.extend(out.attempts);
                    continue;
                }
                detected = found;
            }

            attempts.extend(out.attempts);
            warnings.extend(fallback_note);
            warnings.extend(out.response.warnings.clone());
            // A fix hop answers with span edits; only vetted ones touch the
            // text, and unusable output leaves it as it was.
            let (hop_text, edits) = if fix {
                match crate::core::repair::parse_edits(&out.response.text) {
                    Ok(proposed) => {
                        let (fixed, edits) =
                            crate::core::repair::apply_edits(&text, &req.text, proposed);
                        for e in edits.iter().filter(|e| !e.applied) {
                            warnings.push(format!(
                                "fix: skipped edit {:?} -> {:?}: {}",
                                e.from,
                                e.to,
                                e.reason.as_deref().unwrap_or("rejected")
                            ));
                        }
                        (fixed, Some(edits))
                    }
                    Err(reason) => {
                        warnings.push(format!("fix: no edits applied: {reason}"));
                        (text.clone(), Some(Vec::new()))
                    }
                }
            } else {
                (out.response.text.clone(), None)
            };
            llm_history = (llm_hop && !fix).then(|| (text.clone(), hop_text.clone()));
            chain.push(if fix { "fix".into() } else { to.clone() });
            hops.push(Hop {
                from: from.clone(),
                to: to.clone(),
                provider: out.response.provider.clone(),
                text: hop_text.clone(),
                latency_ms,
                edits,
            });
            text = hop_text;
            from = to;
            i += 1;
        }

        let missing_facts = crate::core::facts::missing_facts(&req.text, &text);
        if !missing_facts.is_empty() {
            warnings.push(format!("fact check: missing {}", missing_facts.join(", ")));
        }
        let provider = hops.last().map(|h| h.provider.clone()).unwrap_or_default();
        Ok(RewriteOutcome {
            response: crate::core::types::RewriteResponse {
                text,
                detected_source: from,
                chain,
                hops,
                provider,
                missing_facts,
                latency_ms: started.elapsed().as_millis() as u64,
                request_id,
                chars: Some(req.text.chars().count()),
                warnings,
                score: None,
                scorer: None,
                candidates: Vec::new(),
            },
            attempts,
        })
    }

    /// Auto chain for a hop whose named engine failed: no LLM providers, not
    /// the failed engine, and not the previous hop's provider unless nothing
    /// else can serve the pair.
    fn hop_fallback(
        &self,
        req: &TranslateRequest,
        failed: &str,
        prev: Option<&str>,
        next: Option<&str>,
    ) -> Vec<String> {
        let failed = self
            .registry
            .get(failed)
            .map_or_else(|| failed.to_string(), |p| p.info().name);
        let Ok(auto) = self.registry.chain(
            None,
            &req.fallback_chain,
            &self.chain,
            &req.source_lang,
            &req.target_lang,
        ) else {
            return Vec::new();
        };
        let names: Vec<String> = auto
            .iter()
            .map(|p| p.info())
            .filter(|info| info.kind != ProviderKind::Llm && info.name != failed)
            .map(|info| info.name)
            .collect();
        let fresh: Vec<String> = names
            .iter()
            .filter(|n| Some(n.as_str()) != prev && Some(n.as_str()) != next)
            .cloned()
            .collect();
        if fresh.is_empty() {
            names
        } else {
            fresh
        }
    }

    /// One provider, every key, every retry: the body of the fallback chain,
    /// callable concurrently. Owns its cooldown bookkeeping so the driver only
    /// merges results.
    async fn run_provider(
        &self,
        provider: Arc<dyn Provider>,
        req: &TranslateRequest,
        preserve: Option<&Preserve>,
        budget: Duration,
        budget_ms: u64,
    ) -> ProviderRun {
        use crate::providers::cli_llm::{PRESERVE_KEY, STYLE_KEY};
        use std::borrow::Cow;

        let info = provider.info();
        let llm = info.kind == ProviderKind::Llm;
        // A fix hop answers with edits, not the text, so spans cannot be checked.
        let fix = req.metadata.get(STYLE_KEY).and_then(|v| v.as_str()) == Some("fix");
        let (req, spans): (Cow<TranslateRequest>, &[String]) = match preserve {
            None => (Cow::Borrowed(req), &[]),
            Some(_) if llm && fix => (Cow::Borrowed(req), &[]),
            Some(p) if llm => {
                let mut r = req.clone();
                r.metadata
                    .insert(PRESERVE_KEY.into(), serde_json::json!(p.spans));
                (Cow::Owned(r), &p.spans)
            }
            Some(p) => {
                let mut r = req.clone();
                r.text = p.masked.clone();
                (Cow::Owned(r), &[])
            }
        };
        let req = req.as_ref();
        let preserver = preserve.filter(|_| !llm).map(|p| &p.preserver);
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
                let result =
                    match tokio::time::timeout(budget, provider.translate_once(req, &cred)).await {
                        Ok(r) => r,
                        Err(_) => Err(Error::Timeout {
                            timeout_ms: budget_ms,
                        }),
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
                        for span in spans.iter().filter(|s| !resp.text.contains(s.as_str())) {
                            resp.warnings
                                .push(format!("preserve: {} dropped {span}", info.name));
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

/// True when `text` is mostly Chinese, Japanese or Korean script although
/// `target` is written in Latin script.
fn cjk_for_latin_target(target: &str, text: &str) -> bool {
    let base = target.split('-').next().unwrap_or(target);
    let non_latin = [
        "zh", "ja", "ko", "ru", "uk", "bg", "sr", "be", "mk", "kk", "el", "he", "ar", "fa", "hi",
        "th",
    ];
    !non_latin.contains(&base)
        && matches!(
            crate::core::lang::detect_script(text),
            Some("zh" | "ja" | "ko")
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::creds::Credential;
    use crate::core::error::Result as R;
    use crate::core::types::{ProviderInfo, ProviderKind};
    use crate::core::TranslateRequest as TR;
    use crate::providers::Provider;
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
                kind: if self.name == "free" {
                    ProviderKind::Web
                } else {
                    ProviderKind::Official
                },
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
                Some("auth") => Err(Error::AuthFailed {
                    provider: self.name.into(),
                    reason: "bad".into(),
                }),
                Some("rate") => Err(Error::RateLimited {
                    provider: self.name.into(),
                    retry_after_secs: Some(1),
                }),
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
            Arc::new(Stub {
                name,
                calls: calls.clone(),
                script,
            }),
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
            rewrite_preset: Some("light".into()),
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
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "no request for missing keys"
        );
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
        Settings {
            chain: Some(vec!["slow".into(), "fast".into()]),
            ..settings
        }
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
        assert_eq!(
            slow_calls.load(Ordering::SeqCst),
            1,
            "the loser was started"
        );
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

    #[tokio::test]
    async fn a_failed_chain_still_reports_its_hops() {
        let home = dir();
        let (bad, _calls) = stub("free", vec![Some("up")]);
        let mut reg = Registry::new();
        reg.register(bad);
        let settings = Settings {
            chain: Some(vec!["free".into()]),
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

        let (result, attempts) = engine.translate_with_attempts(&request(None)).await;

        assert!(result.is_err(), "the only provider fails");
        assert_eq!(attempts.len(), 1, "the failed hop travels with the error");
        assert!(!attempts[0].ok);
        assert_eq!(attempts[0].provider, "free");
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

    type RecordedCalls = Arc<std::sync::Mutex<Vec<(String, String, String)>>>;

    fn recorder(detected: &'static str) -> (Arc<Recorder>, RecordedCalls) {
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        (
            Arc::new(Recorder {
                calls: calls.clone(),
                detected,
            }),
            calls,
        )
    }

    fn single_hop(source: &str) -> crate::core::types::RewriteRequest {
        crate::core::types::RewriteRequest {
            depth: Some(1),
            ..rewrite_req(source, "auto")
        }
    }

    fn rewrite_req(source: &str, pivot: &str) -> crate::core::types::RewriteRequest {
        crate::core::types::RewriteRequest {
            text: "hello".into(),
            source_lang: source.into(),
            pivot: pivot.into(),
            chain: Vec::new(),
            depth: None,
            preset: None,
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: None,
            idempotency_key: Some("rt-1".into()),
            metadata: Default::default(),
            scorer: None,
            best_of: None,
            target_score: None,
        }
    }

    #[tokio::test]
    async fn rewrite_goes_out_through_the_pivot_and_back() {
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
            .rewrite(&rewrite_req("en", "auto"))
            .await
            .expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 3, "two hops out, one hop back");
        assert_eq!(calls[0].0, "en");
        assert_eq!(calls[0].1, "zh", "English goes out through Chinese");
        assert_eq!(calls[1].0, "zh");
        assert_eq!(calls[1].1, "ja");
        assert_eq!(calls[2].0, "ja");
        assert_eq!(calls[2].1, "en");

        assert_eq!(out.response.chain, vec!["zh", "ja", "en"]);
        assert_eq!(out.response.detected_source, "en");
        assert_eq!(out.response.hops[0].text, "hello(zh)");
        assert_eq!(out.response.text, "hello(zh)(ja)(en)");
        let providers: Vec<_> = out
            .response
            .hops
            .iter()
            .map(|h| h.provider.as_str())
            .collect();
        assert_eq!(providers, vec!["recorder"; 3]);
        assert!(out.response.missing_facts.is_empty());
        assert_eq!(out.response.request_id, "rt-1");
        assert_eq!(out.attempts.len(), 3);
    }

    #[tokio::test]
    async fn rewrite_pivot_from_config_replaces_the_guess() {
        let home = dir();
        let (rec, calls) = recorder("en");
        let mut reg = Registry::new();
        reg.register(rec);
        let settings = Settings {
            rewrite_pivot: Some("ja".into()),
            ..test_settings()
        };
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            settings,
        );

        engine.rewrite(&single_hop("en")).await.expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls[0].1, "ja", "the configured pivot beats the default");
        assert_eq!(calls[1].1, "en");
    }

    #[tokio::test]
    async fn rewrite_auto_pivot_uses_english_for_chinese() {
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
            .rewrite(&rewrite_req("zh", "auto"))
            .await
            .expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1, "en", "Chinese goes out through English");
        assert_eq!(calls[1].1, "ja");
        assert_eq!(calls[2].1, "zh");
        assert_eq!(out.response.chain, vec!["en", "ja", "zh"]);
    }

    #[tokio::test]
    async fn rewrite_retries_through_english_when_auto_guesses_the_source() {
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
            .rewrite(&rewrite_req("auto", "auto"))
            .await
            .expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 4, "zh guess, redo into en, ja, then back");
        assert_eq!(calls[0].1, "zh");
        assert_eq!(calls[1].1, "en");
        assert_eq!(calls[2].1, "ja");
        assert_eq!(calls[3].1, "zh");
        assert_eq!(out.response.chain, vec!["en", "ja", "zh"]);
        assert_eq!(out.response.detected_source, "zh");
    }

    #[tokio::test]
    async fn rewrite_moves_a_pivot_that_equals_the_source() {
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
            .rewrite(&rewrite_req("en", "en"))
            .await
            .expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls[0].1, "zh", "a source-language pivot is unusable");
    }

    #[tokio::test]
    async fn rewrite_rejects_an_unknown_pivot() {
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
            .rewrite(&rewrite_req("en", "klingon"))
            .await
            .expect_err("unknown pivot");
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    type MetaCalls = Arc<std::sync::Mutex<Vec<(String, BTreeMap<String, serde_json::Value>)>>>;

    /// Records target language and metadata per call; `kind` decides whether
    /// the rewrite treats it as an LLM hop.
    struct MetaRecorder {
        name: &'static str,
        kind: ProviderKind,
        calls: MetaCalls,
    }

    #[async_trait]
    impl Provider for MetaRecorder {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                display_name: self.name.into(),
                kind: self.kind,
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
            self.calls
                .lock()
                .unwrap()
                .push((req.target_lang.clone(), req.metadata.clone()));
            let fix = req
                .metadata
                .get("not-a-bot.style")
                .is_some_and(|s| s == "fix");
            Ok(TranslateResponse {
                text: if fix {
                    r#"```json
[{"from": "hello", "to": "hullo", "why": "spelling"}, {"from": "zz", "to": "y"}]
```"#
                        .into()
                } else {
                    format!("{}>{}", req.text, req.target_lang)
                },
                detected_source: req.source_lang.clone(),
                target_lang: req.target_lang.clone(),
                provider: self.name.into(),
                engine: self.name.into(),
                latency_ms: 0,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    /// LLM that answers the first hop back to English in Chinese.
    struct WrongScriptOnce {
        en_calls: std::sync::Mutex<usize>,
        always: bool,
    }

    #[async_trait]
    impl Provider for WrongScriptOnce {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: "llm-w".into(),
                display_name: "llm-w".into(),
                kind: ProviderKind::Llm,
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
            let text = if req.target_lang == "en" {
                let mut n = self.en_calls.lock().unwrap();
                *n += 1;
                if *n == 1 || self.always {
                    "你好，世界，这是中文。".to_string()
                } else {
                    "hello again".to_string()
                }
            } else {
                format!("{}>{}", req.text, req.target_lang)
            };
            Ok(TranslateResponse {
                text,
                detected_source: req.source_lang.clone(),
                target_lang: req.target_lang.clone(),
                provider: "llm-w".into(),
                engine: "llm-w".into(),
                latency_ms: 0,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn hop_back_in_the_wrong_script_is_retried_then_rejected() {
        for always in [false, true] {
            let home = dir();
            let mut reg = Registry::new();
            reg.register(Arc::new(WrongScriptOnce {
                en_calls: std::sync::Mutex::new(0),
                always,
            }));
            let engine = Engine::new(
                Arc::new(reg),
                CredentialStore::default(),
                Some(home.path().to_path_buf()),
                test_settings(),
            );
            let mut req = rewrite_req("en", "auto");
            req.chain = vec!["llm-w:zh".into(), "llm-w:en".into()];
            let res = engine.rewrite(&req).await;
            if always {
                assert_eq!(res.unwrap_err().code(), "UPSTREAM_ERROR");
            } else {
                let out = res.expect("retried hop succeeds");
                assert_eq!(out.response.text, "hello again");
                assert!(out.response.warnings.iter().any(|w| w.contains("retried")));
            }
        }
    }

    #[test]
    fn cjk_guard_only_fires_for_latin_targets() {
        assert!(cjk_for_latin_target("en", "你好，世界，这是中文。"));
        assert!(!cjk_for_latin_target("en", "hello world"));
        assert!(!cjk_for_latin_target("zh", "你好，世界"));
        assert!(!cjk_for_latin_target("ja", "こんにちは世界"));
    }

    #[tokio::test]
    async fn chain_rewrite_marks_llm_hops_and_carries_history() {
        let home = dir();
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut reg = Registry::new();
        reg.register(Arc::new(MetaRecorder {
            name: "llm-a",
            kind: ProviderKind::Llm,
            calls: calls.clone(),
        }));
        reg.register(Arc::new(MetaRecorder {
            name: "mt-a",
            kind: ProviderKind::Web,
            calls: calls.clone(),
        }));
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let mut req = rewrite_req("en", "auto");
        req.text = "Ask Aroha".into();
        req.chain = vec![
            "llm-a:zh".into(),
            "llm-a:ja".into(),
            "mt-a:fi".into(),
            "mt-a:en".into(),
        ];

        let out = engine.rewrite(&req).await.expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        let targets: Vec<_> = calls.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(targets, vec!["zh", "ja", "fi", "en"]);
        assert_eq!(calls[0].1["not-a-bot.style"], "humanize");
        assert!(!calls[0].1.contains_key("not-a-bot.history"));
        assert!(
            !calls[0].1.contains_key("not-a-bot.original"),
            "the first hop reads the original itself"
        );
        assert_eq!(calls[1].1["not-a-bot.original"], "Ask Aroha");
        assert_eq!(calls[1].1["not-a-bot.original_lang"], "en");
        assert_eq!(
            calls[1].1["not-a-bot.history"],
            serde_json::json!({"input": "Ask Aroha", "output": "Ask Aroha>zh"})
        );
        assert!(calls[2].1.is_empty(), "MT hops get no style");
        assert_eq!(out.response.chain, vec!["zh", "ja", "fi", "en"]);
        let providers: Vec<_> = out
            .response
            .hops
            .iter()
            .map(|h| h.provider.as_str())
            .collect();
        assert_eq!(providers, vec!["llm-a", "llm-a", "mt-a", "mt-a"]);
        assert_eq!(out.response.hops[1].from, "zh");
        assert_eq!(out.response.text, "Ask Aroha>zh>ja>fi>en");
        assert!(out.response.missing_facts.is_empty());
    }

    #[tokio::test]
    async fn trailing_fix_repairs_in_place_without_a_return_hop() {
        let home = dir();
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut reg = Registry::new();
        reg.register(Arc::new(MetaRecorder {
            name: "llm-a",
            kind: ProviderKind::Llm,
            calls: calls.clone(),
        }));
        reg.register(Arc::new(MetaRecorder {
            name: "mt-a",
            kind: ProviderKind::Web,
            calls: calls.clone(),
        }));
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(home.path().to_path_buf()),
            test_settings(),
        );
        let mut req = rewrite_req("en", "auto");
        req.chain = vec!["mt-a:zh".into(), "mt-a:en".into(), "llm-a:fix".into()];

        let out = engine.rewrite(&req).await.expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 3, "no hop after the fix");
        assert!(
            calls[0].1.is_empty() && calls[1].1.is_empty(),
            "MT hops get no metadata"
        );
        assert_eq!(calls[2].0, "en");
        assert_eq!(calls[2].1["not-a-bot.style"], "fix");
        assert!(
            calls[2].1["not-a-bot.missing"].is_array(),
            "the fix hop gets the fact check's missing tokens"
        );
        assert_eq!(
            out.response.text, "hullo>zh>en",
            "only the vetted edit applies"
        );
        let edits = out.response.hops[2].edits.as_ref().expect("fix hop edits");
        assert!(edits[0].applied);
        assert_eq!(edits[1].reason.as_deref(), Some("from not found"));
        assert!(out.response.hops[0].edits.is_none());
        assert!(out
            .response
            .warnings
            .iter()
            .any(|w| w.starts_with("fix: skipped edit \"zz\"")));
        assert_eq!(calls[2].1["not-a-bot.original"], "hello");
        assert_eq!(out.response.chain, vec!["zh", "en", "fix"]);
        assert_eq!(out.response.hops[2].from, "en");
        assert_eq!(out.response.hops[2].to, "en");
        assert_eq!(out.response.detected_source, "en");

        let mut bad = rewrite_req("en", "auto");
        bad.chain = vec!["zh".into(), "mt-a:fix".into()];
        let err = engine.rewrite(&bad).await.expect_err("MT cannot fix");
        assert_eq!(err.code(), "INVALID_INPUT");
    }

    #[tokio::test]
    async fn depth_three_visits_three_languages_and_flags_lost_facts() {
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
        let mut req = rewrite_req("en", "auto");
        req.text = "Pay NZD 18,900".into();
        req.depth = Some(3);

        let out = engine.rewrite(&req).await.expect("rewrite");

        let calls = calls.lock().unwrap().clone();
        let targets: Vec<_> = calls.iter().map(|c| c.1.as_str()).collect();
        assert_eq!(targets, vec!["zh", "ja", "fi", "en"]);
        assert_eq!(out.response.chain, vec!["zh", "ja", "fi", "en"]);
        // The recorder keeps the text, so nothing is lost.
        assert!(out.response.missing_facts.is_empty());
        assert!(out.response.warnings.is_empty());
    }

    #[tokio::test]
    async fn explicit_chain_ending_in_the_source_adds_no_hop() {
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
        let mut req = rewrite_req("auto", "auto");
        req.chain = vec!["zh".into(), "recorder:en".into()];

        let out = engine.rewrite(&req).await.expect("rewrite");
        assert_eq!(calls.lock().unwrap().len(), 2);
        assert_eq!(out.response.chain, vec!["zh", "en"]);
        assert_eq!(out.response.detected_source, "en");
    }

    type Seen = Arc<std::sync::Mutex<Vec<TR>>>;
    type Reply = fn(&TR) -> R<String>;

    /// Records every request and answers with `reply`.
    struct Spy {
        name: &'static str,
        kind: ProviderKind,
        seen: Seen,
        reply: Reply,
    }

    #[async_trait]
    impl Provider for Spy {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                display_name: self.name.into(),
                kind: self.kind,
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
            self.seen.lock().unwrap().push(req.clone());
            Ok(TranslateResponse {
                text: (self.reply)(req)?,
                detected_source: req.source_lang.clone(),
                target_lang: req.target_lang.clone(),
                provider: self.name.into(),
                engine: self.name.into(),
                latency_ms: 0,
                request_id: String::new(),
                chars: None,
                warnings: Vec::new(),
            })
        }
    }

    fn echo(req: &TR) -> R<String> {
        Ok(req.text.clone())
    }

    fn spy_engine(spies: Vec<(&'static str, ProviderKind, Reply)>) -> (Engine, Seen) {
        let seen: Seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut reg = Registry::new();
        for (name, kind, reply) in spies {
            reg.register(Arc::new(Spy {
                name,
                kind,
                seen: seen.clone(),
                reply,
            }));
        }
        let engine = Engine::new(
            Arc::new(reg),
            CredentialStore::default(),
            Some(dir().keep()),
            test_settings(),
        );
        (engine, seen)
    }

    const CODE_TEXT: &str = "Run `make test` before `git push`.";

    #[tokio::test]
    async fn llm_provider_gets_unmasked_text_and_the_span_list() {
        let (engine, seen) = spy_engine(vec![("llm-a", ProviderKind::Llm, echo)]);
        let mut req = request(Some("llm-a"));
        req.text = CODE_TEXT.into();
        req.preserve = vec!["@code".into()];

        let out = engine.translate(&req).await.unwrap();

        let sent = seen.lock().unwrap()[0].clone();
        assert_eq!(sent.text, CODE_TEXT);
        assert_eq!(
            sent.metadata["not-a-bot.preserve"],
            serde_json::json!(["`make test`", "`git push`"])
        );
        assert_eq!(out.response.text, CODE_TEXT);
        assert!(out.response.warnings.is_empty());
    }

    #[tokio::test]
    async fn llm_dropping_a_span_warns_without_failing() {
        let drop_push: Reply = |req| Ok(req.text.replace("`git push`", "git push"));
        let (engine, _) = spy_engine(vec![("llm-a", ProviderKind::Llm, drop_push)]);
        let mut req = request(Some("llm-a"));
        req.text = CODE_TEXT.into();
        req.preserve = vec!["@code".into()];

        let out = engine.translate(&req).await.unwrap();

        assert_eq!(
            out.response.warnings,
            vec!["preserve: llm-a dropped `git push`"]
        );
    }

    #[tokio::test]
    async fn mt_provider_still_gets_masked_text() {
        let (engine, seen) = spy_engine(vec![("mt-a", ProviderKind::Web, echo)]);
        let mut req = request(Some("mt-a"));
        req.text = CODE_TEXT.into();
        req.preserve = vec!["@code".into()];

        let out = engine.translate(&req).await.unwrap();

        let sent = seen.lock().unwrap()[0].clone();
        assert!(!sent.text.contains('`'), "masked: {}", sent.text);
        assert!(!sent.metadata.contains_key("not-a-bot.preserve"));
        assert_eq!(out.response.text, CODE_TEXT);
    }

    fn rate_limited(_: &TR) -> R<String> {
        Err(Error::RateLimited {
            provider: "stub".into(),
            retry_after_secs: None,
        })
    }

    #[tokio::test]
    async fn named_mt_hop_falls_back_on_a_retryable_error() {
        let (engine, seen) = spy_engine(vec![
            ("mt-a", ProviderKind::Web, echo),
            ("mt-b", ProviderKind::Web, rate_limited),
            ("llm-a", ProviderKind::Llm, echo),
            ("mt-c", ProviderKind::Web, echo),
        ]);
        let mut req = rewrite_req("en", "auto");
        req.chain = vec!["mt-a:zh".into(), "mt-b:fi".into(), "mt-a:en".into()];

        let out = engine.rewrite(&req).await.expect("rewrite");

        let providers: Vec<_> = out
            .response
            .hops
            .iter()
            .map(|h| h.provider.as_str())
            .collect();
        assert_eq!(providers, vec!["mt-a", "mt-c", "mt-a"]);
        assert!(out
            .response
            .warnings
            .contains(&"hop 2: mt-b failed (RATE_LIMITED), served by mt-c".to_string()));
        assert_eq!(seen.lock().unwrap().len(), 4, "mt-a, mt-b, mt-c, mt-a");
    }

    #[tokio::test]
    async fn named_llm_hop_does_not_fall_back() {
        let (engine, seen) = spy_engine(vec![
            ("llm-a", ProviderKind::Llm, rate_limited),
            ("mt-a", ProviderKind::Web, echo),
        ]);
        let mut req = rewrite_req("en", "auto");
        req.chain = vec!["llm-a:zh".into(), "mt-a:en".into()];

        let err = engine.rewrite(&req).await.unwrap_err();

        assert_eq!(err.code(), "RATE_LIMITED");
        assert_eq!(seen.lock().unwrap().len(), 1);
    }
}
