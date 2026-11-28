//! tamagi — 5 clean commands for the whole system (no legacy aliases).
//!
//!   1. tamagi update   atomic OS updates, layers, Flatpaks, boxes, agents, rollback
//!   2. tamagi pkg      unified catalog (Flathub + native layers), remove, search, layer
//!   3. tamagi ai       local AI, coding agents, Shadow CoW, MCP Quadlets, flights
//!   4. tamagi dev      devboxes, local databases, virtual machines, workspaces, try, isolate
//!   5. tamagi sys      doctor, status, why, rewind, shield, setup, perf, desktop, skl, tamagi.toml

mod i18n;
mod plan;
mod state;
mod ui;

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

use i18n::t;
use plan::{Probe, Step};

const PROFILES: &[(&str, &str)] = &[
    ("ai", "AI and agents: Claude Code, Codex, opencode and the development box"),
    ("dev", "Development: Fedora box, Node and Python, Zed and opencode"),
    ("gamer", "Games: native Steam, gamescope with HDR, MangoHud and Heroic"),
    ("office", "Office: Firefox, OnlyOffice and Thunderbird"),
];

/// The 5 primary commands shown by `tamagi` (no legacy aliases).
const COMMANDS: &[(&str, &str)] = &[
    ("update", "atomic updates: OS image, layers, Flatpaks, boxes, agents, rollback"),
    ("pkg", "install, remove, search catalog/Flathub, and reversible sysext layers"),
    ("ai", "AI agents, Shadow CoW, Egress Guard, flights, MCP Quadlets, local llama.cpp"),
    ("dev", "development boxes, local databases, VMs, workspaces, try and isolate"),
    ("sys", "doctor, status, why, rewind, shield, setup, perf, desktop, skl, state"),
];

const FILE_COMMANDS: &[(&str, &str)] = &[
    ("init", "create tamagi.toml from the current state"),
    ("export", "print or save the current state"),
    ("diff", "show what is missing"),
    ("apply", "apply what is missing"),
];

fn die(msg: &str) -> ! {
    let _ = io::stdout().flush();
    eprintln!("{msg}");
    exit(1)
}

fn exec_helper(prog_name: &str, args: &[String]) -> ! {
    let Some(prog) = state::which(prog_name) else {
        die(&format!("tamagi: {}: {prog_name}", t("missing helper")));
    };
    let err = Command::new(&prog).args(args).exec();
    die(&format!("tamagi: {}: {}", prog.display(), strerror(&err)));
}

fn exec_libexec(name: &str, args: &[String]) -> ! {
    let path = PathBuf::from("/usr/libexec/tamagi").join(name);
    if !path.is_file() {
        die(&format!("tamagi: {}: {}", t("missing helper"), path.display()));
    }
    let err = Command::new(&path).args(args).exec();
    die(&format!("tamagi: {}: {}", path.display(), strerror(&err)));
}

fn cmd_tiling(args: &[String]) {
    let action = args.first().map(String::as_str).unwrap_or("toggle");
    let new = match action {
        "status" => state::tiling(),
        "on" => true,
        "off" => false,
        "toggle" => !state::tiling(),
        _ => die("usage: tamagi sys desktop tiling [on|off|toggle|status]"),
    };
    if action != "status" {
        let file = state::tiling_file();
        let written = file.parent().map(fs::create_dir_all).unwrap_or(Ok(())).and_then(|_| fs::write(&file, if new { "true\n" } else { "false\n" }));
        if let Err(e) = written {
            die(&format!("{}: {}: {}", t("error"), file.display(), strerror(&e)));
        }
    }
    println!("tiling: {}", if new { t("running") } else { t("off") });
}

fn cmd_profiles() {
    for (name, desc) in PROFILES {
        println!("  tamagi sys apply {name:<7} {}", t(desc));
    }
    println!("\n{}", t("A profile only adds what is missing; nothing is removed."));
}

