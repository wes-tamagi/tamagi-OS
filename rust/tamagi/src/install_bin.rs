//! tamagi-install — everything optional, in one catalog (+ live Flathub).
//! Static Rust binary (replaces the former Go build stage).

mod i18n;
mod ui;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use i18n::t;

const FLATHUB_SEARCH_URL: &str = "https://flathub.org/api/v2/search";
const DUAL_BROWSERS: &[&str] = &["chrome", "firefox", "vivaldi", "chromium"];
const ALL_BROWSERS: &[&str] = &["chrome", "firefox", "vivaldi", "chromium", "opera-gx"];

fn is_dual_browser(id: &str) -> bool {
    DUAL_BROWSERS.contains(&id)
}

fn native_layer_name(pkg: &str) -> String {
    let layer_name: String = pkg
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .take(32)
        .collect();
    if layer_name.starts_with(|c: char| c.is_ascii_lowercase()) {
        format!("pkg-{layer_name}")
    } else {
        "pkg-custom".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogItem {
    id: String,
    cat: String,
    size: String,
    action: String,
    desc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FlathubHit {
    id: String,
    name: String,
    summary: String,
    verified: bool,
}

fn tl(k: &str) {
    println!("{}", t(k));
}

fn env_or(k: &str, def: &str) -> String {
    env::var(k).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| def.to_string())
}

fn user_home() -> PathBuf {
    env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn cache_dir() -> PathBuf {
    let base = env::var_os("XDG_CACHE_HOME").filter(|c| !c.is_empty()).map(PathBuf::from).unwrap_or_else(|| user_home().join(".cache"));
    base.join("tamagi")
}

fn runtime_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR").filter(|r| !r.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn which(name: &str) -> bool {
    let path = env::var_os("PATH").unwrap_or_else(|| "/usr/local/bin:/usr/bin:/bin".into());
    env::split_paths(&path).map(|d| d.join(name)).any(|p| fs::metadata(&p).map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0).unwrap_or(false))
}

fn run_quiet(name: &str, args: &[&str]) -> bool {
    Command::new(name).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

fn run_out(env_extra: &[(&str, &str)], name: &str, args: &[&str]) -> (i32, String) {
    let mut cmd = Command::new(name);
    cmd.args(args).stdin(Stdio::null()).stderr(Stdio::null());
    for (k, v) in env_extra {
        cmd.env(k, v);
    }
    match cmd.output() {
        Ok(o) => (o.status.code().unwrap_or(1), String::from_utf8_lossy(&o.stdout).into_owned()),
        Err(_) => (127, String::new()),
    }
}

fn parse_catalog_files(cat_text: &str, fp_text: &str) -> Vec<CatalogItem> {
    let mut items = Vec::new();
    for ln in cat_text.lines() {
        if ln.is_empty() || ln.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = ln.splitn(5, '\t').collect();
        if parts.len() == 5 {
            items.push(CatalogItem {
                id: parts[0].to_string(),
                cat: parts[1].to_string(),
                size: parts[2].to_string(),
                action: parts[3].to_string(),
                desc: parts[4].to_string(),
            });
        }
    }
    for raw in fp_text.lines() {
        let ln = raw.trim();
        if ln.is_empty() || ln.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = ln.split_whitespace().collect();
        if fields.len() < 6 || fields[2] != "opcional" {
            continue;
        }
        let (remote, app, cat, size) = (fields[0], fields[1], fields[3], fields[4]);
        let bytes = ln.as_bytes();
        let mut pos = 0;
        let mut idx = 0;
        for _ in 0..5 {
            while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
                pos += 1;
            }
            while pos < bytes.len() && bytes[pos] != b' ' && bytes[pos] != b'\t' {
                pos += 1;
            }
            idx = pos;
        }
        let desc = ln[idx..].trim();
        let base_name = desc.split(" — ").next().unwrap_or(desc);
        let id = base_name.to_lowercase().replace(' ', "-");
        items.push(CatalogItem {
            id,
            cat: cat.to_string(),
            size: size.to_string(),
            action: format!("flatpak:{remote}:{app}"),
            desc: desc.to_string(),
        });
    }
    items
}

fn load_catalog() -> Vec<CatalogItem> {
    let cat_path = env_or("TAMAGI_CATALOG", "/usr/share/tamagi/catalog.tsv");
    let fp_path = env_or("TAMAGI_FLATPAKS", "/usr/share/tamagi/flatpaks.list");
    let cat_text = fs::read_to_string(cat_path).unwrap_or_default();
    let fp_text = fs::read_to_string(fp_path).unwrap_or_default();
    parse_catalog_files(&cat_text, &fp_text)
}

impl CatalogItem {
    fn label(&self) -> String {
        format!("[{}] {} (≈ {})", t(&self.cat), t(&self.desc), self.size)
    }
}

fn cmd_list(items: &[CatalogItem]) {
    for it in items {
        println!("  {:<18} {}", it.id, it.label());
    }
}

fn is_installed(it: &CatalogItem) -> bool {
    let a = it.action.as_str();
    if let Some(rest) = a.strip_prefix("flatpak:") {
        let app = rest.rsplit(':').next().unwrap_or("");
        return run_quiet("flatpak", &["info", "--user", app]) || run_quiet("flatpak", &["info", app]);
    }
    if let Some(key) = a.strip_prefix("gaming:") {
        if key == "steam" && which("steam") {
            return true;
        }
        let pkg = match key {
            "base" => "gamescope",
            "overlay" => "goverlay",
            other => other,
        };
        return fs::read_to_string("/var/lib/tamagi/layers/gaming.request")
            .map(|s| s.lines().any(|l| l.trim() == pkg))
            .unwrap_or(false);
    }
    if a.starts_with("run:tamagi-gaming ge-proton") {
        let compat = user_home().join(".local/share/Steam/compatibilitytools.d");
        return fs::read_dir(compat)
            .map(|rd| rd.flatten().any(|e| e.file_name().to_string_lossy().starts_with("GE-Proton")))
            .unwrap_or(false);
    }
    if a == "agent:local" {
        return false;
    }
    if let Some(want) = a.strip_prefix("agent:") {
        let (rc, out) = run_out(&[], "tamagi-agent", &["installed"]);
        return rc == 0 && out.lines().any(|l| l.trim() == want);
    }
    if let Some(rest) = a.strip_prefix("run:tamagi-browser install ") {
        let key = rest.split_whitespace().next().unwrap_or("");
        let (rc, out) = run_out(&[], "tamagi-browser", &["installed"]);
        let prefix = format!("{key}:");
        return rc == 0 && out.lines().any(|l| l.starts_with(&prefix));
    }
    if a.starts_with("run:tamagi-vm ") {
        return which("virsh") && which("cockpit-bridge");
    }
    if a.starts_with("run:tamagi-windows ") {
        let cfg = env::var_os("XDG_CONFIG_HOME").filter(|c| !c.is_empty()).map(PathBuf::from).unwrap_or_else(|| user_home().join(".config"));
        return cfg.join("containers/systemd/tamagi-windows.container").is_file();
    }
    if a.starts_with("run:tamagi-devbox create ") {
        let distro = a.split_whitespace().last().unwrap_or("");
        let cname = if distro == "fedora" { "dev".to_string() } else { format!("dev-{distro}") };
        return run_quiet("podman", &["container", "exists", &cname]);
    }
    if a.contains("tailscale-setup") {
        return which("tailscale");
    }
    if a.starts_with("run:tamagi-layer add ") {
        let pkg = a.split_whitespace().last().unwrap_or("");
        return which(pkg);
    }
    if a.contains("llm-scripts-setup") {
        return fs::metadata(user_home().join(".local/bin/llm")).map(|m| (m.permissions().mode() & 0o111) != 0).unwrap_or(false);
    }
    if a.contains("vscode-setup") {
        return which("code");
    }
    if a.contains("lazydocker-setup") {
        return fs::metadata(user_home().join(".local/bin/lazydocker")).map(|m| (m.permissions().mode() & 0o111) != 0).unwrap_or(false);
    }
    if a.contains("nvidia-setup cuda") {
        return Path::new("/var/lib/tamagi/layers/nvidia-cuda.request").is_file();
    }
    if a.contains("nvidia-setup") {
        return Path::new("/var/lib/tamagi/layers/video-nvidia.request").is_file();
    }
    if a.contains("lark-setup") {
        return Path::new("/var/lib/tamagi/layers/lark.request").is_file();
    }
    if a.contains("sniffnet-setup") {
        return which("sniffnet");
    }
    if a.contains("docker-setup") {
        return which("dockerd");
    }
    false
}

fn flathub_remote() {
    if !run_quiet("flatpak", &["remote-add", "--user", "--if-not-exists", "flathub", "/etc/flatpak/remotes.d/flathub.flatpakrepo"]) {
        let _ = run_quiet("flatpak", &["remote-add", "--user", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo"]);
    }
    let _ = run_quiet("flatpak", &["remote-add", "--user", "--if-not-exists", "cosmic", "/etc/flatpak/remotes.d/cosmic.flatpakrepo"]);
}

fn runtime_of(remote: &str, app: &str) -> String {
    let (rc, out) = run_out(&[("LC_ALL", "C")], "flatpak", &["remote-info", "--user", remote, app]);
    if rc != 0 {
        return String::new();
    }
    for ln in out.lines() {
        if let Some(rest) = ln.trim().strip_prefix("Runtime:") {
            return rest.trim().to_string();
        }
    }
    String::new()
}

fn runtime_here(r: &str) -> bool {
    run_quiet("flatpak", &["info", "--user", &format!("runtime/{r}")])
}

fn prefetch_pid_path() -> PathBuf {
    runtime_dir().join("tamagi-prefetch.pid")
}

fn prefetch_mark_path() -> PathBuf {
    cache_dir().join("prefetched-runtime")
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).is_dir() && run_quiet("kill", &["-0", &pid.to_string()])
}

fn cmd_prefetch() {
    let _ = fs::create_dir_all(cache_dir());
    let (rc, out) = run_out(&[], "nmcli", &["-t", "-f", "GENERAL.METERED", "dev", "show"]);
    if rc == 0 && out.contains(":yes") {
        return;
    }
    let pid_path = prefetch_pid_path();
    let _ = fs::write(&pid_path, format!("{}\n", std::process::id()));
    flathub_remote();
    let app = env_or("TAMAGI_PREFETCH_APP", "org.mozilla.firefox");
    let r = runtime_of("flathub", &app);
    if !r.is_empty() && !runtime_here(&r) {
        let log_file = fs::File::create(cache_dir().join("prefetch.log")).ok();
        let mut cmd = Command::new("flatpak");
        cmd.args(["install", "--user", "-y", "--noninteractive", "flathub", &format!("runtime/{r}")]);
        if let Some(f) = log_file {
            if let Ok(f2) = f.try_clone() {
                cmd.stdout(Stdio::from(f)).stderr(Stdio::from(f2));
            }
        }
        if cmd.status().is_ok_and(|s| s.success()) {
            let _ = fs::write(prefetch_mark_path(), format!("{r}\n"));
        }
    }
    let _ = fs::remove_file(&pid_path);
}

fn prefetch_wait() {
    let Ok(text) = fs::read_to_string(prefetch_pid_path()) else { return };
    let Ok(pid) = text.trim().parse::<u32>() else { return };
    if pid == 0 || !pid_alive(pid) {
        return;
    }
    tl("Finishing the download of the shared Flatpak runtime (started while you were choosing)…");
    while pid_alive(pid) {
        thread::sleep(Duration::from_secs(1));
    }
}

fn cmd_prefetch_cleanup() {
    prefetch_wait();
    let Ok(text) = fs::read_to_string(prefetch_mark_path()) else { return };
    let r = text.trim();
    if !r.is_empty() {
        let (_, out) = run_out(&[], "flatpak", &["list", "--user", "--app", "--columns=application"]);
        if out.trim().is_empty() {
            let _ = run_quiet("flatpak", &["uninstall", "--user", "-y", "--noninteractive", &format!("runtime/{r}")]);
        }
    }
    let _ = fs::remove_file(prefetch_mark_path());
}

fn runtimes_note(remote: &str, apps: &[String]) {
    let mut apps_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for app in apps {
        if run_quiet("flatpak", &["info", "--user", app]) {
            continue;
        }
        let r = runtime_of(remote, app);
        if r.is_empty() {
            continue;
        }
        if !apps_of.contains_key(&r) {
            order.push(r.clone());
        }
        let short = app.rsplit('.').next().unwrap_or(app.as_str()).to_string();
        apps_of.entry(r).or_default().push(short);
    }
    if order.is_empty() {
        return;
    }
    println!("  {}:", t("Shared runtimes these apps use"));
    let mut missing = 0;
    for r in &order {
        let display_ref = r.replacen("/x86_64/", " ", 1);
        let app_list = apps_of.get(r).map(|v| v.join(" ")).unwrap_or_default();
        if runtime_here(r) {
            println!("    {:<46} {}  ({})", display_ref, t("already here"), app_list);
        } else {
            missing += 1;
            println!("    {:<46} {}  ({})", display_ref, t("to download"), app_list);
        }
    }
    if missing > 1 {
        tl("  More than one runtime to download: each is a large download (about 1 GB).");
    }
}

fn json_string(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn flathub_search(query: &str) -> Vec<FlathubHit> {
    let body = format!("{{\"query\":{},\"hits_per_page\":12}}", json_string(query));
    if let Ok(resp) = Command::new("curl")
        .args(["-fsSL", "-m", "15", "-H", "Content-Type: application/json", "-d", &body, FLATHUB_SEARCH_URL])
        .output()
    {
        if resp.status.success() {
            if let Ok(mut jq) = Command::new("jq")
                .args(["-r", ".hits[]? | select(.app_id != null and .app_id != \"\") | [(.app_id // \"\"), (.name // \"\"), (.summary // \"\"), (if .verification_verified then \"1\" else \"0\" end)] | @tsv"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = jq.stdin.take() {
                    let _ = stdin.write_all(&resp.stdout);
                }
                if let Ok(out) = jq.wait_with_output() {
                    let hits: Vec<FlathubHit> = String::from_utf8_lossy(&out.stdout)
                        .lines()
                        .filter_map(|ln| {
                            let cols: Vec<&str> = ln.split('\t').collect();
                            (cols.len() >= 4 && !cols[0].is_empty()).then(|| FlathubHit {
                                id: cols[0].to_string(),
                                name: cols[1].to_string(),
                                summary: cols[2].to_string(),
                                verified: cols[3] == "1",
                            })
                        })
                        .collect();
                    if !hits.is_empty() {
                        return hits;
                    }
                }
            }
        }
    }
    flathub_remote();
    let (rc, out) = run_out(&[("LC_ALL", "C")], "flatpak", &["search", "--user", "--columns=application,name,description", query]);
    if rc != 0 {
        return Vec::new();
    }
    out.lines()
        .filter_map(|ln| {
            let cols: Vec<&str> = ln.split('\t').collect();
            (cols.len() >= 2 && cols[0].contains('.')).then(|| FlathubHit {
                id: cols[0].to_string(),
                name: cols[1].to_string(),
                summary: cols.get(2).copied().unwrap_or("").to_string(),
                verified: false,
            })
        })
        .collect()
}

fn is_app_id(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() >= 3
        && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
}

fn cmd_search(items: &[CatalogItem], query: &str) -> i32 {
    if query.trim().is_empty() {
        eprintln!("usage: tamagi install search <text>");
        return 2;
    }
    let low = query.to_lowercase();
    let local: Vec<&CatalogItem> = items
        .iter()
        .filter(|it| {
            it.id.to_lowercase().contains(&low)
                || it.cat.to_lowercase().contains(&low)
                || it.desc.to_lowercase().contains(&low)
        })
        .collect();
    if !local.is_empty() {
        println!("{}:", t("In the Tamagi catalog (native, drivers, Bazzite, AI, Flatpak)"));
        for it in &local {
            let kind = if is_dual_browser(&it.id) {
                "Native / Flatpak"
            } else if it.action.starts_with("flatpak:") {
                "Flatpak"
            } else {
                "Native/System"
            };
            println!("  {:<22} [{}] {}", it.id, kind, it.label());
        }
        println!();
    }
    let hits = flathub_search(query);
    if hits.is_empty() && local.is_empty() {
        println!("{}: {query}", t("Nothing in catalog or Flathub for"));
        return 1;
    }
    if !hits.is_empty() {
        println!("Flathub:");
        for h in hits {
            let mark = if h.verified { " ✓" } else { "" };
            println!("  {:<44} {} — {}{}", h.id, h.name, h.summary, mark);
        }
        println!();
        tl("✓ = published by the app's own developer. Install with: tamagi install <app id>");
    }
    0
}

fn choose_browser_action(id: &str) -> String {
    if is_dual_browser(id) && io::stdin().is_terminal() && io::stdout().is_terminal() && which("gum") {
        let opt_fp = format!("flatpak  {}", t("Flatpak — Flathub app, sandboxed (recommended)"));
        let opt_nat = format!("native   {}", t("native — RPM in a layer over the image (asks for the password)"));
        let opt_box = format!("box      {}", t("box — in a Fedora box, with hardware video decoding"));
        let header = format!("{id} — {}", t("Install how?"));
        if let Ok(mut child) = Command::new("gum")
            .args(["choose", "--height", "6", "--header", &header, &opt_fp, &opt_nat, &opt_box])
            .stdout(Stdio::piped())
            .spawn()
        {
            if let Ok(out) = child.wait_with_output() {
                if out.status.success() {
                    let pick = String::from_utf8_lossy(&out.stdout);
                    let method = pick.split_whitespace().next().unwrap_or("flatpak");
                    if matches!(method, "flatpak" | "native" | "box") {
                        return format!("run:tamagi-browser install {id} --{method}");
                    }
                }
            }
        }
    }
    format!("run:tamagi-browser install {id} --flatpak")
}

fn cmd_native_install(items: &[CatalogItem], pkgs: &[String]) -> i32 {
    if pkgs.is_empty() {
        eprintln!("usage: tamagi install --native <package|id>…");
        return 2;
    }
    let mut actions = Vec::new();
    let mut raw_pkgs = Vec::new();
    for p in pkgs {
        if is_dual_browser(p) {
            actions.push(format!("run:tamagi-browser install {p} --native"));
        } else if let Some(it) = items.iter().find(|i| &i.id == p && !i.action.starts_with("flatpak:") && !i.action.starts_with("run:tamagi-browser ")) {
            actions.push(it.action.clone());
        } else {
            raw_pkgs.push(p.clone());
        }
    }
    let mut rc = 0;
    if !actions.is_empty() {
        rc = run_actions(&actions);
    }
    if !raw_pkgs.is_empty() {
        let layer = native_layer_name(&raw_pkgs[0]);
        let mut cmd = Command::new("tamagi-layer");
        cmd.arg("add").arg(&layer).args(&raw_pkgs);
        if !cmd.status().is_ok_and(|s| s.success()) {
            rc = 1;
        }
    }
    rc
}

fn cmd_flatpak_install(items: &[CatalogItem], pkgs: &[String]) -> i32 {
    if pkgs.is_empty() {
        eprintln!("usage: tamagi install --flatpak <app|id>…");
        return 2;
    }
    let mut picked = Vec::new();
    for want in pkgs {
        if ALL_BROWSERS.contains(&want.as_str()) {
            picked.push(format!("run:tamagi-browser install {want} --flatpak"));
            continue;
        }
        if let Some(it) = items.iter().find(|i| &i.id == want && i.action.starts_with("flatpak:")) {
            picked.push(it.action.clone());
            continue;
        }
        let (act, rc) = flathub_resolve(want);
        if rc == 0 {
            picked.push(act);
            continue;
        }
        if rc == 2 {
            return 2;
        }
        eprintln!("{}: {}: {want}", t("error"), t("not in the catalog and not on Flathub"));
        return 2;
    }
    run_actions(&picked)
}

fn resolve_layer_target(target: &str, layers_dir: &Path, ext_dir: &Path) -> String {
    if layers_dir.join(format!("{target}.request")).is_file() || ext_dir.join(format!("{target}.raw")).is_file() {
        return target.to_string();
    }
    let prefixed = native_layer_name(target);
    if layers_dir.join(format!("{prefixed}.request")).is_file() || ext_dir.join(format!("{prefixed}.raw")).is_file() {
        return prefixed;
    }
    target.to_string()
}

fn remove_one(items: &[CatalogItem], target: &str) -> bool {
    if let Some(it) = items.iter().find(|i| i.id == target) {
        if let Some(rest) = it.action.strip_prefix("flatpak:") {
            let app = rest.rsplit(':').next().unwrap_or(target);
            return Command::new("flatpak").args(["uninstall", "--user", "-y", app]).status().is_ok_and(|s| s.success());
        }
        if let Some(rest) = it.action.strip_prefix("run:tamagi-browser install ") {
            let key = rest.split_whitespace().next().unwrap_or(target);
            return Command::new("tamagi-browser").args(["remove", key]).status().is_ok_and(|s| s.success());
        }
        if it.action.starts_with("run:tamagi-layer add ") {
            let layer = it.action.split_whitespace().nth(2).unwrap_or(target);
            return Command::new("tamagi-layer").args(["remove", layer]).status().is_ok_and(|s| s.success());
        }
        if let Some(rest) = it.action.strip_prefix("run:tamagi-devbox create ") {
            let distro = rest.split_whitespace().next().unwrap_or("");
            let cname = if distro == "fedora" { "dev".to_string() } else { format!("dev-{distro}") };
            return Command::new("tamagi-devbox").args(["remove", &cname]).status().is_ok_and(|s| s.success());
        }
        if it.action.starts_with("run:tamagi-windows ") {
            return Command::new("tamagi-windows").arg("remove").status().is_ok_and(|s| s.success());
        }
        if let Some(rest) = it.action.strip_prefix("run:tamagi-webapp install ") {
            let name = rest.split_whitespace().next().unwrap_or(target);
            return Command::new("tamagi-webapp").args(["remove", name]).status().is_ok_and(|s| s.success());
        }
        if it.action == "run:/usr/libexec/tamagi/nvidia-setup cuda" {
            return Command::new("/usr/libexec/tamagi/nvidia-setup").arg("remove-cuda").status().is_ok_and(|s| s.success());
        }
        if it.action.starts_with("run:tamagi-gaming ge-proton") {
            return Command::new("tamagi-gaming").args(["ge-proton", "remove"]).status().is_ok_and(|s| s.success());
        }
        if let Some(script) = it.action.strip_prefix("run:/usr/libexec/tamagi/") {
            let bin = format!("/usr/libexec/tamagi/{}", script.split_whitespace().next().unwrap_or(""));
            return Command::new(bin).arg("remove").status().is_ok_and(|s| s.success());
        }
        if it.action.starts_with("gaming:") {
            return Command::new("tamagi-gaming").arg("remove").status().is_ok_and(|s| s.success());
        }
        if let Some(agent) = it.action.strip_prefix("agent:") {
            return Command::new("tamagi-agent").args(["remove", agent]).status().is_ok_and(|s| s.success());
        }
    }
    if ALL_BROWSERS.contains(&target) {
        return Command::new("tamagi-browser").args(["remove", target]).status().is_ok_and(|s| s.success());
    }
    if is_app_id(target) {
        return Command::new("flatpak").args(["uninstall", "--user", "-y", target]).status().is_ok_and(|s| s.success());
    }
    let layer = resolve_layer_target(target, Path::new("/var/lib/tamagi/layers"), Path::new("/var/lib/extensions"));
    Command::new("tamagi-layer").args(["remove", &layer]).status().is_ok_and(|s| s.success())
}

fn interactive_remove(items: &[CatalogItem]) -> i32 {
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut seen_ids: Vec<String> = Vec::new();

    for it in items {
        if is_installed(it) {
            let kind = if it.action.starts_with("flatpak:") { "Flatpak" } else { "Native/System" };
            entries.push((format!("[{kind}] {} — {}", it.id, t(&it.desc)), it.id.clone()));
            seen_ids.push(it.id.clone());
            if let Some(rest) = it.action.strip_prefix("flatpak:") {
                if let Some(app) = rest.rsplit(':').next() {
                    seen_ids.push(app.to_string());
                }
            }
            if it.action.starts_with("run:tamagi-layer add ") {
                if let Some(layer) = it.action.split_whitespace().nth(2) {
                    seen_ids.push(layer.to_string());
                }
            }
        }
    }

    if let Ok(rd) = fs::read_dir("/var/lib/tamagi/layers") {
        let mut layers: Vec<String> = rd
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                (p.extension().and_then(|s| s.to_str()) == Some("request"))
                    .then(|| p.file_stem().and_then(|s| s.to_str()).map(String::from))
                    .flatten()
            })
            .collect();
        layers.sort();
        for l in layers {
            if !seen_ids.contains(&l) && !l.starts_with("browser-") {
                let pkgs = fs::read_to_string(format!("/var/lib/tamagi/layers/{l}.request"))
                    .unwrap_or_default()
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                entries.push((format!("[Native layer] {l} ({pkgs})"), l));
            }
        }
    }

    let (rc_fp, out_fp) = run_out(&[], "flatpak", &["list", "--user", "--app", "--columns=application,name"]);
    if rc_fp == 0 {
        for ln in out_fp.lines() {
            let mut cols = ln.splitn(2, '\t');
            let app = cols.next().unwrap_or("").trim();
            let name = cols.next().unwrap_or(app).trim();
            if !app.is_empty() && !seen_ids.iter().any(|s| s == app) {
                entries.push((format!("[Flatpak] {name} ({app})"), app.to_string()));
            }
        }
    }

    if entries.is_empty() {
        tl("Nothing installed to remove.");
        return 0;
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        for (lbl, id) in &entries {
            println!("  {:<22} {lbl}", id);
        }
        return 0;
    }

    let labels: Vec<String> = entries.iter().map(|(l, _)| l.clone()).collect();
    let Ok(mut child) = Command::new("/usr/libexec/tamagi/gum-multi")
        .args(["--height", "20", "--header", &t("Space toggles · Enter removes · nothing selected: removes nothing")])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        return 1;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(format!("{}\n", labels.join("\n")).as_bytes());
    }
    let Ok(out) = child.wait_with_output() else { return 1 };
    if !out.status.success() {
        return 0;
    }
    let chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if chosen.is_empty() {
        tl("Nothing selected.");
        return 0;
    }
    let targets: Vec<String> = chosen
        .lines()
        .filter_map(|line| entries.iter().find(|(l, _)| l == line).map(|(_, id)| id.clone()))
        .collect();
    cmd_remove(items, &targets)
}

fn cmd_remove(items: &[CatalogItem], targets: &[String]) -> i32 {
    if targets.is_empty() {
        return interactive_remove(items);
    }
    let mut rc = 0;
    for target in targets {
        if !remove_one(items, target) {
            rc = 1;
        }
    }
    rc
}

fn flathub_resolve(want: &str) -> (String, i32) {
    if env::var("TAMAGI_INSTALL_NO_FLATHUB").as_deref() == Ok("1") {
        return (String::new(), 1);
    }
    if is_app_id(want) {
        flathub_remote();
        if run_quiet("flatpak", &["remote-info", "--user", "flathub", want]) {
            return (format!("flatpak:flathub:{want}"), 0);
        }
    }
    let hits = flathub_search(want);
    if hits.is_empty() {
        return (String::new(), 1);
    }
    let low = want.to_lowercase();
    let mut exact: Option<(String, String)> = None;
    let mut labels = Vec::with_capacity(hits.len());
    for h in &hits {
        let mark = if h.verified { " ✓" } else { "" };
        labels.push(format!("{}  {} — {}{}", h.id, h.name, h.summary, mark));
        if exact.is_none() {
            let short = h.id.rsplit('.').next().unwrap_or(h.id.as_str());
            if h.name.to_lowercase() == low || short.to_lowercase() == low || h.id.to_lowercase() == low {
                exact = Some((h.id.clone(), h.name.clone()));
            }
        }
    }
    if exact.is_none() && hits.len() == 1 {
        exact = Some((hits[0].id.clone(), hits[0].name.clone()));
    }
    let (exact_id, exact_name) = match exact {
        Some(pair) => pair,
        None if io::stdin().is_terminal() && io::stdout().is_terminal() => {
            let header = format!("{}: {want}", t("Not in the catalog. On Flathub"));
            let Ok(mut child) = Command::new("gum")
                .args(["choose", "--height", "14", "--header", &header])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
            else {
                return (String::new(), 2);
            };
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(format!("{}\n", labels.join("\n")).as_bytes());
            }
            let Ok(out) = child.wait_with_output() else { return (String::new(), 2) };
            if !out.status.success() {
                return (String::new(), 2);
            }
            let pick = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let Some((id, rest)) = pick.split_once("  ") else { return (String::new(), 2) };
            let name = rest.trim().split(" — ").next().unwrap_or("").to_string();
            (id.to_string(), name)
        }
        None => {
            eprintln!("{}: {want}", t("Not in the catalog. On Flathub"));
            for lbl in labels {
                eprintln!("  {lbl}");
            }
            eprintln!("{}: tamagi install <app id>", t("Install one with"));
            return (String::new(), 2);
        }
    };
    println!("{}: {exact_name} ({exact_id})", t("From Flathub"));
    (format!("flatpak:flathub:{exact_id}"), 0)
}

