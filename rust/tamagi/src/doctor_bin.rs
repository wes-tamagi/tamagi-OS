//! tamagi-doctor — autonomous health, integrity, and performance diagnostics.
//! Static Rust binary (replaces the former Go build stage).

mod i18n;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};
use std::thread;

use i18n::{detect_lang, t};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CheckItem {
    id: &'static str,
    status: &'static str,
    summary: String,
    fix: String,
}

fn user_home() -> PathBuf {
    env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg_data_home() -> PathBuf {
    env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()).map(PathBuf::from).unwrap_or_else(|| user_home().join(".local/share"))
}

fn which(name: &str) -> bool {
    let path = env::var_os("PATH").unwrap_or_else(|| "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin".into());
    env::split_paths(&path).map(|d| d.join(name)).any(|p| fs::metadata(&p).map(|m| m.is_file() && (m.permissions().mode() & 0o111) != 0).unwrap_or(false))
}

fn run_out(name: &str, args: &[&str]) -> (i32, String) {
    let mut cmd = if which("timeout") && name != "timeout" {
        let mut c = Command::new("timeout");
        c.arg("5").arg(name).args(args);
        c
    } else {
        let mut c = Command::new(name);
        c.args(args);
        c
    };
    match cmd.stdin(Stdio::null()).stderr(Stdio::null()).output() {
        Ok(o) => (o.status.code().unwrap_or(1), String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Err(_) => (127, String::new()),
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn format_json(checks: &[CheckItem], fixed: &[String]) -> String {
    let mut s = String::from("{\n  \"checks\": [\n");
    for (i, c) in checks.iter().enumerate() {
        let comma = if i + 1 < checks.len() { "," } else { "" };
        s.push_str(&format!(
            "    {{\n      \"id\": {},\n      \"status\": {},\n      \"summary\": {},\n      \"fix\": {}\n    }}{comma}\n",
            json_escape(c.id),
            json_escape(c.status),
            json_escape(&c.summary),
            json_escape(&c.fix)
        ));
    }
    s.push_str("  ],\n  \"fixed\": [");
    if !fixed.is_empty() {
        s.push('\n');
        for (i, f) in fixed.iter().enumerate() {
            let comma = if i + 1 < fixed.len() { "," } else { "" };
            s.push_str(&format!("    {}{comma}\n", json_escape(f)));
        }
        s.push_str("  ]\n}\n");
    } else {
        s.push_str("]\n}\n");
    }
    s
}

fn check_image() -> CheckItem {
    if let Ok(text) = fs::read_to_string("/var/lib/tamagi/quarantine.list") {
        let count = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).count();
        if count > 0 {
            return CheckItem { id: "image", status: "WARN", summary: format!("{}: {count}", t("Quarantined digest")), fix: "tamagi-update quarantine".into() };
        }
    }
    let pending = fs::read_dir("/var/lib/tamagi/boot-failures")
        .map(|rd| rd.flatten().filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("txt")).count())
        .unwrap_or(0);
    if pending > 0 {
        return CheckItem { id: "image", status: "WARN", summary: format!("{}: {pending}", t("Pending boot report")), fix: "tamagi-boot-report show".into() };
    }
    let mut ver = String::from("Tamagi");
    if let Ok(text) = fs::read_to_string("/usr/lib/os-release") {
        for ln in text.lines() {
            if let Some(rest) = ln.strip_prefix("PRETTY_NAME=") {
                ver = rest.trim_matches('"').to_string();
                break;
            }
        }
    }
    CheckItem { id: "image", status: "OK", summary: ver, fix: String::new() }
}

fn check_kernel() -> CheckItem {
    let kver = fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let flav = fs::read_to_string("/usr/share/tamagi/kernel-flavor").map(|s| s.trim().to_string()).unwrap_or_default();
    let has_scx = Path::new("/sys/kernel/sched_ext").is_dir();
    let (rc, sb) = run_out("mokutil", &["--sb-state"]);
    if rc == 0 && sb.to_lowercase().contains("enabled") && flav == "ogc" {
        let cert = "/etc/pki/akmods/certs/akmods-ublue.der";
        if Path::new(cert).is_file() {
            let (rc2, _) = run_out("mokutil", &["--test-key", cert]);
            if rc2 == 0 {
                return CheckItem {
                    id: "kernel",
                    status: "WARN",
                    summary: format!("{kver} ({flav}): {}", t("Universal Blue Secure Boot key missing")),
                    fix: "tamagi sys setup secure-boot-key".into(),
                };
            }
        }
    }
    let mut detail = kver;
    if !flav.is_empty() {
        detail.push_str(&format!(" ({flav})"));
    }
    if has_scx {
        detail.push_str(" · sched_ext");
    }
    CheckItem { id: "kernel", status: "OK", summary: detail, fix: String::new() }
}

fn parse_df_bytes(output: &str) -> Option<(u64, u64)> {
    let line = output.lines().nth(1)?;
    let cols: Vec<&str> = line.split_whitespace().collect();
    if cols.len() < 4 {
        return None;
    }
    let total: u64 = cols[1].parse().ok()?;
    let avail: u64 = cols[3].parse().ok()?;
    (total > 0).then_some((total, avail))
}

fn check_storage() -> CheckItem {
    let home = user_home();
    let home_str = home.to_string_lossy().into_owned();
    let (rc, df_out) = run_out("df", &["-PB1", &home_str]);
    let Some((total, free_bytes)) = (rc == 0).then(|| parse_df_bytes(&df_out)).flatten() else {
        return CheckItem { id: "storage", status: "WARN", summary: t("could not measure disk space"), fix: String::new() };
    };
    let free_pct = (free_bytes as f64 * 100.0) / (total as f64);
    let free_gb = free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if free_pct < 5.0 || free_gb < 5.0 {
        return CheckItem {
            id: "storage",
            status: "FAIL",
            summary: format!("{}: {free_gb:.1} GiB ({free_pct:.0}%)", t("critical disk space")),
            fix: "flatpak uninstall --unused && podman system prune".into(),
        };
    }
    if free_pct < 12.0 || free_gb < 15.0 {
        return CheckItem {
            id: "storage",
            status: "WARN",
            summary: format!("{}: {free_gb:.1} GiB ({free_pct:.0}%)", t("low disk space")),
            fix: "flatpak uninstall --unused".into(),
        };
    }
    let self_uid = fs::metadata("/proc/self").map(|m| m.uid()).unwrap_or(0);
    for rel in [".local", ".local/state", ".local/share"] {
        let d = home.join(rel);
        if let Ok(meta) = fs::metadata(&d) {
            if meta.uid() != self_uid {
                let user = env::var("USER").ok().filter(|u| !u.is_empty()).unwrap_or_else(|| "$USER".into());
                return CheckItem {
                    id: "storage",
                    status: "FAIL",
                    summary: format!("{} {}", d.display(), t("is owned by another user (root?)")),
                    fix: format!("sudo chown {user}: {} {}", home.join(".local").display(), home.join(".local/state").display()),
                };
            }
        }
    }
    if Path::new("/etc/snapper/configs/root").is_file() {
        return CheckItem {
            id: "storage",
            status: "WARN",
            summary: format!("{free_gb:.1} GiB · {}", t("snapper configured on / (conflicts with bootc)")),
            fix: "sudo /usr/libexec/tamagi/home-snapshots prune".into(),
        };
    }
    for vm_dir in [PathBuf::from("/var/lib/libvirt/images"), xdg_data_home().join("libvirt/images")] {
        if vm_dir.is_dir() {
            let s = vm_dir.to_string_lossy().into_owned();
            let (rc, out) = run_out("lsattr", &["-d", &s]);
            if rc == 0 {
                if let Some(flags) = out.split_whitespace().next() {
                    if !flags.contains('C') {
                        return CheckItem {
                            id: "storage",
                            status: "WARN",
                            summary: format!("{free_gb:.1} GiB · {}", t("VM directory missing NoCoW (+C) attribute")),
                            fix: format!("chattr +C {s}"),
                        };
                    }
                }
            }
        }
    }
    CheckItem { id: "storage", status: "OK", summary: format!("{free_gb:.1} GiB {} ({free_pct:.0}%)", t("free")), fix: String::new() }
}

fn mem_total_kb() -> u64 {
    fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("MemTotal:")).and_then(|l| l.split_whitespace().nth(1)).and_then(|v| v.parse().ok()))
        .unwrap_or(0)
}

