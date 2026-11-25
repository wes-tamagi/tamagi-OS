//! The wanted state (a tamagi.toml) and the plan to reach it: checking the
//! file and working out what is missing.

use std::collections::BTreeSet;

use toml::{Table, Value};

use crate::i18n::t;
use crate::state::{pystr, slug, truthy, State, AGENTS, BOX_DISTROS, BROWSERS, BROWSER_METHODS, MCP_SERVERS, PERF_PROFILES};

/// Sections and the keys each one takes (`None`: free names, e.g. box = distro).
const KNOWN: &[(&str, Option<&[&str]>)] = &[
    ("appearance", Some(&["theme", "tiling"])),
    ("browsers", Some(&["install", "default"])),
    ("flatpak", Some(&["apps"])),
    ("mise", Some(&["tools"])),
    ("agents", Some(&["install"])),
    ("mcp", Some(&["servers"])),
    ("catalog", Some(&["install"])),
    ("boxes", None),    // name = distro
    ("layers", None),   // name = [Fedora packages]
    ("webapps", None),  // name = address
    ("services", Some(&["autoupdate", "security", "performance"])),
];
/// Sections of commands that no longer exist: read and ignored, with a note.
const RETIRED_SECTIONS: &[&str] = &["packages"];

/// One thing to do: what it is, the command that does it, whether it asks for the password.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub desc: String,
    pub cmd: Vec<String>,
    pub root: bool,
}

/// What the plan needs to ask the machine beyond the state already read.
pub trait Probe {
    fn box_exists(&self, name: &str) -> bool;
    /// Every item `tamagi-install installed` lists.
    fn catalog_installed(&self) -> BTreeSet<String>;
    /// Whether the .desktop file of the web app with this slug exists.
    fn webapp_exists(&self, slug: &str) -> bool;
}

// ── Reading the file ────────────────────────────────────────────────────────

/// Sections of retired commands are left out, with a warning each.
pub fn modernize(wanted: Table, warnings: &mut Vec<String>) -> Table {
    let mut result = Table::new();
    for (section, values) in wanted {
        if RETIRED_SECTIONS.contains(&section.as_str()) {
            warnings.push(format!("{}: [{section}] {}", t("warning"), t("no longer exists (use a box: tamagi devbox); ignored")));
            continue;
        }
        result.insert(section, values);
    }
    result
}

fn section<'a>(wanted: &'a Table, name: &str) -> Option<&'a Table> {
    wanted.get(name).and_then(Value::as_table)
}

/// The list under [section] key (empty if absent or not a list).
fn list<'a>(wanted: &'a Table, name: &str, key: &str) -> &'a [Value] {
    section(wanted, name).and_then(|s| s.get(key)).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn pairs<'a>(wanted: &'a Table, name: &str) -> impl Iterator<Item = (&'a String, &'a Value)> {
    section(wanted, name).into_iter().flat_map(|s| s.iter())
}

/// "firefox:native" -> ("firefox", "native"); the method defaults to flatpak.
pub fn browser_spec(item: &Value) -> (String, String) {
    let text = pystr(item);
    let (name, method) = text.split_once(':').unwrap_or((text.as_str(), ""));
    (name.to_string(), if method.is_empty() { "flatpak".to_string() } else { method.to_string() })
}

fn is_name(name: &str, extra: &[char], max: Option<usize>) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || extra.contains(&c))
        && max.is_none_or(|m| name.len() <= m)
}

