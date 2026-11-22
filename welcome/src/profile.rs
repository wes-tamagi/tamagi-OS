//! The ready-made profiles (`/usr/share/tamagi/profiles/<name>.toml`): read
//! only to SHOW what each one installs. Applying is `tamagi apply -y <name>`.

use crate::catalog::{short_name, Item};
use crate::method::Method;
use std::path::PathBuf;

pub const DEFAULT_DIR: &str = "/usr/share/tamagi/profiles";

/// The profiles offered, in order. The first is the suggestion.
pub const PROFILES: [&str; 4] = ["ai", "dev", "gamer", "office"];

/// One thing a profile file asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wanted {
    /// `[agents] install = ["claude"]`
    Agent(String),
    /// `[boxes] dev = "fedora"` (the distribution)
    DevBox(String),
    /// `[catalog] install = ["vscode"]` (catalog id)
    Catalog(String),
    /// `[flatpak] apps = ["dev.zed.Zed"]`
    Flatpak(String),
    /// `[browsers] install = ["firefox"]`
    Browser(String),
    /// `[mise] tools = ["node@lts"]`, installed in the development box
    Tool(String),
}

/// Reads the few shapes the profile files use: `[section]`, `key = "text"`
/// and `key = ["a", "b"]` on one line. Anything else is ignored: this only
/// feeds the preview of the card.
pub fn parse(text: &str) -> Vec<Wanted> {
    let mut wanted = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = strip_comment(line).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.trim().to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, values) = (key.trim(), strings(value));
        let make: Option<fn(String) -> Wanted> = match (section.as_str(), key) {
            ("agents", "install") => Some(Wanted::Agent),
            ("catalog", "install") => Some(Wanted::Catalog),
            ("browsers", "install") => Some(Wanted::Browser),
            ("flatpak", "apps") => Some(Wanted::Flatpak),
            ("mise", "tools") => Some(Wanted::Tool),
            ("boxes", _) => Some(Wanted::DevBox),
            _ => None,
        };
        if let Some(make) = make {
            wanted.extend(values.into_iter().map(make));
        }
    }
    wanted
}

/// The line without a `#` comment (a `#` inside quotes stays).
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (pos, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..pos],
            _ => {}
        }
    }
    line
}

/// Every double-quoted string of a value: `"a"` or `["a", "b"]`.
fn strings(value: &str) -> Vec<String> {
    value.split('"').skip(1).step_by(2).map(str::to_string).collect()
}

/// A line of the preview: what is installed and how.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// English description when it comes from the catalog (a translation
    /// key); otherwise the plain name written in the profile.
    pub desc: String,
    pub method: Method,
    pub size_mb: Option<f64>,
}

impl Entry {
    /// The short name shown in cards, from the description already translated.
    pub fn name(&self, translate: impl Fn(&str) -> String) -> String {
        short_name(&translate(&self.desc))
    }
}

/// Matches what the profile asks for with the catalog, to show names, sizes
/// and the install method of each.
pub fn resolve(wanted: &[Wanted], catalog: &[Item]) -> Vec<Entry> {
    let by_action = |action: &str| catalog.iter().find(|i| i.action == action);
    let by_id = |id: &str| catalog.iter().find(|i| i.id == id);
    let from = |item: &Item| Entry { desc: item.desc.clone(), method: item.method(), size_mb: item.size_mb() };
    wanted
        .iter()
        .map(|w| match w {
            Wanted::Agent(name) => by_action(&format!("agent:{name}")).map(from).unwrap_or_else(|| Entry { desc: name.clone(), method: Method::User, size_mb: None }),
            Wanted::DevBox(distro) => by_action(&format!("run:tamagi-devbox create {distro}")).map(from).unwrap_or_else(|| Entry { desc: distro.clone(), method: Method::Box, size_mb: None }),
            Wanted::Catalog(id) => by_id(id).map(from).unwrap_or_else(|| Entry { desc: id.clone(), method: Method::Command, size_mb: None }),
            Wanted::Browser(id) => by_id(id).map(from).unwrap_or_else(|| Entry { desc: id.clone(), method: Method::Flatpak, size_mb: None }),
            Wanted::Flatpak(app) => catalog
                .iter()
                .find(|i| i.method() == Method::Flatpak && i.id == *app)
                .map(from)
                .unwrap_or_else(|| Entry { desc: app.rsplit('.').next().unwrap_or(app).to_string(), method: Method::Flatpak, size_mb: None }),
            Wanted::Tool(tool) => Entry { desc: tool.clone(), method: Method::Box, size_mb: None },
        })
        .collect()
}

/// The entries grouped by install method, in the order of `Method::ALL`.
pub fn by_method(entries: &[Entry]) -> Vec<(Method, Vec<&Entry>)> {
    Method::ALL
        .iter()
        .filter_map(|&m| {
            let group: Vec<&Entry> = entries.iter().filter(|e| e.method == m).collect();
            (!group.is_empty()).then_some((m, group))
        })
        .collect()
}

pub fn dir() -> PathBuf {
    std::env::var_os("TAMAGI_PROFILES").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(DEFAULT_DIR))
}

