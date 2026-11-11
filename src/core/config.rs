//! Settings: config file, env overrides, defaults.

use crate::core::error::{Error, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Provider attempts kept in flight by default. Racing the top three keeps a
/// hung endpoint from adding its whole timeout to the response, at the cost of
/// up to three concurrent upstream calls.
pub const DEFAULT_RACE_WIDTH: usize = 3;

/// Default `auto` chain: free web endpoints in priority order. Keyed providers
/// are not listed; the registry appends whichever are configured, by ascending
/// priority, after these. Keep this list and the provider `priority` fields in
/// sync — web endpoints run before official APIs by design.
///
/// `reverso` is deliberately absent: its endpoint rejects this client's TLS
/// fingerprint, so every automatic request would burn a round trip before
/// falling through. It stays registered for explicit use; see
/// [`crate::providers::DEFAULT_EXCLUDED`].
pub const DEFAULT_AUTO_CHAIN: &[&str] = &[
    "google",
    "edge",
    "iciba",
    "iflyrec",
    "modernmt",
    "volcengine",
    "lara",
    "lingvanex",
    "mirai",
    "papago",
    "sogou",
    "mymemory",
    "systran",
];

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    /// Overrides the `auto` expansion.
    pub chain: Option<Vec<String>>,
    pub default_target: Option<String>,
    pub timeout_ms: Option<u64>,
    pub retries: Option<u32>,
    pub concurrency: Option<usize>,
    /// Provider attempts kept in flight at once. `1` is strictly sequential.
    pub race_width: Option<usize>,
    /// Default pivot for `rewrite` when the request says `auto`.
    pub rewrite_pivot: Option<String>,
    /// Default `rewrite` preset when the request names no chain, preset,
    /// depth or pivot (`light`, `standard`, `strong`).
    pub rewrite_preset: Option<String>,
    /// Engine behind the LLM hops of the `standard` and `strong` presets.
    pub rewrite_llm: Option<String>,
    /// Detector command for rewrite candidates (see `RewriteRequest::scorer`).
    pub rewrite_scorer: Option<String>,
    /// Presets tried in order when `--target-score` is not met.
    pub rewrite_ladder: Option<Vec<String>>,
    pub log: Option<String>,
    /// provider -> extra knobs (base_url, model, instance, ...).
    pub provider: BTreeMap<String, ProviderSettings>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ProviderSettings {
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub region: Option<String>,
    pub instance: Option<String>,
    pub tier: Option<String>,
    /// Executable path for CLI-backed providers (`claude-cli`, `opencode-cli`).
    pub bin: Option<String>,
}

impl Settings {
    pub fn load(path: Option<PathBuf>) -> Result<Self> {
        let mut settings = Settings::default();
        let Some(file) = path.or_else(default_path) else {
            return Ok(settings.apply_env());
        };
        if file.exists() {
            let text = std::fs::read_to_string(&file)
                .map_err(|e| Error::Config(format!("{}: {e}", file.display())))?;
            settings = toml::from_str::<Settings>(&text)
                .map_err(|e| Error::Config(format!("{}: {e}", file.display())))?;
        }
        Ok(settings.apply_env())
    }

    fn apply_env(mut self) -> Self {
        if let Ok(v) = std::env::var("NOT_A_BOT_CHAIN") {
            let chain: Vec<String> = v
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !chain.is_empty() {
                self.chain = Some(chain);
            }
        }
        if let Ok(v) = std::env::var("NOT_A_BOT_TIMEOUT_MS") {
            if let Ok(ms) = v.parse() {
                self.timeout_ms = Some(ms);
            }
        }
        if let Ok(v) = std::env::var("NOT_A_BOT_LOG") {
            self.log = Some(v);
        }
        if let Ok(v) = std::env::var("NOT_A_BOT_REWRITE_PIVOT") {
            self.rewrite_pivot = Some(v);
        }
        if let Ok(v) = std::env::var("NOT_A_BOT_REWRITE_PRESET") {
            self.rewrite_preset = Some(v);
        }
        if let Ok(v) = std::env::var("NOT_A_BOT_REWRITE_LLM") {
            self.rewrite_llm = Some(v);
        }
        self
    }

    pub fn chain(&self) -> Vec<String> {
        self.chain
            .clone()
            .unwrap_or_else(|| DEFAULT_AUTO_CHAIN.iter().map(|s| s.to_string()).collect())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_ms.unwrap_or(20_000))
    }

    pub fn retries(&self) -> u32 {
        self.retries.unwrap_or(2)
    }

    /// How many providers may be in flight for one request. The default races
    /// three so a hung endpoint cannot hide the healthy ones behind it; `1`
    /// restores the strict sequential walk.
    pub fn race_width(&self) -> usize {
        self.race_width.unwrap_or(DEFAULT_RACE_WIDTH).max(1)
    }

    /// `rewrite_pivot` when it names a language; `auto` and blank values
    /// leave the per-request default in charge.
    pub fn pivot_override(&self) -> Option<String> {
        self.rewrite_pivot
            .as_deref()
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty() && p != "auto")
    }

    /// Preset used when a rewrite request asks for nothing specific.
    pub fn rewrite_preset(&self) -> String {
        self.rewrite_preset
            .as_deref()
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "standard".to_string())
    }

    /// Engine for preset LLM hops.
    pub fn rewrite_llm(&self) -> String {
        self.rewrite_llm
            .as_deref()
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| "opencode-cli".to_string())
    }

    /// Configured scorer command; blank means none.
    pub fn rewrite_scorer(&self) -> Option<String> {
        self.rewrite_scorer
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }

    /// Escalation presets for `--target-score`, in order.
    pub fn rewrite_ladder(&self) -> Vec<String> {
        self.rewrite_ladder
            .clone()
            .unwrap_or_else(|| vec!["standard".into(), "strong".into()])
            .into_iter()
            .map(|p| p.trim().to_lowercase())
            .filter(|p| !p.is_empty())
            .collect()
    }

    pub fn provider(&self, name: &str) -> ProviderSettings {
        self.provider.get(name).cloned().unwrap_or_default()
    }
}

