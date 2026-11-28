//! What touches the machine: the "done" stamp, the two small things the
//! terminal assistant does when it starts, the encrypted-disk condition, and
//! running the installers while reading their output.

use crate::i18n::config_dir;
use crate::plan::Step;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// `${XDG_CONFIG_HOME:-~/.config}/tamagi/firstrun-done`
pub fn stamp_path() -> PathBuf {
    config_dir().join("tamagi").join("firstrun-done")
}

pub fn write_stamp() {
    let path = stamp_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if !path.exists() {
        let _ = std::fs::write(&path, "");
    }
}

/// The clock: 24 hours, except in English, which uses AM/PM.
pub fn military_time(lang: &str) -> &'static str {
    if lang == "en" {
        "false"
    } else {
        "true"
    }
}

/// Writes the 12/24 h choice where the COSMIC clock applet reads it.
pub fn write_clock(lang: &str) {
    let dir = PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/cosmic/com.system76.CosmicAppletTime/v1");
    if std::fs::create_dir_all(&dir).is_ok() {
        let _ = std::fs::write(dir.join("military_time"), format!("{}\n", military_time(lang)));
    }
}

/// A command of its own session, with no terminal: `sudo` then asks for the
/// password through SUDO_ASKPASS (this same program) instead of a tty.
fn detached(argv: &[String]) -> Option<Command> {
    let (program, args) = argv.split_first()?;
    let mut command = Command::new(program);
    command.args(args).stdin(Stdio::null());
    if let Ok(exe) = std::env::current_exe() {
        command.env("SUDO_ASKPASS", exe);
    }
    // sudo runs the askpass program with the prompt as its only argument:
    // this variable is how the program knows it is being asked for a password.
    command.env(crate::ASKPASS_ENV, "1");
    if let Some((dir, real)) = sudo_shim() {
        let mut path = std::ffi::OsString::from(dir);
        if let Some(rest) = std::env::var_os("PATH") {
            path.push(":");
            path.push(rest);
        }
        command.env("PATH", path).env(crate::REAL_SUDO_ENV, real);
    }
    // SAFETY: setsid is async-signal-safe and touches no memory of the parent.
    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    Some(command)
}

/// The first `name` in PATH that is an executable file.
fn find_in_path(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|path| path.metadata().map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false))
}

/// Where the `sudo` link of this run lives.
fn shim_dir() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").filter(|v| !v.is_empty()).map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join(format!("tamagi-welcome-{}", std::process::id()))
}

/// A directory holding `sudo` -> this program, put first in the PATH of the
/// installers, and the real sudo it calls with `-A` (see `cli::sudo_args`).
/// With no terminal, sudo only uses SUDO_ASKPASS by itself when DISPLAY is
/// set; the link makes it ask through the dialog on any session.
/// None when there is no sudo, or the link cannot be made: the installers
/// then run with SUDO_ASKPASS alone.
fn sudo_shim() -> Option<&'static (PathBuf, PathBuf)> {
    static SHIM: std::sync::OnceLock<Option<(PathBuf, PathBuf)>> = std::sync::OnceLock::new();
    SHIM.get_or_init(|| {
        let real = find_in_path("sudo")?;
        let exe = std::env::current_exe().ok()?;
        let dir = shim_dir();
        std::fs::create_dir_all(&dir).ok()?;
        let link = dir.join("sudo");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(exe, &link).ok()?;
        Some((dir, real))
    })
    .as_ref()
}

/// Removes the `sudo` link and its directory (at the end of the program).
pub fn remove_sudo_shim() {
    let dir = shim_dir();
    let _ = std::fs::remove_file(dir.join("sudo"));
    let _ = std::fs::remove_dir(dir);
}

/// Starts a command in the background, quiet, and does not wait for it
/// (a thread reaps it, so no zombie is left behind).
pub fn spawn_quiet(argv: &[&str]) {
    let argv: Vec<String> = argv.iter().map(|a| a.to_string()).collect();
    std::thread::spawn(move || {
        run_quiet(&argv);
    });
}

