//! tamagi-why — instant provenance inspector for an immutable atomic OS.
//!
//! Answers "where did this file, command, port, or systemd unit come from?"
//! across composefs base image, systemd-sysext layers, Flatpak (--user),
//! Podman Quadlets (MCP/DB), distrobox launchers, active leases, and CoW shadows.

mod i18n;

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

use i18n::t;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    kind: String,
    origin: String,
    detail: String,
}

fn home_dir() -> PathBuf {
    env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn run_stdout(prog: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(prog).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn which(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH").unwrap_or_else(|| "/usr/local/bin:/usr/bin:/bin".into());
    for dir in env::split_paths(&path) {
        let p = dir.join(name);
        if fs::metadata(&p).map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0).unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

fn edition_label() -> String {
    let ed = fs::read_to_string("/usr/share/tamagi/edition").unwrap_or_else(|_| "dxai".into());
    match ed.trim() {
        "skl" => "Tamagi-SKL".into(),
        "skl-os" => "SKL".into(),
        _ => "Tamagi-DxAI".into(),
    }
}

fn composefs_tag() -> &'static str {
    if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
        if cmdline.split_whitespace().any(|t| t.starts_with("composefs.digest=") || t.starts_with("composefs=")) {
            return " [composefs + fs-verity]";
        }
    }
    ""
}

fn parse_port(target: &str) -> Option<u16> {
    let s = target.strip_prefix(':').or_else(|| target.strip_prefix("port:")).unwrap_or(target);
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        return s.parse::<u16>().ok().filter(|p| *p > 0);
    }
    None
}

fn inspect_port(port: u16) -> Vec<Finding> {
    let mut out = Vec::new();
    let pstr = port.to_string();

    // 1. Active leases
    let lease_dir = PathBuf::from(env::var("TAMAGI_LEASE_DIR").unwrap_or_else(|_| "/var/lib/tamagi/leases".into()));
    if let Ok(rd) = fs::read_dir(&lease_dir) {
        for ent in rd.flatten() {
            let name = ent.file_name().to_string_lossy().into_owned();
            if name.contains(&format!("port-{pstr}")) || name.contains(&format!("port_{pstr}")) {
                let text = fs::read_to_string(ent.path()).unwrap_or_default();
                let until = text.lines().find(|l| l.starts_with("UNTIL=") || l.starts_with("EXPIRES=")).unwrap_or("active").to_string();
                out.push(Finding { kind: "port".into(), origin: "tamagi lease".into(), detail: format!("TCP/UDP {port} ({until})") });
            }
        }
    }

    // 2. Local AI llama.cpp default port
    if port == 9931 {
        out.push(Finding { kind: "port".into(), origin: "tamagi ai (llama.cpp)".into(), detail: "127.0.0.1:9931 local OpenAI/Anthropic inference server".into() });
    }
    if port == 9090 {
        out.push(Finding { kind: "port".into(), origin: "cockpit.socket".into(), detail: "Cockpit Web Console (socket-activated)".into() });
    }

    // 3. User Podman Quadlets (DB / MCP / Windows)
    let quad_dir = env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("containers/systemd");
    if let Ok(rd) = fs::read_dir(&quad_dir) {
        for ent in rd.flatten() {
            if let Ok(body) = fs::read_to_string(ent.path()) {
                if body.contains(&format!(":{pstr}:")) || body.contains(&format!("={pstr}:")) {
                    out.push(Finding {
                        kind: "port".into(),
                        origin: format!("Podman Quadlet ({})", ent.file_name().to_string_lossy()),
                        detail: ent.path().display().to_string(),
                    });
                }
            }
        }
    }

    // 4. Live listening sockets via `ss`
    if let Some(ss) = run_stdout("ss", &["-tulpn"]) {
        for line in ss.lines() {
            if line.contains(&format!(":{pstr} ")) || line.ends_with(&format!(":{pstr}")) {
                out.push(Finding { kind: "socket".into(), origin: "live listener".into(), detail: line.split_whitespace().collect::<Vec<_>>().join(" ") });
            }
        }
    }

    if out.is_empty() {
        out.push(Finding { kind: "port".into(), origin: "unclaimed".into(), detail: format!("Port {port} has no active listener, Quadlet, or lease") });
    }
    out
}

