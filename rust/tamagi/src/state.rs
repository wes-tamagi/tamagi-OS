//! The current state of the machine, read from the other `tamagi-*` commands
//! and from files, and its TOML form (`tamagi init`, `tamagi export`).

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::i18n::t;

pub const BROWSERS: &[&str] = &["chrome", "firefox", "vivaldi", "chromium"];
pub const BROWSER_METHODS: &[&str] = &["flatpak", "native", "box"];
pub const BOX_DISTROS: &[&str] = &["fedora", "ubuntu", "opensuse", "arch"];
pub const AGENTS: &[&str] = &["claude", "antigravity", "codex", "clio", "qwen", "opencode", "gemini", "openclaw"];
pub const MCP_SERVERS: &[&str] = &["os", "fetch", "memory", "git", "postgres", "playwright", "filesystem"];
pub const PERF_PROFILES: &[&str] = &["eco", "balanced", "performance", "latency", "ai"];
/// Layers made by other commands (tamagi vm, gaming, browser --native,
/// Tailscale, VS Code, Docker, NVIDIA, Lark, Sniffnet) carry options a plain
/// `tamagi layer add` has not: they are described by [catalog] and [browsers], not by [layers].
pub const MANAGED_LAYERS: &[&str] = &["virt", "gaming", "tailscale", "vscode", "docker", "video-nvidia", "nvidia-cuda", "lark", "sniffnet"];

// ── Folders ─────────────────────────────────────────────────────────────────

pub fn home_dir() -> PathBuf {
    env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// $XDG_CONFIG_HOME (as written, even empty: the same as the Python tools) or ~/.config.
pub fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home_dir().join(".config"))
}

pub fn data_dir() -> PathBuf {
    env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home_dir().join(".local/share"))
}

pub fn default_file() -> PathBuf {
    config_dir().join("tamagi").join("tamagi.toml")
}

fn browser_box() -> String {
    env::var("TAMAGI_BROWSER_BOX").unwrap_or_else(|_| "browsers".into())
}

// ── Running other commands ──────────────────────────────────────────────────

/// Output of a command, or "" if it cannot be started.
pub fn out(cmd: &[&str]) -> String {
    match Command::new(cmd[0]).args(&cmd[1..]).stdin(Stdio::inherit()).stderr(Stdio::piped()).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => String::new(),
    }
}

pub fn ok(cmd: &[&str]) -> bool {
    Command::new(cmd[0])
        .args(&cmd[1..])
        .stdin(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// The program in PATH, as `shutil.which`.
pub fn which(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let path = env::var_os("PATH").unwrap_or_else(|| "/bin:/usr/bin".into());
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|p| fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false))
}

// ── Small text helpers ──────────────────────────────────────────────────────

/// "My App!" -> "my-app": the name of a web app's .desktop file.
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    let mut dash = false;
    for c in name.to_lowercase().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            s.push(c);
            dash = false;
        } else if !dash {
            s.push('-');
            dash = true;
        }
    }
    let s = s.strip_prefix('-').unwrap_or(&s);
    s.strip_suffix('-').unwrap_or(s).to_string()
}

/// A JSON string, which is also a valid TOML basic string (non-ASCII kept).
pub fn quoted(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\u{8}' => o.push_str("\\b"),
            '\u{c}' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn quoted_list(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|x| quoted(x)).collect::<Vec<_>>().join(", "))
}

fn bare_key(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// ── Current state ───────────────────────────────────────────────────────────

/// What the machine has now. Pairs keep the order they were found in.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct State {
    pub theme: String,
    pub tiling: bool,
    /// ["firefox:flatpak", "chrome:box", ...] as printed by tamagi-browser.
    pub browsers: Vec<String>,
    pub default_browser: String,
    pub flatpak: Vec<String>,
    /// (tool, version) of mise's global configuration, sorted.
    pub mise: Vec<(String, String)>,
    pub agents: Vec<String>,
    pub mcp: Vec<String>,
    pub catalog: Vec<String>,
    pub boxes: Vec<(String, String)>,
    pub layers: Vec<(String, Vec<String>)>,
    pub webapps: Vec<(String, String)>,
    pub autoupdate: bool,
    pub security: String,
    pub performance: String,
}

/// Inserts or replaces, keeping the first position (as a Python dict does).
fn put<V>(pairs: &mut Vec<(String, V)>, key: String, value: V) {
    match pairs.iter_mut().find(|(k, _)| *k == key) {
        Some(slot) => slot.1 = value,
        None => pairs.push((key, value)),
    }
}

pub fn box_exists(name: &str) -> bool {
    ok(&["podman", "container", "exists", name])
}

fn browsers_installed() -> Vec<String> {
    out(&["tamagi-browser", "installed"])
        .split_whitespace()
        .filter(|l| BROWSERS.contains(&l.split(':').next().unwrap_or("")))
        .map(String::from)
        .collect()
}

