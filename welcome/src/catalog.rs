//! The catalog of everything optional: `catalog.tsv` plus the `opcional`
//! rows of `flatpaks.list` (the same two files `tamagi install` reads).

use crate::method::{classify, Method};
use std::path::PathBuf;

pub const DEFAULT_CATALOG: &str = "/usr/share/tamagi/catalog.tsv";
pub const DEFAULT_FLATPAKS: &str = "/usr/share/tamagi/flatpaks.list";

/// The category of the browsers: they have a page of their own.
pub const BROWSERS: &str = "Browsers";

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Catalog id, or the app id of a Flatpak.
    pub id: String,
    /// English category (a translation key).
    pub category: String,
    /// Approximate size as written in the file ("260MB", "1.5GB", "2-9GB").
    pub size: String,
    /// What `tamagi-install --actions` receives.
    pub action: String,
    /// English description (a translation key).
    pub desc: String,
}

impl Item {
    pub fn method(&self) -> Method {
        classify(&self.action)
    }

    pub fn size_mb(&self) -> Option<f64> {
        parse_size_mb(&self.size)
    }
}

/// `catalog.tsv`: id, category, size, action, description, TAB separated.
pub fn parse_catalog(text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.splitn(5, '\t');
        let (Some(id), Some(category), Some(size), Some(action)) = (fields.next(), fields.next(), fields.next(), fields.next()) else {
            continue;
        };
        if id.is_empty() || action.is_empty() {
            continue;
        }
        items.push(Item {
            id: id.to_string(),
            category: category.to_string(),
            size: size.to_string(),
            action: action.to_string(),
            desc: fields.next().unwrap_or("").trim_end().to_string(),
        });
    }
    items
}

/// `flatpaks.list`: remote, app id, kind, category, size and the description
/// to the end of the line, separated by blanks. Only `opcional` rows are offered.
pub fn parse_flatpaks(text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    for line in text.lines() {
        let mut rest = line.trim_start();
        if rest.is_empty() || rest.starts_with('#') {
            continue;
        }
        let mut fields = [""; 5];
        let mut complete = true;
        for field in &mut fields {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            if end == 0 {
                complete = false;
                break;
            }
            *field = &rest[..end];
            rest = rest[end..].trim_start();
        }
        let [remote, app, kind, category, size] = fields;
        if !complete || kind != "opcional" {
            continue;
        }
        items.push(Item {
            id: app.to_string(),
            category: category.to_string(),
            size: size.to_string(),
            action: format!("flatpak:{remote}:{app}"),
            desc: rest.trim_end().to_string(),
        });
    }
    items
}

fn path_from_env(var: &str, default: &str) -> PathBuf {
    std::env::var_os(var).filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(default))
}

/// Both files, in the order the terminal assistant lists them. A missing file
/// is an empty list: the pages still work, with nothing to offer.
pub fn load() -> Vec<Item> {
    let mut items = Vec::new();
    if let Ok(text) = std::fs::read_to_string(path_from_env("TAMAGI_CATALOG", DEFAULT_CATALOG)) {
        items.extend(parse_catalog(&text));
    }
    if let Ok(text) = std::fs::read_to_string(path_from_env("TAMAGI_FLATPAKS", DEFAULT_FLATPAKS)) {
        items.extend(parse_flatpaks(&text));
    }
    items
}

/// "260MB" -> 260, "1.5GB" -> 1536, "2-9GB" -> 2048 (the smaller end of a range).
pub fn parse_size_mb(size: &str) -> Option<f64> {
    let size = size.trim();
    let split = size.find(|c: char| c.is_ascii_alphabetic())?;
    let (number, unit) = size.split_at(split);
    let number = number.split('-').next()?.trim().replace(',', ".");
    let value: f64 = number.parse().ok()?;
    let factor = match unit.trim().to_ascii_uppercase().as_str() {
        "KB" | "K" => 1.0 / 1024.0,
        "MB" | "M" => 1.0,
        "GB" | "G" => 1024.0,
        "TB" | "T" => 1024.0 * 1024.0,
        _ => return None,
    };
    Some(value * factor)
}

/// 260 -> "260 MB", 1536 -> "1.5 GB", 6144 -> "6 GB".
pub fn format_size(mb: f64) -> String {
    if mb.round() >= 1000.0 {
        let gb = (mb / 1024.0 * 10.0).round() / 10.0;
        if gb.fract() == 0.0 {
            format!("{gb:.0} GB")
        } else {
            format!("{gb:.1} GB")
        }
    } else {
        format!("{:.0} MB", mb.round())
    }
}

