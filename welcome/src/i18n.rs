//! Translations (en/pt/es/fr), the same scheme as the rest of Tamagi.
//!
//! English is the source language: the key of a message is its English text.
//! The catalogs are `/usr/share/tamagi/i18n/<lang>.tsv` (two TAB separated
//! columns) and the language is found in the same order as
//! `/usr/libexec/tamagi/tamagi_i18n.py`:
//!   TAMAGI_LANG > LC_ALL > LC_MESSAGES > ~/.config/tamagi/lang > LANG

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

pub const DEFAULT_DIR: &str = "/usr/share/tamagi/i18n";

/// "pt_BR.UTF-8" -> "pt"; anything that is not pt/en/es/fr is English.
pub fn lang_code(raw: &str) -> &'static str {
    let code = raw.split('_').next().unwrap_or("");
    let code = code.split('.').next().unwrap_or("").to_lowercase();
    match code.as_str() {
        "pt" => "pt",
        "es" => "es",
        "fr" => "fr",
        _ => "en",
    }
}

/// The language, given a way to read the environment and the first line of
/// `~/.config/tamagi/lang` (read lazily: only when no variable above it is set).
pub fn detect_lang_with(env: impl Fn(&str) -> Option<String>, config_lang: impl FnOnce() -> Option<String>) -> &'static str {
    let get = |name: &str| env(name).filter(|v| !v.is_empty());
    let raw = get("TAMAGI_LANG")
        .or_else(|| get("LC_ALL"))
        .or_else(|| get("LC_MESSAGES"))
        .or_else(|| config_lang().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
        .or_else(|| get("LANG"))
        .unwrap_or_else(|| "en".to_string());
    lang_code(&raw)
}

pub fn config_dir() -> PathBuf {
    match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"),
    }
}

pub fn detect_lang() -> &'static str {
    detect_lang_with(
        |name| std::env::var(name).ok(),
        || {
            let text = std::fs::read_to_string(config_dir().join("tamagi").join("lang")).ok()?;
            text.trim().lines().next().map(str::to_string)
        },
    )
}

/// One language: the table of its `.tsv` file.
#[derive(Debug, Default, Clone)]
pub struct Catalog {
    pub lang: &'static str,
    table: HashMap<String, String>,
}

impl Catalog {
    /// Reads the text of a `.tsv` catalog: `key<TAB>translation`, `#` comments.
    pub fn parse(lang: &'static str, text: &str) -> Self {
        let mut table = HashMap::new();
        for line in text.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('\t') {
                if !key.is_empty() && !value.is_empty() {
                    table.insert(key.to_string(), value.to_string());
                }
            }
        }
        Self { lang, table }
    }

    /// The catalog of `lang` in `dir`; English (or a missing file) has no table.
    pub fn load(lang: &'static str, dir: &std::path::Path) -> Self {
        if lang == "en" {
            return Self { lang, table: HashMap::new() };
        }
        match std::fs::read_to_string(dir.join(format!("{lang}.tsv"))) {
            Ok(text) => Self::parse(lang, &text),
            Err(_) => Self { lang, table: HashMap::new() },
        }
    }

    pub fn from_env() -> Self {
        let dir = std::env::var_os("TAMAGI_I18N_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_DIR));
        Self::load(detect_lang(), &dir)
    }

    /// The translation of `text`, or `text` itself when there is none.
    pub fn t<'a>(&'a self, text: &'a str) -> &'a str {
        self.table.get(text).map(String::as_str).unwrap_or(text)
    }
}

static CURRENT: OnceLock<Catalog> = OnceLock::new();

fn current() -> &'static Catalog {
    CURRENT.get_or_init(Catalog::from_env)
}

/// The language of this run ("en", "pt", "es" or "fr").
pub fn lang() -> &'static str {
    current().lang
}

/// Translates a message of the interface. The literal strings passed here are
/// the ones collected in NEW_STRINGS.txt (see tools/new-strings.sh).
pub fn tr(text: &str) -> String {
    current().t(text).to_string()
}

/// First letter in upper case ("not now" -> "Not now"): some keys shared with
/// the terminal assistant are written in lower case.
pub fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| vars.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
    }

    #[test]
    fn codes() {
        assert_eq!(lang_code("pt_BR.UTF-8"), "pt");
        assert_eq!(lang_code("FR"), "fr");
        assert_eq!(lang_code("es.UTF-8"), "es");
        assert_eq!(lang_code("en_US"), "en");
        assert_eq!(lang_code("de_DE.UTF-8"), "en");
        assert_eq!(lang_code("C"), "en");
        assert_eq!(lang_code(""), "en");
    }

    #[test]
    fn priority_order() {
        let all = [("TAMAGI_LANG", "fr"), ("LC_ALL", "pt_BR.UTF-8"), ("LC_MESSAGES", "es_ES"), ("LANG", "en_US.UTF-8")];
        assert_eq!(detect_lang_with(env(&all), || Some("es".into())), "fr");
        assert_eq!(detect_lang_with(env(&all[1..]), || Some("es".into())), "pt");
        assert_eq!(detect_lang_with(env(&all[2..]), || Some("fr".into())), "es");
        // The file of the user comes before LANG...
        assert_eq!(detect_lang_with(env(&all[3..]), || Some("pt\n".into())), "pt");
        // ...and LANG is what is left.
        assert_eq!(detect_lang_with(env(&[("LANG", "es_AR.UTF-8")]), || None), "es");
        assert_eq!(detect_lang_with(env(&[]), || None), "en");
    }

    #[test]
    fn empty_values_are_skipped() {
        let vars = [("TAMAGI_LANG", ""), ("LC_ALL", ""), ("LANG", "pt_BR.UTF-8")];
        assert_eq!(detect_lang_with(env(&vars), || Some("  ".into())), "pt");
    }

    #[test]
    fn config_file_is_not_read_when_a_variable_wins() {
        let lang = detect_lang_with(env(&[("LC_ALL", "pt_BR")]), || panic!("must not be read"));
        assert_eq!(lang, "pt");
    }

    #[test]
    fn lookup() {
        let c = Catalog::parse("pt", "# comment\nYes\tSim\nBroken line\n\tno key\nEmpty\t\nGames\tJogos\textra\n");
        assert_eq!(c.t("Yes"), "Sim");
        assert_eq!(c.t("Games"), "Jogos\textra");
        assert_eq!(c.t("Broken line"), "Broken line");
        assert_eq!(c.t("Empty"), "Empty");
        assert_eq!(c.t("Unknown"), "Unknown");
    }

    #[test]
    fn english_has_no_table() {
        let c = Catalog::load("en", std::path::Path::new("/nonexistent"));
        assert_eq!(c.t("Yes"), "Yes");
        let c = Catalog::load("pt", std::path::Path::new("/nonexistent"));
        assert_eq!(c.t("Yes"), "Yes");
    }

    #[test]
    fn capital() {
        assert_eq!(capitalized("not now"), "Not now");
        assert_eq!(capitalized("élan"), "Élan");
        assert_eq!(capitalized(""), "");
    }
}
