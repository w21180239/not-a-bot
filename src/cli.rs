//! Command line surface. stdout carries machine-readable output only;
//! diagnostics go to stderr so agents can pipe results directly.

use crate::core::config::{Settings, CONFIG_TEMPLATE};
use crate::core::creds::CredentialStore;
use crate::core::engine::Engine;
use crate::core::error::{Error, Result};
use crate::providers::Registry;
use crate::core::types::{ErrorResponse, HealthRecord, TranslateRequest};
use clap::{Parser, Subcommand};
use std::io::Write;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "translatex",
    version,
    about = "Agent-first translation CLI. Structured JSON output, provider fallback, key rotation.",
    long_about = None,
    after_help = "EXAMPLES\n  \
      translatex translate \"Hello world\" -t zh\n  \
      translatex translate -t zh --json '{\"text\":\"Hello\",\"sourceLang\":\"en\"}'\n  \
      echo 'Hello' | translatex translate -t ja\n  \n\
      translatex batch --file jobs.ndjson --parallel 4\n  \
      translatex providers --json\n  \n\
      translatex languages --provider baidu\n  \
      translatex keys deepl --json\n  \n\
      translatex health\n  \
      translatex schema\n  \
      translatex roundtrip \"In today's landscape, …\" --from en\n\n\
EXIT CODES\n  \
  0 success  1 request error  2 config error  3 chain exhausted  4 network error"
)]
pub struct Cli {
    /// Output format. json prints a single object, text prints the bare translation.
    #[arg(long, value_enum, default_value = "json", global = true)]
    pub format: Format,

    /// Print one compact JSON object per line instead of pretty output.
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Include per-attempt fallback trail in the result.
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Provider attempts to keep in flight for `auto` (1 = strictly sequential).
    #[arg(long, global = true, value_name = "N")]
    pub race: Option<usize>,

    /// Override the config file path.
    #[arg(long, global = true, env = "TRANSLATEX_CONFIG")]
    pub config: Option<String>,

    /// Override the credentials file path.
    #[arg(long, global = true, env = "TRANSLATEX_CREDENTIALS")]
    pub credentials: Option<String>,

    /// Emit logs on stderr at this level (error, warn, info, debug).
    #[arg(long, global = true, env = "TRANSLATEX_LOG")]
    pub log: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    Json,
    Text,
}

#[derive(Subcommand)]
pub enum Command {
    /// Translate one text. Reads stdin when no positional text is given.
    Translate {
        /// Text to translate. Omit to read stdin.
        text: Option<String>,

        /// Target language code (zh, en, ja, ...).
        #[arg(short, long)]
        to: Option<String>,

        /// Source language code, or auto.
        #[arg(short = 's', long, default_value = "auto")]
        from: String,

        /// Provider name, or auto to walk the fallback chain.
        #[arg(short, long, default_value = "auto")]
        provider: String,

        /// Comma-separated provider list to try before the default chain.
        #[arg(long, value_delimiter = ',')]
        fallback: Vec<String>,

        /// Regex for spans that must not be translated. Repeatable.
        #[arg(long)]
        preserve: Vec<String>,

        /// Request timeout in milliseconds.
        #[arg(long)]
        timeout_ms: Option<u64>,

        /// Caller-supplied id; echoed back and used to key cooldowns.
        #[arg(long)]
        request_id: Option<String>,

        /// Full request as a JSON object; replaces the other flags.
        #[arg(long)]
        json: Option<String>,
    },

    /// Translate out through a pivot language and back, to strip machine-flavoured phrasing.
    #[command(alias = "polish")]
    Roundtrip {
        /// Text to round-trip. Omit to read stdin.
        text: Option<String>,

        /// Source language code, or auto.
        #[arg(short = 's', long, default_value = "auto")]
        from: String,

        /// Bridge language, or auto for the recommended pair.
        #[arg(long, default_value = "auto")]
        pivot: String,

        /// Provider name, or auto to walk the fallback chain.
        #[arg(short, long, default_value = "auto")]
        provider: String,

        /// Comma-separated provider list to try before the default chain.
        #[arg(long, value_delimiter = ',')]
        fallback: Vec<String>,

        /// Regex for spans that must not be translated. Repeatable.
        #[arg(long)]
        preserve: Vec<String>,

        /// Per-hop timeout in milliseconds.
        #[arg(long)]
        timeout_ms: Option<u64>,

        /// Caller-supplied id; echoed back.
        #[arg(long)]
        request_id: Option<String>,

        /// Full request as a JSON object; replaces the other flags.
        #[arg(long)]
        json: Option<String>,
    },