fn check_performance() -> CheckItem {
    let mut issues = Vec::new();
    if !Path::new("/sys/block/zram0/disksize").is_file() {
        issues.push(t("no zram0"));
    }
    let (rc, db) = run_out("sysctl", &["-n", "vm.dirty_bytes"]);
    if rc == 0 && db == "0" && mem_total_kb() >= 16 * 1024 * 1024 {
        issues.push("vm.dirty_bytes=0".into());
    }
    if !issues.is_empty() {
        return CheckItem { id: "performance", status: "WARN", summary: issues.join(", "), fix: "tamagi-perf status".into() };
    }
    let (_, mut cc) = run_out("sysctl", &["-n", "net.ipv4.tcp_congestion_control"]);
    if cc.is_empty() {
        cc = "-".into();
    }
    CheckItem { id: "performance", status: "OK", summary: format!("zram · TCP {cc}"), fix: String::new() }
}

fn check_systemd() -> CheckItem {
    let (rc1, sys_f) = run_out("systemctl", &["--failed", "--no-legend", "--plain"]);
    let (rc2, usr_f) = run_out("systemctl", &["--user", "--failed", "--no-legend", "--plain"]);
    if rc1 != 0 && rc2 != 0 {
        return CheckItem { id: "systemd", status: "OK", summary: t("systemd unavailable in this environment"), fix: String::new() };
    }
    let mut failed: Vec<String> = format!("{sys_f}\n{usr_f}")
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect();
    if !failed.is_empty() {
        failed.truncate(4);
        return CheckItem {
            id: "systemd",
            status: "WARN",
            summary: format!("{}: {}", t("failed units"), failed.join(", ")),
            fix: "systemctl --failed && systemctl --user --failed".into(),
        };
    }
    CheckItem { id: "systemd", status: "OK", summary: t("no failed units"), fix: String::new() }
}

