//! Provider trait, registry and per-provider implementations.

use crate::core::creds::Credential;
use crate::core::error::{Error, Result};
pub use crate::core::types::ProviderKind;
use crate::core::types::{ProviderInfo, TranslateRequest, TranslateResponse};
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A provider translates with one credential at a time. Implementations must
/// be cheap to clone-share and safe to call concurrently.
#[async_trait]
pub trait Provider: Send + Sync {
    fn info(&self) -> ProviderInfo;

    /// Credential labels this provider rotates through. Default: a single
    /// `default` set, so keyed providers expose their config labels.
    fn credential_labels(&self) -> Vec<String> {
        vec!["default".into()]
    }

    /// Translate using exactly one credential. Return `Err` with a typed
    /// error; the engine decides about key cooldown and fallback.
    async fn translate_once(
        &self,
        req: &TranslateRequest,
        cred: &Credential,
    ) -> Result<TranslateResponse>;

    /// Cheap liveness probe. Providers that bill per call should use a
    /// metadata endpoint instead of a real translation.
    async fn probe(&self, cred: &Credential) -> Result<()> {
        let req = TranslateRequest {
            text: "ok".into(),
            source_lang: "en".into(),
            target_lang: "zh".into(),
            provider: None,
            fallback_chain: Vec::new(),
            preserve: Vec::new(),
            timeout_ms: Some(10_000),
            idempotency_key: None,
            metadata: BTreeMap::new(),
        };
        self.translate_once(&req, cred).await.map(|_| ())
    }

    /// Whether this provider can serve a canonical target language.
    fn supports(&self, target: &str) -> bool {
        self.info().languages.iter().any(|l| l == target || l == "*")
    }

    /// Whether this provider can serve a language pair. `source` is `auto`
    /// when the caller wants detection. Defaults to the target check; override
    /// for endpoints whose support is zh-anchored rather than per-language.
    fn supports_pair(&self, source: &str, target: &str) -> bool {
        let _ = source;
        self.supports(target)
    }
}

/// Every provider name this binary can serve, configured or not, including
/// retired names kept alive as aliases. Distinguishes an unknown name from one
/// that simply has no credentials yet.
pub const KNOWN_PROVIDERS: &[&str] = &[
    // Free web endpoints
    "google", "edge", "iciba", "iflyrec", "modernmt", "volcengine", "lara",
    "lingvanex", "mirai", "papago", "reverso", "sogou", "mymemory", "systran",
    "libretranslate",
    // Retired upstream, aliased to `iflyrec`
    "iflytek",
    // Official APIs
    "deepl", "azure", "googlecloud", "baidu", "aliyun", "tencent", "youdao",
    "caiyun", "niutrans",
    // LLM
    "openai", "anthropic", "deepseek",
];

pub fn is_known_provider(name: &str) -> bool {
    KNOWN_PROVIDERS.contains(&name)
}

/// Registered providers the registry never appends by priority.
///
/// `reverso` fails Cloudflare's TLS-fingerprint check: rustls is rejected while
/// browsers and Python `requests` get through. Appending it to every automatic
/// chain buys a failed round trip, so it only runs when the caller names it in
/// `--provider`, `--fallback` or the configured `chain`. It stays visible in
/// `translatex providers`.
pub const DEFAULT_EXCLUDED: &[&str] = &["reverso"];

pub fn is_default_excluded(name: &str) -> bool {
    DEFAULT_EXCLUDED.contains(&name)
}

