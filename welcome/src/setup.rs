//! The installer of the live media, without the window: the choices it offers,
//! how the answers are checked, and the answers file it hands to the back end.
//!
//! The back end is `/usr/libexec/tamagi-install <disk> <answers>` (root, through
//! pkexec): it partitions, encrypts and runs `bootc install`. The answers file
//! is one `KEY=value` per line, with no quotes and no expansion; the same file
//! `tamagi-autoinstall` writes for an install with no questions.

use std::collections::BTreeMap;

/// The languages of the installer: (code, name in that language, locale).
pub const LANGS: [(&str, &str, &str); 4] = [
    ("en", "English (United States)", "en_US.UTF-8"),
    ("pt", "Português (Brasil)", "pt_BR.UTF-8"),
    ("es", "Español", "es_ES.UTF-8"),
    ("fr", "Français", "fr_FR.UTF-8"),
];

pub fn locale_of(lang: &str) -> &'static str {
    LANGS.iter().find(|(code, _, _)| *code == lang).map(|(_, _, locale)| *locale).unwrap_or("en_US.UTF-8")
}

/// A keyboard: the xkb layout and variant (the desktop) and the console keymap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyboard {
    pub layout: &'static str,
    pub variant: &'static str,
    pub keymap: &'static str,
    pub name: &'static str,
}

pub const KEYBOARDS: [Keyboard; 10] = [
    Keyboard { layout: "us", variant: "", keymap: "us", name: "English (US)" },
    Keyboard { layout: "us", variant: "intl", keymap: "us-acentos", name: "English (US, international, with accents)" },
    Keyboard { layout: "br", variant: "", keymap: "br-abnt2", name: "Português (Brasil, ABNT2)" },
    Keyboard { layout: "pt", variant: "", keymap: "pt-latin1", name: "Português (Portugal)" },
    Keyboard { layout: "es", variant: "", keymap: "es", name: "Español (España)" },
    Keyboard { layout: "latam", variant: "", keymap: "la-latin1", name: "Español (Latinoamérica)" },
    Keyboard { layout: "fr", variant: "", keymap: "fr", name: "Français (AZERTY)" },
    Keyboard { layout: "ca", variant: "", keymap: "cf", name: "Français (Canada)" },
    Keyboard { layout: "ch", variant: "fr", keymap: "fr_CH", name: "Français (Suisse)" },
    Keyboard { layout: "be", variant: "", keymap: "be-latin1", name: "Français (Belgique)" },
];

/// The keyboard that goes with a language (index in KEYBOARDS).
pub fn default_keyboard(lang: &str) -> usize {
    let layout = match lang {
        "pt" => "br",
        "es" => "es",
        "fr" => "fr",
        _ => "us",
    };
    KEYBOARDS.iter().position(|k| k.layout == layout && k.variant.is_empty()).unwrap_or(0)
}

/// The time zone that goes with a language, before anyone picks one.
pub fn default_timezone(lang: &str) -> &'static str {
    match lang {
        "pt" => "America/Sao_Paulo",
        "es" => "Europe/Madrid",
        "fr" => "Europe/Paris",
        _ => "UTC",
    }
}

/// The zones whose name contains every word of `query` (any case, `_` = space).
pub fn filter_zones<'a>(zones: &'a [String], query: &str) -> Vec<&'a String> {
    let words: Vec<String> = query.split_whitespace().map(|w| w.to_lowercase().replace('_', " ")).collect();
    zones
        .iter()
        .filter(|zone| {
            let name = zone.to_lowercase().replace('_', " ");
            words.iter().all(|w| name.contains(w.as_str()))
        })
        .collect()
}

/// What is wrong with a user name (a translation key), if anything.
pub fn check_username(user: &str, taken: impl Fn(&str) -> bool) -> Result<(), &'static str> {
    let mut chars = user.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase() || c == '_');
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if !first_ok || !rest_ok || user.len() > 32 {
        return Err("The username must start with a lowercase letter and contain only lowercase letters, digits, hyphen or underscore.");
    }
    if user == "live" || user == "root" || taken(user) {
        return Err("That username is reserved by the system. Choose another one.");
    }
    Ok(())
}

/// The full name as it can go in /etc/passwd: no `:`, `,` or line breaks.
pub fn clean_fullname(name: &str) -> String {
    name.chars().filter(|c| !matches!(c, ':' | ',' | '\n' | '\r')).collect::<String>().trim().to_string()
}

