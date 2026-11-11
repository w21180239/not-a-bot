//! Detector scores for roundtrip candidates: best-of selection and the preset
//! ladder behind `targetScore`. Fact preservation outranks the score.

use crate::core::config::Settings;
use crate::core::engine::RoundtripOutcome;
use crate::core::error::{Error, Result};
use crate::core::types::{Candidate, RoundtripRequest};
use async_trait::async_trait;
use std::cmp::Ordering;
use std::future::Future;
use std::path::Path;
use std::time::Instant;

pub const MAX_BEST_OF: u8 = 5;

#[async_trait]
pub trait Scorer: Send + Sync {
    /// Program name for the response; never its arguments.
    fn name(&self) -> &str;
    /// AI-likeness in [0,1], higher = more AI-like. `Err` is a short reason.
    async fn score(&self, text: &str) -> std::result::Result<f64, String>;
}

/// An external command: candidate on stdin, score on the last stdout line.
#[derive(Debug, Clone)]
pub struct CommandScorer {
    program: String,
    args: Vec<String>,
    name: String,
    timeout_ms: u64,
}

impl CommandScorer {
    pub fn parse(command: &str, timeout_ms: u64) -> Result<Self> {
        let mut words =
            split_command(command).map_err(|e| Error::InvalidInput(format!("scorer: {e}")))?;
        if words.is_empty() {
            return Err(Error::InvalidInput("scorer: empty command".into()));
        }
        let program = words.remove(0);
        let name = Path::new(&program)
            .file_name()
            .map_or_else(|| program.clone(), |n| n.to_string_lossy().into_owned());
        Ok(Self {
            program,
            args: words,
            name,
            timeout_ms,
        })
    }
}

#[async_trait]
impl Scorer for CommandScorer {
    fn name(&self) -> &str {
        &self.name
    }

    async fn score(&self, text: &str) -> std::result::Result<f64, String> {
        use crate::process::{command, resolve_on_path, run_with_input, RunError};
        let bin = resolve_on_path(&self.program)
            .ok_or_else(|| format!("cannot find {}", self.program))?;
        let mut cmd = command(&bin);
        cmd.args(&self.args);
        let out = run_with_input(cmd, text.to_string(), self.timeout_ms)
            .await
            .map_err(|e| match e {
                RunError::Spawn(e) => format!("cannot start: {e}"),
                RunError::Wait(e) => format!("waiting: {e}"),
                RunError::Timeout => format!("timed out after {} ms", self.timeout_ms),
            })?;
        // Stderr is not echoed: it may quote the candidate.
        if !out.status.success() {
            return Err(format!("exited with {}", out.status));
        }
        parse_score(&String::from_utf8_lossy(&out.stdout))
    }
}