#[derive(Default)]
pub struct Registry {
    entries: BTreeMap<String, Arc<dyn Provider>>,
    order: Vec<String>,
    /// Retired engine name -> live engine that replaces it, so old names keep working.
    aliases: BTreeMap<String, String>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, provider: Arc<dyn Provider>) {
        let name = provider.info().name.clone();
        if !self.order.contains(&name) {
            self.order.push(name.clone());
        }
        self.entries.insert(name, provider);
    }

    /// Point a retired name at the provider that now serves it.
    pub fn alias(&mut self, from: &str, to: &str) {
        if self.entries.contains_key(to) {
            self.aliases.insert(from.to_string(), to.to_string());
        }
    }

    pub fn aliases(&self) -> &BTreeMap<String, String> {
        &self.aliases
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        match self.entries.get(name) {
            Some(p) => Some(p.clone()),
            None => self
                .aliases
                .get(name)
                .and_then(|target| self.entries.get(target))
                .cloned(),
        }
    }

    pub fn names(&self) -> &[String] {
        &self.order
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.order.iter().filter_map(|n| self.entries.get(n))
    }

    /// Providers in attempt order for a request, de-duplicated.
    ///
    /// `fallback_chain` keeps the caller's order. The configured `default_order`
    /// follows it; anything else registered is appended by ascending priority
    /// (web endpoints before keyed APIs). A pinned provider still honours
    /// `fallback_chain`, so `--provider deepl --fallback openai,google` degrades
    /// as written.
    pub fn chain(
        &self,
        requested: Option<&str>,
        fallback: &[String],
        default_order: &[String],
        source: &str,
        target: &str,
    ) -> Result<Vec<Arc<dyn Provider>>> {
        let source = crate::core::lang::normalize(source);
        let detects = source == "auto";
        let accepts = |p: &Arc<dyn Provider>| {
            let info = p.info();
            (target == "auto" || p.supports_pair(&source, target))
                && (!detects || info.auto_detect)
        };

        let mut seen: Vec<String> = Vec::new();
        let mut out: Vec<Arc<dyn Provider>> = Vec::new();

        fn push(
            registry: &Registry,
            name: &str,
            out: &mut Vec<Arc<dyn Provider>>,
            seen: &mut Vec<String>,
        ) {
            if let Some(p) = registry.get(name) {
                // Dedupe on the resolved provider so an alias and its target
                // cannot both land in the chain.
                let real = p.info().name;
                if !seen.contains(&real) {
                    seen.push(real);
                    out.push(p);
                }
            }
        }

        if let Some(name) = requested {
            let Some(provider) = self.get(name) else {
                return Err(if is_known_provider(name) {
                    Error::ProviderNotConfigured { name: name.to_string() }
                } else {
                    Error::ProviderNotFound { name: name.to_string() }
                });
            };

            if accepts(&provider) {
                out.push(provider);
                for name in fallback.iter().map(String::as_str) {
                    push(self, name, &mut out, &mut seen);
                }
                return Ok(out);
            }

            // A pinned provider that cannot serve this request is only an error
            // when there is nothing to fall back to.
            if fallback.is_empty() {
                let info = provider.info();
                return Err(if detects && !info.auto_detect {
                    Error::AutoDetectUnsupported { provider: info.name }
                } else {
                    Error::LanguagePairNotSupported {
                        from_lang: source.clone(),
                        target: target.to_string(),
                        provider: info.name,
                    }
                });
            }

            for name in fallback.iter().map(String::as_str) {
                push(self, name, &mut out, &mut seen);
            }
            out.retain(|p| accepts(p));
            if out.is_empty() {
                return Err(Error::NoProvider);
            }
            return Ok(out);
        }

        for name in fallback.iter().map(String::as_str) {
            push(self, name, &mut out, &mut seen);
        }
        for name in default_order.iter().map(String::as_str) {
            push(self, name, &mut out, &mut seen);
        }
        let mut rest: Vec<Arc<dyn Provider>> = self.iter().cloned().collect();
        rest.sort_by_key(|p| p.info().priority);
        for provider in rest {
            let real = provider.info().name;
            if is_default_excluded(&real) {
                continue;
            }
            if !seen.contains(&real) {
                seen.push(real);
                out.push(provider);
            }
        }

        out.retain(|p| accepts(p));
        if out.is_empty() {
            return Err(Error::NoProvider);
        }
        Ok(out)
    }

    pub fn describe(&self) -> Vec<serde_json::Value> {
        self.iter()
            .map(|p| {
                let info = p.info();
                serde_json::to_value(&info).unwrap_or_else(|_| serde_json::Value::Null)
            })
            .collect()
    }
}