fn inspect_unit(unit: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    let quad_dir = env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("containers/systemd");
    let stem = unit.trim_end_matches(".service");
    for ext in ["container", "image"] {
        let q = quad_dir.join(format!("{stem}.{ext}"));
        if q.is_file() {
            out.push(Finding { kind: "unit".into(), origin: "Podman Quadlet".into(), detail: q.display().to_string() });
        }
    }
    for base in ["/etc/systemd/system", "/usr/lib/systemd/system", "/usr/lib/systemd/user"] {
        let p = Path::new(base).join(unit);
        if p.exists() {
            let src = if base.starts_with("/etc") { "machine config (/etc)".into() } else { format!("base image ({})", edition_label()) };
            let rpm = run_stdout("rpm", &["-qf", "--qf", "%{NAME}-%{VERSION}", &p.to_string_lossy()]);
            let detail = match rpm {
                Some(pkg) => format!("{} (RPM: {pkg})", p.display()),
                None => p.display().to_string(),
            };
            out.push(Finding { kind: "unit".into(), origin: src, detail });
        }
    }
    if out.is_empty() {
        out.push(Finding { kind: "unit".into(), origin: "not found".into(), detail: unit.to_string() });
    }
    out
}

fn find_layer_for_pkg(pkg_name: &str) -> Option<String> {
    let meta_dir = PathBuf::from(env::var("TAMAGI_LAYER_META").unwrap_or_else(|_| "/var/lib/tamagi/layers".into()));
    let rd = fs::read_dir(&meta_dir).ok()?;
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().into_owned();
        if let Some(layer) = name.strip_suffix(".packages") {
            if let Ok(text) = fs::read_to_string(ent.path()) {
                if text.lines().any(|l| l.split_whitespace().next() == Some(pkg_name)) {
                    return Some(layer.to_string());
                }
            }
        }
    }
    None
}

fn inspect_path(raw: &str, path: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    let real = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let s = real.to_string_lossy().into_owned();
    let home = home_dir().to_string_lossy().into_owned();

    if s.contains("/tamagi/shadows/") {
        out.push(Finding { kind: "path".into(), origin: "CoW shadow workspace".into(), detail: s });
        return out;
    }
    if s.contains("/flatpak/") {
        out.push(Finding { kind: "path".into(), origin: "Flatpak (--user)".into(), detail: s });
        return out;
    }
    if s.starts_with(&format!("{home}/.local/bin/")) {
        let content = fs::read_to_string(&real).unwrap_or_default();
        if content.contains("tamagi-agent") {
            out.push(Finding { kind: "command".into(), origin: "AI agent launcher (tamagi ai)".into(), detail: s });
        } else if content.contains("distrobox enter") {
            out.push(Finding { kind: "command".into(), origin: "devbox exported binary (distrobox)".into(), detail: s });
        } else {
            out.push(Finding { kind: "command".into(), origin: "user binary (~/.local/bin)".into(), detail: s });
        }
        return out;
    }
    if s.starts_with("/var/lib/extensions/") {
        out.push(Finding { kind: "layer".into(), origin: "systemd-sysext image".into(), detail: s });
        return out;
    }
    if s.starts_with("/etc/") {
        out.push(Finding { kind: "config".into(), origin: "machine /etc overlay".into(), detail: s });
        return out;
    }
    if let Some(pkg) = run_stdout("rpm", &["-qf", "--qf", "%{NAME}", &s]) {
        let cfs = composefs_tag();
        if let Some(layer) = find_layer_for_pkg(&pkg) {
            out.push(Finding { kind: "file".into(), origin: format!("sysext layer '{layer}'"), detail: format!("{s} (RPM: {pkg})") });
        } else {
            out.push(Finding { kind: "file".into(), origin: format!("base image ({}){cfs}", edition_label()), detail: format!("{s} (RPM: {pkg})") });
        }
        return out;
    }
    if s.starts_with("/usr/") {
        let cfs = composefs_tag();
        // File in /usr not in host rpmdb: either Tamagi built-in compiled binary or active sysext
        if raw.starts_with("tamagi") || s.contains("/tamagi/") || s.ends_with("/llama-server") || s.ends_with("/btm") || s.ends_with("/starship") || s.ends_with("/cfsctl") {
            out.push(Finding { kind: "file".into(), origin: format!("{} built-in{cfs}", edition_label()), detail: s });
        } else {
            out.push(Finding { kind: "file".into(), origin: format!("active sysext or image overlay (/usr){cfs}"), detail: s });
        }
        return out;
    }
    out.push(Finding { kind: "path".into(), origin: "local filesystem".into(), detail: s });
    out
}