/// What the profile `name` installs; empty when its file cannot be read
/// (the card then shows only its description).
pub fn load(name: &str, catalog: &[Item]) -> Vec<Entry> {
    match std::fs::read_to_string(dir().join(format!("{name}.toml"))) {
        Ok(text) => resolve(&parse(&text), catalog),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{parse_catalog, parse_flatpaks};

    const DEV: &str = "# Tamagi profile: development.\n\
[boxes]\n\
dev = \"fedora\"\n\
\n\
[mise]\n\
tools = [\"node@lts\", \"python@3.13\"]  # comment\n\
\n\
[catalog]\n\
install = [\"vscode\"]\n\
\n\
[flatpak]\n\
apps = [\"dev.zed.Zed\"]\n\
\n\
[agents]\n\
install = [\"opencode\"]\n\
\n\
[services]\n\
autoupdate = true\n";

    fn catalog() -> Vec<Item> {
        let mut items = parse_catalog(
            "devbox-fedora\tDevelopment\t1.5GB\trun:tamagi-devbox create fedora\tFedora development box (distrobox)\n\
vscode\tDevelopment\t450MB\trun:/usr/libexec/tamagi/vscode-setup\tVS Code with Dev Containers (Podman), native (asks for the password)\n\
opencode\tAI agents\t120MB\tagent:opencode\topencode — open-source coding agent\n\
firefox\tBrowsers\t260MB\trun:tamagi-browser install firefox --flatpak\tFirefox — Mozilla's browser (Flatpak)\n",
        );
        items.extend(parse_flatpaks("flathub dev.zed.Zed opcional Development 250MB Zed — code editor\n"));
        items
    }

    #[test]
    fn reads_the_sections() {
        assert_eq!(
            parse(DEV),
            [
                Wanted::DevBox("fedora".into()),
                Wanted::Tool("node@lts".into()),
                Wanted::Tool("python@3.13".into()),
                Wanted::Catalog("vscode".into()),
                Wanted::Flatpak("dev.zed.Zed".into()),
                Wanted::Agent("opencode".into()),
            ]
        );
        assert_eq!(parse("[browsers]\ninstall = [\"firefox\"]\n"), [Wanted::Browser("firefox".into())]);
        assert_eq!(parse("# only comments\n\n[services]\nautoupdate = true\n"), []);
        assert_eq!(parse("[agents]\ninstall = [\"a#b\"] # c\n"), [Wanted::Agent("a#b".into())]);
    }

    #[test]
    fn resolves_against_the_catalog() {
        let entries = resolve(&parse(DEV), &catalog());
        let seen: Vec<(String, Method)> = entries.iter().map(|e| (e.name(|s| s.to_string()), e.method)).collect();
        assert_eq!(
            seen,
            [
                ("Fedora development box".to_string(), Method::Box),
                ("node@lts".to_string(), Method::Box),
                ("python@3.13".to_string(), Method::Box),
                ("VS Code with Dev Containers".to_string(), Method::Native),
                ("Zed".to_string(), Method::Flatpak),
                ("opencode".to_string(), Method::User),
            ]
        );
        assert_eq!(entries[0].size_mb, Some(1536.0));
        assert_eq!(entries[1].size_mb, None);
    }

    #[test]
    fn unknown_names_still_show() {
        let entries = resolve(&[Wanted::Agent("new".into()), Wanted::Flatpak("org.example.App".into()), Wanted::Browser("brave".into())], &[]);
        assert_eq!(entries[0], Entry { desc: "new".into(), method: Method::User, size_mb: None });
        assert_eq!(entries[1], Entry { desc: "App".into(), method: Method::Flatpak, size_mb: None });
        assert_eq!(entries[2].method, Method::Flatpak);
    }

    #[test]
    fn groups_follow_the_legend_order() {
        let entries = resolve(&parse(DEV), &catalog());
        let groups: Vec<(Method, usize)> = by_method(&entries).into_iter().map(|(m, g)| (m, g.len())).collect();
        assert_eq!(groups, [(Method::Flatpak, 1), (Method::Native, 1), (Method::Box, 3), (Method::User, 1)]);
    }

    /// The four profiles of the image resolve completely against its catalog.
    #[test]
    fn real_profiles() {
        let Some(share) = crate::test_share_dir() else {
            return; // built outside the repository
        };
        let mut items = parse_catalog(&std::fs::read_to_string(share.join("catalog.tsv")).unwrap());
        items.extend(parse_flatpaks(&std::fs::read_to_string(share.join("flatpaks.list")).unwrap()));
        for name in PROFILES {
            let text = std::fs::read_to_string(share.join("profiles").join(format!("{name}.toml"))).unwrap();
            let entries = resolve(&parse(&text), &items);
            assert!(!entries.is_empty(), "{name}");
            for e in &entries {
                assert_ne!(e.method, Method::Command, "{name}: {}", e.desc);
                // Everything except the mise tools is found in the catalog.
                assert!(e.size_mb.is_some() || e.desc.contains('@'), "{name}: {} not in the catalog", e.desc);
            }
        }
    }
}