/// Runs a command to the end, quiet. False when it fails or does not exist.
pub fn run_quiet(argv: &[String]) -> bool {
    let Some(mut command) = detached(argv) else {
        return false;
    };
    command.stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

/// `lsblk -no TYPE` lists a `crypt` device: the disk is encrypted (LUKS).
pub fn has_crypt_device(lsblk_types: &str) -> bool {
    lsblk_types.lines().any(|line| line.trim() == "crypt")
}

/// The kernel command line already unlocks with the TPM (`grep -qw`).
pub fn cmdline_unlocks_with_tpm(cmdline: &str) -> bool {
    const NEEDLE: &str = "tpm2-device=auto";
    let word = |c: char| c.is_alphanumeric() || c == '_';
    cmdline.match_indices(NEEDLE).any(|(pos, _)| {
        let before = cmdline[..pos].chars().next_back();
        let after = cmdline[pos + NEEDLE.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// Maps a PCI vendor hex ID (`/sys/class/drm/card*/device/vendor`) to a friendly GPU vendor label.
pub fn gpu_vendor_name(vendor_hex: &str) -> Option<&'static str> {
    match vendor_hex.trim().to_ascii_lowercase().as_str() {
        "0x10de" => Some("NVIDIA"),
        "0x1002" | "0x1022" => Some("AMD Radeon"),
        "0x8086" => Some("Intel"),
        "0x1af4" | "0x1b36" | "0x1234" => Some("Virtual GPU"),
        _ => None,
    }
}

/// Detects active GPU vendors on the system for the First Boot GPU & Drivers page.
/// `TAMAGI_WELCOME_GPUS` overrides detection for deterministic tests and screenshots.
pub fn detect_gpus() -> Vec<String> {
    if let Ok(forced) = std::env::var("TAMAGI_WELCOME_GPUS") {
        return forced
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
    }
    let mut gpus: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.starts_with("card") && !name.contains('-'))
            .collect();
        names.sort();
        for card in names {
            let vendor_path = format!("/sys/class/drm/{card}/device/vendor");
            if let Ok(vendor) = std::fs::read_to_string(vendor_path) {
                if let Some(label) = gpu_vendor_name(&vendor) {
                    if !gpus.iter().any(|g| g == label) {
                        gpus.push(label.to_string());
                    }
                }
            }
        }
    }
    if std::path::Path::new("/dev/accel/accel0").exists() {
        gpus.push("NPU".to_string());
    }
    gpus
}

/// Whether to ask about unlocking the encrypted disk with the TPM chip: an
/// encrypted disk, a TPM, and not unlocking with it yet.
/// `TAMAGI_WELCOME_TPM=1|0` forces the answer (tests, screenshots).
pub fn tpm_question_applies() -> bool {
    match std::env::var("TAMAGI_WELCOME_TPM").as_deref() {
        Ok("1") => return true,
        Ok("0") => return false,
        _ => {}
    }
    let types = Command::new("lsblk").args(["-no", "TYPE"]).stderr(Stdio::null()).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    has_crypt_device(&types)
        && std::path::Path::new("/sys/class/tpm/tpm0").exists()
        && !cmdline_unlocks_with_tpm(&std::fs::read_to_string("/proc/cmdline").unwrap_or_default())
}

/// What the installation reports to the window.
#[derive(Debug, Clone)]
pub enum Event {
    /// The command `index` of the plan started.
    Started(usize),
    /// A line of its output (raw: colours still in).
    Output(String),
    /// It ended; false when it failed.
    Ended(usize, bool),
    /// Everything ran, the stamp is written.
    Finished,
}

/// Runs one command, sending each line of stdout and stderr as it arrives.
fn run_step(step: &Step, send: &impl Fn(Event)) -> bool {
    let Some(mut command) = detached(&step.argv) else {
        return false;
    };
    let (mut reader, writer) = match std::io::pipe() {
        Ok(pipe) => pipe,
        Err(e) => {
            send(Event::Output(format!("{}: {e}", step.argv[0])));
            return false;
        }
    };
    match writer.try_clone() {
        Ok(clone) => command.stdout(clone).stderr(writer),
        Err(_) => command.stdout(writer).stderr(Stdio::null()),
    };
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            send(Event::Output(format!("{}: {e}", step.argv[0])));
            return false;
        }
    };
    // The parent's copies of the write end must close, or the read never ends.
    drop(command);
    let mut pending = String::new();
    let mut chunk = [0u8; 4096];
    // Bytes of a UTF-8 character cut at the end of a chunk.
    let mut partial: Vec<u8> = Vec::new();
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                partial.extend_from_slice(&chunk[..n]);
                let valid = match std::str::from_utf8(&partial) {
                    Ok(_) => partial.len(),
                    Err(e) if e.error_len().is_none() => e.valid_up_to(),
                    Err(_) => partial.len(),
                };
                let text = String::from_utf8_lossy(&partial[..valid]).into_owned();
                partial.drain(..valid);
                for line in crate::progress::split_lines(&mut pending, &text) {
                    send(Event::Output(line));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    if !pending.is_empty() {
        send(Event::Output(pending));
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

/// Runs the whole plan in order (a failure does not stop the next command,
/// as in the terminal assistant), then the notification, the cleanup of an
/// unused prefetched runtime, and the stamp.
pub fn run_plan(steps: &[Step], notification: Option<(String, String)>, send: impl Fn(Event)) {
    for (index, step) in steps.iter().enumerate() {
        send(Event::Started(index));
        let ok = run_step(step, &send);
        send(Event::Ended(index, ok));
    }
    if let (false, Some((title, body))) = (steps.is_empty(), notification) {
        let argv = ["notify-send", "-a", "Tamagi", "-i", "system-software-install", &title, &body].map(String::from);
        run_quiet(&argv);
    }
    finish();
    send(Event::Finished);
}

/// The end of the assistant, with or without anything installed.
pub fn finish() {
    run_quiet(&["tamagi-install".to_string(), "prefetch-cleanup".to_string()]);
    write_stamp();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn clock() {
        assert_eq!(military_time("en"), "false");
        assert_eq!(military_time("pt"), "true");
        assert_eq!(military_time("fr"), "true");
    }

    #[test]
    fn gpu_vendors() {
        assert_eq!(gpu_vendor_name("0x10de\n"), Some("NVIDIA"));
        assert_eq!(gpu_vendor_name("0x1002"), Some("AMD Radeon"));
        assert_eq!(gpu_vendor_name("0x8086"), Some("Intel"));
        assert_eq!(gpu_vendor_name("0x1af4"), Some("Virtual GPU"));
        assert_eq!(gpu_vendor_name("0x9999"), None);
    }

    #[test]
    fn crypt_devices() {
        assert!(has_crypt_device("disk\npart\ncrypt\nlvm\n"));
        assert!(!has_crypt_device("disk\npart\npart\n"));
        assert!(!has_crypt_device("cryptic\n"));
        assert!(!has_crypt_device(""));
    }

    #[test]
    fn kernel_command_line() {
        assert!(cmdline_unlocks_with_tpm("root=UUID=x rd.luks.options=tpm2-device=auto quiet"));
        assert!(cmdline_unlocks_with_tpm("rd.luks.options=discard,tpm2-device=auto\n"));
        assert!(!cmdline_unlocks_with_tpm("root=UUID=x quiet"));
        assert!(!cmdline_unlocks_with_tpm("tpm2-device=automatic"));
        assert!(!cmdline_unlocks_with_tpm("xtpm2-device=auto"));
    }

    fn step(script: &str) -> Step {
        Step { argv: vec!["sh".into(), "-c".into(), script.into()], title: "test", weight: 1 }
    }

    #[test]
    fn output_arrives_line_by_line_from_both_streams() {
        let lines = Mutex::new(Vec::new());
        let ok = run_step(&step("echo '── [1/2] one'; echo err >&2; printf 'a\\rb\\n'; printf 'no newline'"), &|e| {
            if let Event::Output(line) = e {
                lines.lock().unwrap().push(line);
            }
        });
        assert!(ok);
        assert_eq!(*lines.lock().unwrap(), ["── [1/2] one", "err", "a", "b", "no newline"]);
    }

    #[test]
    fn failure_and_missing_program() {
        assert!(!run_step(&step("exit 3"), &|_| {}));
        let missing = Step { argv: vec!["/nonexistent/tamagi-nothing".into()], title: "x", weight: 1 };
        let lines = Mutex::new(Vec::new());
        assert!(!run_step(&missing, &|e| {
            if let Event::Output(line) = e {
                lines.lock().unwrap().push(line);
            }
        }));
        assert!(lines.lock().unwrap()[0].starts_with("/nonexistent/tamagi-nothing: "));
    }

    #[test]
    fn children_find_the_sudo_shim_first() {
        if find_in_path("sudo").is_none() {
            return; // no sudo on this machine: no shim
        }
        let lines = Mutex::new(Vec::new());
        run_step(&step("readlink \"$(command -v sudo)\"; echo \"$TAMAGI_WELCOME_REAL_SUDO\""), &|e| {
            if let Event::Output(line) = e {
                lines.lock().unwrap().push(line);
            }
        });
        let lines = lines.lock().unwrap();
        assert_eq!(std::path::Path::new(&lines[0]), std::env::current_exe().unwrap());
        assert_eq!(std::path::Path::new(&lines[1]), find_in_path("sudo").unwrap());
    }

    #[test]
    fn children_get_askpass_and_no_terminal() {
        let lines = Mutex::new(Vec::new());
        run_step(&step("echo \"$TAMAGI_WELCOME_ASKPASS ${SUDO_ASKPASS:+set}\"; if [ -t 0 ]; then echo tty; else echo notty; fi"), &|e| {
            if let Event::Output(line) = e {
                lines.lock().unwrap().push(line);
            }
        });
        assert_eq!(*lines.lock().unwrap(), ["1 set", "notty"]);
    }
}
