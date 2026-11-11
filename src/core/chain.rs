//! Round-trip planning: turn a request's chain, preset, depth or pivot into
//! hops. Pure, so precedence and validation are testable without providers.

use crate::core::config::Settings;
use crate::core::error::{Error, Result};
use crate::core::lang::{alternate_pivot, default_pivot, is_chinese, is_known, normalize};
use crate::core::types::RewriteRequest;

pub const PRESETS: &[&str] = &["light", "standard", "strong"];

/// Hop target: a language, back to the (possibly still undetected) source, or
/// a repair pass that stays in the current language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Lang(String),
    Source,
    Fix,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HopSpec {
    /// `None` races the fallback chain.
    pub engine: Option<String>,
    pub to: Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub hops: Vec<HopSpec>,
    /// Built from a guess about an `auto` source; rebuild once detection says
    /// the guessed first hop is the source language itself.
    pub guessed: bool,
    /// The final hop back to the source was appended to an explicit chain, so
    /// it is dropped when the chain already ends in the detected source.
    pub appended_back: bool,
}

/// `[engine:]lang` (`claude-cli:zh`, `zh`, `auto:ja`) or `engine:fix`, a
/// repair pass by an LLM engine (`is_llm`) against the original text.
pub fn parse_hop(spec: &str, is_llm: &dyn Fn(&str) -> bool) -> Result<HopSpec> {
    let spec = spec.trim();
    let (engine, lang) = match spec.split_once(':') {
        Some((engine, lang)) => (Some(engine.trim().to_lowercase()), lang),
        None => (None, spec),
    };
    let engine = engine.filter(|e| !e.is_empty() && e != "auto");
    if lang.trim().eq_ignore_ascii_case("fix") {
        return match engine {
            Some(engine) if is_llm(&engine) => Ok(HopSpec {
                engine: Some(engine),
                to: Target::Fix,
            }),
            _ => Err(Error::InvalidInput(format!(
                "chain hop {spec:?}: fix needs an LLM engine, e.g. claude-cli:fix"
            ))),
        };
    }
    let code = normalize(lang);
    if code == "auto" || !is_known(&code) {
        return Err(Error::InvalidInput(format!(
            "chain hop {spec:?}: unknown language {:?}",
            lang.trim()
        )));
    }
    Ok(HopSpec {
        engine,
        to: Target::Lang(code),
    })
}

/// Hops for a request. `source` is the normalised source language (`auto`
/// allowed). Precedence: `chain` > `preset` > `depth` > non-auto `pivot` >
/// configured `rewrite_preset`.
pub fn plan(
    req: &RewriteRequest,
    settings: &Settings,
    source: &str,
    is_llm: &dyn Fn(&str) -> bool,
) -> Result<Plan> {
    if !req.chain.is_empty() {
        return explicit(&req.chain, source, is_llm);
    }
    if let Some(preset) = &req.preset {
        return expand_preset(preset, source, settings, is_llm);
    }
    if let Some(depth) = req.depth {
        return expand_depth(depth, source, settings);
    }
    let pivot = normalize(&req.pivot);
    if pivot != "auto" {
        return single_pivot(&pivot, source);
    }
    expand_preset(&settings.rewrite_preset(), source, settings, is_llm)
}

fn explicit(chain: &[String], source: &str, is_llm: &dyn Fn(&str) -> bool) -> Result<Plan> {
    let mut hops = chain
        .iter()
        .map(|s| parse_hop(s, is_llm))
        .collect::<Result<Vec<_>>>()?;
    let mut prev = source.to_string();
    for hop in &hops {
        // A repair pass stays in the current language.
        let Target::Lang(lang) = &hop.to else {
            continue;
        };
        if *lang == prev {
            return Err(Error::InvalidInput(format!(
                "chain hop into {lang} follows text already in {lang}"
            )));
        }
        prev = lang.clone();
    }
    let appended_back = prev != source || source == "auto";
    if appended_back {
        hops.push(HopSpec {
            engine: None,
            to: Target::Source,
        });
    }
    Ok(Plan {
        hops,
        guessed: false,
        appended_back,
    })
}

fn single_pivot(pivot: &str, source: &str) -> Result<Plan> {
    if !is_known(pivot) {
        return Err(Error::InvalidInput(format!(
            "unknown pivot language {pivot}"
        )));
    }
    // Sending text to its own language is not a round trip.
    let pivot = if pivot == source {
        alternate_pivot(pivot)
    } else {
        pivot
    };
    Ok(mt(vec![pivot.to_string()], false))
}