    /// Translate many requests from an NDJSON file or stdin.
    Batch {
        /// NDJSON input, one TranslateRequest per line. Defaults to stdin.
        #[arg(short, long)]
        file: Option<String>,

        /// Requests to run at once.
        #[arg(long, default_value_t = 4)]
        parallel: usize,
    },

    /// List providers with capability and configuration state.
    Providers,

    /// List language codes accepted by a provider, or all canonical codes.
    Languages {
        #[arg(short, long, default_value = "all")]
        provider: String,
    },

    /// Key rotation state: cooldowns and failure counts.
    Keys {
        /// Limit to one provider.
        provider: Option<String>,

        /// Clear all cooldowns.
        #[arg(long)]
        reset: bool,
    },

    /// Probe every configured provider.
    Health,

    /// Print the JSON Schema for the request and response types.
    Schema,

    /// Print the tool manifest for agent discovery.
    Manifest,

    /// Print or write the default config file template.
    Init {
        /// Write to this path instead of stdout.
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Generate a shell completion script.
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// Exit codes agents branch on.
pub mod exit {
    pub const OK: i32 = 0;
    pub const REQUEST: i32 = 1;
    pub const CONFIG: i32 = 2;
    pub const CHAIN: i32 = 3;
    pub const NETWORK: i32 = 4;
}

pub fn exit_code(err: &Error) -> i32 {
    match err.code() {
        "CONFIG_ERROR" => exit::CONFIG,
        "CHAIN_EXHAUSTED" | "NO_PROVIDER" => exit::CHAIN,
        "NETWORK_ERROR" | "TIMEOUT" => exit::NETWORK,
        _ => exit::REQUEST,
    }
}

pub async fn run(cli: Cli) -> i32 {
    init_tracing(cli.log.as_deref());
    match dispatch(&cli).await {
        Ok(()) => exit::OK,
        Err(err) => {
            emit_error(&cli, &err);
            exit_code(&err)
        }
    }
}

fn init_tracing(level: Option<&str>) {
    use tracing_subscriber::EnvFilter;
    let filter = level
        .and_then(|l| EnvFilter::try_new(l).ok())
        .unwrap_or_else(|| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}

async fn dispatch(cli: &Cli) -> Result<()> {
    match &cli.command {
        Command::Schema => emit_json(cli, &schema_document()?),
        Command::Manifest => emit_json(cli, &manifest_document()?),
        Command::Init { path } => {
            match path {
                Some(p) => {
                    let path = std::path::PathBuf::from(p);
                    if let Some(dir) = path.parent() {
                        if !dir.as_os_str().is_empty() {
                            std::fs::create_dir_all(dir)
                                .map_err(|e| Error::Config(format!("{p}: {e}")))?;
                        }
                    }
                    std::fs::write(&path, CONFIG_TEMPLATE)
                        .map_err(|e| Error::Config(format!("{p}: {e}")))?;
                    eprintln!("wrote {p}");
                }
                None => print!("{CONFIG_TEMPLATE}"),
            }
            Ok(())
        }
        Command::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(
                *shell,
                &mut Cli::command(),
                "translatex",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        other => {
            let ctx = Ctx::build(cli)?;
            match other {
                Command::Translate { .. } => cmd_translate(cli, &ctx).await,
                Command::Roundtrip { .. } => cmd_roundtrip(cli, &ctx).await,
                Command::Batch { .. } => cmd_batch(cli, &ctx).await,
                Command::Providers => cmd_providers(cli, &ctx),
                Command::Languages { provider } => cmd_languages(cli, &ctx, provider),
                Command::Keys { provider, reset } => cmd_keys(cli, &ctx, provider.as_deref(), *reset),
                Command::Health => cmd_health(cli, &ctx).await,
                _ => unreachable!("handled above"),
            }
        }
    }
}

struct Ctx {
    engine: Engine,
    registry: Arc<Registry>,
}

pub fn build_registry(creds: &CredentialStore, settings: &Settings) -> Registry {
    use crate::providers::*;
    let mut reg = Registry::new();

    // LibreTranslate is self-hosted software and no public instance is live, so
    // it is registered only when a base_url is configured.
    match settings.provider("libretranslate").base_url.as_deref() {
        Some(base) if !base.trim().is_empty() => {
            reg.register(Arc::new(libretranslate::LibreTranslate::new(Some(base))));
        }
        _ => {}
    }
    reg.register(Arc::new(google::GoogleFree::new()));
    reg.register(Arc::new(volcengine::Volcengine::new()));
    // New free providers
    reg.register(Arc::new(mymemory::MyMemory::new(None)));
    reg.register(Arc::new(edge::Edge::new()));
    reg.register(Arc::new(iciba::Iciba::new()));
    reg.register(Arc::new(lingvanex::Lingvanex::new()));
    reg.register(Arc::new(mirai::Mirai::new()));
    reg.register(Arc::new(systran::SysTran::new()));
    reg.register(Arc::new(papago::Papago::new()));
    reg.register(Arc::new(reverso::Reverso::new()));
    reg.register(Arc::new(sogou::Sogou::new()));
    reg.register(Arc::new(iflyrec::Iflyrec::new()));
    reg.register(Arc::new(lara::Lara::new()));
    reg.register(Arc::new(modernmt::ModernMt::new()));
    // fanyi.xfyun.cn now needs a logged-in account, so the old `iflytek` web
    // endpoints are gone; iflyrec is iFlytek's open one. Same move as the fork.
    reg.alias("iflytek", "iflyrec");
    build_keyed(&mut reg, creds, settings);
    reg
}

/// Every provider reads its credential from the `Credential` argument, so the
/// constructor only takes non-secret settings. Registration is gated on the
/// presence of a complete credential set.
fn build_keyed(reg: &mut Registry, creds: &CredentialStore, settings: &Settings) {
    use crate::providers::*;
    let has = |p: &str| creds.is_configured(p);
    let opts = |p: &str| settings.provider(p);

    if has("deepl") {
        reg.register(Arc::new(deepl::DeepL::new(&opts("deepl"))));
    }
    if has("azure") {
        reg.register(Arc::new(azure::Azure::new(&opts("azure"))));
    }
    if has("googlecloud") {
        reg.register(Arc::new(googlecloud::GoogleCloud::new(&opts("googlecloud"))));
    }
    if has("baidu") {
        reg.register(Arc::new(baidu::Baidu::new()));
    }
    if has("aliyun") {
        reg.register(Arc::new(aliyun::Aliyun::new(&opts("aliyun"))));
    }
    if has("tencent") {
        reg.register(Arc::new(tencent::Tencent::new(&opts("tencent"))));
    }
    if has("youdao") {
        reg.register(Arc::new(youdao::Youdao::new()));
    }
    if has("caiyun") {
        reg.register(Arc::new(caiyun::Caiyun::new(&opts("caiyun"))));
    }
    if has("niutrans") {
        reg.register(Arc::new(niutrans::Niutrans::new(&opts("niutrans"))));
    }
    if has("openai") {
        reg.register(Arc::new(llm::Llm::openai(&opts("openai"))));
    }
    if has("anthropic") {
        reg.register(Arc::new(llm::Llm::anthropic(&opts("anthropic"))));
    }
    if has("deepseek") {
        reg.register(Arc::new(llm::Llm::deepseek(&opts("deepseek"))));
    }
}

impl Ctx {
    fn build(cli: &Cli) -> Result<Self> {
        let mut settings = Settings::load(cli.config.as_ref().map(std::path::PathBuf::from))?;
        if let Some(width) = cli.race {
            settings.race_width = Some(width);
        }
        let creds = CredentialStore::load(
            cli.credentials.as_ref().map(std::path::PathBuf::from),
        )?;
        if let Some(w) = creds.warning() {
            tracing::warn!("{w}");
        }
        let registry = Arc::new(build_registry(&creds, &settings));
        let engine = Engine::new(
            registry.clone(),
            creds,
            crate::core::config::cache_dir(),
            settings,
        );
        Ok(Self { engine, registry })
    }
}

fn read_stdin_text() -> Result<String> {
    use std::io::Read;
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::InvalidInput(format!("read stdin: {e}")))?;
    let text = buf.trim_end_matches(['\r', '\n']).to_string();
    if text.is_empty() {
        return Err(Error::InvalidInput("no text: pass an argument or pipe stdin".into()));
    }
    Ok(text)
}

async fn cmd_translate(cli: &Cli, ctx: &Ctx) -> Result<()> {
    let Command::Translate {
        text,
        to,
        from,
        provider,
        fallback,
        preserve,
        timeout_ms,
        request_id,
        json,
    } = &cli.command
    else {
        unreachable!()
    };

    let req: TranslateRequest = match json {
        Some(raw) => serde_json::from_str(raw)
            .map_err(|e| Error::InvalidInput(format!("--json: {e}")))?,
        None => {
            let target = to.clone().ok_or_else(|| {
                Error::InvalidInput("missing --to <language>".into())
            })?;
            TranslateRequest {
                text: text.clone().map(Ok).unwrap_or_else(read_stdin_text)?,
                source_lang: from.clone(),
                target_lang: target,
                provider: normalize_provider(provider),
                fallback_chain: fallback.clone(),
                preserve: preserve.clone(),
                timeout_ms: *timeout_ms,
                idempotency_key: request_id.clone(),
                metadata: Default::default(),
            }
        }
    };

    match ctx.engine.translate(&req).await {
        Ok(out) => {
            if cli.format == Format::Text {
                println!("{}", out.response.text);
                Ok(())
            } else {
                emit_json(cli, &render_success(cli, &out))
            }
        }
        Err(err) => Err(err),
    }
}

fn normalize_provider(name: &str) -> Option<String> {
    match name.trim().to_lowercase().as_str() {
        "" | "auto" => None,
        other => Some(other.to_string()),
    }
}

async fn cmd_roundtrip(cli: &Cli, ctx: &Ctx) -> Result<()> {
    use crate::core::types::RoundtripRequest;

    let Command::Roundtrip {
        text,
        from,
        pivot,
        provider,
        fallback,
        preserve,
        timeout_ms,
        request_id,
        json,
    } = &cli.command
    else {
        unreachable!()
    };

    let req: RoundtripRequest = match json {
        Some(raw) => serde_json::from_str(raw)
            .map_err(|e| Error::InvalidInput(format!("--json: {e}")))?,
        None => RoundtripRequest {
            text: text.clone().map(Ok).unwrap_or_else(read_stdin_text)?,
            source_lang: from.clone(),
            pivot: pivot.clone(),
            provider: normalize_provider(provider),
            fallback_chain: fallback.clone(),
            preserve: preserve.clone(),
            timeout_ms: *timeout_ms,
            idempotency_key: request_id.clone(),
            metadata: Default::default(),
        },
    };

    let out = ctx.engine.roundtrip(&req).await?;
    if cli.format == Format::Text {
        println!("{}", out.response.text);
        return Ok(());
    }
    let mut root = serde_json::to_value(&out.response).unwrap_or_default();
    if cli.verbose {
        if let Some(obj) = root.as_object_mut() {
            obj.insert(
                "attempts".into(),
                serde_json::to_value(&out.attempts).unwrap_or_default(),
            );
        }
    }
    emit_json(cli, &root)
}

fn render_success(cli: &Cli, out: &crate::core::engine::Outcome) -> serde_json::Value {
    let mut root = serde_json::to_value(&out.response).unwrap_or_default();
    if cli.verbose {
        if let Some(obj) = root.as_object_mut() {
            obj.insert(
                "attempts".into(),
                serde_json::to_value(&out.attempts).unwrap_or_default(),
            );
        }
    }
    root
}

async fn cmd_batch(cli: &Cli, ctx: &Ctx) -> Result<()> {
    let Command::Batch { file, parallel } = &cli.command else { unreachable!() };
    let raw = match file {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| Error::InvalidInput(format!("{path}: {e}")))?,
        None => {
            let mut buf = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                .map_err(|e| Error::InvalidInput(format!("read stdin: {e}")))?;
            buf
        }
    };

    let mut lines: Vec<(usize, TranslateRequest)> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        // Strip BOM (U+FEFF) that may appear at the start of the first line
        let line = line.trim_start_matches('\u{FEFF}').trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<TranslateRequest>(line) {
            Ok(req) => lines.push((i, req)),
            Err(e) => warnings.push(format!("line {}: {e}", i + 1)),
        }
    }
    for w in &warnings {
        tracing::warn!("{w}");
    }
    if lines.is_empty() {
        return Err(Error::InvalidInput("no valid requests".into()));
    }

    use futures::StreamExt;
    let engine = &ctx.engine;
    let out = std::io::stdout();
    let mut out = out.lock();

    let results = futures::stream::iter(lines)
        .map(|(idx, req)| async move {
            let body = match engine.translate(&req).await {
                Ok(o) => serde_json::json!({
                    "index": idx, "ok": true, "result": o.response,
                }),
                Err(e) => serde_json::json!({
                    "index": idx, "ok": false, "error": e.wire(),
                }),
            };
            body
        })
        .buffer_unordered(*parallel)
        .collect::<Vec<_>>()
        .await;

    // Keep input order in the output regardless of completion order.
    let mut results = results;
    results.sort_by_key(|v| v.get("index").and_then(|i| i.as_u64()).unwrap_or(0));
    for body in results {
        writeln!(out, "{body}")
            .map_err(|e| Error::Internal(format!("write: {e}")))?;
    }
    Ok(())
}

fn cmd_providers(cli: &Cli, ctx: &Ctx) -> Result<()> {
    let rows = ctx.registry.describe();
    if cli.format == Format::Text {
        for row in &rows {
            let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let needs = row.get("needsKey").and_then(|v| v.as_bool()).unwrap_or(false);
            let configured = row.get("configured").and_then(|v| v.as_bool()).unwrap_or(false);
            let status = if !needs {
                "free"
            } else if configured {
                "configured"
            } else {
                "no keys"
            };
            println!("{name}\t{status}");
        }
        return Ok(());
    }
    emit_json(cli, &serde_json::json!({ "providers": rows }))
}

fn cmd_languages(cli: &Cli, ctx: &Ctx, provider: &str) -> Result<()> {
    // Provider capabilities come from the registry, not a static table: a
    // provider that advertises `*` genuinely accepts any canonical code.
    let codes: Vec<String> = if provider == "all" {
        crate::core::lang::LANGUAGES
            .iter()
            .map(|(c, _)| c.to_string())
            .collect()
    } else {
        let info = ctx
            .registry
            .get(provider)
            .ok_or_else(|| {
                if crate::providers::is_known_provider(provider) {
                    Error::ProviderNotConfigured { name: provider.into() }
                } else {
                    Error::ProviderNotFound { name: provider.into() }
                }
            })?
            .info();
        if info.languages.iter().any(|l| l == "*") {
            crate::core::lang::LANGUAGES
                .iter()
                .map(|(c, _)| c.to_string())
                .collect()
        } else {
            info.languages
                .iter()
                .filter(|c| crate::core::lang::is_known(c))
                .cloned()
                .collect()
        }
    };
    emit_json(
        cli,
        &serde_json::json!({ "provider": provider, "languages": codes }),
    )
}

fn cmd_keys(cli: &Cli, ctx: &Ctx, provider: Option<&str>, reset: bool) -> Result<()> {
    let names: Vec<String> = match provider {
        Some(p) => vec![p.to_string()],
        None => ctx.engine.creds().configured_providers(),
    };
    if reset {
        for name in &names {
            if let Some(path) = crate::core::keys::state_path(
                crate::core::config::cache_dir().as_deref(),
                name,
            ) {
                let _ = std::fs::remove_file(path);
            }
        }
        eprintln!("cooldowns cleared");
        return Ok(());
    }

    let now = chrono::Utc::now();
    let mut out = serde_json::Map::new();
    for name in names {
        let labels = ctx
            .registry
            .get(&name)
            .map(|p| p.credential_labels())
            .unwrap_or_default();
        let path = crate::core::keys::state_path(
            crate::core::config::cache_dir().as_deref(),
            &name,
        );
        let ring = crate::core::keys::KeyRing::new(labels, path);
        out.insert(
            name,
            serde_json::to_value(ring.snapshot(now)).unwrap_or_default(),
        );
    }
    emit_json(cli, &serde_json::Value::Object(out))
}

async fn cmd_health(cli: &Cli, ctx: &Ctx) -> Result<()> {
    let mut records = Vec::new();
    for provider in ctx.registry.iter() {
        let info = provider.info();
        let keys = ctx.engine.creds().keys(&info.name);
        if info.needs_key && keys.is_empty() {
            records.push(HealthRecord {
                provider: info.name.clone(),
                configured: false,
                healthy: false,
                latency_ms: None,
                error: Some("no credentials".into()),
            });
            continue;
        }
        let cred = keys
            .first()
            .cloned()
            .unwrap_or_else(|| crate::core::creds::Credential {
                provider: info.name.clone(),
                label: "default".into(),
                fields: Default::default(),
            });
        let started = std::time::Instant::now();
        match provider.probe(&cred).await {
            Ok(()) => records.push(HealthRecord {
                provider: info.name.clone(),
                configured: true,
                healthy: true,
                latency_ms: Some(started.elapsed().as_millis() as u64),
                error: None,
            }),
            Err(e) => records.push(HealthRecord {
                provider: info.name.clone(),
                configured: !info.needs_key,
                healthy: false,
                latency_ms: None,
                error: Some(e.to_string()),
            }),
        }
    }
    let all_healthy = records.iter().all(|r| !r.configured || r.healthy);
    emit_json(cli, &serde_json::json!({ "healthy": all_healthy, "providers": records }))?;
    if all_healthy {
        Ok(())
    } else {
        Err(Error::Internal("one or more providers unhealthy".into()))
    }
}

fn emit_json(cli: &Cli, value: &serde_json::Value) -> Result<()> {
    let out = if cli.quiet {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .map_err(|e| Error::Internal(e.to_string()))?;
    println!("{out}");
    Ok(())
}

fn emit_error(cli: &Cli, err: &Error) {
    let body = ErrorResponse {
        error: err.wire(),
        attempts: Vec::new(),
        request_id: String::new(),
    };
    match serde_json::to_string(&body) {
        Ok(json) if cli.format == Format::Json => eprintln!("{json}"),
        _ => eprintln!("error: {err}"),
    }
}

pub fn schema_document() -> Result<serde_json::Value> {
    use schemars::schema_for;

    let mut defs = serde_json::Map::new();
    for (name, value) in [
        (
            "TranslateRequest",
            serde_json::to_value(schema_for!(TranslateRequest))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "TranslateResponse",
            serde_json::to_value(schema_for!(crate::core::types::TranslateResponse))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "ErrorResponse",
            serde_json::to_value(schema_for!(ErrorResponse))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "ProviderInfo",
            serde_json::to_value(schema_for!(crate::core::types::ProviderInfo))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "HealthRecord",
            serde_json::to_value(schema_for!(HealthRecord))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "AttemptRecord",
            serde_json::to_value(schema_for!(crate::core::types::AttemptRecord))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "RoundtripRequest",
            serde_json::to_value(schema_for!(crate::core::types::RoundtripRequest))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
        (
            "RoundtripResponse",
            serde_json::to_value(schema_for!(crate::core::types::RoundtripResponse))
                .map_err(|e| Error::Internal(e.to_string()))?,
        ),
    ] {
        defs.insert(name.to_string(), value);
    }

    let mut doc = serde_json::Map::new();
    doc.insert(
        "$schema".into(),
        serde_json::Value::String("https://json-schema.org/draft/2020-12/schema".into()),
    );
    doc.insert(
        "title".into(),
        serde_json::Value::String("translatex".into()),
    );
    doc.insert(
        "version".into(),
        serde_json::Value::String(crate::core::types::VERSION.to_string()),
    );
    doc.insert("definitions".into(), serde_json::Value::Object(defs));
    Ok(serde_json::Value::Object(doc))
}

#[cfg(test)]
fn schema_types() -> &'static [&'static str] {
    &[
        "TranslateRequest",
        "TranslateResponse",
        "ErrorResponse",
        "ProviderInfo",
        "HealthRecord",
        "AttemptRecord",
        "RoundtripRequest",
        "RoundtripResponse",
    ]
}

/// Machine-readable description of the CLI operations an agent can call.
pub fn manifest_document() -> Result<serde_json::Value> {
    let translate_schema = serde_json::to_value(schemars::schema_for!(TranslateRequest))
        .map_err(|e| Error::Internal(e.to_string()))?;
    let roundtrip_schema = serde_json::to_value(schemars::schema_for!(
        crate::core::types::RoundtripRequest
    ))
    .map_err(|e| Error::Internal(e.to_string()))?;
    Ok(serde_json::json!({
        "name": "translatex",
        "version": crate::core::types::VERSION,
        "description": "Translation with web-first provider fallback, key rotation and structured output.",
        "interface": "cli",
        "skill": "https://github.com/w21180239/translatex/blob/main/integrations/skill/translatex/SKILL.md",
        "commands": [
            {
                "name": "translate",
                "command": "translatex translate --json '{request}' --format json",
                "description": "Translate text between languages. Falls back across providers and rotates API keys automatically.",
                "inputSchema": translate_schema
            },
            {
                "name": "roundtrip",
                "command": "translatex roundtrip --json '{request}' --format json",
                "description": "Translate through a pivot language and back into the source language, which rewrites the phrasing and strips machine-flavoured cadence.",
                "inputSchema": roundtrip_schema
            },
            {
                "name": "providers",
                "command": "translatex providers --format json",
                "description": "List translation providers, their capabilities and whether credentials are configured."
            },
            {
                "name": "languages",
                "command": "translatex languages --provider <name> --format json",
                "description": "List supported language codes, optionally filtered by provider."
            },
            {
                "name": "health",
                "command": "translatex health --format json",
                "description": "Probe configured providers and report latency and errors."
            }
        ],
        "install": {
            "cargo": "cargo install translatex",
            "npm": "npm install -g translatex",
            "binary": "https://github.com/w21180239/translatex/releases/latest"
        }
    }))
}


#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn exit_codes_are_distinct() {
        let codes = [exit::OK, exit::REQUEST, exit::CONFIG, exit::CHAIN, exit::NETWORK];
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert_eq!(unique.len(), codes.len());
    }

