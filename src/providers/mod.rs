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
}

#[derive(Default)]
pub struct Registry {
    entries: BTreeMap<String, Arc<dyn Provider>>,
    order: Vec<String>,
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

    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.entries.get(name).cloned()
    }

    pub fn names(&self) -> &[String] {
        &self.order
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<dyn Provider>> {
        self.order.iter().filter_map(|n| self.entries.get(n))
    }

    /// Providers in attempt order for a request, de-duplicated.
    pub fn chain(
        &self,
        requested: Option<&str>,
        fallback: &[String],
        default_order: &[String],
        target: &str,
    ) -> Result<Vec<Arc<dyn Provider>>> {
        let mut seen: Vec<String> = Vec::new();
        let mut chain: Vec<Arc<dyn Provider>> = Vec::new();
        let mut push = |name: &str| {
            if let Some(p) = self.get(name) {
                if !seen.iter().any(|s| s == name) {
                    seen.push(name.to_string());
                    chain.push(p);
                }
            }
        };

        if let Some(name) = requested {
            if self.get(name).is_none() {
                return Err(Error::ProviderNotFound { name: name.to_string() });
            }
            push(name);
            return Ok(chain);
        }

        for name in fallback.iter().map(String::as_str) {
            push(name);
        }
        for name in default_order.iter().map(String::as_str) {
            push(name);
        }
        for name in &self.order {
            push(name);
        }

        let chain: Vec<_> = chain
            .into_iter()
            .filter(|p| p.supports(target) || target == "auto")
            .collect();
        if chain.is_empty() {
            return Err(Error::NoProvider);
        }
        Ok(chain)
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
pub mod iflytek;
pub mod libretranslate;
pub mod lingvanex;
pub mod llm;
pub mod mirai;
pub mod mymemory;
pub mod niutrans;
pub mod papago;
pub mod reverso;
pub mod sogou;
pub mod systran;
pub mod tencent;
pub mod volcengine;
pub mod youdao;