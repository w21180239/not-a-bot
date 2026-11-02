//! Credentials: multiple keys per provider, loaded from config file and env.

use crate::core::error::{Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Required fields per provider. An entry is only usable when all are present.
static KEY_TABLE: Lazy<BTreeMap<&'static str, &'static [&'static str]>> = Lazy::new(|| {
    BTreeMap::from([
        ("deepl", &["api_key"][..]),
        ("azure", &["api_key"][..]),
        ("googlecloud", &["api_key"][..]),
        ("baidu", &["appid", "secret_key"][..]),
        ("aliyun", &["access_key_id", "access_key_secret"][..]),
        ("tencent", &["secret_id", "secret_key"][..]),
        ("youdao", &["app_key", "app_secret"][..]),
        ("caiyun", &["token"][..]),
        ("niutrans", &["api_key"][..]),
        ("openai", &["api_key"][..]),
        ("anthropic", &["api_key"][..]),
        ("deepseek", &["api_key"][..]),
    ])
});

/// Section names accepted in `credentials.toml`, mapped onto canonical names.
/// Covers the names used by the legacy `translators` config so an existing
/// file loads unchanged.
const SECTION_ALIASES: &[(&str, &str)] = &[
    ("deeplapi", "deepl"),
    ("deepl_api", "deepl"),
    ("googlecloud", "googlecloud"),
    ("google_cloud", "googlecloud"),
    ("google", "googlecloud"),
    ("baiduapi", "baidu"),
    ("baidu_api", "baidu"),
    ("aliyunapi", "aliyun"),
    ("aliyun_api", "aliyun"),
    ("alibaba", "aliyun"),
    ("tencentapi", "tencent"),
    ("youdaoapi", "youdao"),
    ("youdao_api", "youdao"),
    ("caiyunapi", "caiyun"),
    ("caiyun_api", "caiyun"),
    ("niutransapi", "niutrans"),
    ("volcengineapi", "volcengine"),
    ("volc_engine_api", "volcengine"),
    ("azure", "azure"),
];

/// Env-var prefixes for a canonical provider: its own name plus every alias.
fn env_prefixes(provider: &str) -> Vec<String> {
    let mut names = vec![provider.to_uppercase()];
    for (alias, target) in SECTION_ALIASES {
        if *target == provider {
            names.push(alias.to_uppercase());
        }
    }
    names
}

/// Canonical provider name for a config section or env prefix.
pub fn canonical_provider(name: &str) -> String {
    let lower = name.trim().to_lowercase().replace('-', "_");
    match SECTION_ALIASES.iter().find(|(alias, _)| *alias == lower) {
        Some((_, target)) => (*target).to_string(),
        None => lower,
    }
}

/// Field names accepted per provider, including legacy spellings.
const FIELD_ALIASES: &[(&str, &str)] = &[
    ("secretaccesskey", "secret_access_key"),
    ("secret_key", "secret_key"),
    ("accesskeyid", "access_key_id"),
    ("accesskeysecret", "access_key_secret"),
    ("apikey", "api_key"),
    ("appkey", "app_key"),
    ("appsecret", "app_secret"),
];

fn canonical_field(field: &str) -> String {
    let lower = field.trim().to_lowercase().replace('-', "_");
    match FIELD_ALIASES.iter().find(|(alias, _)| *alias == lower) {
        Some((_, target)) => (*target).to_string(),
        None => lower,
    }
}

/// One credential set for a provider. `label` distinguishes entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub provider: String,
    pub label: String,
    pub fields: BTreeMap<String, String>,
}

impl Credential {
    pub fn get(&self, field: &str) -> Option<&str> {
        self.fields.get(field).map(String::as_str)
    }