/// What is wrong in the file, one line each. `catalog_ids` is only asked when
/// the file lists catalog items.
pub fn problems(wanted: &Table, catalog_ids: &dyn Fn() -> BTreeSet<String>) -> Vec<String> {
    let mut problems = Vec::new();
    for (name, values) in wanted {
        match KNOWN.iter().find(|(known, _)| known == name) {
            None => problems.push(format!("{} [{name}]", t("unknown section"))),
            Some((_, keys)) => match (values.as_table(), keys) {
                (None, _) => problems.push(format!("[{name}] {}", t("must be a section"))),
                (Some(table), Some(keys)) => {
                    problems.extend(table.keys().filter(|k| !keys.contains(&k.as_str())).map(|k| format!("{} {name}.{k}", t("unknown key"))))
                }
                (Some(_), None) => {}
            },
        }
    }
    for item in list(wanted, "browsers", "install") {
        let (name, method) = browser_spec(item);
        if !BROWSERS.contains(&name.as_str()) {
            problems.push(format!("{}: {name}", t("unknown browser")));
        } else if !BROWSER_METHODS.contains(&method.as_str()) {
            problems.push(format!("{} {name}: {} '{method}'", t("browser"), t("unknown method")));
        }
    }
    for name in list(wanted, "agents", "install") {
        if !name.as_str().is_some_and(|n| AGENTS.contains(&n)) {
            problems.push(format!("{}: {}", t("unknown agent"), pystr(name)));
        }
    }
    for srv in list(wanted, "mcp", "servers") {
        if !srv.as_str().is_some_and(|s| MCP_SERVERS.contains(&s)) {
            problems.push(format!("{}: {}", t("unknown MCP server"), pystr(srv)));
        }
    }
    let items = list(wanted, "catalog", "install");
    if !items.is_empty() {
        let known = catalog_ids();
        for item in items {
            if !known.is_empty() && !item.as_str().is_some_and(|i| known.contains(i)) {
                problems.push(format!("{}: {}", t("is not in the catalog"), pystr(item)));
            }
        }
    }
    for (name, distro) in pairs(wanted, "boxes") {
        if !distro.as_str().is_some_and(|d| BOX_DISTROS.contains(&d)) {
            problems.push(format!("{} {name}: {} '{}'", t("box"), t("unknown distro"), pystr(distro)));
        }
        if !is_name(name, &['_', '-'], None) {
            problems.push(format!("{}: {name}", t("invalid box name")));
        }
    }
    for (name, pkgs) in pairs(wanted, "layers") {
        if !is_name(name, &['-'], Some(40)) {
            problems.push(format!("{}: {name}", t("invalid layer name")));
        }
        let good = pkgs.as_array().is_some_and(|l| !l.is_empty() && l.iter().all(|p| p.as_str().is_some_and(|s| !s.is_empty())));
        if !good {
            problems.push(format!("{} {name}: {}", t("layer"), t("must be a list of packages")));
        }
    }
    for (name, url) in pairs(wanted, "webapps") {
        let url = pystr(url);
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            problems.push(format!("{} {name}: {}", t("web app"), t("URL must start with http:// or https://")));
        }
    }
    let service = |key: &str| section(wanted, "services").and_then(|s| s.get(key));
    if service("security").is_some_and(|v| !matches!(v.as_str(), Some("balanced" | "strict"))) {
        problems.push(t("services.security must be balanced or strict"));
    }
    if service("performance").is_some_and(|v| !v.as_str().is_some_and(|p| PERF_PROFILES.contains(&p))) {
        problems.push(t("services.performance must be eco, balanced, performance or latency"));
    }
    problems
}

// ── Plan ────────────────────────────────────────────────────────────────────

fn step(desc: String, cmd: &[&str], root: bool) -> Step {
    Step { desc, cmd: cmd.iter().map(|s| s.to_string()).collect(), root }
}