fn check_layers() -> CheckItem {
    let (rc, out) = run_out("tamagi-layer", &["stale"]);
    let stale: Vec<&str> = if rc == 0 { out.lines().map(str::trim).filter(|s| !s.is_empty()).collect() } else { Vec::new() };
    if !stale.is_empty() {
        return CheckItem {
            id: "layers",
            status: "WARN",
            summary: format!("{}: {}", t("stale layers"), stale.join(", ")),
            fix: "tamagi-layer rebuild".into(),
        };
    }
    CheckItem { id: "layers", status: "OK", summary: t("layers up to date"), fix: String::new() }
}

fn check_containers() -> CheckItem {
    let (rc, out) = run_out("podman", &["ps", "-a", "--filter", "label=manager=distrobox", "--format", "{{.Names}}"]);
    if rc != 0 {
        return CheckItem { id: "containers", status: "OK", summary: t("podman unavailable in this environment"), fix: String::new() };
    }
    let boxes: Vec<&str> = out.lines().map(str::trim).filter(|s| !s.is_empty()).collect();
    let mut summary = format!("{} {}", boxes.len(), t("distrobox containers"));
    if !boxes.is_empty() {
        let show: Vec<&str> = boxes.into_iter().take(5).collect();
        summary.push_str(&format!(" ({})", show.join(", ")));
    }
    CheckItem { id: "containers", status: "OK", summary, fix: String::new() }
}