fn agents_installed() -> Vec<String> {
    out(&["tamagi-agent", "installed"]).split_whitespace().filter(|a| AGENTS.contains(a)).map(String::from).collect()
}

fn mcp_installed() -> Vec<String> {
    out(&["tamagi-mcp", "installed"]).split_whitespace().filter(|s| MCP_SERVERS.contains(s)).map(String::from).collect()
}

pub fn catalog_ids() -> BTreeSet<String> {
    out(&["tamagi-install", "ids"]).split_whitespace().map(String::from).collect()
}

/// Every item `tamagi-install installed` lists (first column).
pub fn catalog_installed_all() -> BTreeSet<String> {
    out(&["tamagi-install", "installed"]).lines().map(|l| l.split('\t').next().unwrap_or("").to_string()).collect()
}

/// Catalog items on this machine that no other section describes (native
/// games, virtual machines, Windows, Tailscale): from the output of
/// `tamagi-install installed`, one "item<TAB>action" per line.
pub fn catalog_own(listing: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in listing.lines() {
        let (item, action) = line.split_once('\t').unwrap_or((line, ""));
        if (action.starts_with("gaming:") || action.starts_with("run:"))
            && !action.starts_with("run:tamagi-browser")
            && !action.starts_with("run:tamagi-devbox")
        {
            found.push(item.to_string());
        }
    }
    found
}

/// mise's global tools: name -> version, sorted by name.
pub fn mise_tools(config: &str) -> Vec<(String, String)> {
    let Ok(doc) = config.parse::<toml::Table>() else { return Vec::new() };
    let Some(toml::Value::Table(tools)) = doc.get("tools") else { return Vec::new() };
    let mut found: Vec<(String, String)> = tools.iter().map(|(n, v)| (n.clone(), pystr(v))).collect();
    found.sort();
    found
}

pub fn mise_global() -> Vec<(String, String)> {
    fs::read_to_string(config_dir().join("mise").join("config.toml")).map(|s| mise_tools(&s)).unwrap_or_default()
}

/// The distro of a box, from the name of its image.
pub fn distro_of(image: &str) -> Option<&'static str> {
    BOX_DISTROS.iter().copied().find(|d| image.contains(d) || (*d == "opensuse" && image.contains("tumbleweed")))
}

fn user_boxes() -> Vec<(String, String)> {
    let names = out(&["podman", "ps", "-a", "--filter", "label=manager=distrobox", "--format", "{{.Names}}\t{{.Image}}"]);
    let browser_box = browser_box();
    let mut boxes = Vec::new();
    for line in names.lines() {
        let (name, image) = line.split_once('\t').unwrap_or((line, ""));
        if name == browser_box || name == "navegadores" {
            continue;
        }
        if let Some(distro) = distro_of(image) {
            put(&mut boxes, name.to_string(), distro.to_string());
        }
    }
    boxes
}

fn sorted_files(dir: &Path, prefix: &str, suffix: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with(prefix) && n.ends_with(suffix) && n.len() >= prefix.len() + suffix.len()).unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// (name, address) of a web app's .desktop file, if it is one of ours.
pub fn webapp_entry(text: &str) -> Option<(String, String)> {
    let name = text.split('\n').find_map(|l| l.strip_prefix("Name="))?;
    let url = text.split('\n').find_map(|l| l.strip_prefix("Comment=Web app: "))?;
    text.contains("X-Tamagi-WebApp=true").then(|| (name.to_string(), url.to_string()))
}

fn webapps() -> Vec<(String, String)> {
    let mut apps = Vec::new();
    for f in sorted_files(&data_dir().join("applications"), "webapp-", ".desktop") {
        let Ok(bytes) = fs::read(&f) else { continue };
        if let Some((name, url)) = webapp_entry(&String::from_utf8_lossy(&bytes)) {
            put(&mut apps, name, url);
        }
    }
    apps
}

/// Layers of tamagi-layer: name -> requested packages.
fn layers() -> Vec<(String, Vec<String>)> {
    let mut found = Vec::new();
    for f in sorted_files(Path::new("/var/lib/tamagi/layers"), "", ".request") {
        let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
        if MANAGED_LAYERS.contains(&stem.as_str()) || stem.starts_with("browser-") {
            continue;
        }
        if let Ok(text) = fs::read_to_string(&f) {
            found.push((stem, text.split_whitespace().map(String::from).collect()));
        }
    }
    found
}

pub fn tiling_file() -> PathBuf {
    config_dir().join("cosmic/com.system76.CosmicComp/v1/autotile")
}