fn expand_depth(depth: u8, source: &str, settings: &Settings) -> Result<Plan> {
    if !(1..=3).contains(&depth) {
        return Err(Error::InvalidInput(format!(
            "depth must be 1-3, got {depth}"
        )));
    }
    let configured = settings.pivot_override().map(|p| normalize(&p));
    if depth == 1 {
        if let Some(pivot) = &configured {
            return single_pivot(pivot, source);
        }
    }
    // A configured pivot leads; the rest follow the default path, skipping the
    // source and repeats (a Japanese source must not hop into Japanese).
    let path: [&str; 4] = if is_chinese(source) {
        ["en", "ja", "fi", "ko"]
    } else {
        ["zh", "ja", "fi", "ko"]
    };
    let mut langs: Vec<String> = configured
        .into_iter()
        .filter(|p| p.as_str() != source)
        .collect();
    for lang in path {
        if langs.len() == depth as usize {
            break;
        }
        if lang != source && !langs.iter().any(|l| l == lang) {
            langs.push(lang.to_string());
        }
    }
    Ok(mt(langs, source == "auto"))
}

fn mt(langs: Vec<String>, guessed: bool) -> Plan {
    let mut hops: Vec<HopSpec> = langs
        .into_iter()
        .map(|lang| HopSpec {
            engine: None,
            to: Target::Lang(lang),
        })
        .collect();
    hops.push(HopSpec {
        engine: None,
        to: Target::Source,
    });
    Plan {
        hops,
        guessed,
        appended_back: false,
    }
}

fn expand_preset(
    name: &str,
    source: &str,
    settings: &Settings,
    is_llm: &dyn Fn(&str) -> bool,
) -> Result<Plan> {
    let name = name.trim().to_lowercase();
    let llm = settings.rewrite_llm();
    let hop = |engine: &str, to: Target| HopSpec {
        engine: Some(engine.to_string()),
        to,
    };
    let lang = |code: &str| Target::Lang(code.to_string());
    let out = if is_chinese(source) { "en" } else { "zh" };
    let hops = match name.as_str() {
        "light" => return expand_depth(2, source, settings),
        "standard" => vec![hop(&llm, lang(out)), hop(&llm, Target::Source)],
        "strong" => vec![
            hop(&llm, lang(out)),
            hop(&llm, lang("ja")),
            hop("google", lang("fi")),
            hop("edge", Target::Source),
            parse_hop(&format!("{llm}:fix"), is_llm)?,
        ],
        other => {
            return Err(Error::InvalidInput(format!(
                "unknown rewrite preset {other:?}; valid: {}",
                PRESETS.join(", ")
            )))
        }
    };
    Ok(Plan {
        hops,
        guessed: source == "auto",
        appended_back: false,
    })
}

