//! What the installer window asks of the live system: the disks, the TPM, the
//! time zones, the password hash, and the two root programs (the detection of
//! a home folder to keep, and the installation itself), run through pkexec.
//!
//! For screenshots and tests, every program can be replaced:
//!   TAMAGI_SETUP_LSBLK     a file with the `lsblk -dbnP` output to show
//!   TAMAGI_SETUP_BACKEND   instead of /usr/libexec/tamagi-install
//!   TAMAGI_SETUP_DETECT    instead of /usr/libexec/tamagi-detect-home
//!   TAMAGI_SETUP_PRIV      the program that runs them as root (default: pkexec,
//!                          or nothing when already root; empty: nothing)

use crate::setup::{parse_disks, Disk, Found, Keyboard};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

/// `$XDG_RUNTIME_DIR/tamagi-install`, only for this user (mode 700).
pub fn work_dir() -> PathBuf {
    let base = env("XDG_RUNTIME_DIR").unwrap_or_else(|| "/run".into());
    let dir = PathBuf::from(base).join("tamagi-install");
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    dir
}

/// Writes a file only this user can read (passwords, answers).
pub fn write_secret(path: &std::path::Path, content: &str) -> bool {
    let _ = std::fs::remove_file(path);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .and_then(|mut f| f.write_all(content.as_bytes()))
        .is_ok()
}

pub fn uefi() -> bool {
    env("TAMAGI_SETUP_LSBLK").is_some() || std::path::Path::new("/sys/firmware/efi").is_dir()
}

/// Edition picked at ISO boot (`tamagi.edition=dxai|skl|skl-os`), overridable via env.
pub fn edition() -> String {
    if let Some(v) = env("TAMAGI_SETUP_EDITION") {
        return match v.as_str() {
            "skl-os" => "skl-os".into(),
            "skl" => "skl".into(),
            _ => "dxai".into(),
        };
    }
    if let Ok(cmdline) = std::fs::read_to_string("/proc/cmdline") {
        for tok in cmdline.split_whitespace() {
            if let Some(val) = tok.strip_prefix("tamagi.edition=") {
                if val == "skl" || val == "skl-os" {
                    return val.into();
                }
            }
        }
    }
    "dxai".into()
}

/// Secure Boot is enabled in the UEFI firmware.
pub fn secure_boot() -> bool {
    if let Some(v) = env("TAMAGI_SETUP_SECUREBOOT") {
        return v == "1";
    }
    if env("TAMAGI_SETUP_LSBLK").is_some() {
        return false;
    }
    std::fs::read("/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c")
        .is_ok_and(|b| b.get(4) == Some(&1))
}

/// A TPM 2.0 chip is there.
pub fn has_tpm2() -> bool {
    if let Some(v) = env("TAMAGI_SETUP_TPM") {
        return v == "1";
    }
    std::fs::read_to_string("/sys/class/tpm/tpm0/tpm_version_major").is_ok_and(|v| v.trim() == "2")
}

/// The disk the live system started from (never offered).
fn live_disk() -> Vec<String> {
    let out = |args: &[&str]| Command::new(args[0]).args(&args[1..]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let source = out(&["findmnt", "-no", "SOURCE", "/run/initramfs/live"]);
    if source.is_empty() {
        return Vec::new();
    }
    let parent = out(&["lsblk", "-npo", "PKNAME", &source]);
    let mut skip = vec![source];
    if let Some(p) = parent.lines().next().filter(|p| !p.is_empty()) {
        skip.push(p.to_string());
    }
    skip
}

pub fn disks() -> Vec<Disk> {
    if let Some(file) = env("TAMAGI_SETUP_LSBLK") {
        return parse_disks(&std::fs::read_to_string(file).unwrap_or_default(), &[]);
    }
    let output = Command::new("lsblk").args(["-dbnP", "-o", "PATH,SIZE,MODEL,TYPE,TRAN,RM,RO"]).output();
    let text = output.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    parse_disks(&text, &live_disk())
}

pub fn timezones() -> Vec<String> {
    let listed = Command::new("timedatectl").args(["list-timezones", "--no-pager"]).output().ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).filter(|l| !l.is_empty()).collect::<Vec<_>>()).unwrap_or_default();
    if listed.is_empty() {
        ["UTC", "America/Sao_Paulo", "America/New_York", "Europe/Lisbon", "Europe/Madrid", "Europe/Paris"].iter().map(|s| s.to_string()).collect()
    } else {
        listed
    }
}

