//! agent-jail — builds the bubblewrap jail of `tamagi agents run` and enters it.
//!
//! It runs WHERE THE JAIL IS BUILT: on the host, or inside the development box
//! (called there as /run/host/usr/libexec/tamagi/agent-jail). The two places
//! look different — in a box, /home is a real folder with the host's home
//! mounted in it and the whole host is visible under /run/host — so the list of
//! what to hide has to be worked out on the spot, never on the other side.
//!
//!   agent-jail [--offline] [--print] --root <project> --cwd <dir>
//!              [--shadow <cow-copy>] [--egress-sock <unix-sock>] [--egress-port <port>]
//!              [--state "<rel> <rel>…"] [--rw <dir>]… [--setenv NAME=value]…
//!              -- <command> [args]
//!
//! Inside the jail the command sees the project folder (read-write, or backed
//! transparently by a Btrfs Copy-on-Write `--shadow` directory so the real
//! working tree is never touched), its own state folders (--state, relative to
//! the home), the tools installed in the home (read-only) and a read-only
//! system. When `--egress-sock` is passed, the network namespace is airgapped
//! (`--unshare-net`) and traffic is bridged exclusively through the outer
//! domain-allowlist proxy (`egress-guard`).

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

/// Everything personal or outside the project: emptied in the jail.
const HIDDEN: &[&str] = &[
    "/var/home", "/home", "/root", "/var/roothome", "/run/media", "/media", "/mnt", "/var/mnt",
    "/srv", "/var/srv", "/run/host", "/run/libvirt",
];

/// Tools installed in the home, visible read-only.
const HOME_TOOLS: &[&str] = &[
    ".local/bin", ".local/share/claude", ".local/share/tamagi/agents", ".local/share/tamagi/clio",
    ".local/share/tamagi/mcp", ".config/mise", ".gitconfig",
];

const INNER_GUARD_PATH: &str = "/tmp/.tamagi-egress-guard";

#[derive(Debug, PartialEq)]
struct Options {
    offline: bool,
    print: bool,
    root: String,
    cwd: String,
    shadow: Option<String>,
    egress_sock: Option<String>,
    egress_port: u16,
    state: Vec<String>,
    rw: Vec<String>,
    envs: Vec<(String, String)>,
    command: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            offline: false,
            print: false,
            root: String::new(),
            cwd: String::new(),
            shadow: None,
            egress_sock: None,
            egress_port: 18080,
            state: Vec::new(),
            rw: Vec::new(),
            envs: Vec::new(),
            command: Vec::new(),
        }
    }
}

/// What the jail is built from, apart from the options: where we are.
struct Place {
    home: PathBuf,
    uid: u32,
    /// The development box this runs in (CONTAINER_ID), if any.
    container: Option<String>,
}

fn fail(msg: &str) -> ! {
    eprintln!("agent-jail: {msg}");
    exit(2)
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut o = Options::default();
    let mut i = 0;
    let value = |i: &mut usize, name: &str| -> Result<String, String> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| format!("{name} needs a value"))
    };
    while i < args.len() {
        match args[i].as_str() {
            "--offline" => o.offline = true,
            "--print" => o.print = true,
            "--root" => o.root = value(&mut i, "--root")?,
            "--cwd" => o.cwd = value(&mut i, "--cwd")?,
            "--shadow" => o.shadow = Some(value(&mut i, "--shadow")?),
            "--egress-sock" => o.egress_sock = Some(value(&mut i, "--egress-sock")?),
            "--egress-port" => {
                let p = value(&mut i, "--egress-port")?;
                o.egress_port = p.parse::<u16>().ok().filter(|&n| n >= 1024).ok_or_else(|| format!("invalid --egress-port: {p}"))?;
            }
            "--state" => o.state = value(&mut i, "--state")?.split_whitespace().map(String::from).collect(),
            "--rw" => o.rw.push(value(&mut i, "--rw")?),
            "--setenv" => {
                let kv = value(&mut i, "--setenv")?;
                let (k, v) = kv.split_once('=').unwrap_or((kv.as_str(), kv.as_str()));
                o.envs.push((k.to_string(), v.to_string()));
            }
            "--" => {
                o.command = args[i + 1..].to_vec();
                break;
            }
            other => return Err(format!("unknown option: {other}")),
        }
        i += 1;
    }
    if o.command.is_empty() {
        return Err("no command".into());
    }
    Ok(o)
}

/// A state folder is relative to the home and never climbs out of it.
fn safe_relative(rel: &str) -> bool {
    !rel.is_empty() && !rel.starts_with('/') && Path::new(rel).components().all(|c| matches!(c, std::path::Component::Normal(_)))
}