/// Language for a hop back to the source: the detected language when known,
/// otherwise the recommended partner of the first hop. Never the language the
/// text is already in.
pub fn resolve_source(detected: &str, first_hop: &str, prev: &str) -> String {
    let back = if detected == "auto" || !is_known(detected) {
        default_pivot(first_hop).to_string()
    } else {
        detected.to_string()
    };
    if back == prev {
        alternate_pivot(prev).to_string()
    } else {
        back
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn llm(name: &str) -> bool {
        name.ends_with("-cli")
    }

    #[test]
    fn fix_hops_need_an_llm_engine() {
        assert_eq!(
            parse_hop("claude-cli:fix", &llm).unwrap(),
            HopSpec {
                engine: Some("claude-cli".into()),
                to: Target::Fix
            }
        );
        for bad in ["google:fix", "fix", "auto:fix"] {
            assert_eq!(
                parse_hop(bad, &llm).unwrap_err().code(),
                "INVALID_INPUT",
                "{bad}"
            );
        }
    }

    #[test]
    fn chain_ending_in_fix_adds_no_return_hop() {
        let mut r = req();
        r.chain = vec!["zh".into(), "edge:en".into(), "opencode-cli:fix".into()];
        let p = plan(&r, &Settings::default(), "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "en", "FIX"]);
        assert!(!p.appended_back);

        // A fix outside the source language still returns afterwards.
        r.chain = vec!["zh".into(), "claude-cli:fix".into()];
        let p = plan(&r, &Settings::default(), "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "FIX", "SRC"]);
        // A fix between hops does not count as a language change.
        r.chain = vec!["zh".into(), "claude-cli:fix".into(), "zh".into()];
        assert!(plan(&r, &Settings::default(), "en", &llm).is_err());
    }

    #[test]
    fn strong_preset_ends_with_a_repair_pass() {
        let mut r = req();
        let s = Settings::default();
        r.preset = Some("strong".into());
        let p = plan(&r, &s, "zh", &llm).unwrap();
        assert_eq!(langs(&p), vec!["en", "ja", "fi", "SRC", "FIX"]);
        assert_eq!(
            engines(&p),
            vec![
                "opencode-cli",
                "opencode-cli",
                "google",
                "edge",
                "opencode-cli"
            ]
        );
        let s = Settings {
            rewrite_llm: Some("google".into()),
            ..Settings::default()
        };
        assert_eq!(
            plan(&r, &s, "en", &llm).unwrap_err().code(),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn old_preset_names_are_rejected() {
        let s = Settings::default();
        for old in ["mt", "mt-deep", "llm", "mt-fix", "llm-fix"] {
            let mut r = req();
            r.preset = Some(old.into());
            let err = plan(&r, &s, "en", &llm).unwrap_err();
            assert_eq!(err.code(), "INVALID_INPUT", "{old}");
            assert!(err.to_string().contains("light, standard, strong"), "{old}");
        }
    }

    #[test]
    fn default_preset_is_standard() {
        let s = Settings::default();
        assert_eq!(s.rewrite_preset(), "standard");
        assert_eq!(s.rewrite_ladder(), vec!["standard", "strong"]);
        let p = plan(&req(), &s, "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "SRC"]);
    }

    fn req() -> RewriteRequest {
        serde_json::from_str(r#"{"text":"hi"}"#).unwrap()
    }

    fn langs(plan: &Plan) -> Vec<String> {
        plan.hops
            .iter()
            .map(|h| match &h.to {
                Target::Lang(l) => l.clone(),
                Target::Source => "SRC".into(),
                Target::Fix => "FIX".into(),
            })
            .collect()
    }

    fn engines(plan: &Plan) -> Vec<String> {
        plan.hops
            .iter()
            .map(|h| h.engine.clone().unwrap_or_else(|| "auto".into()))
            .collect()
    }

    #[test]
    fn hop_specs_parse() {
        assert_eq!(
            parse_hop("claude-cli:zh-CN", &llm).unwrap(),
            HopSpec {
                engine: Some("claude-cli".into()),
                to: Target::Lang("zh".into())
            }
        );
        assert_eq!(parse_hop(" ja ", &llm).unwrap().engine, None);
        assert_eq!(parse_hop("auto:fi", &llm).unwrap().engine, None);
        for bad in ["klingon", "google:", "edge:auto", ""] {
            assert_eq!(
                parse_hop(bad, &llm).unwrap_err().code(),
                "INVALID_INPUT",
                "{bad}"
            );
        }
    }

    #[test]
    fn explicit_chain_appends_the_way_back() {
        let mut r = req();
        r.chain = vec!["claude-cli:zh".into(), "google:fi".into()];
        let p = plan(&r, &Settings::default(), "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "fi", "SRC"]);
        assert_eq!(engines(&p), vec!["claude-cli", "google", "auto"]);
        assert!(p.appended_back && !p.guessed);

        r.chain = vec!["zh".into(), "edge:en".into()];
        let p = plan(&r, &Settings::default(), "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "en"], "already ends in the source");
        assert!(!p.appended_back);
    }

    #[test]
    fn explicit_chain_rejects_repeated_languages() {
        let mut r = req();
        r.chain = vec!["zh".into(), "google:zh".into()];
        let err = plan(&r, &Settings::default(), "en", &llm).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
        r.chain = vec!["en".into(), "zh".into()];
        let err = plan(&r, &Settings::default(), "en", &llm).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT", "first hop equals the source");
    }

    #[test]
    fn depth_follows_the_source_family() {
        let mut r = req();
        r.depth = Some(3);
        let p = plan(&r, &Settings::default(), "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "ja", "fi", "SRC"]);
        assert_eq!(engines(&p), vec!["auto"; 4]);
        let p = plan(&r, &Settings::default(), "zh", &llm).unwrap();
        assert_eq!(langs(&p), vec!["en", "ja", "fi", "SRC"]);
        r.depth = Some(1);
        let p = plan(&r, &Settings::default(), "auto", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "SRC"]);
        assert!(p.guessed);
        for bad in [0, 4] {
            r.depth = Some(bad);
            assert_eq!(
                plan(&r, &Settings::default(), "en", &llm)
                    .unwrap_err()
                    .code(),
                "INVALID_INPUT"
            );
        }
    }

    #[test]
    fn presets_expand_for_both_families() {
        let mut r = req();
        let s = Settings::default();
        r.preset = Some("strong".into());
        let p = plan(&r, &s, "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "ja", "fi", "SRC", "FIX"]);
        let p = plan(&r, &s, "zh", &llm).unwrap();
        assert_eq!(langs(&p), vec!["en", "ja", "fi", "SRC", "FIX"]);

        r.preset = Some("standard".into());
        let s = Settings {
            rewrite_llm: Some("opencode-cli".into()),
            ..Settings::default()
        };
        let p = plan(&r, &s, "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["zh", "SRC"]);
        assert_eq!(engines(&p), vec!["opencode-cli", "opencode-cli"]);
        assert_eq!(langs(&plan(&r, &s, "zh", &llm).unwrap()), vec!["en", "SRC"]);

        r.preset = Some("light".into());
        assert_eq!(
            langs(&plan(&r, &s, "en", &llm).unwrap()),
            vec!["zh", "ja", "SRC"]
        );

        r.preset = Some("bogus".into());
        let err = plan(&r, &s, "en", &llm).unwrap_err();
        assert_eq!(err.code(), "INVALID_INPUT");
        assert!(err.to_string().contains("light, standard, strong"));
    }

    #[test]
    fn precedence_is_chain_preset_depth_pivot_setting() {
        let s = Settings {
            rewrite_preset: Some("strong".into()),
            ..Settings::default()
        };
        let mut r = req();
        r.chain = vec!["ko".into()];
        r.preset = Some("standard".into());
        r.depth = Some(2);
        r.pivot = "ja".into();
        assert_eq!(langs(&plan(&r, &s, "en", &llm).unwrap()), vec!["ko", "SRC"]);
        r.chain.clear();
        assert_eq!(
            engines(&plan(&r, &s, "en", &llm).unwrap())[0],
            "opencode-cli"
        );
        r.preset = None;
        assert_eq!(
            langs(&plan(&r, &s, "en", &llm).unwrap()),
            vec!["zh", "ja", "SRC"]
        );
        r.depth = None;
        assert_eq!(langs(&plan(&r, &s, "en", &llm).unwrap()), vec!["ja", "SRC"]);
        r.pivot = "auto".into();
        assert_eq!(engines(&plan(&r, &s, "en", &llm).unwrap())[3], "edge");
    }

    #[test]
    fn configured_pivot_steers_the_default_path() {
        let s = Settings {
            rewrite_pivot: Some("ja".into()),
            ..Settings::default()
        };
        let mut r = req();
        r.depth = Some(1);
        let p = plan(&r, &s, "auto", &llm).unwrap();
        assert_eq!(langs(&p), vec!["ja", "SRC"]);
        assert!(!p.guessed, "a configured pivot is not a guess");
        // The light preset (depth 2) still leads with the configured pivot.
        r.depth = None;
        r.preset = Some("light".into());
        let p = plan(&r, &s, "en", &llm).unwrap();
        assert_eq!(langs(&p), vec!["ja", "zh", "SRC"]);
    }

    #[test]
    fn depth_path_skips_the_source_language() {
        let mut r = req();
        r.depth = Some(2);
        let s = Settings::default();
        assert_eq!(
            langs(&plan(&r, &s, "ja", &llm).unwrap()),
            vec!["zh", "fi", "SRC"]
        );
        r.depth = Some(3);
        assert_eq!(
            langs(&plan(&r, &s, "fi", &llm).unwrap()),
            vec!["zh", "ja", "ko", "SRC"]
        );
    }

    #[test]
    fn pivot_equal_to_source_moves() {
        let mut r = req();
        r.pivot = "en".into();
        assert_eq!(
            langs(&plan(&r, &Settings::default(), "en", &llm).unwrap()),
            vec!["zh", "SRC"]
        );
        r.pivot = "klingon".into();
        assert_eq!(
            plan(&r, &Settings::default(), "en", &llm)
                .unwrap_err()
                .code(),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn source_resolution_avoids_the_current_language() {
        assert_eq!(resolve_source("en", "zh", "fi"), "en");
        assert_eq!(resolve_source("auto", "zh", "zh"), "en");
        assert_eq!(resolve_source("zh", "zh", "zh"), "en");
    }
}
