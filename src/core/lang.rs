//! Canonical language codes and per-provider mappings.

use std::collections::HashMap;

/// Canonical code -> English name. Keys are what the CLI and API accept.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("auto", "auto detect"),
    ("en", "English"),
    ("zh", "Chinese (Simplified)"),
    ("zh-TW", "Chinese (Traditional)"),
    ("yue", "Cantonese"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("fr", "French"),
    ("de", "German"),
    ("es", "Spanish"),
    ("pt", "Portuguese"),
    ("it", "Italian"),
    ("ru", "Russian"),
    ("ar", "Arabic"),
    ("nl", "Dutch"),
    ("pl", "Polish"),
    ("tr", "Turkish"),
    ("th", "Thai"),
    ("vi", "Vietnamese"),
    ("id", "Indonesian"),
    ("ms", "Malay"),
    ("hi", "Hindi"),
    ("he", "Hebrew"),
    ("sv", "Swedish"),
    ("da", "Danish"),
    ("fi", "Finnish"),
    ("no", "Norwegian"),
    ("cs", "Czech"),
    ("el", "Greek"),
    ("hu", "Hungarian"),
    ("ro", "Romanian"),
    ("bg", "Bulgarian"),
    ("et", "Estonian"),
    ("lv", "Latvian"),
    ("lt", "Lithuanian"),
    ("sk", "Slovak"),
    ("sl", "Slovenian"),
    ("hr", "Croatian"),
    ("sr", "Serbian"),
    ("uk", "Ukrainian"),
    ("fa", "Persian"),
    ("ur", "Urdu"),
    ("bn", "Bengali"),
    ("ta", "Tamil"),
    ("te", "Telugu"),
    ("mr", "Marathi"),
    ("gu", "Gujarati"),
    ("kn", "Kannada"),
    ("ml", "Malayalam"),
    ("sw", "Swahili"),
    ("af", "Afrikaans"),
    ("ca", "Catalan"),
    ("eu", "Basque"),
    ("gl", "Galician"),
    ("is", "Icelandic"),
    ("ga", "Irish"),
    ("cy", "Welsh"),
    ("sq", "Albanian"),
    ("az", "Azerbaijani"),
    ("bs", "Bosnian"),
    ("mn", "Mongolian"),
    ("ne", "Nepali"),
    ("ps", "Pashto"),
    ("so", "Somali"),
    ("tl", "Filipino"),
    ("zu", "Zulu"),
];

/// Aliases accepted on input, mapped onto canonical codes.
const ALIASES: &[(&str, &str)] = &[
    ("en-us", "en"),
    ("en-gb", "en"),
    ("zh-cn", "zh"),
    ("zh-hans", "zh"),
    ("chinese", "zh"),
    ("zh-hant", "zh-TW"),
    ("zh-cht", "zh-TW"),
    ("tw", "zh-TW"),
    ("jp", "ja"),
    ("japanese", "ja"),
    ("kr", "ko"),
    ("korean", "ko"),
    ("klingon", "en"),
    ("iw", "he"),
    ("fil", "tl"),
    ("tgl", "tl"),
    ("somal", "so"),
];

/// Normalise user input: trim, case-fold where safe, apply aliases.
pub fn normalize(code: &str) -> String {
    let raw = code.trim();
    if raw.is_empty() {
        return "auto".into();
    }
    let lower = raw.to_lowercase();
    if lower == "auto" || lower == "detect" || lower == "auto-detect" {
        return "auto".into();
    }
    // Preserve the canonical mixed case (zh-TW) after alias lookup.
    for (alias, target) in ALIASES {
        if lower == *alias {
            return (*target).to_string();
        }
    }
    for (canonical, _) in LANGUAGES {
        if lower == canonical.to_lowercase() {
            return (*canonical).to_string();
        }
    }
    lower
}

pub fn is_known(code: &str) -> bool {
    code == "auto" || LANGUAGES.iter().any(|(c, _)| *c == code)
}

pub fn name_of(code: &str) -> Option<&'static str> {
    LANGUAGES.iter().find(|(c, _)| *c == code).map(|(_, n)| *n)
}

/// Per-provider override table builder: `map(&[("zh","zh-CN"), ...])`.
pub fn map(entries: &[(&'static str, &'static str)]) -> HashMap<&'static str, &'static str> {
    entries.iter().copied().collect()
}