fn inspect_target(target: &str) -> Vec<Finding> {
    if let Some(port) = parse_port(target) {
        return inspect_port(port);
    }
    if target.ends_with(".service") || target.ends_with(".socket") || target.ends_with(".timer") || target.ends_with(".path") {
        return inspect_unit(target);
    }
    let p = Path::new(target);
    if p.exists() {
        return inspect_path(target, p);
    }
    if let Some(bin) = which(target) {
        return inspect_path(target, &bin);
    }
    if let Some(layer) = find_layer_for_pkg(target) {
        return vec![Finding { kind: "package".into(), origin: format!("sysext layer '{layer}'"), detail: format!("RPM {target}") }];
    }
    if let Some(info) = run_stdout("flatpak", &["info", "--user", target]) {
        let ver = info.lines().find(|l| l.trim_start().starts_with("Version:")).unwrap_or("installed").trim();
        return vec![Finding { kind: "flatpak".into(), origin: "Flatpak (--user)".into(), detail: format!("{target} ({ver})") }];
    }
    vec![Finding { kind: "unknown".into(), origin: "not found".into(), detail: format!("No path, command, port, unit, layer, or Flatpak matched '{target}'") }]
}

fn json_escape(s: &str) -> String {
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

fn usage() {
    println!("tamagi why — {}", t("explain where any file, command, port, or service came from"));
    println!();
    println!("  tamagi why <path|command|port|unit> [--json]");
    println!("  Examples:");
    println!("    tamagi why /usr/bin/llama-server");
    println!("    tamagi why 9931");
    println!("    tamagi why cockpit.socket");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut json = false;
    let mut target: Option<String> = None;
    for a in &args {
        match a.as_str() {
            "-h" | "--help" | "help" => {
                usage();
                return;
            }
            "--json" => json = true,
            _ if target.is_none() => target = Some(a.clone()),
            _ => {}
        }
    }
    let Some(target) = target else {
        usage();
        exit(2);
    };
    let findings = inspect_target(&target);
    if json {
        print!("{{\"target\":{},\"findings\":[", json_escape(&target));
        for (i, f) in findings.iter().enumerate() {
            if i > 0 {
                print!(",");
            }
            print!(
                "{{\"kind\":{},\"origin\":{},\"detail\":{}}}",
                json_escape(&f.kind),
                json_escape(&f.origin),
                json_escape(&f.detail)
            );
        }
        println!("]}}");
        return;
    }
    println!("{}: {target}", t("Target"));
    for f in &findings {
        println!("  [{:<7}] {:<30} {}", f.kind, f.origin, f.detail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ports() {
        assert_eq!(parse_port("9931"), Some(9931));
        assert_eq!(parse_port(":8080"), Some(8080));
        assert_eq!(parse_port("port:5432"), Some(5432));
        assert_eq!(parse_port("0"), None);
        assert_eq!(parse_port("bash"), None);
    }

    #[test]
    fn explains_local_ai_port() {
        let f = inspect_port(9931);
        assert!(f.iter().any(|x| x.origin.contains("llama.cpp")));
    }

    #[test]
    fn escapes_json_strings() {
        assert_eq!(json_escape("a\"b\\c\nd"), "\"a\\\"b\\\\c\\nd\"");
    }
}