pub fn default_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("NOT_A_BOT_CONFIG") {
        return Some(PathBuf::from(p));
    }
    base_dir().map(|d| d.join("config.toml"))
}

pub fn cache_dir() -> Option<PathBuf> {
    let dir = base_dir()?;
    let cache = dir.join("cache");
    let _ = std::fs::create_dir_all(&cache);
    Some(cache)
}

/// `~/.config/not-a-bot`, honouring `XDG_CONFIG_HOME`.
fn base_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("NOT_A_BOT_HOME") {
        return Some(PathBuf::from(dir));
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("not-a-bot"));
    }
    directories::BaseDirs::new().map(|d| d.home_dir().join(".config").join("not-a-bot"))
}

pub const CONFIG_TEMPLATE: &str = r#"# not-a-bot config
# chain expands `--provider auto`; keyed providers are used only when configured.
# chain = ["deepl", "azure", "google", "googlecloud", "baidu", "libretranslate"]
# default_target = "zh"
# timeout_ms = 20000
# retries = 2
# race_width = 3   # provider attempts in flight; 1 is strictly sequential
# rewrite_pivot = "zh"   # "auto" picks zh for non-Chinese text, en for Chinese
# rewrite_preset = "standard"  # light (2 MT hops) | standard (LLM out and back) | strong
# rewrite_llm = "opencode-cli"   # engine for preset LLM hops: opencode-cli | claude-cli
# rewrite_scorer = "python score.py"   # detector: text on stdin, score 0-1 on the last stdout line
# rewrite_ladder = ["standard", "strong"]   # presets tried when --target-score is not met
# log = "warn"

# [provider.googlecloud]
# region = "global"

# [provider.claude-cli]
# model = "haiku"
# bin = "C:/Users/me/.local/bin/claude.exe"   # default: resolved on PATH

# [provider.opencode-cli]
# model = "opencode-go/deepseek-v4.1-flash"

# [provider.llm]
# base_url = "https://api.openai.com/v1"
# model = "gpt-4o-mini"
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn defaults_produce_a_chain() {
        let s = Settings::default();
        let chain = s.chain();
        // Web-first: the auto chain starts at free web endpoints, not keyed APIs.
        assert_eq!(chain.first().map(String::as_str), Some("google"));
        assert!(chain.contains(&"edge".to_string()));
        assert_eq!(s.retries(), 2);
        assert_eq!(s.timeout(), std::time::Duration::from_millis(20_000));
    }

    #[test]
    fn file_and_env_override() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "chain = [\"google\"]\ntimeout_ms = 1234\n").unwrap();
        let s = Settings::load(Some(path)).unwrap();
        assert_eq!(s.chain(), vec!["google".to_string()]);
        assert_eq!(s.timeout_ms, Some(1234));
    }

    #[test]
    fn unknown_key_is_a_config_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "bogus = 1\n").unwrap();
        let err = Settings::load(Some(path)).unwrap_err();
        assert_eq!(err.code(), "CONFIG_ERROR");
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let s = Settings::load(Some(PathBuf::from("/nope/none.toml"))).unwrap_or_default();
        assert!(!s.chain().is_empty());
    }
}