fn is_real_dir(p: &Path) -> bool {
    fs::symlink_metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn push(args: &mut Vec<OsString>, items: &[&dyn AsRef<std::ffi::OsStr>]) {
    for i in items {
        args.push(i.as_ref().to_os_string());
    }
}

fn find_egress_guard() -> Option<PathBuf> {
    for cand in [
        env::var("TAMAGI_EGRESS_GUARD_BIN").ok().map(PathBuf::from),
        Some(PathBuf::from("/usr/libexec/tamagi/egress-guard")),
        Some(PathBuf::from("/run/host/usr/libexec/tamagi/egress-guard")),
    ]
    .into_iter()
    .flatten()
    {
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// The bubblewrap arguments. `root`, `cwd`, and optional `shadow` are already canonical.
fn build(o: &Options, place: &Place, root: &Path, cwd: &Path, shadow: Option<&Path>, guard_bin: Option<&Path>) -> Vec<OsString> {
    let home = &place.home;
    let mut a: Vec<OsString> = Vec::new();
    for s in ["--die-with-parent", "--unshare-pid", "--unshare-ipc", "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp"] {
        a.push(s.into());
    }
    let session = format!("/run/user/{}", place.uid);
    for d in HIDDEN.iter().copied().chain(std::iter::once(session.as_str())) {
        if is_real_dir(Path::new(d)) {
            push(&mut a, &[&"--tmpfs", &d]);
        }
    }
    push(&mut a, &[&"--tmpfs", home]);

    // Project bind: either the live directory or an isolated CoW shadow mounted
    // at the exact same canonical path `root` so incremental build caches work.
    let backing = shadow.unwrap_or(root);
    push(&mut a, &[&"--bind", &backing, &root]);
    for rel in [".git/hooks", ".git/config"] {
        let src = backing.join(rel);
        let dst = root.join(rel);
        if src.exists() {
            push(&mut a, &[&"--ro-bind", &src, &dst]);
        }
    }

    for rel in HOME_TOOLS {
        let p = home.join(rel);
        if p.exists() {
            push(&mut a, &[&"--ro-bind", &p, &p]);
        }
    }
    let mut path = home.join(".local/bin").into_os_string();
    if let Some(boxname) = place.container.as_deref().filter(|b| !b.is_empty()) {
        let data = home.join(format!(".local/share/mise-{boxname}"));
        if data.is_dir() {
            push(&mut a, &[&"--ro-bind", &data, &data]);
            push(&mut a, &[&"--setenv", &"MISE_DATA_DIR", &data]);
            push(&mut a, &[&"--setenv", &"MISE_CACHE_DIR", &home.join(format!(".cache/mise-{boxname}"))]);
            push(&mut a, &[&"--setenv", &"MISE_STATE_DIR", &home.join(format!(".local/state/mise-{boxname}"))]);
            let mut p = data.join("shims").into_os_string();
            p.push(":");
            p.push(&path);
            path = p;
        }
    } else {
        let data = home.join(".local/share/mise");
        if data.is_dir() {
            push(&mut a, &[&"--ro-bind", &data, &data]);
            let cfg = home.join(".config/mise");
            if cfg.is_dir() {
                push(&mut a, &[&"--ro-bind", &cfg, &cfg]);
            }
            let mut p = data.join("shims").into_os_string();
            p.push(":");
            p.push(&path);
            path = p;
        }
    }

    for rel in &o.state {
        let p = home.join(rel);
        push(&mut a, &[&"--bind", &p, &p]);
    }
    for d in &o.rw {
        push(&mut a, &[&"--bind", d, d]);
    }

    if o.offline || o.egress_sock.is_some() {
        a.push("--unshare-net".into());
    }
    if let Some(sock) = &o.egress_sock {
        if !o.offline {
            push(&mut a, &[&"--bind", sock, sock]);
            if let Some(gb) = guard_bin {
                push(&mut a, &[&"--ro-bind", &gb, &INNER_GUARD_PATH]);
            }
            let proxy = format!("http://127.0.0.1:{}", o.egress_port);
            for k in ["HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"] {
                push(&mut a, &[&"--setenv", &k, &proxy]);
            }
            push(&mut a, &[&"--setenv", &"NO_PROXY", &"127.0.0.1,localhost"]);
            push(&mut a, &[&"--setenv", &"no_proxy", &"127.0.0.1,localhost"]);
            push(&mut a, &[&"--setenv", &"TAMAGI_EGRESS_GUARD", &"1"]);
        }
    }
    if shadow.is_some() {
        push(&mut a, &[&"--setenv", &"TAMAGI_AGENT_SHADOW", &"1"]);
    }

    path.push(":/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin");
    push(&mut a, &[&"--setenv", &"HOME", home]);
    push(&mut a, &[&"--setenv", &"TAMAGI_AGENT_JAIL", &"1"]);
    push(&mut a, &[&"--setenv", &"PATH", &path]);
    push(&mut a, &[&"--setenv", &"XDG_CONFIG_HOME", &home.join(".config")]);
    push(&mut a, &[&"--setenv", &"XDG_DATA_HOME", &home.join(".local/share")]);
    push(&mut a, &[&"--setenv", &"XDG_CACHE_HOME", &home.join(".cache")]);
    push(&mut a, &[&"--setenv", &"XDG_STATE_HOME", &home.join(".local/state")]);
    for v in ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS", "WAYLAND_DISPLAY", "DISPLAY", "SSH_AUTH_SOCK"] {
        push(&mut a, &[&"--unsetenv", &v]);
    }
    push(&mut a, &[&"--chdir", &cwd]);
    for (k, v) in &o.envs {
        push(&mut a, &[&"--setenv", k, v]);
    }
    a
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    let o = parse(&argv).unwrap_or_else(|e| fail(&e));
    if o.root.is_empty() || !Path::new(&o.root).is_dir() {
        fail("--root <project> is required");
    }
    let home = env::var_os("HOME").map(PathBuf::from).filter(|h| h.is_absolute()).unwrap_or_else(|| fail("HOME is not set"));
    let home = fs::canonicalize(&home).unwrap_or(home);
    let root = fs::canonicalize(&o.root).unwrap_or_else(|_| fail("--root <project> is required"));
    if root == home || root == Path::new("/") {
        fail("the project cannot be the whole home folder");
    }
    let shadow = o.shadow.as_ref().map(|s| {
        let p = fs::canonicalize(s).unwrap_or_else(|_| fail("--shadow directory does not exist"));
        if !p.is_dir() || p == home || p == Path::new("/") || p == root {
            fail("--shadow must be a separate project copy directory");
        }
        p
    });
    let guard_bin = if o.egress_sock.is_some() && !o.offline {
        let gb = find_egress_guard();
        if gb.is_none() && !o.print {
            fail("egress-guard helper is missing (/usr/libexec/tamagi/egress-guard)");
        }
        gb
    } else {
        None
    };

    let mut cwd = fs::canonicalize(if o.cwd.is_empty() { &o.root } else { &o.cwd }).unwrap_or_else(|_| root.clone());
    if !cwd.starts_with(&root) {
        cwd = root.clone();
    }
    if let Some(bad) = o.state.iter().find(|rel| !safe_relative(rel)) {
        fail(&format!("--state takes folders relative to the home: {bad}"));
    }
    let place = Place {
        uid: fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0),
        container: env::var("CONTAINER_ID").ok(),
        home,
    };
    for rel in &o.state {
        let _ = fs::create_dir_all(place.home.join(rel));
    }
    for d in &o.rw {
        let _ = fs::create_dir_all(d);
    }
    let args = build(&o, &place, &root, &cwd, shadow.as_deref(), guard_bin.as_deref());
    if o.print {
        for a in &args {
            println!("{}", a.to_string_lossy());
        }
        return;
    }
    let mut final_cmd: Vec<String> = Vec::new();
    if let Some(sock) = &o.egress_sock {
        if !o.offline {
            final_cmd.extend([
                INNER_GUARD_PATH.to_string(),
                "--bridge".to_string(),
                sock.clone(),
                o.egress_port.to_string(),
                "--".to_string(),
            ]);
        }
    }
    final_cmd.extend(o.command);

    let err = Command::new("bwrap").args(&args).arg("--").args(&final_cmd).exec();
    if err.kind() == std::io::ErrorKind::NotFound {
        eprintln!("agent-jail: bubblewrap (bwrap) is missing here");
        exit(127);
    }
    eprintln!("agent-jail: bwrap: {err}");
    exit(126)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    fn place(home: &str) -> Place {
        Place { home: PathBuf::from(home), uid: 1000, container: None }
    }
    fn has_pair(args: &[OsString], a: &str, b: &str) -> bool {
        args.windows(2).any(|w| w[0] == a && w[1] == b)
    }
    fn has_triple(args: &[OsString], a: &str, b: &str, c: &str) -> bool {
        args.windows(3).any(|w| w[0] == a && w[1] == b && w[2] == c)
    }

    #[test]
    fn parses_options_and_command() {
        let o = parse(&strs(&[
            "--offline", "--root", "/p", "--cwd", "/p/x", "--shadow", "/s", "--egress-sock", "/tmp/e.sock",
            "--egress-port", "19000", "--state", ".a .b", "--rw", "/w", "--setenv", "K=v=w", "--", "cmd", "--print",
        ]))
        .unwrap();
        assert!(o.offline && !o.print);
        assert_eq!(o.root, "/p");
        assert_eq!(o.shadow.as_deref(), Some("/s"));
        assert_eq!(o.egress_sock.as_deref(), Some("/tmp/e.sock"));
        assert_eq!(o.egress_port, 19000);
        assert_eq!(o.state, strs(&[".a", ".b"]));
        assert_eq!(o.envs, vec![("K".to_string(), "v=w".to_string())]);
        assert_eq!(o.command, strs(&["cmd", "--print"]));
    }

    #[test]
    fn rejects_unknown_option_and_missing_command() {
        assert!(parse(&strs(&["--nope", "--", "x"])).is_err());
        assert!(parse(&strs(&["--root", "/p"])).is_err());
        assert!(parse(&strs(&["--root", "/p", "--"])).is_err());
        assert!(parse(&strs(&["--root"])).is_err());
        assert!(parse(&strs(&["--root", "/p", "--egress-port", "80", "--", "x"])).is_err());
    }

    #[test]
    fn state_must_stay_inside_the_home() {
        assert!(safe_relative(".config/agent"));
        assert!(!safe_relative("/etc"));
        assert!(!safe_relative("../other"));
        assert!(!safe_relative(".config/../../x"));
        assert!(!safe_relative(""));
    }

    #[test]
    fn home_is_emptied_and_project_is_the_only_writable_bind() {
        let o = parse(&strs(&["--root", "/proj", "--", "true"])).unwrap();
        let a = build(&o, &place("/var/home/u"), Path::new("/proj"), Path::new("/proj"), None, None);
        assert!(has_pair(&a, "--tmpfs", "/var/home/u"));
        assert!(has_pair(&a, "--ro-bind", "/"));
        let binds: Vec<_> = a.windows(2).filter(|w| w[0] == "--bind").map(|w| w[1].clone()).collect();
        assert_eq!(binds, vec![OsString::from("/proj")]);
        assert!(has_pair(&a, "--chdir", "/proj"));
        assert!(!a.iter().any(|x| x == "--unshare-net"));
    }

    #[test]
    fn shadow_mounts_cow_copy_over_canonical_project_root() {
        let o = parse(&strs(&["--root", "/proj", "--shadow", "/shadows/s1", "--", "true"])).unwrap();
        let a = build(&o, &place("/var/home/u"), Path::new("/proj"), Path::new("/proj"), Some(Path::new("/shadows/s1")), None);
        assert!(has_triple(&a, "--bind", "/shadows/s1", "/proj"));
        assert!(has_pair(&a, "TAMAGI_AGENT_SHADOW", "1"));
    }

    #[test]
    fn egress_guard_airgaps_network_and_exports_proxy() {
        let o = parse(&strs(&["--root", "/proj", "--egress-sock", "/tmp/eg.sock", "--egress-port", "18088", "--", "true"])).unwrap();
        let a = build(
            &o,
            &place("/var/home/u"),
            Path::new("/proj"),
            Path::new("/proj"),
            None,
            Some(Path::new("/usr/libexec/tamagi/egress-guard")),
        );
        assert!(a.iter().any(|x| x == "--unshare-net"));
        assert!(has_triple(&a, "--bind", "/tmp/eg.sock", "/tmp/eg.sock"));
        assert!(has_triple(&a, "--ro-bind", "/usr/libexec/tamagi/egress-guard", INNER_GUARD_PATH));
        assert!(has_pair(&a, "HTTPS_PROXY", "http://127.0.0.1:18088"));
        assert!(has_pair(&a, "TAMAGI_EGRESS_GUARD", "1"));
    }

    #[test]
    fn session_and_keys_are_unset_and_offline_cuts_the_network() {
        let o = parse(&strs(&["--offline", "--root", "/proj", "--setenv", "A=1", "--", "true"])).unwrap();
        let a = build(&o, &place("/h"), Path::new("/proj"), Path::new("/proj"), None, None);
        for v in ["XDG_RUNTIME_DIR", "DBUS_SESSION_BUS_ADDRESS", "WAYLAND_DISPLAY", "DISPLAY", "SSH_AUTH_SOCK"] {
            assert!(has_pair(&a, "--unsetenv", v), "{v}");
        }
        assert!(a.iter().any(|x| x == "--unshare-net"));
        assert!(has_pair(&a, "TAMAGI_AGENT_JAIL", "1"));
        let n = a.len();
        assert_eq!(a[n - 3..].to_vec(), vec![OsString::from("--setenv"), "A".into(), "1".into()]);
    }

    #[test]
    fn state_and_rw_folders_are_bound_writable() {
        let o = parse(&strs(&["--root", "/proj", "--state", ".agent", "--rw", "/data/x", "--", "true"])).unwrap();
        let a = build(&o, &place("/h"), Path::new("/proj"), Path::new("/proj"), None, None);
        assert!(has_pair(&a, "--bind", "/h/.agent"));
        assert!(has_pair(&a, "--bind", "/data/x"));
    }
}