fn last_line(path: &Path) -> String {
    let Ok(text) = fs::read_to_string(path) else { return String::new() };
    text.replace('\r', "\n").trim().lines().last().unwrap_or("").to_string()
}

fn print_tail(path: &Path, n: usize) {
    let Ok(text) = fs::read_to_string(path) else { return };
    let lines: Vec<&str> = text.trim().lines().collect();
    let start = lines.len().saturating_sub(n);
    for ln in &lines[start..] {
        println!("{ln}");
    }
}

fn run_actions(actions: &[String]) -> i32 {
    let mut by_remote: HashMap<String, Vec<String>> = HashMap::new();
    let mut remote_order: Vec<String> = Vec::new();
    let mut flatpak_apps: Vec<String> = Vec::new();
    let mut gaming: Vec<String> = Vec::new();
    let mut runs: Vec<String> = Vec::new();
    let mut agents: Vec<String> = Vec::new();

    for a in actions {
        if let Some(rest) = a.strip_prefix("flatpak:") {
            if let Some((r, app)) = rest.split_once(':') {
                if !by_remote.contains_key(r) {
                    remote_order.push(r.to_string());
                }
                by_remote.entry(r.to_string()).or_default().push(app.to_string());
                flatpak_apps.push(app.to_string());
            }
        } else if let Some(g) = a.strip_prefix("gaming:") {
            gaming.push(g.to_string());
        } else if let Some(ag) = a.strip_prefix("agent:") {
            agents.push(ag.to_string());
        } else if let Some(r) = a.strip_prefix("run:") {
            runs.push(r.to_string());
        }
    }

    let started = Instant::now();
    let mut tx_rows: Vec<(String, String, String, String)> = Vec::new();
    for app in &flatpak_apps {
        tx_rows.push((app.clone(), "Flatpak".into(), "user".into(), "install".into()));
    }
    for g in &gaming {
        tx_rows.push((g.clone(), "Gaming".into(), "layer".into(), "install".into()));
    }
    for ag in &agents {
        tx_rows.push((ag.clone(), "AI Agent".into(), "user".into(), "install".into()));
    }
    for rcmd in &runs {
        let label = rcmd.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
        tx_rows.push((label, "Native".into(), "system".into(), "install".into()));
    }
    ui::print_tx_table(&t("Transaction summary"), &tx_rows);

    let mut total = 0;
    if !remote_order.is_empty() {
        total += 1;
    }
    if !gaming.is_empty() {
        total += 1;
    }
    total += runs.len() + agents.len();
    let mut n: usize = 0;
    let mut step = |msg: &str| {
        n += 1;
        println!();
        let bar = ui::progress_bar(n.saturating_sub(1), total, 14);
        ui::step_progress(n, total, &format!("{bar} {msg}"));
    };

    let mut failed: Vec<String> = Vec::new();
    let mut fp_child: Option<std::process::Child> = None;
    let fp_log_path = cache_dir().join("install-flatpak.log");

    if !remote_order.is_empty() {
        let _ = fs::create_dir_all(cache_dir());
        flathub_remote();
        step(&format!("Flatpak: {}", flatpak_apps.join(" ")));
        for r in &remote_order {
            if let Some(apps) = by_remote.get(r) {
                runtimes_note(r, apps);
            }
        }
        prefetch_wait();
        if total > 1 {
            let mut script = String::new();
            for r in &remote_order {
                if let Some(apps) = by_remote.get(r) {
                    let quoted: Vec<String> = apps.iter().map(|a| format!("'{}'", a.replace('\'', "'\\''"))).collect();
                    script.push_str(&format!("flatpak install --user -y --noninteractive --or-update '{r}' {} || exit 1\n", quoted.join(" ")));
                }
            }
            let mut cmd = Command::new("bash");
            cmd.args(["-c", &script]);
            if let Ok(lf) = fs::File::create(&fp_log_path) {
                if let Ok(lf2) = lf.try_clone() {
                    cmd.stdout(Stdio::from(lf)).stderr(Stdio::from(lf2));
                }
            }
            match cmd.spawn() {
                Ok(child) => {
                    fp_child = Some(child);
                    tl("  Downloading in the background while the rest is installed.");
                }
                Err(_) => failed.extend(flatpak_apps.iter().cloned()),
            }
        } else {
            let mut ok = true;
            for r in &remote_order {
                if let Some(apps) = by_remote.get(r) {
                    let mut c = Command::new("flatpak");
                    c.args(["install", "--user", "-y", "--noninteractive", "--or-update", r]).args(apps);
                    if !c.status().is_ok_and(|s| s.success()) {
                        ok = false;
                    }
                }
            }
            if !ok {
                failed.extend(flatpak_apps.iter().cloned());
            }
        }
    }

    if !gaming.is_empty() {
        step(&format!("{}: {}", t("Native games"), gaming.join(" ")));
        if !Command::new("tamagi-gaming").arg("native").args(&gaming).status().is_ok_and(|s| s.success()) {
            failed.push("gaming".into());
        }
    }

    for rcmd in &runs {
        step(rcmd);
        let parts: Vec<&str> = rcmd.split_whitespace().collect();
        if !parts.is_empty() && !Command::new(parts[0]).args(&parts[1..]).status().is_ok_and(|s| s.success()) {
            failed.push(rcmd.clone());
        }
    }

    for ag in &agents {
        step(&format!("{}: {ag}", t("AI agent")));
        if !Command::new("tamagi-agent").args(["install", ag]).status().is_ok_and(|s| s.success()) {
            failed.push(ag.clone());
        }
    }

    if let Some(mut child) = fp_child {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    failed.extend(flatpak_apps.iter().cloned());
                    print_tail(&fp_log_path, 5);
                }
            }
            _ => {
                println!();
                let sp = ui::Spinner::start(t("Waiting for the Flatpaks still downloading…"));
                let status = child.wait();
                match status {
                    Ok(st) if st.success() => sp.finish_ok(Some("Flatpak")),
                    _ => {
                        sp.finish_err(Some("Flatpak"));
                        failed.extend(flatpak_apps.iter().cloned());
                        print_tail(&fp_log_path, 5);
                    }
                }
            }
        }
    }

    println!();
    if !failed.is_empty() {
        println!("{} {}: {}", ui::red("✗"), t("Could not install"), failed.join(" "));
        println!("{}: tamagi pkg", t("Try again with"));
        return 1;
    }
    let secs = started.elapsed().as_secs_f32();
    println!("{} {} {}", ui::green("✓"), t("Everything installed."), ui::dim(&format!("({secs:.1}s)")));
    0
}