/// The steps that take the machine from `now` to the wanted state. Nothing
/// that the file does not mention is touched.
pub fn plan(wanted: &Table, now: &State, probe: &dyn Probe) -> Vec<Step> {
    let mut steps = Vec::new();
    let on_off = |on: bool| if on { "on" } else { "off" };

    if let Some(ap) = section(wanted, "appearance") {
        if let Some(theme) = ap.get("theme") {
            if theme.as_str() != Some(now.theme.as_str()) {
                let theme = pystr(theme);
                steps.push(step(format!("{}: {theme}", t("theme")), &["tamagi-theme", "set", &theme], false));
            }
        }
        if let Some(tiling) = ap.get("tiling").map(truthy) {
            if tiling != now.tiling {
                let word = if tiling { t("running") } else { t("off") };
                steps.push(step(format!("tiling: {word}"), &["tamagi", "tiling", on_off(tiling)], false));
            }
        }
    }

    let have: BTreeSet<&str> = now.browsers.iter().map(|b| b.split(':').next().unwrap_or("")).collect();
    for item in list(wanted, "browsers", "install") {
        let (name, method) = browser_spec(item);
        if !have.contains(name.as_str()) {
            steps.push(step(
                format!("{}: {name} ({method})", t("browser")),
                &["tamagi-browser", "install", &name, &format!("--{method}")],
                method == "native",
            ));
        }
    }
    if let Some(default) = section(wanted, "browsers").and_then(|b| b.get("default")).filter(|d| truthy(d)) {
        if default.as_str() != Some(now.default_browser.as_str()) {
            let default = pystr(default);
            steps.push(step(format!("{}: {default}", t("default browser")), &["tamagi-browser", "default", &default], false));
        }
    }

    for app in list(wanted, "flatpak", "apps") {
        if !app.as_str().is_some_and(|a| now.flatpak.iter().any(|x| x == a)) {
            let app = pystr(app);
            steps.push(step(format!("Flatpak: {app}"), &["flatpak", "install", "--user", "-y", "--noninteractive", &app], false));
        }
    }

    for tool in list(wanted, "mise", "tools").iter().filter_map(Value::as_str) {
        let (name, version) = tool.split_once('@').unwrap_or((tool, ""));
        let have = now.mise.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str());
        if have.is_none() || (!version.is_empty() && have != Some(version)) {
            steps.push(step(format!("mise: {tool}"), &["tamagi-devbox", "lang", tool], false));
        }
    }

    for name in list(wanted, "agents", "install") {
        if !name.as_str().is_some_and(|n| now.agents.iter().any(|x| x == n)) {
            let name = pystr(name);
            steps.push(step(format!("{}: {name}", t("agent")), &["tamagi-agents", "install", &name], false));
        }
    }

    for srv in list(wanted, "mcp", "servers") {
        if !srv.as_str().is_some_and(|s| now.mcp.iter().any(|x| x == s)) {
            let srv = pystr(srv);
            steps.push(step(format!("MCP: {srv}"), &["tamagi-mcp", "add", &srv], false));
        }
    }

    let mut missing: Vec<String> = list(wanted, "catalog", "install").iter().map(pystr).filter(|i| !now.catalog.contains(i)).collect();
    if !missing.is_empty() {
        let installed = probe.catalog_installed();
        missing.retain(|i| !installed.contains(i));
    }
    if !missing.is_empty() {
        let mut cmd = vec!["tamagi-install"];
        cmd.extend(missing.iter().map(String::as_str));
        steps.push(step(format!("{}: {}", t("catalog"), missing.join(" ")), &cmd, true));
    }

    for (name, distro) in pairs(wanted, "boxes") {
        if !probe.box_exists(name) {
            let distro = pystr(distro);
            steps.push(step(format!("{} {name} ({distro})", t("box")), &["tamagi-devbox", "create", &distro, "--name", name], false));
        }
    }

    for (name, pkgs) in pairs(wanted, "layers") {
        let pkgs: Vec<String> = pkgs.as_array().map(|l| l.iter().map(pystr).collect()).unwrap_or_default();
        let mut want = pkgs.clone();
        want.sort();
        let mut have = now.layers.iter().find(|(n, _)| n == name).map(|(_, p)| p.clone()).unwrap_or_default();
        have.sort();
        if have != want {
            let mut cmd = vec!["tamagi-layer", "add", name.as_str()];
            cmd.extend(pkgs.iter().map(String::as_str));
            steps.push(step(format!("{} {name}: {}", t("layer"), pkgs.join(" ")), &cmd, true));
        }
    }

    for (name, url) in pairs(wanted, "webapps") {
        if !probe.webapp_exists(&slug(name)) {
            steps.push(step(format!("{}: {name}", t("web app")), &["tamagi-webapp", "install", name, &pystr(url)], false));
        }
    }

    if let Some(sv) = section(wanted, "services") {
        if let Some(auto) = sv.get("autoupdate").map(truthy) {
            if auto != now.autoupdate {
                let word = if auto { t("on") } else { t("off") };
                steps.push(step(format!("{}: {word}", t("automatic updates")), &["tamagi-update", "auto", on_off(auto)], true));
            }
        }
        if let Some(security) = sv.get("security").map(pystr).filter(|s| *s != now.security) {
            steps.push(step(format!("{}: {security}", t("security")), &["tamagi-harden", &security], true));
        }
        if let Some(perf) = sv.get("performance").map(pystr).filter(|p| *p != now.performance) {
            steps.push(step(format!("{}: {perf}", t("performance")), &["tamagi-perf", "set", &perf], true));
        }
    }
    steps
}