    pub fn is_complete(&self, provider: &str) -> bool {
        KEY_TABLE
            .get(provider)
            .map(|fields| fields.iter().all(|f| self.get(f).is_some_and(|v| !v.is_empty())))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CredentialStore {
    by_provider: BTreeMap<String, Vec<Credential>>,
    warning: Option<String>,
}

impl CredentialStore {
    pub fn load(path: Option<std::path::PathBuf>) -> Result<Self> {
        let mut store = Self::default();
        let Some(file) = path.or_else(default_path) else {
            store.load_env_only();
            return Ok(store);
        };
        if file.exists() {
            store.load_file(&file)?;
        }
        store.load_env();
        Ok(store)
    }

    /// Parse the credentials TOML. Later tables override earlier ones.
    fn load_file(&mut self, file: &Path) -> Result<()> {
        let text = std::fs::read_to_string(file)
            .map_err(|e| Error::Config(format!("{}: {e}", file.display())))?;
        if permissions_readable_by_others(file) {
            self.warning = Some(format!(
                "credentials file {} is readable by other users; tighten permissions",
                file.display()
            ));
        }
        let value: toml::Value = toml::from_str(&text).map_err(|e| {
            Error::Config(format!("{}: {e}", file.display()))
        })?;
        let root = value.as_table().ok_or_else(|| {
            Error::Config("credentials file must be a table of tables".into())
        })?;

        for (section, table) in root {
            let Some(section_table) = table.as_table() else {
                continue;
            };
            let provider = canonical_provider(section);

            // Flat form: [deeplApi] with fields directly on the table.
            let flat: BTreeMap<String, String> = section_table
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (canonical_field(k), s.to_string())))
                .collect();
            if !flat.is_empty() {
                self.push(Credential {
                    provider: provider.clone(),
                    label: "default".into(),
                    fields: expand_keyring(&provider, flat)?,
                });
            }
            // Labelled form: [deeplApi.work] tables for extra keys.
            for (label, sub) in section_table {
                let Some(sub_table) = sub.as_table() else { continue };
                let fields: BTreeMap<String, String> = sub_table
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (canonical_field(k), s.to_string())))
                    .collect();
                if fields.is_empty() {
                    continue;
                }
                self.push(Credential {
                    provider: provider.clone(),
                    label: label.clone(),
                    fields: expand_keyring(&provider, fields)?,
                });
            }
        }
        Ok(())
    }

    /// `TRANSLATEX_<PROVIDER>_<FIELD>` env vars become an env-labelled entry.
    /// Aliased section names work here too: `TRANSLATEX_DEEPLAPI_API_KEY`.
    fn load_env(&mut self) {
        for (provider, fields) in KEY_TABLE.iter() {
            let mut collected = BTreeMap::new();
            for field in *fields {
                for name in env_prefixes(provider) {
                    let var = format!("TRANSLATEX_{}_{}", name, field.to_uppercase());
                    if let Ok(v) = std::env::var(&var) {
                        if !v.is_empty() {
                            collected.insert((*field).to_string(), v);
                            break;
                        }
                    }
                }
            }
            let cred = Credential {
                provider: (*provider).to_string(),
                label: "env".into(),
                fields: collected,
            };
            if cred.is_complete(provider) {
                self.push(cred);
            }
        }
    }

    fn load_env_only(&mut self) {
        self.load_env();
    }

    fn push(&mut self, cred: Credential) {
        if !cred.is_complete(&cred.provider) {
            return;
        }
        self.by_provider
            .entry(cred.provider.clone())
            .or_default()
            .push(cred);
    }

    /// Keys for a provider in rotation order: config order first, env last.
    pub fn keys(&self, provider: &str) -> &[Credential] {
        self.by_provider
            .get(provider)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// Insert a credential directly. Used by tests and by callers that hold
    /// keys in memory rather than on disk.
    pub fn add(&mut self, cred: Credential) {
        let complete = cred.is_complete(&cred.provider);
        let entry = self.by_provider.entry(cred.provider.clone()).or_default();
        if complete {
            entry.push(cred);
        }
    }

    /// Credential labels in rotation order. Empty when unconfigured.
    pub fn labels(&self, provider: &str) -> Vec<String> {
        self.keys(provider).iter().map(|c| c.label.clone()).collect()
    }

    pub fn is_configured(&self, provider: &str) -> bool {
        !self.keys(provider).is_empty()
    }

    pub fn configured_providers(&self) -> Vec<String> {
        self.by_provider.keys().cloned().collect()
    }

    pub fn warning(&self) -> Option<&str> {
        self.warning.as_deref()
    }
}