fn check_gpu() -> CheckItem {
    let mut names: Vec<String> = fs::read_dir("/dev/dri")
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|n| n.starts_with("renderD"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    if names.is_empty() {
        return CheckItem { id: "gpu", status: "WARN", summary: t("no DRM render node found"), fix: String::new() };
    }
    CheckItem { id: "gpu", status: "OK", summary: format!("{} GPU DRM ({})", names.len(), names.join(", ")), fix: String::new() }
}

fn is_skl() -> bool {
    fs::read_to_string("/usr/share/tamagi/edition").map(|s| s.trim() == "skl").unwrap_or(false)
}

fn check_kvm() -> CheckItem {
    if is_skl() {
        return CheckItem { id: "virtualization", status: "OK", summary: "Tamagi-SKL".into(), fix: String::new() };
    }
    match OpenOptions::new().read(true).write(true).open("/dev/kvm") {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return CheckItem { id: "virtualization", status: "WARN", summary: t("/dev/kvm missing (virtualization disabled in BIOS?)"), fix: String::new() };
        }
        Err(_) => {
            return CheckItem { id: "virtualization", status: "WARN", summary: t("no read/write permission on /dev/kvm"), fix: String::new() };
        }
        Ok(_) => {}
    }
    let summary = if which("virsh") { "/dev/kvm · libvirt · Cockpit Machines OK" } else { "/dev/kvm OK" };
    CheckItem { id: "virtualization", status: "OK", summary: summary.into(), fix: String::new() }
}

fn check_security() -> CheckItem {
    let (rc, mut enf) = run_out("getenforce", &[]);
    if rc == 0 && enf.to_lowercase() != "enforcing" {
        return CheckItem { id: "security", status: "WARN", summary: format!("SELinux: {enf}"), fix: "sudo setenforce 1".into() };
    }
    let mode = if Path::new("/etc/sysctl.d/90-tamagi-strict.conf").is_file() { t("strict") } else { t("balanced") };
    if enf.is_empty() {
        enf = "-".into();
    }
    CheckItem { id: "security", status: "OK", summary: format!("SELinux {enf} · {} {mode}", t("mode")), fix: String::new() }
}

fn check_desktop() -> CheckItem {
    let apps = xdg_data_home().join("applications");
    let mut bad = Vec::new();
    if which("desktop-file-validate") && apps.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(&apps)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().and_then(|x| x.to_str()) == Some("desktop")).collect())
            .unwrap_or_default();
        entries.sort();
        for f in entries {
            let s = f.to_string_lossy().into_owned();
            let (rc, _) = run_out("desktop-file-validate", &[&s]);
            if rc != 0 {
                if let Some(name) = f.file_name().and_then(|n| n.to_str()) {
                    bad.push(name.to_string());
                }
            }
        }
    }
    if !bad.is_empty() {
        let first = bad[0].clone();
        bad.truncate(3);
        return CheckItem {
            id: "desktop",
            status: "WARN",
            summary: format!("{}: {}", t("invalid .desktop entries"), bad.join(", ")),
            fix: format!("desktop-file-validate {}/{first}", apps.display()),
        };
    }
    if which("agy") || user_home().join(".local/bin/agy").is_file() {
        if !Path::new("/usr/share/applications/agy.desktop").is_file() && !apps.join("agy.desktop").is_file() {
            return CheckItem {
                id: "desktop",
                status: "WARN",
                summary: t("agy is installed but missing from the menu"),
                fix: "tamagi-doctor --fix".into(),
            };
        }
    }
    CheckItem { id: "desktop", status: "OK", summary: t("menu and .desktop entries are valid"), fix: String::new() }
}

