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

/// Chinese covers Simplified, Traditional and Cantonese; all three share the
/// property that matters here, a syntax far from English.
pub fn is_chinese(code: &str) -> bool {
    matches!(code, "zh" | "zh-TW" | "yue")
}

/// Recommended bridge language for a round trip. Chinese input goes out through
/// English; everything else goes out through Chinese, which rewrites word order
/// and drops the filler that makes model output read like model output.
pub fn default_pivot(source: &str) -> &'static str {
    if is_chinese(source) {
        "en"
    } else {
        "zh"
    }
}

/// The other half of the recommended pair, used when the first pivot turns out
/// to be the source language.
pub fn alternate_pivot(pivot: &str) -> &'static str {
    if is_chinese(pivot) {
        "en"
    } else {
        "zh"
    }
}

/// Language implied by the script of `text`, for providers that do not detect:
/// kana -> ja, Hangul -> ko, Han -> zh, Cyrillic -> ru. `None` for Latin and
/// anything else. A script counts when it covers at least 30% of the letters;
/// any kana alongside Han means Japanese.
pub fn detect_script(text: &str) -> Option<&'static str> {
    let (mut han, mut kana, mut hangul, mut cyrillic, mut letters) = (0usize, 0, 0, 0, 0);
    for c in text.chars().filter(|c| c.is_alphabetic()) {
        letters += 1;
        match c {
            '\u{3040}'..='\u{30FF}' => kana += 1,
            '\u{AC00}'..='\u{D7AF}' | '\u{1100}'..='\u{11FF}' => hangul += 1,
            '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' => han += 1,
            '\u{0400}'..='\u{04FF}' => cyrillic += 1,
            _ => {}
        }
    }
    let share = |n: usize| letters > 0 && n * 10 >= letters * 3;
    if kana > 0 && share(han + kana) {
        Some("ja")
    } else if share(hangul) {
        Some("ko")
    } else if share(han) {
        Some("zh")
    } else if share(cyrillic) {
        Some("ru")
    } else {
        None
    }
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

/// Whether a named provider accepts a canonical code. Providers advertise `*`
/// when they accept any code the endpoints understand.
pub fn provider_accepts(languages: &[String], code: &str) -> bool {
    languages.iter().any(|l| l == "*" || l == code)
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
    fn script_detection() {
        assert_eq!(detect_script("这次更新很重要。"), Some("zh"));
        assert_eq!(detect_script("今回の更新は重要です。"), Some("ja"));
        assert_eq!(detect_script("이번 업데이트는 중요합니다."), Some("ko"));
        assert_eq!(detect_script("Это обновление важно."), Some("ru"));
        assert_eq!(detect_script("This update matters."), None);
        assert_eq!(detect_script("12345"), None);
        assert_eq!(detect_script("Aroha 需要 Tom 的批准"), Some("zh"));
    }

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
        assert_eq!(normalize("Klingon"), "klingon");
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
