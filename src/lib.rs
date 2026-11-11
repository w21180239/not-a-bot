//! Agent-first translation library: providers, fallback engine, key rotation.

pub mod core;
pub mod providers;

pub mod cli;
pub mod process;
pub mod retry;

pub use core::{config, creds, engine, error, http, keys, lang, preserve, types};
pub use error::{Error, Result};
pub use providers::{Provider, Registry};
pub use types::{
    AttemptRecord, HealthRecord, ProviderInfo, ProviderKind, RewriteRequest, RewriteResponse,
    TranslateRequest, TranslateResponse,
};

pub use core::{Settings, DEFAULT_AUTO_CHAIN};
pub use creds::{Credential, CredentialStore};
pub use engine::{Engine, Outcome};