/// The size as written in the file, spaced like `format_size` ("1.5GB" -> "1.5 GB").
pub fn pretty_size(size: &str) -> String {
    match size.find(|c: char| c.is_ascii_alphabetic()) {
        Some(split) if split > 0 => format!("{} {}", &size[..split], &size[split..]),
        _ => size.to_string(),
    }
}

/// The name of an item, without the explanation that follows it:
/// "Claude Code — Anthropic's coding agent" -> "Claude Code",
/// "Steam + steam-devices, native like on Bazzite (asks for the password)" -> "Steam + steam-devices".
/// Works on the translated text too (", nativo", ", natif").
pub fn short_name(desc: &str) -> String {
    let mut name = desc;
    for cut in [" — ", " – ", ", nati", " ("] {
        if let Some(pos) = name.find(cut) {
            if pos > 0 {
                name = &name[..pos];
            }
        }
    }
    name.trim().to_string()
}

/// The explanation after the name ("Anthropic's coding agent"), if any.
pub fn tagline(desc: &str) -> Option<String> {
    for cut in [" — ", " – "] {
        if let Some(pos) = desc.find(cut) {
            let rest = desc[pos + cut.len()..].trim();
            if !rest.is_empty() {
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// The text without a parenthesis at its end: "Game manager, native (asks
/// for the password)" -> "Game manager, native". Used where a badge already
/// says what the parenthesis says.
pub fn without_trailing_note(text: &str) -> &str {
    let trimmed = text.trim_end();
    if trimmed.ends_with(')') {
        if let Some(pos) = trimmed.rfind(" (") {
            if pos > 0 {
                return trimmed[..pos].trim_end();
            }
        }
    }
    trimmed
}

/// Categories in the order they first appear, without the browsers.
pub fn categories(items: &[Item]) -> Vec<&str> {
    let mut seen: Vec<&str> = Vec::new();
    for item in items {
        if item.category != BROWSERS && !seen.contains(&item.category.as_str()) {
            seen.push(&item.category);
        }
    }
    seen
}

/// Whether an item matches what was typed in the search field. `haystack` is
/// the text shown for it (already translated).
pub fn matches(query: &str, haystack: &[&str]) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let text = haystack.join(" ").to_lowercase();
    query.split_whitespace().all(|word| text.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = "# comment\n\
firefox\tBrowsers\t260MB\trun:tamagi-browser install firefox --flatpak\tFirefox — Mozilla's browser (Flatpak)\n\
\n\
steam\tGames\t900MB\tgaming:steam\tSteam + steam-devices, native like on Bazzite (asks for the password)\n\
local-ai\tAI agents\t2-9GB\tagent:local\tLocal AI — the Tamagi agent with a model on your computer (llama.cpp)\n\
broken line without tabs\n\
nodesc\tGames\t1MB\tgaming:x\n";

    const FLATPAKS: &str = "# comment\n\
flathub com.github.tchx84.Flatseal               essencial   Sistema         2MB    Flatseal — permissões dos apps Flatpak\n\
flathub io.missioncenter.MissionCenter           opcional    System         35MB   Mission Center — system monitor\n\
cosmic  io.github.cosmic_utils.Examine opcional COSMIC 100MB Examine — hardware information  \n\
flathub short.line opcional\n";

    #[test]
    fn catalog_lines() {
        let items = parse_catalog(CATALOG);
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].id, "firefox");
        assert_eq!(items[0].category, "Browsers");
        assert_eq!(items[0].action, "run:tamagi-browser install firefox --flatpak");
        assert_eq!(items[0].desc, "Firefox — Mozilla's browser (Flatpak)");
        assert_eq!(items[1].method(), Method::Native);
        assert_eq!(items[2].size, "2-9GB");
        assert_eq!(items[3].desc, "");
    }

    #[test]
    fn flatpak_lines() {
        let items = parse_flatpaks(FLATPAKS);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "io.missioncenter.MissionCenter");
        assert_eq!(items[0].action, "flatpak:flathub:io.missioncenter.MissionCenter");
        assert_eq!(items[0].category, "System");
        assert_eq!(items[0].size, "35MB");
        assert_eq!(items[0].desc, "Mission Center — system monitor");
        assert_eq!(items[1].action, "flatpak:cosmic:io.github.cosmic_utils.Examine");
        assert_eq!(items[1].desc, "Examine — hardware information");
        assert_eq!(items[1].method(), Method::Flatpak);
    }

    #[test]
    fn sizes() {
        assert_eq!(parse_size_mb("260MB"), Some(260.0));
        assert_eq!(parse_size_mb("1.5GB"), Some(1536.0));
        assert_eq!(parse_size_mb("2-9GB"), Some(2048.0));
        assert_eq!(parse_size_mb("6GB"), Some(6144.0));
        assert_eq!(parse_size_mb("1MB"), Some(1.0));
        assert_eq!(parse_size_mb("big"), None);
        assert_eq!(parse_size_mb(""), None);
        assert_eq!(parse_size_mb("12"), None);
        assert_eq!(format_size(260.0), "260 MB");
        assert_eq!(format_size(1536.0), "1.5 GB");
        assert_eq!(format_size(6144.0), "6 GB");
        assert_eq!(format_size(999.6), "1 GB");
        assert_eq!(format_size(0.0), "0 MB");
        assert_eq!(pretty_size("1.5GB"), "1.5 GB");
        assert_eq!(pretty_size("2-9GB"), "2-9 GB");
        assert_eq!(pretty_size("?"), "?");
    }

    #[test]
    fn names() {
        assert_eq!(short_name("Claude Code — Anthropic's coding agent"), "Claude Code");
        assert_eq!(short_name("Steam + steam-devices, native like on Bazzite (asks for the password)"), "Steam + steam-devices");
        assert_eq!(short_name("gamescope, MangoHud and GameMode, native, with the HDR gaming session (asks for the password)"), "gamescope, MangoHud and GameMode");
        assert_eq!(short_name("Fedora development box (distrobox)"), "Fedora development box");
        assert_eq!(short_name("VS Code with Dev Containers (Podman), native (asks for the password)"), "VS Code with Dev Containers");
        assert_eq!(short_name("Google Chrome (Flatpak)"), "Google Chrome");
        assert_eq!(short_name("Steam + steam-devices, nativo como no Bazzite (pede a senha)"), "Steam + steam-devices");
        assert_eq!(short_name("Vivaldi"), "Vivaldi");
        assert_eq!(tagline("Claude Code — Anthropic's coding agent").as_deref(), Some("Anthropic's coding agent"));
        assert_eq!(tagline("Vivaldi"), None);
        assert_eq!(without_trailing_note("Game manager, native (asks for the password)"), "Game manager, native");
        assert_eq!(without_trailing_note("Gerenciador de jogos, nativo (pede a senha) "), "Gerenciador de jogos, nativo");
        assert_eq!(without_trailing_note("coding agent (in the dev box) and more"), "coding agent (in the dev box) and more");
        assert_eq!(without_trailing_note("(only a note)"), "(only a note)");
        assert_eq!(without_trailing_note("plain"), "plain");
    }

    #[test]
    fn category_order_skips_browsers() {
        let mut items = parse_catalog(CATALOG);
        items.extend(parse_flatpaks(FLATPAKS));
        assert_eq!(categories(&items), ["Games", "AI agents", "System", "COSMIC"]);
    }

    #[test]
    fn search() {
        assert!(matches("", &["anything"]));
        assert!(matches("  STEAM ", &["Steam + steam-devices", "Games"]));
        assert!(matches("games steam", &["Steam + steam-devices", "Games"]));
        assert!(!matches("steam office", &["Steam + steam-devices", "Games"]));
        assert!(matches("flatpak", &["GIMP", "Flatpak"]));
    }

    /// The files shipped in the image parse completely.
    #[test]
    fn real_files() {
        let Some(share) = crate::test_share_dir() else {
            return; // built outside the repository
        };
        let catalog_text = std::fs::read_to_string(share.join("catalog.tsv")).unwrap();
        let flatpaks = std::fs::read_to_string(share.join("flatpaks.list")).unwrap();
        let catalog = parse_catalog(&catalog_text);
        let data_lines = catalog_text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).count();
        assert_eq!(catalog.len(), data_lines);
        assert!(catalog.iter().all(|i| !i.desc.is_empty() && i.size_mb().is_some()), "every catalog line has a description and a size");
        let flatpaks = parse_flatpaks(&flatpaks);
        assert!(!flatpaks.is_empty());
        assert!(flatpaks.iter().all(|i| i.action.starts_with("flatpak:") && !i.desc.is_empty() && i.size_mb().is_some()));
        assert!(flatpaks.iter().all(|i| !i.id.contains("Flatseal")), "essential apps are not offered");
    }
}