/// A user name that already exists on the live system (bin, daemon…).
pub fn user_taken(name: &str) -> bool {
    Command::new("getent").args(["passwd", name]).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

/// SHA-512 crypt of the password, as /etc/shadow keeps it.
pub fn hash_password(password: &str) -> Option<String> {
    let mut child = Command::new("openssl").args(["passwd", "-6", "-stdin"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    child.stdin.take()?.write_all(password.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    let hash = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (out.status.success() && hash.starts_with("$6$")).then_some(hash)
}

/// Starts the check of the image on the media; the back end takes its result.
pub fn start_media_check(state: &std::path::Path) {
    let payload = std::fs::read_to_string("/etc/tamagi-live.conf").ok().and_then(|t| t.lines().find_map(|l| l.strip_prefix("PAYLOAD_DIR=").map(str::to_string)));
    let Some(payload) = payload.filter(|p| std::path::Path::new(p).is_dir()) else { return };
    let _ = std::fs::remove_dir_all(state);
    let _ = Command::new("setsid").args(["/usr/libexec/tamagi-verify-media", &payload]).arg(state).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// The keyboard of the live session, so passwords are typed the way they will be later.
pub fn apply_keyboard(keyboard: &Keyboard) {
    let dir = crate::i18n::config_dir().join("cosmic/com.system76.CosmicComp/v1");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ron = format!(
        "(\n    rules: \"\",\n    model: \"\",\n    layout: \"{}\",\n    variant: \"{}\",\n    options: None,\n    repeat_delay: 600,\n    repeat_rate: 25,\n)\n",
        keyboard.layout, keyboard.variant
    );
    let _ = std::fs::write(dir.join("xkb_config"), ron);
}

/// The program and the arguments that run `program` as root.
fn as_root(program: String, args: Vec<String>) -> Command {
    let runner = match std::env::var("TAMAGI_SETUP_PRIV") {
        Ok(v) => v,
        Err(_) if unsafe { libc::geteuid() } == 0 => String::new(),
        Err(_) => "pkexec".into(),
    };
    if runner.is_empty() {
        let mut c = Command::new(program);
        c.args(args);
        c
    } else {
        let mut c = Command::new(runner);
        c.arg(program).args(args);
        c
    }
}

/// What can be kept on `disk`. Exit code 2: the password did not open it.
pub fn detect_home(disk: &str, keyfile: Option<&str>) -> (Found, i32) {
    let program = env("TAMAGI_SETUP_DETECT").unwrap_or_else(|| "/usr/libexec/tamagi-detect-home".into());
    let mut args = vec![disk.to_string()];
    if let Some(file) = keyfile {
        args.extend(["--key-file".to_string(), file.to_string()]);
    }
    match as_root(program, args).stderr(Stdio::null()).output() {
        Ok(out) => (Found::parse(&String::from_utf8_lossy(&out.stdout)), out.status.code().unwrap_or(1)),
        Err(_) => (Found::default(), 1),
    }
}

/// What the installation says while it runs.
#[derive(Debug, Clone)]
pub enum Event {
    Line(String),
    Ended(i32),
}

/// Runs the back end on `disk`; every line of its output goes to `send` and to
/// `log`, and the exit code last.
pub fn install(disk: &str, answers: &std::path::Path, log: &std::path::Path, send: impl Fn(Event) + Send + Sync + 'static) {
    let program = env("TAMAGI_SETUP_BACKEND").unwrap_or_else(|| "/usr/libexec/tamagi-install".into());
    let args = vec![disk.to_string(), answers.to_string_lossy().into_owned()];
    let log_file = std::fs::OpenOptions::new().create(true).append(true).mode(0o600).open(log).ok();
    let log_file = std::sync::Arc::new(std::sync::Mutex::new(log_file));
    let send = std::sync::Arc::new(send);
    let mut child = match as_root(program, args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(child) => child,
        Err(e) => {
            send(Event::Line(format!("tamagi-install: {e}")));
            send(Event::Ended(127));
            return;
        }
    };
    let pump = |stream: Box<dyn std::io::Read + Send>| {
        let send = send.clone();
        let log_file = log_file.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if let Ok(mut guard) = log_file.lock() {
                    if let Some(f) = guard.as_mut() {
                        let _ = writeln!(f, "{line}");
                    }
                }
                send(Event::Line(line));
            }
        })
    };
    let out = child.stdout.take().map(|s| pump(Box::new(s)));
    let err = child.stderr.take().map(|s| pump(Box::new(s)));
    for t in [out, err].into_iter().flatten() {
        let _ = t.join();
    }
    let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
    send(Event::Ended(code));
}

pub fn reboot() {
    let _ = Command::new("systemctl").arg("reboot").spawn();
}