fn cmd_list() {
    let width = COMMANDS.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    let edition = fs::read_to_string("/usr/share/tamagi/edition").unwrap_or_else(|_| "dxai".into());
    let label = match edition.trim() {
        "skl" => "Tamagi-SKL",
        "skl-os" => "SKL",
        _ => "Tamagi-DxAI",
    };
    ui::phase(&format!("tamagi ({label}) — {}", t("one command for the whole system")));
    println!();
    for (name, desc) in COMMANDS {
        println!("  tamagi {}  {}", ui::bold(&format!("{name:<width$}")), t(desc));
    }
    println!("\n{}: tamagi sys profiles  ·  tamagi sys apply ai | dev | gamer | office", ui::dim(&t("Ready-made profiles")));
}

// ── Subcommand routers (the 8 commands) ─────────────────────────────────────

fn route_ai(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            ui::phase(&format!("tamagi ai — {}", t("AI agents, Shadow CoW, flights, MCP Quadlets, and local llama.cpp")));
            println!();
            println!("  tamagi ai run [opts] <agent>       run coding agent jailed (--shadow, --egress-guard)");
            println!("  tamagi ai flights [list|show|clear] agent flight recorder (audit + rollback)");
            println!("  tamagi ai shadow <list|diff|apply|drop>  manage CoW shadow workspaces");
            println!("  tamagi ai jail [opts]              open a shell in the agent jail");
            println!("  tamagi ai agents [list|install|remove|update]");
            println!("  tamagi ai mcp [list|add|remove|sync|status]");
            println!("  tamagi ai ask [prompt]             chat with local system agent");
            println!("  tamagi ai code [args]              launch OpenCode connected to Tamagi AI & MCP");
            println!("  tamagi ai local <setup|presets|chat|status|rm>  local llama.cpp models");
            exit(0);
        }
        Some("run" | "jail" | "shadow" | "shadows" | "flights") => exec_helper("tamagi-agent", args),
        Some("agents") => exec_helper("tamagi-agent", &args[1..]),
        Some("mcp") => exec_helper("tamagi-mcp", &args[1..]),
        Some("opencode") => {
            let mut forwarded = Vec::with_capacity(args.len());
            forwarded.push("code".into());
            forwarded.extend_from_slice(&args[1..]);
            exec_helper("tamagi-ai", &forwarded)
        }
        Some("ask") => {
            let mut forwarded = Vec::with_capacity(args.len());
            forwarded.push("ask".into());
            forwarded.extend_from_slice(&args[1..]);
            exec_helper("tamagi-agent", &forwarded)
        }
        Some("local") => exec_helper("tamagi-ai", &args[1..]),
        Some(_) => exec_helper("tamagi-ai", args),
    }
}

fn route_dev(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            ui::phase(&format!("tamagi dev — {}", t("development boxes, local databases, VMs, workspaces, try and isolate")));
            println!();
            println!("  tamagi dev box [create|enter|lang|list|update|remove]  distrobox dev environments");
            println!("  tamagi dev db [add|url|shell|snapshot|restore]         local Podman Quadlet databases");
            println!("  tamagi dev vm [install|status|open]                    virtual machines (QEMU/KVM + Cockpit)");
            println!("  tamagi dev workspace [init|status|up|enter|run]        per-project workspaces");
            println!("  tamagi dev try <pkg> [opts]                            disposable package container");
            println!("  tamagi dev isolate <file> [opts]                       disposable untrusted sandbox");
            exit(0);
        }
        Some("box") => exec_helper("tamagi-devbox", &args[1..]),
        Some("db") => exec_helper("tamagi-db", &args[1..]),
        Some("vm") => exec_helper("tamagi-vm", &args[1..]),
        Some("workspace") => exec_helper("tamagi-workspace", &args[1..]),
        Some("try") => exec_helper("tamagi-try", &args[1..]),
        Some("isolate") => exec_helper("tamagi-isolate", &args[1..]),
        Some(other) => die(&format!("tamagi dev: {}: {other} (box | db | vm | workspace | try | isolate)", t("unknown command"))),
    }
}

