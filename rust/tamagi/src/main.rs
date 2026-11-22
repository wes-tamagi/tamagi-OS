//! tamagi — one command for the whole system.
//!
//!     tamagi                    lists every command
//!     tamagi <command> [...]    runs it (tamagi doctor, tamagi update, tamagi devbox ...)
//!
//! The system in one file (~/.config/tamagi/tamagi.toml):
//!
//!     tamagi init               writes the file from the current state
//!     tamagi export [file]      prints (or writes) the current state
//!     tamagi diff [file]        shows what is missing for the machine to match the file
//!     tamagi apply [file]       does what is missing (-y does not ask)
//!     tamagi apply <profile>    a ready-made profile: ai, dev, gamer, office
//!     tamagi apply <https URL>  a file someone shared (shows the plan and asks)
//!     tamagi profiles           lists the profiles
//!
//! Every `tamagi <command>` is the program `tamagi-<command>`; command names are
//! the same in every language.

mod i18n;
mod plan;
mod state;

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

/// Commands shown by `tamagi` with no arguments: (group, name, description).
const COMMANDS: &[(&str, &str, &str)] = &[
    ("System", "update", "updates everything: image, Flatpaks, boxes"),
    ("System", "rewind", "goes back in time: system, files and apps"),
    ("System", "doctor", "health, integrity and performance check"),
    ("System", "boot-report", "boot health check and rolled-back updates"),
    ("System", "snap", "history of the home folder (snapshots)"),
    ("System", "layer", "Fedora packages on top of the image"),
    ("System", "perf", "performance profiles and sched_ext"),
    ("System", "harden", "security: status, strict, balanced, usb, malloc"),
    ("System", "privacy", "one-touch privacy mode"),
    ("System", "guest", "disposable guest session"),
    ("System", "lang", "system language"),
    ("System", "setup", "machine chores: Secure Boot key, TPM unlock, firmware, clean"),
    ("Install", "install", "everything optional, in one catalog"),
    ("Install", "browser", "browsers: Flatpak, native or box"),
    ("Install", "agents", "AI agents: Claude Code, Codex, Qwen Code, opencode…"),
    ("Install", "ai", "local AI (llama.cpp): models and server"),
    ("Install", "agent", "chat with the local AI agent"),
    ("Install", "gaming", "native games (like on Bazzite) or Flatpak"),
    ("Install", "vm", "virtual machines (native virt-manager)"),
    ("Install", "windows", "Windows 11 in a container (dockur)"),
    ("Install", "webapp", "turns a website into an app"),
    ("Install", "try", "tries a package in a disposable box"),
    ("Development", "devbox", "development boxes (Fedora, Ubuntu, openSUSE, Arch)"),
    ("Development", "db", "development databases"),
    ("Development", "workspace", "declarative per-project environments"),
    ("Development", "isolate", "runs a program in a disposable jail"),
    ("Desktop", "theme", "terminal theme (follows COSMIC)"),
    ("Desktop", "panel", "bar style"),
    ("Desktop", "font", "monospace font"),
    ("Desktop", "tiling", "automatic tiling: on, off or toggle"),
    ("Desktop", "keys", "keyboard shortcuts"),
    ("Desktop", "capture", "screenshot, OCR and screen recording"),
    ("Desktop", "remind", "reminder after a while"),
    ("Desktop", "menu", "command palette"),
];

/// The four commands of the system file, with their one-line descriptions.
const FILE_COMMANDS: &[(&str, &str)] = &[
    ("init", "create the file from the current state"),
    ("export", "print or save the current state"),
    ("diff", "show what is missing"),
    ("apply", "apply what is missing"),
];

/// Ends with a message on the error output and exit code 1.
fn die(msg: &str) -> ! {
    let _ = io::stdout().flush();
    eprintln!("{msg}");
    exit(1)
}

// ── Small built-in commands ─────────────────────────────────────────────────

fn cmd_tiling(args: &[String]) {
    let action = args.first().map(String::as_str).unwrap_or("toggle");
    let new = match action {
        "status" => state::tiling(),
        "on" => true,
        "off" => false,
        "toggle" => !state::tiling(),
        _ => die("usage: tamagi tiling [on|off|toggle|status]"),
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
        println!("  tamagi apply {name:<7} {}", t(desc));
    }
    println!("\n{}", t("A profile only adds what is missing; nothing is removed."));
}

fn cmd_list() {
    let width = COMMANDS.iter().map(|(_, name, _)| name.len()).max().unwrap_or(0);
    println!("tamagi — {}\n", t("one command for the whole system"));
    let mut group = "";
    for (g, name, desc) in COMMANDS {
        if *g != group {
            group = g;
            println!("{}", t(g));
        }
        println!("  tamagi {name:<width$}  {}", t(desc));
    }
    println!("\n{}: tamagi profiles  ·  tamagi apply ai | dev | gamer | office", t("Ready-made profiles"));
    println!("\n{}", t("The system in one file"));
    for (name, desc) in FILE_COMMANDS {
        println!("  tamagi {name:<width$}  {}", t(desc));
    }
}

// ── The system file: where it comes from ────────────────────────────────────

/// The text of an I/O error without Rust's " (os error N)" tail.
fn strerror(e: &io::Error) -> String {
    let text = e.to_string();
    match text.rfind(" (os error ") {
        Some(i) => text[..i].to_string(),
        None => text,
    }
}