fn check_ai() -> CheckItem {
    if is_skl() {
        return CheckItem { id: "ai", status: "OK", summary: "Tamagi-SKL".into(), fix: String::new() };
    }
    if !Path::new("/usr/lib64/ggml/libggml-vulkan.so").is_file() {
        return CheckItem { id: "ai", status: "WARN", summary: t("llama.cpp Vulkan backend missing"), fix: String::new() };
    }
    let (rc, mut state) = run_out("systemctl", &["--user", "is-active", "llama-server.service"]);
    if rc != 0 || state.is_empty() {
        state = t("stopped");
    }
    CheckItem { id: "ai", status: "OK", summary: format!("Vulkan · llama-server: {state}"), fix: String::new() }
}

fn run_safe_fixes() -> Vec<String> {
    let mut actions = Vec::new();
    let apps = xdg_data_home().join("applications");
    if apps.is_dir() && which("update-desktop-database") {
        let s = apps.to_string_lossy().into_owned();
        let _ = run_out("update-desktop-database", &[&s]);
        actions.push("update-desktop-database".into());
    }
    if which("agy") || user_home().join(".local/bin/agy").is_file() {
        if !Path::new("/usr/share/applications/agy.desktop").is_file() && !apps.join("agy.desktop").is_file() {
            if fs::create_dir_all(&apps).is_ok() {
                let desktop = "[Desktop Entry]\nType=Application\nName=Antigravity (agy)\nTryExec=agy\n\
Exec=/usr/libexec/tamagi/popup --class agy --title \"Antigravity (agy)\" agy\n\
StartupWMClass=agy\nIcon=utilities-terminal\nTerminal=false\nCategories=Development;\n\
Keywords=agy;antigravity;google;gemini;ia;ai;agent;\n";
                if fs::write(apps.join("agy.desktop"), desktop).is_ok() {
                    let s = apps.to_string_lossy().into_owned();
                    let _ = run_out("update-desktop-database", &[&s]);
                    actions.push("agy.desktop".into());
                }
            }
        }
    }
    let (rc, usr_f) = run_out("systemctl", &["--user", "--failed", "--no-legend", "--plain"]);
    if rc == 0 && !usr_f.trim().is_empty() {
        let _ = run_out("systemctl", &["--user", "reset-failed"]);
        actions.push("systemctl --user reset-failed".into());
    }
    actions
}

