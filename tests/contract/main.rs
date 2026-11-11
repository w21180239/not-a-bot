//! Wire-contract tests.
//!
//! Everything asserted here is a shape an agent parses, so a change to any of
//! it is a breaking change and should fail CI rather than surface at runtime.

use not_a_bot::cli::{manifest_document, schema_document};
use not_a_bot::core::config::Settings;
use not_a_bot::core::creds::CredentialStore;
use not_a_bot::core::types::{ProviderKind, TranslateRequest, TranslateResponse};
use not_a_bot::providers::KNOWN_PROVIDERS;
use not_a_bot::{Error, Registry};
use std::collections::BTreeMap;

const WIRE_TYPES: [&str; 9] = [
    "TranslateRequest",
    "TranslateResponse",
    "ErrorResponse",
    "ProviderInfo",
    "HealthRecord",
    "AttemptRecord",
    "RewriteRequest",
    "RewriteResponse",
    "LintResponse",
];

fn default_registry() -> Registry {
    not_a_bot::cli::build_registry(&CredentialStore::default(), &Settings::default())
}

#[test]
fn schema_publishes_every_wire_type() {
    let doc = schema_document().expect("schema document");
    assert_eq!(
        doc["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(doc["version"], not_a_bot::core::types::VERSION);
    assert_eq!(doc["title"], "not-a-bot");

    let defs = doc["definitions"].as_object().expect("definitions object");
    for name in WIRE_TYPES {
        assert!(defs.contains_key(name), "schema is missing {name}");
    }
}

#[test]
fn translate_schema_requires_text_and_target() {
    let doc = schema_document().expect("schema document");
    let req = &doc["definitions"]["TranslateRequest"];
    let required: Vec<&str> = req["required"]
        .as_array()
        .expect("required array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(required.contains(&"text"), "text must be required");
    assert!(
        required.contains(&"targetLang"),
        "targetLang must be required"
    );
    // The wire contract is camelCase; a snake_case property would break clients.
    let props = req["properties"].as_object().expect("properties");
    assert!(props.contains_key("targetLang"));
    assert!(!props.contains_key("target_lang"));
}

#[test]
fn request_accepts_only_the_camel_case_contract() {
    let req: TranslateRequest =
        serde_json::from_str(r#"{"text":"hi","targetLang":"zh"}"#).expect("minimal request");
    assert_eq!(req.source_lang, "auto", "sourceLang defaults to auto");
    assert!(req.provider.is_none());

    let snake = serde_json::from_str::<TranslateRequest>(r#"{"text":"hi","target_lang":"zh"}"#);
    assert!(snake.is_err(), "snake_case must not be accepted");

    let unknown =
        serde_json::from_str::<TranslateRequest>(r#"{"text":"hi","targetLang":"zh","bogus":1}"#);
    assert!(unknown.is_err(), "unknown fields must be rejected");
}

#[test]
fn response_serialises_camel_case_and_omits_empty_fields() {
    let resp = TranslateResponse {
        text: "你好".into(),
        detected_source: "en".into(),
        target_lang: "zh".into(),
        provider: "google".into(),
        engine: "google".into(),
        latency_ms: 12,
        request_id: "abc".into(),
        chars: Some(2),
        warnings: Vec::new(),
    };
    let value = serde_json::to_value(&resp).expect("serialise");
    for key in [
        "detectedSource",
        "targetLang",
        "latencyMs",
        "requestId",
        "provider",
    ] {
        assert!(value.get(key).is_some(), "response is missing {key}");
    }
    assert!(
        value.get("warnings").is_none(),
        "empty warnings should stay off the wire"
    );
    assert!(value.get("detected_source").is_none());
}

#[test]
fn provider_catalogue_honours_kind_and_priority_bands() {
    for provider in default_registry().iter() {
        let info = provider.info();
        assert!(
            KNOWN_PROVIDERS.contains(&info.name.as_str()),
            "{} is registered but missing from KNOWN_PROVIDERS",
            info.name
        );

        // Bands are a contract: web 1-12, official 20-27, LLM 40+.
        let band = match info.kind {
            ProviderKind::Web => 1..=19,
            ProviderKind::Official => 20..=39,
            ProviderKind::Llm => 40..=99,
        };
        assert!(
            band.contains(&info.priority),
            "{} has priority {} outside the band for {:?}",
            info.name,
            info.priority,
            info.kind
        );

        assert!(
            !info.languages.is_empty(),
            "{} advertises no languages",
            info.name
        );
        assert!(info.max_chars > 0, "{} advertises no size limit", info.name);
        for code in &info.languages {
            assert!(
                code == "*" || not_a_bot::core::lang::is_known(code),
                "{} advertises a non-canonical code {code}",
                info.name
            );
        }
    }
}

#[test]
fn keyed_providers_need_credentials_to_register() {
    for provider in default_registry().iter() {
        let info = provider.info();
        assert!(
            !info.needs_key,
            "{} needs a key but registered with no credentials",
            info.name
        );
    }
}

#[test]
fn retired_names_resolve_without_joining_the_catalogue() {
    let registry = default_registry();
    // `iflytek` is retired upstream and aliased to `iflyrec`.
    let resolved = registry.get("iflytek").expect("alias resolves");
    assert_eq!(resolved.info().name, "iflyrec");
    assert!(
        !registry.names().iter().any(|n| n == "iflytek"),
        "an alias must not appear as a registered provider"
    );
}

#[test]
fn error_wire_shape_carries_the_agent_decision() {
    let wire = Error::RateLimited {
        provider: "sogou".into(),
        retry_after_secs: Some(300),
    }
    .wire();
    assert_eq!(wire.code, "RATE_LIMITED");
    assert!(wire.retryable);
    assert_eq!(wire.retry_after_secs, Some(300));
    assert_eq!(wire.suggested_action, "backoff_and_retry");

    let value = serde_json::to_value(&wire).expect("serialise");
    assert_eq!(value["retryAfterSecs"], 300);
    assert_eq!(value["suggestedAction"], "backoff_and_retry");
    assert!(value.get("retry_after_secs").is_none());
    assert!(value.get("code").is_some() && value.get("message").is_some());
}

#[test]
fn retry_policy_matches_the_documented_table() {
    assert!(Error::Timeout { timeout_ms: 10 }.retryable());
    assert!(Error::Network("reset".into()).retryable());
    assert!(!Error::AuthFailed {
        provider: "deepl".into(),
        reason: "401".into()
    }
    .retryable());
    assert!(Error::Upstream {
        provider: "x".into(),
        status: 503,
        body: String::new()
    }
    .retryable());
    assert!(!Error::Upstream {
        provider: "x".into(),
        status: 404,
        body: String::new()
    }
    .retryable());

    // Only auth, quota and rate-limit failures cool a key.
    assert!(Error::AuthFailed {
        provider: "x".into(),
        reason: "y".into()
    }
    .cools_key());
    assert!(!Error::Timeout { timeout_ms: 1 }.cools_key());

    assert_eq!(
        Error::ProviderNotConfigured {
            name: "deepl".into()
        }
        .code(),
        "PROVIDER_NOT_CONFIGURED"
    );
    assert_eq!(
        Error::AutoDetectUnsupported {
            provider: "reverso".into()
        }
        .code(),
        "AUTO_DETECT_UNSUPPORTED"
    );
}

#[test]
fn manifest_describes_the_cli_surface() {
    let manifest = manifest_document().expect("manifest");
    assert_eq!(manifest["interface"], "cli");
    assert!(
        manifest.get("transport").is_none(),
        "the manifest must not describe a server transport"
    );

    let mut names: Vec<&str> = manifest["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .map(|c| c["name"].as_str().expect("command name"))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        vec![
            "health",
            "languages",
            "lint",
            "providers",
            "rewrite",
            "translate"
        ]
    );

    for command in manifest["commands"].as_array().expect("commands array") {
        assert!(
            command["command"].as_str().is_some_and(|c| !c.is_empty()),
            "{} has no command line",
            command["name"]
        );
    }

    // `translate` and `rewrite` carry their request schema for generators.
    for name in ["translate", "rewrite"] {
        let command = manifest["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap();
        assert_eq!(command["inputSchema"]["type"], "object");
    }
}

#[test]
fn request_metadata_is_free_form() {
    // `metadata` is caller bookkeeping: any JSON shape, never interpreted.
    let req: TranslateRequest = serde_json::from_value(serde_json::json!({
        "text": "hi",
        "targetLang": "zh",
        "metadata": { "traceId": "abc", "nested": { "n": 1 } }
    }))
    .expect("metadata accepted");
    assert_eq!(req.metadata["traceId"], "abc");
    assert_eq!(
        req.metadata.len(),
        2,
        "metadata round-trips without filtering"
    );

    // Defaults are empty rather than absent on the type.
    let plain: TranslateRequest =
        serde_json::from_str(r#"{"text":"hi","targetLang":"zh"}"#).unwrap();
    assert_eq!(plain.metadata, BTreeMap::new());
}