pub fn tiling() -> bool {
    for path in [tiling_file(), PathBuf::from("/usr/share/cosmic/com.system76.CosmicComp/v1/autotile")] {
        if let Ok(text) = fs::read_to_string(path) {
            return text.trim() == "true";
        }
    }
    true
}

fn perf_profile() -> String {
    fs::read_to_string("/var/lib/tamagi/perf-profile")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| PERF_PROFILES.contains(&v.as_str()))
        .unwrap_or_else(|| "balanced".into())
}

fn default_browser() -> String {
    fs::read_to_string(config_dir().join("tamagi/browser-default")).map(|s| s.trim().to_string()).unwrap_or_default()
}

pub fn current_state() -> State {
    let theme = out(&["tamagi-theme", "current"]);
    let flatpak: BTreeSet<String> = out(&["flatpak", "list", "--app", "--columns=application"]).split_whitespace().map(String::from).collect();
    State {
        theme: if theme.is_empty() { "ipe".into() } else { theme },
        tiling: tiling(),
        browsers: browsers_installed(),
        default_browser: default_browser(),
        flatpak: flatpak.into_iter().collect(),
        mise: mise_global(),
        agents: agents_installed(),
        mcp: mcp_installed(),
        catalog: catalog_own(&out(&["tamagi-install", "installed"])),
        boxes: user_boxes(),
        layers: layers(),
        webapps: webapps(),
        autoupdate: ok(&["systemctl", "--user", "is-enabled", "tamagi-autoupdate.timer"]),
        security: if Path::new("/etc/sysctl.d/90-tamagi-strict.conf").exists() { "strict".into() } else { "balanced".into() },
        performance: perf_profile(),
    }
}

// ── Export ──────────────────────────────────────────────────────────────────

/// The state as a tamagi.toml, with a note above each section.
pub fn to_toml(state: &State) -> String {
    let flag = |b: bool| if b { "true" } else { "false" }.to_string();
    let pairs = |v: &[(String, String)]| v.iter().map(|(k, x)| (k.clone(), quoted(x))).collect::<Vec<_>>();
    let one = |k: &str, v: String| vec![(k.to_string(), v)];
    let sections: Vec<(&str, &str, Vec<(String, String)>)> = vec![
        (
            "appearance",
            "theme: tamagi theme list · the wallpaper is chosen in COSMIC Settings",
            vec![("theme".into(), quoted(&state.theme)), ("tiling".into(), flag(state.tiling))],
        ),
        (
            "browsers",
            "chrome, firefox, vivaldi or chromium; method after the colon: flatpak (default), native or box",
            vec![("install".into(), quoted_list(&state.browsers)), ("default".into(), quoted(&state.default_browser))],
        ),
        ("flatpak", "Flathub or COSMIC repository IDs", one("apps", quoted_list(&state.flatpak))),
        (
            "mise",
            "languages and tools in the development box, as in `mise use -g`",
            one("tools", quoted_list(&state.mise.iter().map(|(n, v)| format!("{n}@{v}")).collect::<Vec<_>>())),
        ),
        ("agents", "AI agents: claude, antigravity, codex, qwen, opencode, gemini, openclaw", one("install", quoted_list(&state.agents))),
        (
            "mcp",
            "sandboxed MCP servers in Podman Quadlets: fetch, memory, git, postgres, playwright, filesystem",
            one("servers", quoted_list(&state.mcp)),
        ),
        (
            "catalog",
            "catalog items (tamagi install list): steam, gaming-tools, virt-manager, windows, tailscale…",
            one("install", quoted_list(&state.catalog)),
        ),
        ("boxes", "name = distro (fedora, ubuntu, opensuse, arch)", pairs(&state.boxes)),
        (
            "layers",
            "Fedora packages on top of the image (tamagi layer): name = [packages]",
            state.layers.iter().map(|(k, v)| (k.clone(), quoted_list(v))).collect(),
        ),
        ("webapps", "name = URL", pairs(&state.webapps)),
        (
            "services",
            "security: balanced or strict · performance: eco, balanced, performance or latency",
            vec![
                ("autoupdate".into(), flag(state.autoupdate)),
                ("security".into(), quoted(&state.security)),
                ("performance".into(), quoted(&state.performance)),
            ],
        ),
    ];
    let mut lines = vec![
        format!("# {}", t("Tamagi — your system in one file. `tamagi apply` converges the machine to it;")),
        format!("# {}", t("`tamagi diff` shows what is missing. Nothing absent here is removed.")),
        String::new(),
    ];
    for (section, note, values) in sections {
        lines.push(format!("# {}", t(note)));
        lines.push(format!("[{section}]"));
        for (key, value) in values {
            let k = if bare_key(&key) { key } else { quoted(&key) };
            lines.push(format!("{k} = {value}"));
        }
        lines.push(String::new());
    }
    lines.join("\n")
}

// ── TOML values as text ─────────────────────────────────────────────────────