/// Split a command line into words without a shell: whitespace separates,
/// `'...'` and `"..."` group, backslashes stay literal (Windows paths).
pub fn split_command(line: &str) -> std::result::Result<Vec<String>, String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => word.push(c),
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                in_word = true;
            }
            None if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            None => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if quote.is_some() {
        return Err("unbalanced quote".into());
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

/// The last non-empty line as a score in [0,1].
pub fn parse_score(stdout: &str) -> std::result::Result<f64, String> {
    let line = stdout
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .ok_or("no output")?;
    let score: f64 = line
        .parse()
        .map_err(|_| format!("last line is not a number: {:.40}", line))?;
    if !(0.0..=1.0).contains(&score) {
        return Err(format!("score {score} outside [0,1]"));
    }
    Ok(score)
}

/// Index of the best candidate: fewest missing facts, then scored before
/// unscored, then lowest score; ties keep the earliest run.
pub fn select(candidates: &[Candidate]) -> Option<usize> {
    (0..candidates.len()).min_by(|&a, &b| rank(&candidates[a], &candidates[b]))
}

fn rank(a: &Candidate, b: &Candidate) -> Ordering {
    a.missing_facts
        .cmp(&b.missing_facts)
        .then_with(|| match (a.score, b.score) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
}

/// Name of what a request runs: the comma-joined chain, the preset, or the
/// depth/pivot/configured default, in planning precedence.
pub fn label(req: &RoundtripRequest, settings: &Settings) -> String {
    if !req.chain.is_empty() {
        return req.chain.join(",");
    }
    if let Some(preset) = &req.preset {
        return preset.trim().to_lowercase();
    }
    if let Some(depth) = req.depth {
        return format!("depth-{depth}");
    }
    let pivot = req.pivot.trim().to_lowercase();
    if !pivot.is_empty() && pivot != "auto" {
        return format!("pivot-{pivot}");
    }
    settings.roundtrip_preset()
}

/// Validate the scoring options, then run the request `bestOf` times per rung
/// and return the selected run. Without a scorer this is a single `run`.
pub async fn search<F, Fut>(
    req: &RoundtripRequest,
    settings: &Settings,
    scorer: Option<&dyn Scorer>,
    mut run: F,
) -> Result<RoundtripOutcome>
where
    F: FnMut(RoundtripRequest) -> Fut,
    Fut: Future<Output = Result<RoundtripOutcome>>,
{
    let best_of = req.best_of.unwrap_or(1);
    if !(1..=MAX_BEST_OF).contains(&best_of) {
        return Err(Error::InvalidInput(format!(
            "bestOf must be 1-{MAX_BEST_OF}, got {best_of}"
        )));
    }
    if let Some(t) = req.target_score {
        if !(0.0..=1.0).contains(&t) {
            return Err(Error::InvalidInput(format!(
                "targetScore must be between 0 and 1, got {t}"
            )));
        }
    }
    let Some(scorer) = scorer else {
        if best_of > 1 {
            return Err(Error::InvalidInput(
                "bestOf above 1 needs a scorer to choose between runs; pass --scorer or set roundtrip_scorer".into(),
            ));
        }
        if req.target_score.is_some() {
            return Err(Error::InvalidInput(
                "targetScore needs a scorer; pass --scorer or set roundtrip_scorer".into(),
            ));
        }
        return run(req.clone()).await;
    };

    let started = Instant::now();
    let mut base = req.clone();
    // One id across every run, so logs tie them together.
    base.idempotency_key
        .get_or_insert_with(|| uuid::Uuid::new_v4().to_string());
    let mut rungs = vec![(label(&base, settings), base.clone())];
    if req.target_score.is_some() {
        for preset in settings.roundtrip_ladder() {
            if rungs.iter().any(|(l, _)| *l == preset) {
                continue;
            }
            let mut rung = base.clone();
            rung.chain.clear();
            rung.depth = None;
            rung.preset = Some(preset.clone());
            rungs.push((preset, rung));
        }
    }

    let mut runs: Vec<RoundtripOutcome> = Vec::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut notes = Vec::new();
    let mut first_err = None;
    for (rung_label, rung) in rungs {
        for _ in 0..best_of {
            match run(rung.clone()).await {
                Ok(out) => {
                    let score = match scorer.score(&out.response.text).await {
                        Ok(s) => Some(s),
                        Err(reason) => {
                            notes.push(format!(
                                "scorer: run {} ({rung_label}) unscored: {reason}",
                                candidates.len() + 1
                            ));
                            None
                        }
                    };
                    candidates.push(Candidate {
                        preset_or_chain: rung_label.clone(),
                        score,
                        missing_facts: out.response.missing_facts.len(),
                    });
                    runs.push(out);
                }
                Err(err) => {
                    notes.push(format!("{rung_label}: run failed ({})", err.code()));
                    first_err.get_or_insert(err);
                }
            }
        }
        let Some(target) = req.target_score else {
            break;
        };
        // The requested chain failing outright is the answer, as without a scorer.
        let Some(best) = select(&candidates) else {
            break;
        };
        if candidates.iter().all(|c| c.score.is_none())
            || candidates[best].score.is_some_and(|s| s <= target)
        {
            break;
        }
    }

    let Some(best) = select(&candidates) else {
        return Err(first_err.unwrap_or_else(|| Error::InvalidInput("no roundtrip ran".into())));
    };
    if let Some(target) = req.target_score {
        match candidates[best].score {
            _ if candidates.iter().all(|c| c.score.is_none()) => {
                notes.push("target score ignored: no run could be scored".into())
            }
            Some(s) if s <= target => {}
            Some(s) => notes.push(format!("target score not met: best {s:.3} > {target}")),
            None => notes.push(format!(
                "target score not met: the run keeping the most facts is unscored (target {target})"
            )),
        }
    }

    let attempts = runs.iter().flat_map(|r| r.attempts.clone()).collect();
    let mut chosen = runs.swap_remove(best);
    chosen.attempts = attempts;
    let response = &mut chosen.response;
    response.warnings.extend(notes);
    response.score = candidates[best].score;
    response.scorer = Some(scorer.name().to_string());
    response.latency_ms = started.elapsed().as_millis() as u64;
    if candidates.len() > 1 {
        response.candidates = candidates;
    }
    Ok(chosen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::RoundtripResponse;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    struct FakeScorer(HashMap<String, f64>);

    #[async_trait]
    impl Scorer for FakeScorer {
        fn name(&self) -> &str {
            "fake"
        }
        async fn score(&self, text: &str) -> std::result::Result<f64, String> {
            self.0.get(text).copied().ok_or_else(|| "boom".to_string())
        }
    }

    fn scorer(pairs: &[(&str, f64)]) -> FakeScorer {
        FakeScorer(pairs.iter().map(|(t, s)| (t.to_string(), *s)).collect())
    }

    fn outcome(text: &str, missing: usize) -> RoundtripOutcome {
        RoundtripOutcome {
            response: RoundtripResponse {
                text: text.into(),
                detected_source: "en".into(),
                chain: vec!["en".into()],
                hops: Vec::new(),
                provider: "stub".into(),
                missing_facts: (0..missing).map(|i| format!("fact{i}")).collect(),
                latency_ms: 0,
                request_id: "rt".into(),
                chars: None,
                warnings: vec![format!("hop warning for {text}")],
                score: None,
                scorer: None,
                candidates: Vec::new(),
            },
            attempts: Vec::new(),
        }
    }

    fn req(preset: &str) -> RoundtripRequest {
        let mut r: RoundtripRequest = serde_json::from_str(r#"{"text":"hi"}"#).unwrap();
        r.preset = Some(preset.into());
        r
    }

    fn cand(missing: usize, score: Option<f64>) -> Candidate {
        Candidate {
            preset_or_chain: "x".into(),
            score,
            missing_facts: missing,
        }
    }

    type Calls = Arc<Mutex<Vec<String>>>;

    /// Runner whose text is `<preset>#<run>`; `missing` maps texts to lost facts.
    fn runner(
        calls: Calls,
        missing: &'static [(&'static str, usize)],
    ) -> impl FnMut(RoundtripRequest) -> futures::future::Ready<Result<RoundtripOutcome>> {
        move |r: RoundtripRequest| {
            let mut calls = calls.lock().unwrap();
            let label = r.preset.clone().unwrap_or_default();
            let n = calls.iter().filter(|c| **c == label).count() + 1;
            calls.push(label.clone());
            let text = format!("{label}#{n}");
            let lost = missing
                .iter()
                .find(|(t, _)| *t == text)
                .map_or(0, |(_, m)| *m);
            futures::future::ready(Ok(outcome(&text, lost)))
        }
    }

    #[test]
    fn facts_beat_score_then_lowest_score_wins() {
        let c = [
            cand(1, Some(0.1)),
            cand(0, Some(0.9)),
            cand(0, Some(0.5)),
            cand(0, None),
        ];
        assert_eq!(select(&c), Some(2));
        // None complete: fewest missing, scored before unscored.
        let c = [cand(2, Some(0.1)), cand(1, None), cand(1, Some(0.8))];
        assert_eq!(select(&c), Some(2));
        // Ties keep the earliest run.
        assert_eq!(select(&[cand(0, None), cand(0, None)]), Some(0));
        assert_eq!(select(&[]), None);
    }

    #[tokio::test]
    async fn best_of_picks_the_lowest_score_and_lists_candidates() {
        let calls = Calls::default();
        let mut r = req("llm");
        r.best_of = Some(3);
        let s = scorer(&[("llm#1", 0.7), ("llm#2", 0.2), ("llm#3", 0.5)]);
        let out = search(
            &r,
            &Settings::default(),
            Some(&s),
            runner(calls.clone(), &[]),
        )
        .await
        .unwrap();
        assert_eq!(calls.lock().unwrap().len(), 3);
        assert_eq!(out.response.text, "llm#2");
        assert_eq!(out.response.score, Some(0.2));
        assert_eq!(out.response.scorer.as_deref(), Some("fake"));
        let scores: Vec<_> = out.response.candidates.iter().map(|c| c.score).collect();
        assert_eq!(scores, vec![Some(0.7), Some(0.2), Some(0.5)]);
        assert!(
            !out.response.warnings.iter().any(|w| w.contains("llm#1")),
            "warnings of unselected runs are dropped"
        );
    }

    #[tokio::test]
    async fn a_run_keeping_its_facts_beats_a_lower_score() {
        let mut r = req("llm");
        r.best_of = Some(2);
        let s = scorer(&[("llm#1", 0.1), ("llm#2", 0.8)]);
        let out = search(
            &r,
            &Settings::default(),
            Some(&s),
            runner(Calls::default(), &[("llm#1", 1)]),
        )
        .await
        .unwrap();
        assert_eq!(out.response.text, "llm#2");
        assert_eq!(out.response.candidates[0].missing_facts, 1);
    }

    #[tokio::test]
    async fn scorer_failure_leaves_the_run_unscored_with_a_warning() {
        let mut r = req("llm");
        r.best_of = Some(2);
        let s = scorer(&[("llm#2", 0.6)]);
        let out = search(
            &r,
            &Settings::default(),
            Some(&s),
            runner(Calls::default(), &[]),
        )
        .await
        .unwrap();
        assert_eq!(out.response.text, "llm#2", "scored beats unscored");
        assert_eq!(out.response.candidates[0].score, None);
        assert!(out
            .response
            .warnings
            .iter()
            .any(|w| w.contains("run 1 (llm) unscored: boom")));
    }

    #[tokio::test]
    async fn all_unscored_behaves_as_without_scorer() {
        let calls = Calls::default();
        let mut r = req("mt");
        r.best_of = Some(2);
        r.target_score = Some(0.3);
        let out = search(
            &r,
            &Settings::default(),
            Some(&scorer(&[])),
            runner(calls.clone(), &[]),
        )
        .await
        .unwrap();
        assert_eq!(out.response.text, "mt#1");
        assert_eq!(out.response.score, None);
        assert_eq!(*calls.lock().unwrap(), vec!["mt", "mt"], "no ladder climb");
        assert!(out
            .response
            .warnings
            .iter()
            .any(|w| w.contains("target score ignored")));
    }

    #[tokio::test]
    async fn ladder_stops_at_the_first_rung_meeting_the_target() {
        let calls = Calls::default();
        let mut r = req("mt");
        r.target_score = Some(0.5);
        let s = scorer(&[("mt#1", 0.9), ("llm#1", 0.3), ("lynote-fix#1", 0.1)]);
        let out = search(
            &r,
            &Settings::default(),
            Some(&s),
            runner(calls.clone(), &[]),
        )
        .await
        .unwrap();
        assert_eq!(*calls.lock().unwrap(), vec!["mt", "llm"]);
        assert_eq!(out.response.text, "llm#1");
        assert_eq!(out.response.candidates.len(), 2);
        assert!(!out.response.warnings.iter().any(|w| w.contains("target")));
    }

    #[tokio::test]
    async fn ladder_exhausts_and_returns_the_best_overall() {
        let calls = Calls::default();
        let mut r = req("mt");
        r.target_score = Some(0.2);
        let s = scorer(&[("mt#1", 0.9), ("llm#1", 0.4), ("lynote-fix#1", 0.6)]);
        let out = search(
            &r,
            &Settings::default(),
            Some(&s),
            runner(calls.clone(), &[]),
        )
        .await
        .unwrap();
        assert_eq!(*calls.lock().unwrap(), vec!["mt", "llm", "lynote-fix"]);
        assert_eq!(out.response.text, "llm#1");
        assert!(out
            .response
            .warnings
            .iter()
            .any(|w| w.starts_with("target score not met")));
    }

    #[tokio::test]
    async fn ladder_skips_the_rung_already_tried() {
        let calls = Calls::default();
        let mut r = req("llm");
        r.target_score = Some(0.1);
        let settings = Settings {
            roundtrip_ladder: Some(vec!["LLM".into(), "lynote-fix".into()]),
            ..Default::default()
        };
        let s = scorer(&[("llm#1", 0.5), ("lynote-fix#1", 0.4)]);
        search(&r, &settings, Some(&s), runner(calls.clone(), &[]))
            .await
            .unwrap();
        assert_eq!(*calls.lock().unwrap(), vec!["llm", "lynote-fix"]);
    }

    #[tokio::test]
    async fn options_are_validated() {
        let s = scorer(&[]);
        let cases: [(Option<u8>, Option<f64>, bool); 5] = [
            (Some(2), None, false),
            (None, Some(0.3), false),
            (Some(0), None, true),
            (Some(6), None, true),
            (None, Some(1.5), true),
        ];
        for (best_of, target, with_scorer) in cases {
            let mut r = req("mt");
            r.best_of = best_of;
            r.target_score = target;
            let sc: Option<&dyn Scorer> = with_scorer.then_some(&s as &dyn Scorer);
            let err = search(&r, &Settings::default(), sc, runner(Calls::default(), &[]))
                .await
                .unwrap_err();
            assert_eq!(err.code(), "INVALID_INPUT", "{best_of:?} {target:?}");
        }
    }

    #[tokio::test]
    async fn without_a_scorer_one_run_passes_through() {
        let calls = Calls::default();
        let out = search(
            &req("mt"),
            &Settings::default(),
            None,
            runner(calls.clone(), &[]),
        )
        .await
        .unwrap();
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert_eq!(out.response.score, None);
        assert!(out.response.scorer.is_none());
        assert!(out.response.candidates.is_empty());
    }

    #[test]
    fn labels_follow_planning_precedence() {
        let s = Settings::default();
        let mut r: RoundtripRequest = serde_json::from_str(r#"{"text":"hi"}"#).unwrap();
        assert_eq!(label(&r, &s), "mt");
        r.pivot = "ja".into();
        assert_eq!(label(&r, &s), "pivot-ja");
        r.depth = Some(3);
        assert_eq!(label(&r, &s), "depth-3");
        r.preset = Some("LLM".into());
        assert_eq!(label(&r, &s), "llm");
        r.chain = vec!["google:fi".into(), "edge:en".into()];
        assert_eq!(label(&r, &s), "google:fi,edge:en");
    }

    #[test]
    fn commands_split_without_a_shell() {
        assert_eq!(
            split_command(r#"python "C:\my tools\score.py" --model 'a b'"#).unwrap(),
            vec!["python", r"C:\my tools\score.py", "--model", "a b"]
        );
        assert_eq!(split_command(r#"x """#).unwrap(), vec!["x", ""]);
        assert!(split_command("x 'open").is_err());
        assert!(split_command("   ").unwrap().is_empty());
        let s = CommandScorer::parse(r"C:\bin\detect.exe --fast", 1000).unwrap();
        assert_eq!(
            s.name(),
            if cfg!(windows) {
                "detect.exe"
            } else {
                r"C:\bin\detect.exe"
            }
        );
        assert!(CommandScorer::parse("  ", 1000).is_err());
    }

    #[test]
    fn score_is_the_last_non_empty_line() {
        assert_eq!(parse_score("loading\n0.42\n\n"), Ok(0.42));
        assert!(parse_score("").is_err());
        assert!(parse_score("0.4\ndone").is_err());
        assert!(parse_score("1.2").is_err());
        assert!(parse_score("NaN").is_err());
    }
}