/// A user name from the first name: lowercase ASCII letters and digits.
pub fn suggest_username(fullname: &str) -> String {
    let first = fullname.split_whitespace().next().unwrap_or("");
    let mut out = String::new();
    for c in first.chars().flat_map(char::to_lowercase) {
        let plain = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        };
        if plain.is_ascii_lowercase() || plain.is_ascii_digit() {
            out.push(plain);
        }
    }
    while out.starts_with(|c: char| c.is_ascii_digit()) {
        out.remove(0);
    }
    out.truncate(32);
    out
}

/// Smallest disk the back end accepts (it says the same).
pub const MIN_DISK_BYTES: u64 = 24 * 1024 * 1024 * 1024;
/// Shortest disk password.
pub const MIN_DISK_PASSWORD: usize = 8;
/// The TPM PIN: short, because the chip stops guessing after a few tries.
pub const MIN_TPM_PIN: usize = 6;

/// How the encrypted disk opens at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TpmMode {
    /// TPM chip + a short PIN, bound to no PCR (the default when there is a
    /// TPM): safe against a stolen computer, and updates never lock the disk.
    #[default]
    Pin,
    /// TPM chip with no PIN (PCR 7 only while Secure Boot is on).
    Auto,
    /// The disk password at every startup.
    Off,
}

/// A disk offered for the installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub path: String,
    pub bytes: u64,
    pub model: String,
    pub transport: String,
    pub removable: bool,
}

impl Disk {
    pub fn big_enough(&self) -> bool {
        self.bytes >= MIN_DISK_BYTES
    }
}

/// `KEY="value" KEY="value"` (lsblk -P): the pairs of one line.
fn pairs(line: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut rest = line.trim();
    while let Some(eq) = rest.find("=\"") {
        let key = rest[..eq].trim().to_string();
        let after = &rest[eq + 2..];
        let Some(end) = after.find('"') else { break };
        out.insert(key, after[..end].replace("\\x20", " "));
        rest = &after[end + 1..];
    }
    out
}

/// The disks in the output of
/// `lsblk -dbnP -o PATH,SIZE,MODEL,TYPE,TRAN,RM,RO`: whole disks that can be
/// written, minus the installation media itself (`skip`).
pub fn parse_disks(text: &str, skip: &[String]) -> Vec<Disk> {
    text.lines()
        .map(pairs)
        .filter(|p| p.get("TYPE").map(String::as_str) == Some("disk"))
        .filter(|p| p.get("RO").map(String::as_str) != Some("1"))
        .filter_map(|p| {
            let path = p.get("PATH")?.clone();
            if path.starts_with("/dev/zram") || path.starts_with("/dev/loop") || skip.contains(&path) {
                return None;
            }
            Some(Disk {
                bytes: p.get("SIZE").and_then(|s| s.parse().ok()).unwrap_or(0),
                model: p.get("MODEL").cloned().unwrap_or_default().trim().to_string(),
                transport: p.get("TRAN").cloned().unwrap_or_default(),
                removable: p.get("RM").map(String::as_str) == Some("1"),
                path,
            })
        })
        .collect()
}

/// "512 GB", "1.9 TB": sizes as disk makers print them.
pub fn format_bytes(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 1000.0 {
        format!("{:.1} TB", gb / 1000.0)
    } else {
        format!("{:.0} GB", gb)
    }
}

/// What `tamagi-detect-home` found on a disk.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Found {
    pub keys: BTreeMap<String, String>,
}

impl Found {
    pub fn parse(text: &str) -> Self {
        let keys = text
            .lines()
            .filter_map(|line| line.split_once('='))
            .filter(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_uppercase() || c == '_'))
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Self { keys }
    }

    pub fn get(&self, key: &str) -> &str {
        self.keys.get(key).map(String::as_str).unwrap_or("")
    }

    /// An encrypted partition that has to be unlocked before looking inside.
    pub fn locked(&self) -> bool {
        self.get("PRESERVABLE") == "luks"
    }

    /// A home folder that can be kept.
    pub fn preservable(&self) -> bool {
        matches!(self.get("PRESERVABLE"), "btrfs-subvol" | "ostree-home" | "separate-home")
    }

    pub fn users(&self) -> &str {
        self.get("USERS")
    }

    /// The lines added to the answers to keep it.
    fn answers(&self) -> Vec<(String, String)> {
        let mut out = vec![("PRESERVE_HOME".to_string(), "1".to_string()), ("PRESERVE_MODE".to_string(), self.get("PRESERVABLE").to_string())];
        let luks = if self.get("PRESERVE_LUKS").is_empty() { "0" } else { self.get("PRESERVE_LUKS") };
        out.push(("PRESERVE_LUKS".into(), luks.into()));
        for key in ["HOME_PART", "HOME_SUBVOL", "HOME_RELPATH"] {
            out.push((key.into(), self.get(key).into()));
        }
        let fstype = if self.get("HOME_FSTYPE").is_empty() { "btrfs" } else { self.get("HOME_FSTYPE") };
        out.push(("HOME_FSTYPE".into(), fstype.into()));
        for key in ["SYS_PART", "ESP_PART", "BOOT_PART"] {
            out.push((key.into(), self.get(key).into()));
        }
        out
    }
}