fn route_pkg(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        Some("-h" | "--help" | "help") => {
            ui::phase(&format!("tamagi pkg — {}", t("install, remove, search catalog/Flathub, and reversible sysext layers")));
            println!();
            println!("  tamagi pkg [install|add] [opts] <id|app>…  install from catalog, Flathub or --native");
            println!("  tamagi pkg remove|rm [id|pkg|app]…         remove Flatpak, browser, agent or layer");
            println!("  tamagi pkg search <text>                   search curated catalog + Flathub");
            println!("  tamagi pkg list                            list curated catalog");
            println!("  tamagi pkg layer [add|remove|list|rebuild] reversible systemd-sysext layers");
            println!("  tamagi pkg copr <user/repo> <pkg>…         install COPR package(s) as a reversible layer");
            println!("  tamagi pkg browser|webapp|gaming|windows   specialized installers");
            exit(0);
        }
        Some("install" | "add") => exec_helper("tamagi-install", &args[1..]),
        Some("remove" | "uninstall" | "rm") => {
            let mut forwarded = Vec::with_capacity(args.len());
            forwarded.push("remove".into());
            forwarded.extend_from_slice(&args[1..]);
            exec_helper("tamagi-install", &forwarded)
        }
        Some("layer" | "layers") => exec_helper("tamagi-layer", &args[1..]),
        Some("copr") => {
            if args.len() < 3 || !args[1].contains('/') {
                die("usage: tamagi pkg copr <user/repo> <pkg>…");
            }
            let slug: String = args[1]
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
                .take(28)
                .collect();
            let mut forwarded = vec!["add".into(), format!("copr-{slug}"), "--copr".into(), args[1].clone()];
            forwarded.extend_from_slice(&args[2..]);
            exec_helper("tamagi-layer", &forwarded)
        }
        Some("browser") => exec_helper("tamagi-browser", &args[1..]),
        Some("webapp") => exec_helper("tamagi-webapp", &args[1..]),
        Some("gaming") => exec_helper("tamagi-gaming", &args[1..]),
        Some("windows") => exec_helper("tamagi-windows", &args[1..]),
        _ => exec_helper("tamagi-install", args),
    }
}

fn route_rewind(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        Some("snap") => exec_helper("tamagi-snap", &args[1..]),
        _ => exec_helper("tamagi-rewind", args),
    }
}

fn route_shield(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("tamagi shield — {}", t("security hardening, leases, privacy mode, and Tailscale"));
            println!();
            println!("  tamagi shield harden [status|strict|balanced|usb|malloc]");
            println!("  tamagi shield lease [vm|usb|port|end]");
            println!("  tamagi shield privacy [on|off|status]");
            println!("  tamagi shield tailscale [on|off|status]");
            exit(0);
        }
        Some("harden") => exec_helper("tamagi-harden", &args[1..]),
        Some("lease") => exec_helper("tamagi-lease", &args[1..]),
        Some("privacy") => exec_helper("tamagi-privacy", &args[1..]),
        Some("tailscale") => exec_libexec("tailscale-setup", &args[1..]),
        Some(other) => die(&format!("tamagi shield: {}: {other} (harden | lease | privacy | tailscale)", t("unknown command"))),
    }
}

fn route_desktop(args: &[String]) -> ! {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("tamagi sys desktop — {}", t("COSMIC theme, bar, font, tiling, keys, and capture"));
            println!();
            println!("  tamagi sys desktop theme|panel|font|tiling|keys|capture|menu [...]");
            exit(0);
        }
        Some("theme") => exec_helper("tamagi-theme", &args[1..]),
        Some("panel") => exec_helper("tamagi-panel", &args[1..]),
        Some("font") => exec_helper("tamagi-font", &args[1..]),
        Some("tiling") => {
            cmd_tiling(&args[1..]);
            exit(0);
        }
        Some("keys") => exec_helper("tamagi-keys", &args[1..]),
        Some("capture") => exec_helper("tamagi-capture", &args[1..]),
        Some("menu") => exec_helper("tamagi-menu", &args[1..]),
        Some(other) => die(&format!("tamagi sys desktop: {}: {other}", t("unknown command"))),
    }
}