/// What the machine has and the file does not mention (information only).
pub fn extras(wanted: &Table, now: &State) -> Vec<String> {
    let mut found = Vec::new();
    let listed: BTreeSet<&str> = list(wanted, "flatpak", "apps").iter().filter_map(Value::as_str).collect();
    found.extend(now.flatpak.iter().filter(|a| !listed.contains(a.as_str())).map(|a| format!("Flatpak {a}")));
    let listed: BTreeSet<&str> =
        list(wanted, "mise", "tools").iter().filter_map(Value::as_str).map(|i| i.split('@').next().unwrap_or("")).collect();
    found.extend(now.mise.iter().filter(|(n, _)| !listed.contains(n.as_str())).map(|(n, v)| format!("mise {n}@{v}")));
    let listed_mcp: BTreeSet<&str> = list(wanted, "mcp", "servers").iter().filter_map(Value::as_str).collect();
    found.extend(now.mcp.iter().filter(|s| !listed_mcp.contains(s.as_str())).map(|s| format!("MCP {s}")));
    let named = |name: &str, key: &str| section(wanted, name).is_some_and(|s| s.contains_key(key));
    found.extend(now.boxes.iter().filter(|(n, _)| !named("boxes", n)).map(|(n, _)| format!("{} {n}", t("box"))));
    found.extend(now.layers.iter().filter(|(n, _)| !named("layers", n)).map(|(n, _)| format!("{} {n}", t("layer"))));
    found.extend(now.webapps.iter().filter(|(n, _)| !named("webapps", n)).map(|(n, _)| format!("{} {n}", t("web app"))));
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        boxes: Vec<&'static str>,
        installed: Vec<&'static str>,
        webapps: Vec<&'static str>,
    }
    impl Probe for Fake {
        fn box_exists(&self, name: &str) -> bool {
            self.boxes.contains(&name)
        }
        fn catalog_installed(&self) -> BTreeSet<String> {
            self.installed.iter().map(|s| s.to_string()).collect()
        }
        fn webapp_exists(&self, slug: &str) -> bool {
            self.webapps.contains(&slug)
        }
    }
    const NOTHING: Fake = Fake { boxes: vec![], installed: vec![], webapps: vec![] };

    fn read(text: &str) -> (Table, Vec<String>) {
        let mut warnings = Vec::new();
        let table = modernize(text.parse::<Table>().unwrap(), &mut warnings);
        (table, warnings)
    }
    fn no_ids() -> BTreeSet<String> {
        BTreeSet::new()
    }
    fn now() -> State {
        State { theme: "ipe".into(), tiling: true, security: "balanced".into(), performance: "balanced".into(), ..State::default() }
    }
    fn descs(steps: &[Step]) -> Vec<&str> {
        steps.iter().map(|s| s.desc.as_str()).collect()
    }

    #[test]
    fn retired_sections_are_ignored_with_a_warning() {
        let (w, warnings) = read("[packages]\ninstall = [\"x\"]\n[flatpak]\napps = []\n");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("[packages]"));
        assert_eq!(w.keys().map(String::as_str).collect::<Vec<_>>(), ["flatpak"]);
    }

    #[test]
    fn file_problems_are_listed() {
        let (w, _) = read(
            "[nope]\na = 1\n[appearance]\ncolor = \"x\"\n[browsers]\ninstall = [\"opera\", \"firefox:snap\"]\n[agents]\ninstall = [\"hal\"]\n\
             [mcp]\nservers = [\"badmcp\"]\n\
             [boxes]\nDev = \"debian\"\n[layers]\n\"Bad Name\" = []\n[webapps]\nx = \"ftp://x\"\n[services]\nsecurity = \"max\"\nperformance = \"turbo\"\n",
        );
        let p = problems(&w, &no_ids);
        assert_eq!(
            p,
            vec![
                "unknown section [nope]",
                "unknown key appearance.color",
                "unknown browser: opera",
                "browser firefox: unknown method 'snap'",
                "unknown agent: hal",
                "unknown MCP server: badmcp",
                "box Dev: unknown distro 'debian'",
                "invalid box name: Dev",
                "invalid layer name: Bad Name",
                "layer Bad Name: must be a list of packages",
                "web app x: URL must start with http:// or https://",
                "services.security must be balanced or strict",
                "services.performance must be eco, balanced, performance or latency",
            ]
        );
    }

    #[test]
    fn catalog_items_are_checked_only_when_the_catalog_answers() {
        let (w, _) = read("[catalog]\ninstall = [\"steam\", \"nope\"]\n");
        assert!(problems(&w, &no_ids).is_empty());
        let ids = || ["steam".to_string()].into_iter().collect::<BTreeSet<_>>();
        assert_eq!(problems(&w, &ids), vec!["is not in the catalog: nope"]);
        let (w, _) = read("[appearance]\ntheme = \"x\"\n");
        let never = || -> BTreeSet<String> { panic!("the catalog is not asked without [catalog]") };
        assert!(problems(&w, &never).is_empty());
    }

    #[test]
    fn nothing_to_do_when_the_machine_matches() {
        let (w, _) = read("[appearance]\ntheme = \"ipe\"\ntiling = true\n[services]\nsecurity = \"balanced\"\nautoupdate = false\n");
        assert!(plan(&w, &now(), &NOTHING).is_empty());
    }

    #[test]
    fn plan_lists_what_is_missing_in_the_order_of_the_sections() {
        let (w, _) = read(
            "[appearance]\ntheme = \"nord\"\ntiling = false\n[browsers]\ninstall = [\"firefox\", \"chrome:native\", \"vivaldi:box\"]\ndefault = \"firefox\"\n\
             [flatpak]\napps = [\"a.b.C\", \"d.e.F\"]\n[mise]\ntools = [\"node@lts\", \"python@3.13\", \"go\"]\n[agents]\ninstall = [\"claude\", \"codex\"]\n\
             [mcp]\nservers = [\"memory\", \"git\"]\n\
             [catalog]\ninstall = [\"steam\", \"vscode\", \"tailscale\"]\n[boxes]\ndev = \"fedora\"\nold = \"arch\"\n\
             [layers]\ntools = [\"ncdu\", \"htop\"]\nsame = [\"b\", \"a\"]\n[webapps]\n\"My Mail\" = \"https://mail.example\"\nDone = \"https://x\"\n\
             [services]\nautoupdate = true\nsecurity = \"strict\"\nperformance = \"eco\"\n",
        );
        let state = State {
            browsers: vec!["vivaldi:flatpak".into()],
            flatpak: vec!["d.e.F".into()],
            mise: vec![("go".into(), "1.22".into()), ("node".into(), "20".into()), ("python".into(), "3.13".into())],
            agents: vec!["codex".into()],
            mcp: vec!["git".into()],
            catalog: vec!["steam".into()],
            layers: vec![("same".into(), vec!["a".into(), "b".into()])],
            ..now()
        };
        let probe = Fake { boxes: vec!["old"], installed: vec!["steam", "vscode"], webapps: vec!["done"] };
        let steps = plan(&w, &state, &probe);
        assert_eq!(
            descs(&steps),
            [
                "theme: nord",
                "tiling: off",
                "browser: firefox (flatpak)",
                "browser: chrome (native)",
                "default browser: firefox",
                "Flatpak: a.b.C",
                "mise: node@lts",
                "agent: claude",
                "MCP: memory",
                "catalog: tailscale",
                "box dev (fedora)",
                "layer tools: ncdu htop",
                "web app: My Mail",
                "automatic updates: on",
                "security: strict",
                "performance: eco",
            ]
        );
        let cmd = |i: usize| steps[i].cmd.join(" ");
        assert_eq!(cmd(1), "tamagi tiling off");
        assert_eq!(cmd(3), "tamagi-browser install chrome --native");
        assert_eq!(cmd(5), "flatpak install --user -y --noninteractive a.b.C");
        assert_eq!(cmd(8), "tamagi-mcp add memory");
        assert_eq!(cmd(9), "tamagi-install tailscale");
        assert_eq!(cmd(10), "tamagi-devbox create fedora --name dev");
        assert_eq!(cmd(11), "tamagi-layer add tools ncdu htop");
        assert_eq!(cmd(12), "tamagi-webapp install My Mail https://mail.example");
        assert_eq!(cmd(14), "tamagi-harden strict");
        let root: Vec<bool> = steps.iter().map(|s| s.root).collect();
        assert_eq!(root, [false, false, false, true, false, false, false, false, false, true, false, true, false, true, true, true]);
    }

    #[test]
    fn extras_are_what_the_file_does_not_mention() {
        let (w, _) = read("[flatpak]\napps = [\"a.b.C\"]\n[mise]\ntools = [\"node@lts\"]\n[boxes]\ndev = \"fedora\"\n");
        let state = State {
            flatpak: vec!["a.b.C".into(), "x.y.Z".into()],
            mise: vec![("node".into(), "20".into()), ("python".into(), "3.13".into())],
            mcp: vec!["memory".into()],
            boxes: vec![("dev".into(), "fedora".into()), ("other".into(), "arch".into())],
            layers: vec![("tools".into(), vec!["htop".into()])],
            webapps: vec![("Mail".into(), "https://x".into())],
            ..now()
        };
        assert_eq!(extras(&w, &state), ["Flatpak x.y.Z", "mise python@3.13", "MCP memory", "box other", "layer tools", "web app Mail"]);
    }

    #[test]
    fn browser_specs() {
        assert_eq!(browser_spec(&Value::String("firefox".into())), ("firefox".into(), "flatpak".into()));
        assert_eq!(browser_spec(&Value::String("chrome:box".into())), ("chrome".into(), "box".into()));
        assert_eq!(browser_spec(&Value::String("chrome:".into())), ("chrome".into(), "flatpak".into()));
    }

    #[test]
    fn names() {
        assert!(is_name("dev_1-x", &['_', '-'], None));
        assert!(!is_name("1dev", &['_', '-'], None));
        assert!(!is_name("", &['-'], None));
        assert!(is_name(&"a".repeat(40), &['-'], Some(40)));
        assert!(!is_name(&"a".repeat(41), &['-'], Some(40)));
        assert!(!is_name("a_b", &['-'], Some(40)));
    }
}