/// A TOML value as the text a person wrote (strings as they are).
pub fn pystr(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Boolean(b) => if *b { "True" } else { "False" }.to_string(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        other => format!("{other:?}"),
    }
}

/// Whether a value counts as "on" (true, a non-empty text or list, a non-zero number).
pub fn truthy(v: &toml::Value) -> bool {
    match v {
        toml::Value::Boolean(b) => *b,
        toml::Value::String(s) => !s.is_empty(),
        toml::Value::Integer(i) => *i != 0,
        toml::Value::Float(f) => *f != 0.0,
        toml::Value::Array(a) => !a.is_empty(),
        toml::Value::Table(t) => !t.is_empty(),
        toml::Value::Datetime(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("My App!"), "my-app");
        assert_eq!(slug("  YouTube  Music "), "youtube-music");
        assert_eq!(slug("WhatsApp"), "whatsapp");
        assert_eq!(slug("Café 2"), "caf-2");
        assert_eq!(slug("---"), "");
    }

    #[test]
    fn quoting() {
        assert_eq!(quoted("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(quoted("ipê\n\u{1}"), "\"ipê\\n\\u0001\"");
        assert_eq!(quoted_list(&["a".into(), "b".into()]), "[\"a\", \"b\"]");
        assert_eq!(quoted_list(&[]), "[]");
    }

    #[test]
    fn catalog_items_no_other_section_describes() {
        let listing = "firefox\trun:tamagi-browser install firefox --flatpak\nsteam\tgaming:steam\nvirt-manager\trun:tamagi-vm install\ndevbox-fedora\trun:tamagi-devbox create fedora\nclaude\tagent:claude\nzed\tflatpak:flathub:dev.zed.Zed";
        assert_eq!(catalog_own(listing), vec!["steam", "virt-manager"]);
    }

    #[test]
    fn mise_config() {
        let tools = mise_tools("[tools]\npython = \"3.13\"\nnode = \"lts\"\n[settings]\nx = 1\n");
        assert_eq!(tools, vec![("node".to_string(), "lts".to_string()), ("python".to_string(), "3.13".to_string())]);
        assert!(mise_tools("not toml =").is_empty());
        assert!(mise_tools("[other]\na = 1\n").is_empty());
    }

    #[test]
    fn box_distros() {
        assert_eq!(distro_of("registry.fedoraproject.org/fedora-toolbox:45"), Some("fedora"));
        assert_eq!(distro_of("registry.opensuse.org/opensuse/distrobox:latest"), Some("opensuse"));
        assert_eq!(distro_of("quay.io/toolbx/tumbleweed-toolbox"), Some("opensuse"));
        assert_eq!(distro_of("quay.io/toolbx/arch-toolbox"), Some("arch"));
        assert_eq!(distro_of("docker.io/library/debian"), None);
    }

    #[test]
    fn webapp_files() {
        let text = "[Desktop Entry]\nName=Mail\nComment=Web app: https://mail.example\nX-Tamagi-WebApp=true\n";
        assert_eq!(webapp_entry(text), Some(("Mail".to_string(), "https://mail.example".to_string())));
        assert_eq!(webapp_entry("Name=Mail\nComment=Web app: https://x\n"), None);
        assert_eq!(webapp_entry("Name=Mail\nX-Tamagi-WebApp=true\n"), None);
    }

    #[test]
    fn export_text() {
        let state = State {
            theme: "ipe".into(),
            tiling: true,
            browsers: vec!["firefox:flatpak".into()],
            default_browser: "firefox".into(),
            mise: vec![("node".into(), "lts".into())],
            boxes: vec![("dev".into(), "fedora".into())],
            layers: vec![("tools".into(), vec!["htop".into(), "ncdu".into()])],
            webapps: vec![("My Mail".into(), "https://mail.example".into())],
            security: "balanced".into(),
            performance: "balanced".into(),
            ..State::default()
        };
        let text = to_toml(&state);
        assert!(text.contains("[appearance]\ntheme = \"ipe\"\ntiling = true\n"));
        assert!(text.contains("[browsers]\ninstall = [\"firefox:flatpak\"]\ndefault = \"firefox\"\n"));
        assert!(text.contains("[mise]\ntools = [\"node@lts\"]\n"));
        assert!(text.contains("[boxes]\ndev = \"fedora\"\n"));
        assert!(text.contains("[layers]\ntools = [\"htop\", \"ncdu\"]\n"));
        assert!(text.contains("[webapps]\n\"My Mail\" = \"https://mail.example\"\n"));
        assert!(text.contains("[services]\nautoupdate = false\nsecurity = \"balanced\"\nperformance = \"balanced\"\n"));
        assert!(text.ends_with("\n"));
        // what is written can be read back
        assert!(text.parse::<toml::Table>().is_ok());
    }
}
