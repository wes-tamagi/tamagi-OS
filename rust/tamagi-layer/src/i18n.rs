//! Messages in four languages: English (the source), Portuguese (Brazil),
//! Spanish and French — the same catalogs and the same rules as the shell
//! library /usr/libexec/tamagi/i18n.
//!
//! The key is the English text itself; translations live in
//! /usr/share/tamagi/i18n/<language>.tsv (English<TAB>translation, one per
//! line). With no translation, the English text is shown. Language priority:
//!   TAMAGI_LANG > LC_ALL > LC_MESSAGES > ~/.config/tamagi/lang > LANG
//! Languages other than pt/es/fr fall back to English.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Catalog {
    table: HashMap<String, String>,
}

fn nonempty(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty())
}

/// The language code out of a raw value such as "pt_BR.UTF-8".
pub fn code(raw: &str) -> &'static str {
    let low = raw.to_lowercase();
    if low.starts_with("pt") {
        "pt"
    } else if low.starts_with("es") {
        "es"
    } else if low.starts_with("fr") {
        "fr"
    } else {
        "en"
    }
}

fn detect() -> &'static str {
    if let Some(v) = nonempty("TAMAGI_LANG").or_else(|| nonempty("LC_ALL")).or_else(|| nonempty("LC_MESSAGES")) {
        return code(&v);
    }
    let config = nonempty("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config"));
    // A readable file decides, even when it is empty (then: English).
    if let Ok(text) = fs::read_to_string(config.join("tamagi/lang")) {
        return code(text.lines().next().unwrap_or(""));
    }
    code(&nonempty("LANG").unwrap_or_else(|| "en_US".into()))
}

/// One catalog line: English<TAB>translation. Comments and half lines are skipped.
pub fn parse_line(line: &str) -> Option<(&str, &str)> {
    let (k, v) = line.split_once('\t')?;
    if k.is_empty() || k.starts_with('#') || v.is_empty() {
        return None;
    }
    Some((k, v))
}

impl Catalog {
    pub fn load() -> Catalog {
        let lang = detect();
        let mut table = HashMap::new();
        if lang != "en" {
            let dir = nonempty("TAMAGI_I18N_DIR").unwrap_or_else(|| "/usr/share/tamagi/i18n".into());
            if let Ok(text) = fs::read_to_string(format!("{dir}/{lang}.tsv")) {
                for (k, v) in text.lines().filter_map(parse_line) {
                    table.insert(k.to_string(), v.to_string());
                }
            }
        }
        Catalog { table }
    }

    /// The translation of `text`, or `text` itself.
    pub fn t<'a>(&'a self, text: &'a str) -> &'a str {
        self.table.get(text).map(String::as_str).unwrap_or(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes() {
        assert_eq!(code("pt_BR.UTF-8"), "pt");
        assert_eq!(code("PT"), "pt");
        assert_eq!(code("es_ES"), "es");
        assert_eq!(code("fr"), "fr");
        assert_eq!(code("de_DE.UTF-8"), "en");
        assert_eq!(code("en_US.UTF-8"), "en");
        assert_eq!(code(""), "en");
    }

    #[test]
    fn catalog_lines() {
        assert_eq!(parse_line("Quit\tSair"), Some(("Quit", "Sair")));
        assert_eq!(parse_line("a\tb\tc"), Some(("a", "b\tc")));
        assert_eq!(parse_line("# comment\tx"), None);
        assert_eq!(parse_line("no tab"), None);
        assert_eq!(parse_line("key\t"), None);
        assert_eq!(parse_line("\tvalue"), None);
    }
}