/// A TOML mistake on one line: what is wrong and where.
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

/// "scheme://…" with a scheme of lowercase letters.
fn has_scheme(source: &str) -> bool {
    source.split_once("://").is_some_and(|(scheme, _)| !scheme.is_empty() && scheme.chars().all(|c| c.is_ascii_lowercase()))
}

/// A file someone shared: fetched with curl (https only, also after
/// redirects), at most 1 MB.
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

/// A file, the name of a profile or an https address -> (name to show, text
/// already fetched if it came from the network).
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

/// Reads and checks the wanted state; ends the program on a bad file.
fn load(path: &str, fetched: Option<String>) -> toml::Table {
    let text = match fetched {
        Some(text) => text,
        None => match fs::read(path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(e) => die(&format!("{}: {} {path}: {}. {}: tamagi init", t("error"), t("could not read"), strerror(&e), t("Create one with"))),
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

// ── Command line of init / export / diff / apply ────────────────────────────

const USAGE: &str = "usage: tamagi [-h] {init,export,diff,apply} ...";

fn usage_of(cmd: &str) -> String {
    match cmd {
        "init" => "usage: tamagi init [-h]".into(),
        "export" => "usage: tamagi export [-h] [file]".into(),
        "diff" => "usage: tamagi diff [-h] [file]".into(),
        _ => "usage: tamagi apply [-h] [-y] [file]".into(),
    }
}

/// A command-line mistake: the usage line, the reason, exit code 2.
fn usage_error(usage: &str, msg: &str) -> ! {
    eprintln!("{usage}\ntamagi: error: {msg}");
    exit(2)
}

fn main_help() -> ! {
    println!("{USAGE}\n\n{}\n\npositional arguments:\n  {{init,export,diff,apply}}", t("Your system described in a single file."));
    for (name, desc) in FILE_COMMANDS {
        println!("    {name:<20}{}", t(desc));
    }
    println!("\noptions:\n  -h, --help            show this help message and exit");
    exit(0)
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

/// The arguments after init/export/diff/apply: an optional file and, for
/// apply, -y/--yes.
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

// ── init / export / diff / apply ────────────────────────────────────────────

fn save_state(cmd: &str, file: Option<String>) {
    let text = state::to_toml(&state::current_state());
    let target = if cmd == "init" { Some(state::default_file()) } else { file.map(PathBuf::from) };
    let Some(target) = target else {
        println!("{text}");
        return;
    };
    if cmd == "init" && target.exists() {
        die(&format!("{} {}: tamagi diff", target.display(), t("already exists. To see differences")));
    }
    let parent = target.parent().filter(|p| !p.as_os_str().is_empty());
    if let Err(e) = parent.map(fs::create_dir_all).unwrap_or(Ok(())).and_then(|_| fs::write(&target, text)) {
        die(&format!("{}: {}: {}", t("error"), target.display(), strerror(&e)));
    }
    println!("{} {}. {}: tamagi apply", t("Saved to"), target.display(), t("Edit and run"));
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
        die(&format!("{}: {}\n{}", t("Finished with failures in"), failed.join("; "), t("Run `tamagi diff` to see what is still missing.")));
    }
    println!("{}", t("Done: the machine now matches the file."));
}

fn diff_or_apply(cmd: &str, args: FileArgs) {
    let source = args.file.unwrap_or_else(|| state::default_file().to_string_lossy().into_owned());
    let (path, fetched) = resolve(&source);
    // A file from the network is always shown before it changes the machine.
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

/// `[a-z][a-z0-9-]*`: the shape of a command name.
fn is_command_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase()) && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    let first = argv.first().map(String::as_str).unwrap_or("");
    if argv.is_empty() {
        return cmd_list();
    }
    match first {
        "help" | "list" | "commands" => return cmd_list(),
        "profiles" => return cmd_profiles(),
        "tiling" => return cmd_tiling(&argv[1..]),
        _ => {}
    }
    let file_command = FILE_COMMANDS.iter().any(|(name, _)| *name == first);
    // Every other command is the program tamagi-<command>.
    if is_command_name(first) && !file_command {
        if let Some(prog) = state::which(&format!("tamagi-{first}")) {
            let err = Command::new(&prog).args(&argv[1..]).exec();
            die(&format!("tamagi: {}: {}", prog.display(), strerror(&err)));
        }
        die(&format!("tamagi: {}: {first}  ({}: tamagi)", t("unknown command"), t("see")));
    }
    if !file_command {
        if first == "-h" || (first.len() > 2 && "--help".starts_with(first)) {
            main_help();
        }
        if first.starts_with('-') {
            usage_error(USAGE, "the following arguments are required: cmd");
        }
        usage_error(USAGE, &format!("argument cmd: invalid choice: '{first}' (choose from 'init', 'export', 'diff', 'apply')"));
    }
    let args = file_args(first, &argv[1..]);
    match first {
        "init" | "export" => save_state(first, args.file),
        _ => diff_or_apply(first, args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_names() {
        assert!(is_command_name("doctor"));
        assert!(is_command_name("boot-report"));
        assert!(!is_command_name("-h"));
        assert!(!is_command_name("Doctor"));
        assert!(!is_command_name("a_b"));
        assert!(!is_command_name(""));
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

    #[test]
    fn every_listed_command_is_a_valid_name() {
        for (_, name, _) in COMMANDS {
            assert!(is_command_name(name), "{name}");
        }
    }
}
