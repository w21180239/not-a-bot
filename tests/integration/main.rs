//! Integration tests: engine -> provider -> HTTP -> typed error.
//!
//! Each test points a real provider adapter at a mock server, so the request
//! shape, response parsing and error mapping are exercised end to end without
//! touching a vendor endpoint.

use httpmock::prelude::*;
use not_a_bot::core::config::Settings;
use not_a_bot::core::creds::{Credential, CredentialStore};
use not_a_bot::core::engine::Engine;
use not_a_bot::core::types::TranslateRequest;
use not_a_bot::providers::{libretranslate::LibreTranslate, llm::Llm, Provider, Registry};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn request(text: &str, target: &str) -> TranslateRequest {
    TranslateRequest {
        text: text.into(),
        source_lang: "en".into(),
        target_lang: target.into(),
        provider: None,
        fallback_chain: Vec::new(),
        preserve: Vec::new(),
        timeout_ms: None,
        idempotency_key: None,
        metadata: BTreeMap::new(),
    }
}

/// Retries off: these tests assert on the first attempt, not on backoff.
fn settings() -> Settings {
    Settings {
        retries: Some(0),
        ..Default::default()
    }
}

fn engine(registry: Registry, creds: CredentialStore, settings: Settings) -> Engine {
    // No cache dir: cooldown state stays in memory, so tests do not share it.
    Engine::new(Arc::new(registry), creds, None, settings)
}

fn api_key(provider: &str, key: &str) -> Credential {
    Credential {
        provider: provider.into(),
        label: "default".into(),
        fields: [("api_key".to_string(), key.to_string())]
            .into_iter()
            .collect(),
    }
}

#[tokio::test]
async fn libretranslate_round_trip() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/translate");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"translatedText":"你好世界","detectedSourceLanguage":"en"}"#);
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(LibreTranslate::new(Some(&server.base_url()))));

    let out = engine(registry, CredentialStore::default(), settings())
        .translate(&request("Hello world", "zh"))
        .await
        .expect("translate");

    assert_eq!(out.response.text, "你好世界");
    assert_eq!(out.response.provider, "libretranslate");
    assert_eq!(out.response.target_lang, "zh");
    assert_eq!(out.response.detected_source, "en");
    assert_eq!(
        out.response.chars,
        Some(11),
        "source characters are counted"
    );
    assert_eq!(out.attempts.len(), 1);
    assert!(out.attempts[0].ok);
    assert_eq!(mock.hits(), 1);
}

#[tokio::test]
async fn upstream_failure_becomes_a_retryable_typed_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/translate");
        then.status(503).body("service unavailable");
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(LibreTranslate::new(Some(&server.base_url()))));

    let mut req = request("hello", "zh");
    req.provider = Some("libretranslate".into());
    let err = engine(registry, CredentialStore::default(), settings())
        .translate(&req)
        .await
        .expect_err("503 must fail");

    assert_eq!(err.code(), "UPSTREAM_ERROR");
    assert!(err.retryable(), "5xx is retryable");
    assert!(err.to_string().contains("503"), "message names the status");
}

#[tokio::test]
async fn rate_limit_is_reported_with_the_backoff_action() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/translate");
        then.status(429).body("slow down");
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(LibreTranslate::new(Some(&server.base_url()))));

    let mut req = request("hello", "zh");
    req.provider = Some("libretranslate".into());
    let err = engine(registry, CredentialStore::default(), settings())
        .translate(&req)
        .await
        .expect_err("429 must fail");

    assert_eq!(err.code(), "RATE_LIMITED");
    assert!(err.retryable());
    assert_eq!(err.suggested_action(), "backoff_and_retry");
}

#[tokio::test]
async fn llm_sends_the_bearer_token_and_parses_the_completion() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/chat/completions")
            .header("authorization", "Bearer sk-test");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"choices":[{"message":{"role":"assistant","content":"你好"}}]}"#);
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(Llm::new(
        "openai",
        "OpenAI",
        Some(&server.base_url()),
        Some("gpt-test"),
    )));
    let mut creds = CredentialStore::default();
    creds.add(api_key("openai", "sk-test"));

    let out = engine(registry, creds, settings())
        .translate(&request("hello", "zh"))
        .await
        .expect("translate");

    assert_eq!(out.response.text, "你好");
    assert_eq!(out.response.provider, "openai");
    assert_eq!(
        out.response.engine, "gpt-test",
        "the model name is reported as the engine"
    );
    assert_eq!(mock.hits(), 1, "the keyed header matcher must hold");
}

#[tokio::test]
async fn malformed_body_is_a_bad_response_not_a_panic() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/translate");
        then.status(200)
            .header("content-type", "application/json")
            .body("<!doctype html><h1>blocked</h1>");
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(LibreTranslate::new(Some(&server.base_url()))));

    let mut req = request("hello", "zh");
    req.provider = Some("libretranslate".into());
    let err = engine(registry, CredentialStore::default(), settings())
        .translate(&req)
        .await
        .expect_err("HTML is not a translation");

    assert_eq!(err.code(), "BAD_RESPONSE");
    assert!(!err.retryable());
}