fn print_help() {
    print!(
        "tamagi install — unified catalog, live Flathub search, and native sysext packages.

  tamagi install                 interactive catalog (Flathub + native layers + drivers + gaming)
  tamagi install list            list everything in the curated catalog
  tamagi install search <text>   search both the curated catalog and Flathub
  tamagi install <id|name>…      install from catalog (asks Flatpak/native for browsers) or Flathub
  tamagi install --native <pkg>… install browser/catalog app or Fedora RPM(s) natively (sysext)
  tamagi install --flatpak <app>… install app or browser as a sandboxed Flatpak (--user)
  tamagi remove [id|pkg|app]…    remove any installed Flatpak, browser, agent, or native layer
  tamagi install browser|webapp|gaming|windows […]
"
    );
}

fn main() {
    let items = load_catalog();
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        if !io::stdin().is_terminal() {
            cmd_list(&items);
            return;
        }
        let flathub_prompt = format!("[Flathub] {}", t("Search Flathub online…"));
        let native_prompt = format!("[Native] {}", t("Install Fedora package natively (sysext)…"));
        let mut labels = Vec::with_capacity(items.len() + 2);
        labels.push(flathub_prompt.clone());
        labels.push(native_prompt.clone());
        for it in &items {
            labels.push(it.label());
        }
        let Ok(mut child) = Command::new("/usr/libexec/tamagi/gum-multi")
            .args(["--height", "20", "--header", &t("Space toggles · Enter installs · nothing selected: installs nothing")])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        else {
            return;
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(format!("{}\n", labels.join("\n")).as_bytes());
        }
        let Ok(out) = child.wait_with_output() else { return };
        if !out.status.success() {
            return;
        }
        let chosen = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if chosen.is_empty() {
            tl("Nothing selected.");
            return;
        }
        let mut picked = Vec::new();
        for line in chosen.lines() {
            if line == flathub_prompt {
                if let Ok(q_out) = Command::new("gum").args(["input", "--placeholder", &t("App name or Flathub ID…")]).output() {
                    if q_out.status.success() {
                        let q = String::from_utf8_lossy(&q_out.stdout).trim().to_string();
                        if !q.is_empty() {
                            let (act, rc) = flathub_resolve(&q);
                            if rc == 0 {
                                picked.push(act);
                            }
                        }
                    }
                }
                continue;
            }
            if line == native_prompt {
                if let Ok(q_out) = Command::new("gum").args(["input", "--placeholder", &t("Fedora package name(s)…")]).output() {
                    if q_out.status.success() {
                        let q = String::from_utf8_lossy(&q_out.stdout).trim().to_string();
                        let pkgs: Vec<String> = q.split_whitespace().map(str::to_string).collect();
                        if !pkgs.is_empty() {
                            exit(cmd_native_install(&items, &pkgs));
                        }
                    }
                }
                continue;
            }
            if let Some(it) = items.iter().find(|it| it.label() == line) {
                if is_dual_browser(&it.id) {
                    picked.push(choose_browser_action(&it.id));
                } else {
                    picked.push(it.action.clone());
                }
            }
        }
        if picked.is_empty() {
            tl("Nothing selected.");
            return;
        }
        exit(run_actions(&picked));
    }

    match args[0].as_str() {
        "list" => cmd_list(&items),
        "search" => exit(cmd_search(&items, &args[1..].join(" "))),
        "--native" | "native" => exit(cmd_native_install(&items, &args[1..])),
        "--flatpak" | "flatpak" => exit(cmd_flatpak_install(&items, &args[1..])),
        "remove" | "uninstall" | "rm" => exit(cmd_remove(&items, &args[1..])),
        "ids" => {
            for it in &items {
                println!("{}", it.id);
            }
        }
        "installed" => {
            for it in &items {
                if is_installed(it) {
                    println!("{}\t{}", it.id, it.action);
                }
            }
        }
        "--actions" => exit(run_actions(&args[1..])),
        "prefetch" => cmd_prefetch(),
        "prefetch-wait" => prefetch_wait(),
        "prefetch-cleanup" => cmd_prefetch_cleanup(),
        "-h" | "--help" | "help" => print_help(),
        _ => {
            let mut picked = Vec::new();
            for want in &args {
                if is_dual_browser(want) {
                    picked.push(choose_browser_action(want));
                    continue;
                }
                if let Some(it) = items.iter().find(|it| &it.id == want) {
                    picked.push(it.action.clone());
                    continue;
                }
                let (act, rc) = flathub_resolve(want);
                if rc == 0 {
                    picked.push(act);
                    continue;
                }
                if rc == 2 {
                    exit(2);
                }
                eprintln!("{}: {}: {want}  (tamagi install list · tamagi install search {want} · tamagi install --native {want})", t("error"), t("not in the catalog and not on Flathub"));
                exit(2);
            }
            exit(run_actions(&picked));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_catalog_and_optional_flatpaks() {
        let cat = "# comment\nsteam\tGames\t900MB\tgaming:steam\tSteam\n";
        let fp = "flathub org.gnome.Loupe essencial Utilities 20MB Image Viewer\nflathub org.onlyoffice.desktopeditors opcional Office 300MB OnlyOffice — documents and sheets\n";
        let items = parse_catalog_files(cat, fp);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "steam");
        assert_eq!(items[1].id, "onlyoffice");
        assert_eq!(items[1].action, "flatpak:flathub:org.onlyoffice.desktopeditors");
    }

    #[test]
    fn validates_flathub_app_ids() {
        assert!(is_app_id("org.videolan.VLC"));
        assert!(is_app_id("io.github.flattool.Ignition"));
        assert!(!is_app_id("vlc"));
        assert!(!is_app_id("a.b"));
        assert!(!is_app_id("org.bad/slash.App"));
    }

    #[test]
    fn native_layer_and_dual_browsers() {
        assert!(is_dual_browser("chrome"));
        assert!(is_dual_browser("firefox"));
        assert!(!is_dual_browser("opera-gx"));
        assert_eq!(native_layer_name("htop"), "pkg-htop");
        assert_eq!(native_layer_name("ImageMagick"), "pkg-imagemagick");
        assert_eq!(native_layer_name("123bad"), "pkg-custom");
    }

    #[test]
    fn resolves_prefixed_native_layers() {
        let dir = env::temp_dir().join("tamagi-test-layers");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("pkg-htop.request"), "htop\n").unwrap();
        fs::write(dir.join("rocm.request"), "rocm-hip\n").unwrap();
        assert_eq!(resolve_layer_target("htop", &dir, &dir), "pkg-htop");
        assert_eq!(resolve_layer_target("rocm", &dir, &dir), "rocm");
        assert_eq!(resolve_layer_target("missing", &dir, &dir), "missing");
        let _ = fs::remove_dir_all(&dir);
    }
}