pub mod aliyun;
pub mod azure;
pub mod baidu;
pub mod caiyun;
pub mod deepl;
pub mod edge;
pub mod google;
pub mod googlecloud;
pub mod iciba;
pub mod iflyrec;
pub mod lara;
pub mod libretranslate;
pub mod lingvanex;
pub mod llm;
pub mod mirai;
pub mod modernmt;
pub mod mymemory;
pub mod niutrans;
pub mod papago;
pub mod reverso;
pub mod sogou;
pub mod systran;
pub mod tencent;
pub mod volcengine;
pub mod youdao;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::Result as R;
    use async_trait::async_trait;

    struct Stub {
        name: &'static str,
        priority: u8,
        languages: Vec<String>,
        auto_detect: bool,
    }

    #[async_trait]
    impl Provider for Stub {
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.into(),
                display_name: self.name.into(),
                kind: ProviderKind::Web,
                priority: self.priority,
                needs_key: false,
                max_chars: 5000,
                auto_detect: self.auto_detect,
                languages: self.languages.clone(),
                free_tier: None,
                configured: true,
            }
        }

        async fn translate_once(
            &self,
            req: &TranslateRequest,
            _cred: &Credential,
        ) -> R<TranslateResponse> {
            Ok(TranslateResponse {
                text: req.text.clone(),
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

    fn stub(name: &'static str, priority: u8, languages: &[&str]) -> Arc<dyn Provider> {
        Arc::new(Stub {
            name,
            priority,
            languages: languages.iter().map(|l| (*l).to_string()).collect(),
            auto_detect: true,
        })
    }

    fn stub_no_detect(name: &'static str, priority: u8) -> Arc<dyn Provider> {
        Arc::new(Stub {
            name,
            priority,
            languages: vec!["*".into()],
            auto_detect: false,
        })
    }

    fn names(chain: &[Arc<dyn Provider>]) -> Vec<String> {
        chain.iter().map(|p| p.info().name).collect()
    }

    #[test]
    fn unlisted_providers_are_ordered_by_priority() {
        let mut reg = Registry::new();
        reg.register(stub("api", 20, &["*"]));
        reg.register(stub("web", 1, &["*"]));
        let chain = reg
            .chain(None, &[], &[], "en", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["web", "api"]);
    }

    #[test]
    fn configured_chain_order_wins_over_priority() {
        let mut reg = Registry::new();
        reg.register(stub("api", 1, &["*"]));
        reg.register(stub("web", 20, &["*"]));
        let chain = reg
            .chain(None, &[], &["api".to_string(), "web".to_string()], "en", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["api", "web"]);
    }

    #[test]
    fn pinned_provider_still_honours_fallback() {
        let mut reg = Registry::new();
        reg.register(stub("primary", 1, &["*"]));
        reg.register(stub("backup", 30, &["*"]));
        reg.register(stub("never", 2, &["*"]));
        let chain = reg
            .chain(Some("primary"), &["backup".to_string()], &[], "en", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["primary", "backup"]);
    }

    #[test]
    fn pinned_provider_rejects_unsupported_pair() {
        let mut reg = Registry::new();
        reg.register(stub("zh-only", 1, &["zh"]));
        let err = reg
            .chain(Some("zh-only"), &[], &[], "en", "ja")
            .err()
            .expect("pair must be rejected");
        assert_eq!(err.code(), "LANGUAGE_PAIR_NOT_SUPPORTED");
        assert!(err.to_string().contains("zh-only"));
    }

    #[test]
    fn alias_resolves_and_does_not_duplicate() {
        let mut reg = Registry::new();
        reg.register(stub("live", 1, &["*"]));
        reg.alias("retired", "live");
        assert!(reg.get("retired").is_some());
        let chain = reg
            .chain(None, &["retired".to_string(), "live".to_string()], &[], "en", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["live"]);
    }

    #[test]
    fn unknown_provider_is_reported() {
        let reg = Registry::new();
        let err = reg
            .chain(Some("nope"), &[], &[], "en", "zh")
            .err()
            .expect("unknown provider");
        assert_eq!(err.code(), "PROVIDER_NOT_FOUND");
    }

    #[test]
    fn auto_source_skips_providers_without_detection() {
        let mut reg = Registry::new();
        reg.register(stub_no_detect("no-detect", 1));
        reg.register(stub("detects", 2, &["*"]));
        let chain = reg.chain(None, &[], &[], "auto", "zh").expect("chain");
        assert_eq!(names(&chain), vec!["detects"]);
    }

    #[test]
    fn pinned_no_detect_provider_reports_the_real_reason() {
        let mut reg = Registry::new();
        reg.register(stub_no_detect("no-detect", 1));
        let err = reg
            .chain(Some("no-detect"), &[], &[], "auto", "zh")
            .err()
            .expect("detection must be rejected");
        assert_eq!(err.code(), "AUTO_DETECT_UNSUPPORTED");
    }

    #[test]
    fn pinned_provider_hands_over_to_fallback() {
        let mut reg = Registry::new();
        reg.register(stub_no_detect("no-detect", 1));
        reg.register(stub("fallback", 5, &["*"]));
        let chain = reg
            .chain(Some("no-detect"), &["fallback".to_string()], &[], "auto", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["fallback"]);
    }

    #[test]
    fn known_but_unconfigured_provider_is_explicit() {
        let reg = Registry::new();
        // `deepl` is implemented, so this is a configuration problem.
        let err = reg
            .chain(Some("deepl"), &[], &[], "en", "zh")
            .err()
            .expect("unconfigured provider");
        assert_eq!(err.code(), "PROVIDER_NOT_CONFIGURED");
    }

    #[test]
    fn default_excluded_providers_are_only_used_when_named() {
        let mut reg = Registry::new();
        reg.register(stub("reverso", 9, &["*"]));
        reg.register(stub("google", 1, &["*"]));

        // The priority append skips it even though it is registered.
        let chain = reg.chain(None, &[], &[], "en", "zh").expect("chain");
        assert_eq!(names(&chain), vec!["google"]);

        // An explicit fallback list still includes it.
        let chain = reg
            .chain(None, &["reverso".to_string()], &[], "en", "zh")
            .expect("chain");
        assert!(names(&chain).contains(&"reverso".to_string()));

        // So does an explicit `chain` in the config.
        let chain = reg
            .chain(None, &[], &["reverso".to_string()], "en", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["reverso", "google"]);

        // Pinning it works too.
        let chain = reg
            .chain(Some("reverso"), &[], &[], "en", "zh")
            .expect("chain");
        assert_eq!(names(&chain), vec!["reverso"]);
    }
}