#[tokio::test]
async fn chain_moves_on_after_an_upstream_error() {
    let server = MockServer::start();
    let first = server.mock(|when, then| {
        when.method(POST).path("/a/chat/completions");
        then.status(500).body("boom");
    });
    let second = server.mock(|when, then| {
        when.method(POST).path("/b/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"choices":[{"message":{"role":"assistant","content":"你好世界"}}]}"#);
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(Llm::new(
        "openai",
        "OpenAI",
        Some(&format!("{}/a", server.base_url())),
        None,
    )));
    registry.register(Arc::new(Llm::new(
        "deepseek",
        "DeepSeek",
        Some(&format!("{}/b", server.base_url())),
        None,
    )));

    let mut creds = CredentialStore::default();
    creds.add(api_key("openai", "k1"));
    creds.add(api_key("deepseek", "k2"));

    let settings = Settings {
        chain: Some(vec!["openai".to_string(), "deepseek".to_string()]),
        retries: Some(0),
        ..Default::default()
    };
    let out = engine(registry, creds, settings)
        .translate(&request("hello", "zh"))
        .await
        .expect("second provider answers");

    assert_eq!(out.response.provider, "deepseek");
    assert_eq!(out.attempts.len(), 2, "one failed hop, one success");
    assert!(!out.attempts[0].ok);
    assert_eq!(
        out.attempts[0].error_code.as_deref(),
        Some("UPSTREAM_ERROR")
    );
    assert!(out.attempts[1].ok);
    assert_eq!(first.hits(), 1);
    assert_eq!(second.hits(), 1);
}

#[tokio::test]
async fn engine_cuts_the_call_at_the_request_budget() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(POST).path("/translate");
        then.status(200)
            .delay(Duration::from_millis(700))
            .header("content-type", "application/json")
            .body(r#"{"translatedText":"晚到的译文"}"#);
    });

    let mut registry = Registry::new();
    registry.register(Arc::new(LibreTranslate::new(Some(&server.base_url()))));

    let mut req = request("hello", "zh");
    req.provider = Some("libretranslate".into());
    req.timeout_ms = Some(80);

    let started = Instant::now();
    let err = engine(registry, CredentialStore::default(), settings())
        .translate(&req)
        .await
        .expect_err("the budget must be enforced");
    let elapsed = started.elapsed();

    assert_eq!(err.code(), "TIMEOUT");
    assert!(
        elapsed < Duration::from_millis(500),
        "engine must stop at the 80 ms budget, took {elapsed:?}"
    );
}

#[tokio::test]
async fn probe_uses_the_liveness_endpoint() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/languages");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"[{"code":"en"},{"code":"zh"}]"#);
    });

    let provider = LibreTranslate::new(Some(&server.base_url()));
    let cred = Credential {
        provider: "libretranslate".into(),
        label: "default".into(),
        fields: BTreeMap::new(),
    };
    provider.probe(&cred).await.expect("healthy");
    assert_eq!(mock.hits(), 1);
}

/// Passed only when this binary runs as a scorer; plain test runs return at once.
const SCORER_SENTINEL: &str = "not-a-bot-scorer-sentinel";

/// Scorer stub: this test binary, filtered to this test, reads the candidate
/// and prints a score, then exits before libtest prints its summary. Works on
/// every platform without a shell.
#[test]
fn scorer_helper() {
    use std::io::{Read, Write};
    if !std::env::args().any(|a| a == SCORER_SENTINEL) {
        return;
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let score = if input.trim().is_empty() { "1" } else { "0.25" };
    let mut out = std::io::stdout();
    writeln!(out, "\n{score}").unwrap();
    out.flush().unwrap();
    std::process::exit(0);
}

#[tokio::test]
async fn best_of_scores_each_run_with_an_external_command() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST).path("/translate");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"translatedText":"Hello again","detectedSourceLanguage":"en"}"#);
    });
    let mut registry = Registry::new();
    registry.register(Arc::new(LibreTranslate::new(Some(&server.base_url()))));

    let exe = std::env::current_exe().unwrap();
    let mut req: not_a_bot::RewriteRequest =
        serde_json::from_str(r#"{"text":"Hello world","sourceLang":"en"}"#).unwrap();
    req.chain = vec!["libretranslate:zh".into(), "libretranslate:en".into()];
    req.best_of = Some(2);
    req.timeout_ms = Some(30_000);
    req.scorer = Some(format!(
        "\"{}\" scorer_helper --exact --nocapture --test-threads=1 --skip {SCORER_SENTINEL}",
        exe.display()
    ));

    let out = engine(registry, CredentialStore::default(), settings())
        .rewrite(&req)
        .await
        .expect("rewrite");

    let r = &out.response;
    assert_eq!(r.score, Some(0.25), "warnings: {:?}", r.warnings);
    let name = exe.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(r.scorer.as_deref(), Some(name.as_str()));
    assert_eq!(r.candidates.len(), 2);
    assert!(r.candidates.iter().all(|c| c.score == Some(0.25)));
    assert_eq!(mock.hits(), 4, "two runs of two hops");
}