    #[test]
    fn exit_code_maps_error_classes() {
        assert_eq!(exit_code(&Error::Config("x".into())), exit::CONFIG);
        assert_eq!(exit_code(&Error::ChainExhausted), exit::CHAIN);
        assert_eq!(exit_code(&Error::NoProvider), exit::CHAIN);
        assert_eq!(exit_code(&Error::Network("x".into())), exit::NETWORK);
        assert_eq!(exit_code(&Error::InvalidInput("x".into())), exit::REQUEST);
    }

    #[test]
    fn auto_provider_means_no_pin() {
        assert_eq!(normalize_provider("auto"), None);
        assert_eq!(normalize_provider(""), None);
        assert_eq!(normalize_provider("DeepL").as_deref(), Some("deepl"));
    }

    #[test]
    fn schema_document_lists_every_type() {
        let doc = schema_document().unwrap();
        let defs = doc["definitions"].as_object().expect("definitions");
        for ty in schema_types() {
            assert!(defs.contains_key(&ty.to_string()), "missing schema for {ty}");
        }
        assert!(doc.get("version").is_some());
        assert!(doc.get("$schema").is_some());
    }

    #[test]
    fn manifest_lists_cli_commands() {
        let doc = manifest_document().unwrap();
        let names: Vec<&str> = doc["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&"translate"), "got {names:?}");
        assert!(names.contains(&"roundtrip"));
        assert!(names.contains(&"providers"));
        assert_eq!(doc["name"], "translatex");
        assert_eq!(doc["interface"], "cli");
        assert!(doc.get("transport").is_none());
    }

    #[test]
    fn schema_is_stable_between_calls() {
        assert_eq!(
            serde_json::to_string(&schema_document().unwrap()).unwrap(),
            serde_json::to_string(&schema_document().unwrap()).unwrap()
        );
    }
}