fn route_sys(args: &[String]) {
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            ui::phase(&format!("tamagi sys — {}", t("health, setup, performance, desktop, school admin, and declarative state")));
            println!();
            println!("  tamagi sys doctor [--json|--fix|--explain|--report]  system health diagnostics");
            println!("  tamagi sys cfs [status|fsck|images|measure|gc]  composefs-rs & bootc verity");
            println!("  tamagi sys status [--json]                  background jobs and pending updates");
            println!("  tamagi sys why <path|cmd|port|service>      provenance inspector");
            println!("  tamagi sys rewind [snap|list|restore]       unified timeline (home + OS)");
            println!("  tamagi sys shield <harden|lease|privacy|tailscale>  security & Tailscale");
            println!("  tamagi sys boot-report [show]               boot health and rollback reports");
            println!("  tamagi sys setup <tpm-unlock|firmware|bios|clean|switch-image>");
            println!("  tamagi sys perf [status|set|pick|bench]     CPU & sched_ext (scx_rustland / scx_rusty)");
            println!("  tamagi sys lang [list|pick|set]             system language");
            println!("  tamagi sys desktop <theme|panel|font|tiling|keys|capture|menu>");
            println!("  tamagi sys skl <status|history|open|homepage|lock|unlock|reset-session>");
            println!("  tamagi sys init | export | diff | apply | profiles  declarative tamagi.toml");
            return;
        }
        Some("doctor") => exec_helper("tamagi-doctor", &args[1..]),
        Some("cfs" | "composefs") => exec_helper("tamagi-cfs", &args[1..]),
        Some("status") => exec_helper("tamagi-status", &args[1..]),
        Some("why") => exec_helper("tamagi-why", &args[1..]),
        Some("rewind") => route_rewind(&args[1..]),
        Some("shield") => route_shield(&args[1..]),
        Some("boot-report") => exec_helper("tamagi-boot-report", &args[1..]),
        Some("setup") => exec_helper("tamagi-setup", &args[1..]),
        Some("perf") => exec_helper("tamagi-perf", &args[1..]),
        Some("lang") => exec_helper("tamagi-lang", &args[1..]),
        Some("desktop") => route_desktop(&args[1..]),
        Some("skl") => exec_helper("tamagi-skl", &args[1..]),
        Some("profiles") => return cmd_profiles(),
        Some(cmd @ ("init" | "export" | "diff" | "apply")) => {
            let parsed = file_args(cmd, &args[1..]);
            match cmd {
                "init" | "export" => save_state(cmd, parsed.file),
                _ => diff_or_apply(cmd, parsed),
            }
            return;
        }
        Some(other) => die(&format!("tamagi sys: {}: {other} ({}: tamagi sys --help)", t("unknown command"), t("see"))),
    }
}

// ── The system file (tamagi.toml) ───────────────────────────────────────────

fn strerror(e: &io::Error) -> String {
    let text = e.to_string();
    match text.rfind(" (os error ") {
        Some(i) => text[..i].to_string(),
        None => text,
    }
}

fn toml_error(text: &str, e: &toml::de::Error) -> String {
    match e.span() {
        Some(span) => {
            let before = &text[..span.start.min(text.len())];
            let line = before.matches('\n').count() + 1;
            let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            format!("{} (at line {line}, column {column})", e.message())
        }
        None => e.message().to_string(),
    }
}

fn has_scheme(source: &str) -> bool {
    source.split_once("://").is_some_and(|(scheme, _)| !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_lowercase()))
}

fn download(source: &str) -> String {
    const LIMIT: usize = 1_000_000;
    let fetched = Command::new("curl")
        .args(["-fsSL", "--max-time", "30", "--proto", "=https", "--proto-redir", "=https", "--max-filesize"])
        .arg((LIMIT + 1).to_string())
        .args(["--", source])
        .stdin(Stdio::null())
        .output();
    let output = match fetched {
        Ok(o) => o,
        Err(e) => die(&format!("{}: {} {source}: curl: {}", t("error"), t("could not download"), strerror(&e))),
    };
    if output.status.code() == Some(63) || output.stdout.len() > LIMIT {
        die(&format!("{}: {}", t("error"), t("file too large for a tamagi.toml")));
    }
    if !output.status.success() {
        let why = String::from_utf8_lossy(&output.stderr).trim().to_string();
        die(&format!("{}: {} {source}: {why}", t("error"), t("could not download")));
    }
    println!("{} {source}", t("Downloaded from"));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn resolve(source: &str) -> (String, Option<String>) {
    if source.starts_with("https://") {
        return (source.to_string(), Some(download(source)));
    }
    if has_scheme(source) {
        die(&format!("{}: {}", t("error"), t("only https:// addresses are accepted")));
    }
    let profiles = env::var_os("TAMAGI_PROFILES").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/usr/share/tamagi/profiles"));
    let profile = profiles.join(format!("{source}.toml"));
    if !Path::new(source).exists() && profile.is_file() {
        return (profile.to_string_lossy().into_owned(), None);
    }
    (source.to_string(), None)
}

fn load(path: &str, fetched: Option<String>) -> toml::Table {
    let text = match fetched {
        Some(text) => text,
        None => match fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => die(&format!("{}: {} {path}: {}. {}: tamagi sys init", t("error"), t("could not read"), strerror(&e), t("Create one with"))),
        },
    };
    let raw = match text.parse::<toml::Table>() {
        Ok(table) => table,
        Err(e) => die(&format!("{}: {path} {}: {}", t("error"), t("is not valid TOML"), toml_error(&text, &e))),
    };
    let mut warnings = Vec::new();
    let wanted = plan::modernize(raw, &mut warnings);
    for w in warnings {
        eprintln!("{w}");
    }
    let problems = plan::problems(&wanted, &state::catalog_ids);
    if !problems.is_empty() {
        die(&format!("{}:\n  {}", t("file error"), problems.join("\n  ")));
    }
    wanted
}