/// Map a canonical code through a provider table. When no table is supplied
/// the provider is assumed to speak canonical codes. A miss returns `None`,
/// which callers turn into `LanguagePairNotSupported`.
pub fn upstream(
    table: Option<&HashMap<&'static str, &'static str>>,
    canonical: &str,
) -> Option<String> {
    match table {
        None => Some(canonical.to_string()),
        Some(t) => t.get(canonical).map(|v| (*v).to_string()),
    }
}

/// Whether a named provider accepts a canonical code. `"all"` and unknown
/// providers fall back to the canonical list, so `--languages` stays useful.
pub fn provider_supports(provider: &str, code: &str) -> bool {
    let table: Option<&HashMap<&'static str, &'static str>> = match provider {
        "baidu" => Some(&BAIDU),
        _ => None,
    };
    match table {
        Some(t) => t.contains_key(code),
        None => is_known(code),
    }
}

/// Baidu accepts `auto` for detection and its own codes otherwise.
pub fn baidu_source(code: &str) -> String {
    let c = normalize(code);
    if c == "auto" {
        return "auto".into();
    }
    upstream(Some(&BAIDU), &c).unwrap_or(c)
}

pub fn baidu_target(code: &str) -> Option<String> {
    let c = normalize(code);
    upstream(Some(&BAIDU), &c)
}

/// Baidu's language codes differ from ISO for the Romance family and Chinese.
pub static BAIDU: std::sync::LazyLock<HashMap<&'static str, &'static str>> =
    std::sync::LazyLock::new(|| {
        map(&[
            ("auto", "auto"),
            ("en", "en"),
            ("zh", "zh"),
            ("zh-TW", "cht"),
            ("yue", "yue"),
            ("ja", "jp"),
            ("ko", "kor"),
            ("fr", "fra"),
            ("de", "de"),
            ("es", "spa"),
            ("pt", "pt"),
            ("it", "it"),
            ("ru", "ru"),
            ("ar", "ara"),
            ("nl", "nl"),
            ("pl", "pl"),
            ("tr", "tr"),
            ("th", "th"),
            ("vi", "vie"),
            ("id", "id"),
            ("ms", "ms"),
            ("hi", "hi"),
            ("cs", "cs"),
            ("el", "el"),
            ("hu", "hu"),
            ("ro", "ro"),
            ("bg", "bg"),
            ("et", "et"),
            ("lv", "lv"),
            ("lt", "lt"),
            ("sk", "sk"),
            ("sv", "swe"),
            ("da", "dan"),
            ("fi", "fin"),
            ("no", "nob"),
            ("uk", "ukr"),
            ("he", "iw"),
            ("sr", "srp_cyrillic"),
            ("hr", "hrv"),
            ("bn", "bn"),
            ("ur", "ud"),
        ])
    });

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aliases_resolve_to_canonical() {
        assert_eq!(normalize("zh-CN"), "zh");
        assert_eq!(normalize("ZH-Hant"), "zh-TW");
        assert_eq!(normalize("  auto "), "auto");
        assert_eq!(normalize("iw"), "he");
        assert_eq!(normalize("en-US"), "en");
    }

    #[test]
    fn unknown_codes_pass_through_lowercase() {
        assert_eq!(normalize("Klingon"), "en");
        assert_eq!(normalize("xx"), "xx");
        assert!(!is_known("xx"));
        assert!(is_known("auto"));
    }

    #[test]
    fn canonical_table_is_unique_and_named() {
        let mut seen = std::collections::HashSet::new();
        for (code, name) in LANGUAGES {
            assert!(seen.insert(code), "duplicate code {code}");
            assert!(!name.is_empty());
        }
        assert_eq!(name_of("zh-TW"), Some("Chinese (Traditional)"));
    }

    #[test]
    fn baidu_codes_match_provider_docs() {
        assert_eq!(baidu_source("auto"), "auto");
        assert_eq!(baidu_source("AUTO"), "auto");
        assert_eq!(baidu_source("zh-CN"), "zh");
        assert_eq!(baidu_source("ja"), "jp");
        assert_eq!(baidu_target("fr"), Some("fra".into()));
        assert_eq!(baidu_target("zh-TW"), Some("cht".into()));
        assert_eq!(baidu_target("xx"), None, "unknown code must be rejected");
    }

    #[test]
    fn upstream_mapping_falls_back() {
        let t = map(&[("zh", "zh-CN"), ("auto", "detect")]);
        assert_eq!(upstream(Some(&t), "zh").as_deref(), Some("zh-CN"));
        assert_eq!(upstream(Some(&t), "auto").as_deref(), Some("detect"));
        assert_eq!(upstream(Some(&t), "de"), None);
        assert_eq!(upstream(None, "de").as_deref(), Some("de"));
    }
}