fn expand_keyring(_provider: &str, mut fields: BTreeMap<String, String>) -> Result<BTreeMap<String, String>> {
    for (k, v) in fields.iter_mut() {
        if v == "@keyring" {
            let secret = read_keyring(k).ok_or_else(|| {
                Error::Config(format!("no keyring entry for {k}"))
            })?;
            *v = secret;
        }
    }
    Ok(fields)
}

const KEYRING_SERVICE: &str = "translators";

fn read_keyring(account: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, account)
        .ok()?
        .get_password()
        .ok()
}

/// Migrate the legacy `translators` keyring service accounts into this store's
/// naming so existing OS keychain entries keep working.
pub fn legacy_account(provider: &str, field: &str) -> String {
    format!("{provider}.{field}")
}

pub fn default_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("TRANSLATEX_CREDENTIALS") {
        return Some(PathBuf::from(p));
    }
    let home = dirs_home()?;
    Some(home.join(".config").join("translatex").join("credentials.toml"))
}

fn dirs_home() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
}

/// True when the mode grants read to group or other. POSIX only; Windows has
/// no mode bits so the check never fires there.
#[cfg(not(unix))]
fn permissions_readable_by_others(_path: &Path) -> bool {
    false
}

#[cfg(unix)]
fn permissions_readable_by_others(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    // Fail only when the file is not already restricted to the owner.
    meta.permissions().mode() & 0o777 != 0o600
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(dir: &TempDir, body: &str) -> PathBuf {
        let path = dir.path().join("credentials.toml");
        std::fs::write(&path, body).unwrap();
        path
    }

    #[test]
    fn flat_and_labelled_tables_both_load() {
        let dir = TempDir::new().unwrap();
        let path = write(
            &dir,
            r#"
[deeplApi]
api_key = "primary"

[deeplApi.free]
api_key = "secondary"
"#,
        );
        let store = CredentialStore::load(Some(path.clone())).unwrap();
        let keys = store.keys("deepl");
        assert_eq!(keys.len(), 2, "got {keys:?}");
        assert_eq!(keys[0].get("api_key"), Some("primary"));
        assert_eq!(keys[1].label, "free");
        assert_eq!(keys[1].get("api_key"), Some("secondary"));
    }

    #[test]
    fn multi_field_provider_requires_all_fields() {
        let dir = TempDir::new().unwrap();
        let path = write(
            &dir,
            r#"
[baiduApi]
appid = "123"

[youdaoApi]
app_key = "k"
app_secret = "s"
"#,
        );
        let store = CredentialStore::load(Some(path.clone())).unwrap();
        assert!(!store.is_configured("baidu"), "partial credentials rejected");
        assert!(store.is_configured("youdao"));
    }

    #[test]
    fn unknown_tables_are_ignored() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, "[bogus]\napi_key = \"x\"\n");
        let store = CredentialStore::load(Some(path.clone())).unwrap();
        assert!(store.configured_providers().is_empty());
    }

    #[test]
    fn malformed_toml_is_a_config_error() {
        let dir = TempDir::new().unwrap();
        let path = write(&dir, "not = = toml");
        let err = CredentialStore::load(Some(path.clone())).unwrap_err();
        assert_eq!(err.code(), "CONFIG_ERROR");
    }

    #[test]
    fn legacy_keyring_account_name_is_stable() {
        assert_eq!(legacy_account("deeplApi", "api_key"), "deeplApi.api_key");
    }

    #[test]
    fn missing_file_yields_empty_store() {
        let store = CredentialStore::load(Some(PathBuf::from("/nonexistent/creds.toml"))).unwrap();
        assert!(store.configured_providers().is_empty());
    }
}