struct Machine;
impl Probe for Machine {
    fn box_exists(&self, name: &str) -> bool {
        state::box_exists(name)
    }
    fn catalog_installed(&self) -> BTreeSet<String> {
        state::catalog_installed_all()
    }
    fn webapp_exists(&self, slug: &str) -> bool {
        state::data_dir().join("applications").join(format!("webapp-{slug}.desktop")).exists()
    }
}

const USAGE: &str = "usage: tamagi sys [-h] {init,export,diff,apply} ...";

fn usage_of(cmd: &str) -> String {
    match cmd {
        "init" => "usage: tamagi sys init [-h]".into(),
        "export" => "usage: tamagi sys export [-h] [file]".into(),
        "diff" => "usage: tamagi sys diff [-h] [file]".into(),
        _ => "usage: tamagi sys apply [-h] [-y] [file]".into(),
    }
}

fn usage_error(usage: &str, msg: &str) -> ! {
    eprintln!("{usage}\ntamagi: error: {msg}");
    exit(2)
}

fn sub_help(cmd: &str) -> ! {
    println!("{}", usage_of(cmd));
    if cmd != "init" {
        println!("\npositional arguments:\n  file");
    }
    println!("\noptions:\n  -h, --help  show this help message and exit");
    if cmd == "apply" {
        println!("  -y, --yes   {}", t("do not ask before starting"));
    }
    exit(0)
}

struct FileArgs {
    file: Option<String>,
    yes: bool,
}

fn file_args(cmd: &str, args: &[String]) -> FileArgs {
    let mut parsed = FileArgs { file: None, yes: false };
    let mut extra: Vec<&str> = Vec::new();
    let mut only_files = false;
    for a in args {
        let a = a.as_str();
        if !only_files && a == "--" {
            only_files = true;
        } else if !only_files && (a == "-h" || (a.len() > 2 && "--help".starts_with(a))) {
            sub_help(cmd);
        } else if !only_files && cmd == "apply" && (a == "-y" || (a.len() > 2 && "--yes".starts_with(a))) {
            parsed.yes = true;
        } else if !only_files && a.len() > 1 && a.starts_with('-') {
            extra.push(a);
        } else if cmd != "init" && parsed.file.is_none() {
            parsed.file = Some(a.to_string());
        } else {
            extra.push(a);
        }
    }
    if !extra.is_empty() {
        usage_error(USAGE, &format!("unrecognized arguments: {}", extra.join(" ")));
    }
    parsed
}

fn save_state(cmd: &str, file: Option<String>) {
    let text = state::to_toml(&state::current_state());
    let target = if cmd == "init" { Some(state::default_file()) } else { file.map(PathBuf::from) };
    let Some(target) = target else {
        println!("{text}");
        return;
    };
    if cmd == "init" && target.exists() {
        die(&format!("{} {}: tamagi sys diff", target.display(), t("already exists. To see differences")));
    }
    let parent = target.parent().filter(|p| !p.as_os_str().is_empty());
    if let Err(e) = parent.map(fs::create_dir_all).unwrap_or(Ok(())).and_then(|_| fs::write(&target, text)) {
        die(&format!("{}: {}: {}", t("error"), target.display(), strerror(&e)));
    }
    println!("{} {}. {}: tamagi sys apply", t("Saved to"), target.display(), t("Edit and run"));
}