/// Everything the back end needs.
#[derive(Debug, Clone, Default)]
pub struct Answers {
    pub edition: String,
    pub lang: String,
    pub keyboard: usize,
    pub timezone: String,
    pub user: String,
    pub fullname: String,
    pub password_hash: String,
    /// Tamagi-SKL student auto-login account (no sudo).
    pub student_user: Option<String>,
    /// Tamagi-SKL managed Chromium homepage URL.
    pub skl_homepage: Option<String>,
    /// The file with the new disk password, when the disk is encrypted.
    pub luks_keyfile: Option<String>,
    /// How the disk opens with the TPM chip (Off when there is none).
    pub tpm: TpmMode,
    /// The file with the TPM PIN (TpmMode::Pin).
    pub tpm_pinfile: Option<String>,
    /// Keep the home folder found on the disk.
    pub keep: Option<Found>,
    /// The file with the password of the existing encrypted partition kept.
    pub existing_keyfile: Option<String>,
    pub verify_state: String,
}

impl Answers {
    /// The answers file. A value never holds a line break: one that would is
    /// left out, so nothing can add a line of its own.
    pub fn to_text(&self) -> String {
        let keyboard = KEYBOARDS.get(self.keyboard).copied().unwrap_or(KEYBOARDS[0]);
        let is_skl = self.edition == "skl";
        let host = if is_skl { "tamagi-skl" } else { "tamagi" };
        let mut lines: Vec<(String, String)> = Vec::new();
        if !self.edition.is_empty() {
            lines.push(("EDITION".into(), if is_skl { "skl" } else { "dxai" }.into()));
        }
        lines.extend([
            ("LOCALE".into(), locale_of(&self.lang).into()),
            ("KEYMAP".into(), keyboard.keymap.into()),
            ("XKB_LAYOUT".into(), keyboard.layout.into()),
            ("XKB_VARIANT".into(), keyboard.variant.into()),
            ("TIMEZONE".into(), self.timezone.clone()),
            ("HOSTNAME".into(), host.into()),
            ("USER".into(), self.user.clone()),
            ("FULLNAME".into(), if self.fullname.is_empty() { self.user.clone() } else { clean_fullname(&self.fullname) }),
            ("PASSWORD_HASH".into(), self.password_hash.clone()),
        ]);
        if is_skl {
            if let Some(student) = &self.student_user {
                if !student.is_empty() {
                    lines.push(("STUDENT_USER".into(), student.clone()));
                }
            }
            if let Some(url) = &self.skl_homepage {
                if !url.is_empty() {
                    lines.push(("SKL_HOMEPAGE".into(), url.clone()));
                }
            }
        }
        // Keeping an encrypted home: its own password opens it, and the TPM
        // question is about that partition.
        let kept_luks = self.keep.as_ref().is_some_and(|f| f.get("PRESERVE_LUKS") == "1");
        let keyfile = if kept_luks { self.existing_keyfile.clone() } else { self.luks_keyfile.clone() };
        if let Some(file) = keyfile {
            lines.push(("LUKS_KEYFILE".into(), file));
            match (self.tpm, &self.tpm_pinfile) {
                (TpmMode::Pin, Some(pin)) => {
                    lines.push(("LUKS_TPM".into(), "pin".into()));
                    lines.push(("LUKS_TPM_PINFILE".into(), pin.clone()));
                }
                (TpmMode::Auto, _) => lines.push(("LUKS_TPM".into(), "auto".into())),
                _ => {}
            }
        }
        lines.push(("VERIFY_STATE".into(), self.verify_state.clone()));
        if let Some(found) = &self.keep {
            lines.extend(found.answers());
        }
        let mut text = String::new();
        for (key, value) in lines {
            if value.contains(['\n', '\r']) {
                continue;
            }
            text.push_str(&format!("{key}={value}\n"));
        }
        text
    }
}