fn explain_with_ai(results: &[CheckItem]) -> String {
    let problems: Vec<&CheckItem> = results.iter().filter(|r| r.status != "OK").collect();
    if problems.is_empty() {
        return t("No issues found to explain.");
    }
    let lang_name = match detect_lang() {
        "pt" => "português",
        "es" => "español",
        "fr" => "français",
        _ => "English",
    };
    let mut prob_json = String::from("[");
    for (i, p) in problems.iter().enumerate() {
        if i > 0 {
            prob_json.push(',');
        }
        prob_json.push_str(&format!(
            "{{\"id\":{},\"status\":{},\"summary\":{},\"fix\":{}}}",
            json_escape(p.id),
            json_escape(p.status),
            json_escape(&p.summary),
            json_escape(&p.fix)
        ));
    }
    prob_json.push(']');
    let sys_prompt = format!(
        "You are a concise Linux systems architect. Reply in {lang_name} in at most 4 short bullet points explaining the root cause and exact next step based ONLY on the JSON diagnostics."
    );
    let body = format!(
        "{{\"messages\":[{{\"role\":\"system\",\"content\":{}}},{{\"role\":\"user\",\"content\":{}}}],\"temperature\":0.2,\"max_tokens\":300}}",
        json_escape(&sys_prompt),
        json_escape(&prob_json)
    );
    let Ok(resp) = Command::new("curl")
        .args(["-fsSL", "-m", "20", "-H", "Content-Type: application/json", "-d", &body, "http://127.0.0.1:9931/v1/chat/completions"])
        .output()
    else {
        return t("Local AI unavailable (start with: tamagi-ai start).");
    };
    if !resp.status.success() {
        return t("Local AI unavailable (start with: tamagi-ai start).");
    }
    if let Ok(mut jq) = Command::new("jq")
        .args(["-r", ".choices[0].message.content // empty"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(mut stdin) = jq.stdin.take() {
            let _ = io::Write::write_all(&mut stdin, &resp.stdout);
        }
        if let Ok(out) = jq.wait_with_output() {
            let txt = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !txt.is_empty() {
                return txt;
            }
        }
    }
    t("Local AI unavailable (start with: tamagi-ai start).")
}

fn run_all_checks() -> Vec<CheckItem> {
    let fns: [fn() -> CheckItem; 12] = [
        check_image,
        check_kernel,
        check_storage,
        check_performance,
        check_systemd,
        check_layers,
        check_containers,
        check_gpu,
        check_kvm,
        check_security,
        check_desktop,
        check_ai,
    ];
    thread::scope(|s| {
        let handles: Vec<_> = fns.iter().map(|f| s.spawn(f)).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    })
}

fn print_help() {
    println!("tamagi-doctor — {}\n", t("Autonomous health, integrity, and performance diagnostics for Tamagi"));
    println!("  --explain\n    \t{}", t("ask local AI (llama.cpp) for an explanation"));
    println!("  --fix\n    \t{}", t("apply safe user-level fixes"));
    println!("  --json\n    \t{}", t("structured JSON output"));
}

fn main() {
    let mut json_out = false;
    let mut do_fix = false;
    let mut do_explain = false;
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--json" | "-json" => json_out = true,
            "--fix" | "-fix" => do_fix = true,
            "--explain" | "-explain" => do_explain = true,
            "-h" | "--help" | "help" => {
                print_help();
                return;
            }
            other => {
                eprintln!("tamagi-doctor: unknown flag: {other}");
                exit(2);
            }
        }
    }

    let fixed = if do_fix { run_safe_fixes() } else { Vec::new() };
    let results = run_all_checks();

    if json_out {
        print!("{}", format_json(&results, &fixed));
        return;
    }

    if !fixed.is_empty() {
        println!("{}: {}\n", t("Applied fixes"), fixed.join(", "));
    }

    let mut ok_n = 0;
    let mut warn_n = 0;
    let mut fail_n = 0;
    let tty = io::stdout().is_terminal();
    for r in &results {
        let color = match r.status {
            "OK" => {
                ok_n += 1;
                "\x1b[32m"
            }
            "WARN" => {
                warn_n += 1;
                "\x1b[33m"
            }
            "FAIL" => {
                fail_n += 1;
                "\x1b[31m"
            }
            _ => "",
        };
        let tag = format!("{:<4}", r.status);
        let tag = if tty && !color.is_empty() { format!("{color}{tag}\x1b[0m") } else { tag };
        println!("  [{tag}] {:<15} {}", r.id, r.summary);
        if !r.fix.is_empty() {
            println!("           → {}", r.fix);
        }
    }

    println!("\n{}: {ok_n} OK · {warn_n} WARN · {fail_n} FAIL", t("Summary"));
    if do_explain {
        println!("\n── {}", t("Local AI analysis"));
        println!("{}", explain_with_ai(&results));
    }
    if fail_n > 0 {
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_formatting_and_escaping() {
        let checks = vec![CheckItem { id: "image", status: "OK", summary: "Tamagi \"1.0\"".into(), fix: String::new() }];
        let out = format_json(&checks, &["agy.desktop".into()]);
        assert!(out.contains("\"id\": \"image\""));
        assert!(out.contains("\"Tamagi \\\"1.0\\\"\""));
        assert!(out.contains("\"agy.desktop\""));
    }

    #[test]
    fn df_parsing() {
        let sample = "Filesystem 1048576-blocks Used Available Capacity Mounted on\n/dev/vda2 68719476736 10737418240 57982058496 16% /var/home\n";
        assert_eq!(parse_df_bytes(sample), Some((68_719_476_736, 57_982_058_496)));
    }
}