fn confirm() -> bool {
    if !io::stdin().is_terminal() {
        die(&t("Run in a terminal, or pass -y to apply without prompting."));
    }
    print!("{} [s/y/N] ", t("Apply?"));
    let _ = io::stdout().flush();
    let mut answer = String::new();
    if io::stdin().read_line(&mut answer).unwrap_or(0) == 0 {
        println!();
        return false;
    }
    matches!(answer.trim().to_lowercase().as_str(), "s" | "sim" | "y" | "yes" | "o" | "oui")
}

fn run_steps(steps: &[Step]) {
    let mut failed: Vec<&str> = Vec::new();
    for s in steps {
        println!("\n── {}", s.desc);
        let _ = io::stdout().flush();
        let done = state::which(&s.cmd[0]).is_some() && Command::new(&s.cmd[0]).args(&s.cmd[1..]).status().map(|st| st.success()).unwrap_or(false);
        if !done {
            failed.push(&s.desc);
        }
    }
    println!();
    if !failed.is_empty() {
        die(&format!("{}: {}\n{}", t("Finished with failures in"), failed.join("; "), t("Run `tamagi sys diff` to see what is still missing.")));
    }
    println!("{}", t("Done: the machine now matches the file."));
}

fn diff_or_apply(cmd: &str, args: FileArgs) {
    let source = args.file.unwrap_or_else(|| state::default_file().to_string_lossy().into_owned());
    let (path, fetched) = resolve(&source);
    let yes = args.yes && fetched.is_none();
    let wanted = load(&path, fetched);
    let now = state::current_state();
    let steps = plan::plan(&wanted, &now, &Machine);
    if steps.is_empty() {
        println!("{}", t("Nothing to do: the machine already matches the file."));
    } else {
        println!("{} ({}):", if cmd == "diff" { t("Missing") } else { t("Will apply") }, steps.len());
        for s in &steps {
            let tail = if s.root { format!("   [{}]", t("needs password")) } else { String::new() };
            println!("  + {}{tail}", s.desc);
        }
    }
    let more = plan::extras(&wanted, &now);
    if !more.is_empty() && cmd == "diff" {
        println!("{} ({}; {}):", t("On this machine and outside the file"), more.len(), t("nothing is removed"));
        for item in more {
            println!("  · {item}");
        }
    }
    if cmd == "diff" || steps.is_empty() {
        return;
    }
    if !yes && !confirm() {
        die(&t("Nothing was changed."));
    }
    run_steps(&steps);
}

fn is_command_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    let first = argv.first().map(String::as_str).unwrap_or("");
    if argv.is_empty() || matches!(first, "-h" | "--help" | "help") {
        return cmd_list();
    }
    let rest = &argv[1..];
    match first {
        "update" => exec_helper("tamagi-update", rest),
        "pkg" => route_pkg(rest),
        "ai" => route_ai(rest),
        "dev" => route_dev(rest),
        "sys" => route_sys(rest),
        _ => die(&format!("tamagi: {}: {first}  ({}: tamagi)", t("unknown command"), t("see"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_count_is_five_and_valid() {
        assert_eq!(COMMANDS.len(), 5);
        for (name, _) in COMMANDS {
            assert!(is_command_name(name), "{name}");
        }
        assert_eq!(FILE_COMMANDS.len(), 4);
    }

    #[test]
    fn schemes() {
        assert!(has_scheme("http://x"));
        assert!(has_scheme("file:///etc/passwd"));
        assert!(!has_scheme("./a://b"));
        assert!(!has_scheme("profile"));
        assert!(!has_scheme("://x"));
    }

    #[test]
    fn toml_mistakes_say_where() {
        let text = "[a]\nx = 1\nthis is = = not toml\n";
        let e = text.parse::<toml::Table>().unwrap_err();
        let msg = toml_error(text, &e);
        assert!(msg.ends_with("(at line 3, column 6)"), "{msg}");
        assert!(!msg.contains('\n'));
    }

    #[test]
    fn io_error_text() {
        let e = io::Error::from_raw_os_error(2);
        assert_eq!(strerror(&e), "No such file or directory");
    }
}