/// A line of the back end's output: `PROGRESS <0-100> <message>`.
pub fn parse_progress(line: &str) -> Option<(u8, String)> {
    let rest = line.trim().strip_prefix("PROGRESS ")?;
    let (number, message) = rest.split_once(' ').unwrap_or((rest, ""));
    let value: u8 = number.parse().ok()?;
    Some((value.min(100), message.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_and_defaults() {
        assert_eq!(locale_of("pt"), "pt_BR.UTF-8");
        assert_eq!(locale_of("xx"), "en_US.UTF-8");
        assert_eq!(KEYBOARDS[default_keyboard("pt")].keymap, "br-abnt2");
        assert_eq!(KEYBOARDS[default_keyboard("fr")].layout, "fr");
        assert_eq!(KEYBOARDS[default_keyboard("en")].layout, "us");
        assert_eq!(KEYBOARDS[default_keyboard("en")].variant, "");
        assert_eq!(default_timezone("pt"), "America/Sao_Paulo");
        assert_eq!(default_timezone("en"), "UTC");
    }

    #[test]
    fn zones() {
        let zones: Vec<String> = ["America/Sao_Paulo", "America/New_York", "Europe/Lisbon", "UTC"].iter().map(|s| s.to_string()).collect();
        assert_eq!(filter_zones(&zones, "sao paulo"), [&zones[0]]);
        assert_eq!(filter_zones(&zones, "AMERICA").len(), 2);
        assert_eq!(filter_zones(&zones, "new_york"), [&zones[1]]);
        assert_eq!(filter_zones(&zones, "").len(), 4);
        assert!(filter_zones(&zones, "mars").is_empty());
    }

    #[test]
    fn usernames() {
        let none = |_: &str| false;
        assert!(check_username("maria", none).is_ok());
        assert!(check_username("_svc-1", none).is_ok());
        assert!(check_username("Maria", none).is_err());
        assert!(check_username("1ana", none).is_err());
        assert!(check_username("ana silva", none).is_err());
        assert!(check_username("", none).is_err());
        assert!(check_username(&"a".repeat(33), none).is_err());
        assert!(check_username("live", none).is_err());
        assert!(check_username("root", none).is_err());
        assert!(check_username("bin", |u| u == "bin").is_err());
        assert_ne!(check_username("Bad", none), check_username("live", none));
    }

    #[test]
    fn names() {
        assert_eq!(suggest_username("José da Silva"), "jose");
        assert_eq!(suggest_username("  Ñandú Çá "), "nanduca".get(..5).unwrap());
        assert_eq!(suggest_username("2Pac"), "pac");
        assert_eq!(suggest_username(""), "");
        assert_eq!(clean_fullname(" Ana: Lima,\n "), "Ana Lima");
    }

    #[test]
    fn disks() {
        let text = concat!(
            "PATH=\"/dev/nvme0n1\" SIZE=\"512110190592\" MODEL=\"Samsung SSD 980\" TYPE=\"disk\" TRAN=\"nvme\" RM=\"0\" RO=\"0\"\n",
            "PATH=\"/dev/sda\" SIZE=\"31914983424\" MODEL=\"SanDisk\\x20Ultra\" TYPE=\"disk\" TRAN=\"usb\" RM=\"1\" RO=\"0\"\n",
            "PATH=\"/dev/sr0\" SIZE=\"1073741312\" MODEL=\"QEMU DVD-ROM\" TYPE=\"rom\" TRAN=\"sata\" RM=\"1\" RO=\"0\"\n",
            "PATH=\"/dev/zram0\" SIZE=\"8589934592\" MODEL=\"\" TYPE=\"disk\" TRAN=\"\" RM=\"0\" RO=\"0\"\n",
            "PATH=\"/dev/vdb\" SIZE=\"10737418240\" MODEL=\"\" TYPE=\"disk\" TRAN=\"\" RM=\"0\" RO=\"1\"\n",
            "PATH=\"/dev/vda\" SIZE=\"10737418240\" MODEL=\"\" TYPE=\"disk\" TRAN=\"\" RM=\"0\" RO=\"0\"\n",
        );
        let disks = parse_disks(text, &["/dev/sda".to_string()]);
        assert_eq!(disks.iter().map(|d| d.path.as_str()).collect::<Vec<_>>(), ["/dev/nvme0n1", "/dev/vda"]);
        assert_eq!(disks[0].model, "Samsung SSD 980");
        assert!(disks[0].big_enough());
        assert!(!disks[1].big_enough());
        let with_usb = parse_disks(text, &[]);
        assert_eq!(with_usb[1].model, "SanDisk Ultra");
        assert!(with_usb[1].removable);
        assert_eq!(format_bytes(512_110_190_592), "512 GB");
        assert_eq!(format_bytes(1_920_000_000_000), "1.9 TB");
    }

    #[test]
    fn found_on_disk() {
        let f = Found::parse("PRESERVABLE=btrfs-subvol\nUSERS=ana bia\nHOME_PART=/dev/vda2\nHOME_SUBVOL=home\njunk line\nlower=x\n");
        assert!(f.preservable());
        assert!(!f.locked());
        assert_eq!(f.users(), "ana bia");
        assert_eq!(f.get("lower"), "");
        assert!(Found::parse("PRESERVABLE=luks\n").locked());
        let nothing = Found::parse("PRESERVABLE=\n");
        assert!(!nothing.preservable() && !nothing.locked());
    }

    fn base() -> Answers {
        Answers {
            lang: "pt".into(),
            keyboard: default_keyboard("pt"),
            timezone: "America/Sao_Paulo".into(),
            user: "ana".into(),
            fullname: "Ana: Lima".into(),
            password_hash: "$6$salt$hash".into(),
            verify_state: "/run/user/1000/tamagi-install/verify".into(),
            ..Answers::default()
        }
    }

    #[test]
    fn answers_file() {
        let text = base().to_text();
        assert_eq!(
            text,
            "LOCALE=pt_BR.UTF-8\nKEYMAP=br-abnt2\nXKB_LAYOUT=br\nXKB_VARIANT=\nTIMEZONE=America/Sao_Paulo\nHOSTNAME=tamagi\nUSER=ana\nFULLNAME=Ana Lima\nPASSWORD_HASH=$6$salt$hash\nVERIFY_STATE=/run/user/1000/tamagi-install/verify\n"
        );
        let mut a = base();
        a.luks_keyfile = Some("/run/k".into());
        a.tpm = TpmMode::Auto;
        assert!(a.to_text().contains("LUKS_KEYFILE=/run/k\nLUKS_TPM=auto\n"));
        a.tpm = TpmMode::Pin;
        a.tpm_pinfile = Some("/run/pin".into());
        assert!(a.to_text().contains("LUKS_KEYFILE=/run/k\nLUKS_TPM=pin\nLUKS_TPM_PINFILE=/run/pin\n"));
        // PIN mode without a PIN file: no TPM at all, never a TPM without PIN.
        a.tpm_pinfile = None;
        assert!(!a.to_text().contains("LUKS_TPM"));
        a.tpm = TpmMode::Off;
        assert!(!a.to_text().contains("LUKS_TPM"));
    }

    #[test]
    fn keeping_the_home_folder() {
        let mut a = base();
        a.luks_keyfile = Some("/run/new".into());
        a.keep = Some(Found::parse("PRESERVABLE=btrfs-subvol\nHOME_PART=/dev/vda2\nHOME_SUBVOL=home\nESP_PART=/dev/vda1\n"));
        let text = a.to_text();
        assert!(text.contains("LUKS_KEYFILE=/run/new\n"));
        assert!(text.contains("PRESERVE_HOME=1\nPRESERVE_MODE=btrfs-subvol\nPRESERVE_LUKS=0\nHOME_PART=/dev/vda2\nHOME_SUBVOL=home\nHOME_RELPATH=\nHOME_FSTYPE=btrfs\nSYS_PART=\nESP_PART=/dev/vda1\nBOOT_PART=\n"));
        // An encrypted home kept: its own password, not a new one.
        a.keep = Some(Found::parse("PRESERVABLE=btrfs-subvol\nPRESERVE_LUKS=1\n"));
        a.existing_keyfile = Some("/run/old".into());
        a.tpm = TpmMode::Auto;
        let text = a.to_text();
        assert!(text.contains("LUKS_KEYFILE=/run/old\nLUKS_TPM=auto\n"));
        assert!(!text.contains("/run/new"));
    }

    #[test]
    fn no_value_adds_a_line() {
        let mut a = base();
        a.fullname = "Ana\nUSER=root".into();
        a.timezone = "UTC\nUSER=root".into();
        let text = a.to_text();
        assert_eq!(text.lines().filter(|l| l.starts_with("USER=")).count(), 1);
        assert!(!text.contains("TIMEZONE="));
        assert!(text.contains("FULLNAME=AnaUSER=root\n"));
    }

    #[test]
    fn progress_lines() {
        assert_eq!(parse_progress("PROGRESS 40 Copying the system"), Some((40, "Copying the system".into())));
        assert_eq!(parse_progress("PROGRESS 100"), Some((100, String::new())));
        assert_eq!(parse_progress("PROGRESS 250 x"), Some((100, "x".into())));
        assert_eq!(parse_progress("PROGRESS x y"), None);
        assert_eq!(parse_progress("bootc: copying"), None);
    }
}
