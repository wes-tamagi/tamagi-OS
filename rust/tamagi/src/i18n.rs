//! Translations (en/pt/es/fr). English is the source language: the key of a
//! message is its English text. Same TSV catalogs
//! (/usr/share/tamagi/i18n/<lang>.tsv) and the same priority order as
//! /usr/libexec/tamagi/i18n and tamagi_i18n.py:
//!   TAMAGI_LANG > LC_ALL > LC_MESSAGES > ~/.config/tamagi/lang > LANG

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

fn var(name: &str) -> Option<String> {
    env::var(name).ok().filter(|v| !v.is_empty())
}

fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| {
        let home = env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
        home.join(".config")
    })
}

/// "pt_BR.UTF-8" -> "pt"; anything that is not one of the four is English.
pub fn lang_code(raw: &str) -> &'static str {
    let code = raw.split('_').next().unwrap_or("").split('.').next().unwrap_or("").to_lowercase();
    match code.as_str() {
        "pt" => "pt",
        "es" => "es",
        "fr" => "fr",
        _ => "en",
    }
}

pub fn detect_lang() -> &'static str {
    let mut raw = var("TAMAGI_LANG").or_else(|| var("LC_ALL")).or_else(|| var("LC_MESSAGES"));
    if raw.is_none() {
        let cfg = config_dir().join("tamagi").join("lang");
        raw = fs::read_to_string(cfg)
            .ok()
            .and_then(|s| s.trim().lines().next().map(str::to_string))
            .filter(|s| !s.is_empty());
    }
    let raw = raw.or_else(|| var("LANG")).unwrap_or_else(|| "en".into());
    lang_code(&raw)
}

/// A catalog: one "English<TAB>translation" per line; `#` starts a comment.
pub fn parse_catalog(text: &str) -> HashMap<String, String> {
    let mut table = HashMap::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('\t') {
            if !k.is_empty() && !v.is_empty() {
                table.insert(k.to_string(), v.to_string());
            }
        }
    }
    table
}

fn catalog() -> &'static HashMap<String, String> {
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let lang = detect_lang();
        if lang == "en" {
            return HashMap::new();
        }
        let dir = env::var_os("TAMAGI_I18N_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/tamagi/i18n"));
        fs::read(dir.join(format!("{lang}.tsv")))
            .map(|b| parse_catalog(&String::from_utf8_lossy(&b)))
            .unwrap_or_default()
    })
}

/// The message in the language of the system (the English text if there is
/// no translation).
pub fn t(text: &str) -> String {
    catalog().get(text).cloned().unwrap_or_else(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes() {
        assert_eq!(lang_code("pt_BR.UTF-8"), "pt");
        assert_eq!(lang_code("ES"), "es");
        assert_eq!(lang_code("fr.UTF-8"), "fr");
        assert_eq!(lang_code("en_US.UTF-8"), "en");
        assert_eq!(lang_code("de_DE"), "en");
        assert_eq!(lang_code("C"), "en");
    }

    #[test]
    fn catalog_lines() {
        let c = parse_catalog("# comment\nQuit\tSair\n\nno tab here\nempty\t\n\tnokey\nA\tB\tC\r\nQuit\tSair2\n");
        assert_eq!(c.get("Quit").map(String::as_str), Some("Sair2"));
        assert_eq!(c.get("A").map(String::as_str), Some("B\tC"));
        assert_eq!(c.len(), 2);
    }
}
