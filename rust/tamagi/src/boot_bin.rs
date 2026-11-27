//! tamagi-boot — zero-fork static Rust multi-call binary for fast Tamagi boot paths.
//!
//! Dispatches by argv[0] or first argument:
//!   once-per-image check <name>  -> exit 0 if <name> has not run on this image yet
//!   once-per-image done  <name>  -> record current image ID for <name>
//!   loader-conf                  -> ensure `editor no` in $TAMAGI_ESP/loader/loader.conf

use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::exit;

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
}

/// FNV-1a 64-bit hash fallback when /proc/cmdline has neither composefs= nor ostree=.
fn fnv1a_hex(data: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("os-release:{h:016x}")
}

fn image_id_from(cmdline: &str, os_release: &[u8]) -> String {
    for tok in cmdline.split_whitespace() {
        if tok.starts_with("composefs=") || tok.starts_with("ostree=") {
            return tok.to_string();
        }
    }
    fnv1a_hex(os_release)
}

fn current_image_id() -> String {
    let cmdline = fs::read_to_string("/proc/cmdline").unwrap_or_default();
    let os_rel = fs::read("/usr/lib/os-release").unwrap_or_default();
    image_id_from(&cmdline, &os_rel)
}

fn run_once_in(dir: &Path, action: &str, name: &str, id: &str) -> i32 {
    if !valid_name(name) {
        eprintln!("tamagi-boot: invalid name: {name}");
        return 2;
    }
    let stamp = dir.join(name);
    match action {
        "check" => {
            let prev = fs::read_to_string(&stamp).unwrap_or_default();
            if prev.trim() != id { 0 } else { 1 }
        }
        "done" => {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("tamagi-boot: mkdir {}: {e}", dir.display());
                return 1;
            }
            if let Err(e) = fs::write(&stamp, format!("{id}\n")) {
                eprintln!("tamagi-boot: write {}: {e}", stamp.display());
                return 1;
            }
            0
        }
        _ => {
            eprintln!("usage: once-per-image check|done <name>");
            2
        }
    }
}

fn run_loader_conf_in(esp: &Path) -> i32 {
    let loader_dir = esp.join("loader");
    if !loader_dir.is_dir() {
        return 0;
    }
    let conf = loader_dir.join("loader.conf");
    let existing = fs::read_to_string(&conf).unwrap_or_default();
    for line in existing.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some("editor") && matches!(parts.next(), Some("no" | "false" | "0")) {
            return 0;
        }
    }
    let mut out = String::new();
    for line in existing.lines() {
        if line.split_whitespace().next() == Some("editor") {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("editor no\n");

    let tmp = loader_dir.join(".loader.conf.tamagi.tmp");
    let res = (|| -> std::io::Result<()> {
        let mut f = File::create(&tmp)?;
        f.write_all(out.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, &conf)?;
        Ok(())
    })();
    if let Err(e) = res {
        let _ = fs::remove_file(&tmp);
        eprintln!("tamagi-boot: loader-conf failed: {e}");
        return 1;
    }
    println!("systemd-boot: editor no");
    0
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let prog = Path::new(args.first().map(String::as_str).unwrap_or("tamagi-boot"))
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("tamagi-boot");

    let (mode, rest): (&str, &[String]) = match prog {
        "once-per-image" => ("once", &args[1..]),
        "loader-conf" => ("loader-conf", &args[1..]),
        _ => match args.get(1).map(String::as_str) {
            Some("once" | "once-per-image") => ("once", &args[2..]),
            Some("loader-conf") => ("loader-conf", &args[2..]),
            _ => {
                eprintln!("usage: tamagi-boot <once check|done NAME | loader-conf>");
                exit(2);
            }
        },
    };

    let rc = match mode {
        "once" => {
            if rest.len() < 2 {
                eprintln!("usage: once-per-image check|done <name>");
                2
            } else {
                let dir = env::var_os("TAMAGI_ONCE_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("/var/lib/tamagi/once-per-image"));
                run_once_in(&dir, &rest[0], &rest[1], &current_image_id())
            }
        }
        "loader-conf" => {
            let esp = env::var_os("TAMAGI_ESP")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/efi"));
            run_loader_conf_in(&esp)
        }
        _ => 2,
    };
    exit(rc);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_composefs_or_falls_back_to_os_release() {
        assert_eq!(image_id_from("quiet composefs=abc123 rhgb", b"x"), "composefs=abc123");
        assert_eq!(image_id_from("ostree=/ostree/boot.1/fedora/0", b"x"), "ostree=/ostree/boot.1/fedora/0");
        assert!(image_id_from("quiet rhgb", b"PRETTY_NAME=Tamagi").starts_with("os-release:"));
    }

    #[test]
    fn once_per_image_lifecycle() {
        let dir = env::temp_dir().join("tamagi-boot-once-test");
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(run_once_in(&dir, "check", "ldconfig", "composefs=1"), 0);
        assert_eq!(run_once_in(&dir, "done", "ldconfig", "composefs=1"), 0);
        assert_eq!(run_once_in(&dir, "check", "ldconfig", "composefs=1"), 1);
        assert_eq!(run_once_in(&dir, "check", "ldconfig", "composefs=2"), 0);
        assert_eq!(run_once_in(&dir, "check", "../bad", "composefs=1"), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn loader_conf_enforces_editor_no() {
        let esp = env::temp_dir().join("tamagi-boot-esp-test");
        let _ = fs::remove_dir_all(&esp);
        fs::create_dir_all(esp.join("loader")).unwrap();
        fs::write(esp.join("loader/loader.conf"), "timeout 3\neditor yes\n").unwrap();
        assert_eq!(run_loader_conf_in(&esp), 0);
        let content = fs::read_to_string(esp.join("loader/loader.conf")).unwrap();
        assert!(content.lines().any(|l| l == "timeout 3"));
        assert_eq!(content.lines().filter(|l| l.starts_with("editor")).count(), 1);
        assert!(content.lines().any(|l| l == "editor no"));
        let _ = fs::remove_dir_all(&esp);
    }
}
