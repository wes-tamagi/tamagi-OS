//! tamagi-layer — Fedora packages on top of the image, without rebuilding it.
//!
//! A layer is an erofs file in /var/lib/extensions that systemd-sysext merges
//! over /usr. It is built from a copy of the image's package database: only
//! what the requested packages ADD goes into the layer. What it was made from
//! (packages, repositories, units, image version) is kept next to it, in
//! /var/lib/tamagi/layers, so the layer can be rebuilt for a new image.
//!
//! This program owns the arguments, their validation, the bookkeeping and the
//! order of the steps. The part that is shell by nature — dnf, rpm queries,
//! moving /opt, mkfs.erofs — is `inner.sh`, written to the work folder and
//! run as root or inside `podman unshare`.

mod i18n;

use i18n::Catalog;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

/// The script that builds the layer inside the build root (see the header).
const INNER: &str = include_str!("inner.sh");

/// The bookkeeping files of a layer, in the metadata folder.
const META_KINDS: &[&str] = &["request", "coprs", "rpmfusion", "repos", "units", "overlay", "packages", "image"];

extern "C" {
    fn geteuid() -> u32;
}

fn is_root() -> bool {
    // SAFETY: geteuid has no arguments, no failure mode and no side effects.
    unsafe { geteuid() == 0 }
}

// ── What was asked ──────────────────────────────────────────────────────────

/// A layer to build: `add`, `build` and `rebuild` all end up here.
#[derive(Debug, Default, Clone, PartialEq)]
struct Request {
    name: String,
    pkgs: Vec<String>,
    coprs: Vec<String>,
    repos: Vec<String>,
    units: Vec<String>,
    allow_upgrade: bool,
    rpmfusion: bool,
    out: String,
    overlay: String,
}

/// Why the arguments were refused.
#[derive(Debug, PartialEq)]
enum Refused {
    /// No layer name: show the usage text (exit code 2).
    Usage,
    /// A message for `die` (exit code 1). `Key` is translated, `Text` is not.
    Text(String),
    Key(&'static str, String),
}

fn all(s: &str, ok: impl Fn(char) -> bool) -> bool {
    s.chars().all(ok)
}

/// ^[a-z][a-z0-9-]{0,39}$
fn valid_name(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some('a'..='z')) && s.len() <= 40 && all(&s[1..], |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// ^[A-Za-z0-9@_.-]+/[A-Za-z0-9_.-]+$
fn valid_copr(s: &str) -> bool {
    let word = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-');
    match s.split_once('/') {
        Some((owner, project)) => {
            !owner.is_empty() && !project.is_empty() && all(owner, |c| word(c) || c == '@') && all(project, word)
        }
        None => false,
    }
}

/// ^[a-z][a-z0-9-]*=https://[^=[:space:],]+(=https://[^=[:space:],]+)?$
fn valid_repo(s: &str) -> bool {
    let url = |u: &str| match u.strip_prefix("https://") {
        Some(rest) => !rest.is_empty() && all(rest, |c| c != '=' && c != ',' && !c.is_whitespace()),
        None => false,
    };
    let mut parts = s.split('=');
    let id = parts.next().unwrap_or("");
    let base = parts.next();
    let key = parts.next();
    if parts.next().is_some() {
        return false;
    }
    let id_ok = matches!(id.chars().next(), Some('a'..='z')) && all(id, |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    id_ok && base.is_some_and(url) && key.map_or(true, url)
}

/// ^[A-Za-z0-9@_.-]+\.(service|socket|timer|path)$
fn valid_unit(s: &str) -> bool {
    let ok = |c: char| c.is_ascii_alphanumeric() || matches!(c, '@' | '_' | '.' | '-');
    [".service", ".socket", ".timer", ".path"]
        .iter()
        .any(|suffix| s.strip_suffix(suffix).is_some_and(|stem| !stem.is_empty() && all(stem, ok)))
}

/// ^[a-z][a-z0-9-]*$
fn valid_overlay_name(s: &str) -> bool {
    matches!(s.chars().next(), Some('a'..='z')) && all(s, |c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// ^[A-Za-z0-9@][A-Za-z0-9+._:@/-]*$
fn valid_package(s: &str) -> bool {
    let first = |c: char| c.is_ascii_alphanumeric() || c == '@';
    s.chars().next().is_some_and(first) && all(s, |c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '_' | ':' | '@' | '/' | '-'))
}

/// The arguments of `add` and `build`. `overlay_dir` is where --overlay names live.
fn parse_build_args(args: &[String], overlay_dir: &Path) -> Result<Request, Refused> {
    let mut r = Request::default();
    let mut i = 0;
    while i < args.len() {
        let next = args.get(i + 1).map(String::as_str).unwrap_or("");
        match args[i].as_str() {
            "--copr" => {
                if !valid_copr(next) {
                    return Err(Refused::Text("--copr dono/projeto".into()));
                }
                r.coprs.push(next.into());
                i += 1;
            }
            "--rpmfusion" => r.rpmfusion = true,
            "--repo" => {
                if !valid_repo(next) {
                    return Err(Refused::Text("--repo id=https://baseurl[=https://gpgkey]".into()));
                }
                r.repos.push(next.into());
                i += 1;
            }
            "--unit" => {
                if !valid_unit(next) {
                    return Err(Refused::Text("--unit name.service|socket|timer|path".into()));
                }
                r.units.push(next.into());
                i += 1;
            }
            "--overlay" => {
                if !valid_overlay_name(next) || !overlay_dir.join(next).is_dir() {
                    return Err(Refused::Text(format!("--overlay <name in {}>", overlay_dir.display())));
                }
                r.overlay = next.into();
                i += 1;
            }
            "--allow-upgrade" => r.allow_upgrade = true,
            "-o" => {
                if next.is_empty() {
                    return Err(Refused::Text("-o <arquivo>".into()));
                }
                r.out = next.into();
                i += 1;
            }
            opt if opt.starts_with('-') => return Err(Refused::Key("unknown option", format!(": {opt}"))),
            word => {
                if r.name.is_empty() {
                    r.name = word.into();
                } else {
                    r.pkgs.push(word.into());
                }
            }
        }
        i += 1;
    }
    if r.name.is_empty() {
        return Err(Refused::Usage);
    }
    if !valid_name(&r.name) {
        return Err(Refused::Key("invalid name", format!(": '{}'", r.name)));
    }
    if r.pkgs.is_empty() {
        return Err(Refused::Key("specify at least one package", String::new()));
    }
    if let Some(p) = r.pkgs.iter().find(|p| !valid_package(p)) {
        return Err(Refused::Key("invalid package", format!(": {p}")));
    }
    Ok(r)
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// One item per line, as `printf '%s\n' "${list[@]}"` writes it (an empty
/// list is a single empty line).
fn lines_of(list: &[String]) -> String {
    if list.is_empty() {
        "\n".into()
    } else {
        list.iter().map(|s| format!("{s}\n")).collect()
    }
}

/// The non-empty lines of a bookkeeping file.
fn list_of(text: &str) -> Vec<String> {
    text.lines().filter(|l| !l.is_empty()).map(String::from).collect()
}

/// A value of an os-release file (what sourcing it in a shell would give).
fn os_release_value(text: &str, key: &str) -> String {
    for line in text.lines() {
        if let Some(v) = line.trim().strip_prefix(key).and_then(|rest| rest.strip_prefix('=')) {
            let v = v.trim();
            let unquoted = v
                .strip_prefix('"')
                .and_then(|x| x.strip_suffix('"'))
                .or_else(|| v.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
                .unwrap_or(v);
            return unquoted.to_string();
        }
    }
    String::new()
}

/// The version of the running image: IMAGE_VERSION, else OSTREE_VERSION.
fn image_version_of(text: &str) -> String {
    let v = os_release_value(text, "IMAGE_VERSION");
    if v.is_empty() {
        os_release_value(text, "OSTREE_VERSION")
    } else {
        v
    }
}

fn in_path(program: &str) -> bool {
    env::var_os("PATH")
        .map(|p| env::split_paths(&p).any(|d| fs::metadata(d.join(program)).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)))
        .unwrap_or(false)
}

/// A fresh private folder named <prefix>XXXXXX inside `parent`.
fn temp_dir(parent: &Path, prefix: &str) -> std::io::Result<PathBuf> {
    let mut noise = [0u8; 8];
    for attempt in 0..100u32 {
        let from_kernel = fs::File::open("/dev/urandom").and_then(|mut f| std::io::Read::read_exact(&mut f, &mut noise));
        if from_kernel.is_err() {
            let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
            noise = ((t as u64) ^ (u64::from(std::process::id()) << 32) ^ u64::from(attempt)).to_le_bytes();
        }
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let suffix: String = noise[..6].iter().map(|b| ALPHABET[usize::from(*b) % ALPHABET.len()] as char).collect();
        let dir = parent.join(format!("{prefix}{suffix}"));
        match fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "no free temporary name"))
}

/// The work folder of a build. Inside `podman unshare` its files belong to
/// the user's sub-ids, so it has to be removed from in there too.
struct WorkDir(PathBuf);

impl Drop for WorkDir {
    fn drop(&mut self) {
        if is_root() {
            let _ = fs::remove_dir_all(&self.0);
        } else {
            let _ = Command::new("podman").args(["unshare", "rm", "-rf"]).arg(&self.0).status();
        }
    }
}

/// What a build produced.
struct Built {
    /// "name version" lines of the packages the layer adds.
    packages: String,
    /// The /etc files of those packages, as a tar next to the layer (if any).
    etc_tar: Option<PathBuf>,
}

// ── The program ─────────────────────────────────────────────────────────────

struct App {
    tr: Catalog,
    ext: PathBuf,
    meta: PathBuf,
    off: PathBuf,
    overlays: PathBuf,
    root: bool,
}

/// A step that failed: the exit code of the program.
type Step<T = ()> = Result<T, i32>;

/// A file operation that failed: says why, and becomes exit code 1.
fn io_failed(what: &Path) -> impl Fn(std::io::Error) -> i32 + '_ {
    move |e| {
        eprintln!("tamagi-layer: {}: {e}", what.display());
        1
    }
}

impl App {
    fn new() -> App {
        let dir = |var: &str, default: &str| PathBuf::from(env::var(var).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| default.into()));
        App {
            tr: Catalog::load(),
            ext: dir("TAMAGI_LAYER_DIR", "/var/lib/extensions"),
            meta: dir("TAMAGI_LAYER_META", "/var/lib/tamagi/layers"),
            off: dir("TAMAGI_LAYER_OFF", "/var/lib/tamagi/layers-off"),
            overlays: dir("TAMAGI_LAYER_OVERLAYS", "/usr/share/tamagi/layer-overlays"),
            root: is_root(),
        }
    }

    fn t<'a>(&'a self, text: &'a str) -> &'a str {
        self.tr.t(text)
    }

    fn die(&self, msg: &str) -> ! {
        eprintln!("tamagi-layer: {msg}");
        exit(1)
    }

    fn usage(&self, code: i32) -> ! {
        let t = |s| self.tr.t(s);
        println!("tamagi-layer — {}", t("Fedora packages layered over the image (systemd-sysext)"));
        println!();
        println!("  tamagi-layer add <nome> <pacote>…    {}", t("create and enable the layer (needs password)"));
        println!("  tamagi-layer remove <nome>           {}", t("disable and delete the layer"));
        println!("  tamagi-layer list                    {}", t("layers, packages, and status"));
        println!("  tamagi-layer rebuild [nome]          {}", t("rebuild layers against the current image"));
        println!("  tamagi-layer off | on                {}", t("disable / re-enable all layers (without deleting)"));
        println!("  tamagi-layer temp                    {}", t("writable /usr until next reboot"));
        println!("  tamagi-layer build <nome> <pacote>…  {}", t("only build the file (-o <file>), without installing"));
        println!("  --copr owner/project   --rpmfusion   --allow-upgrade");
        println!("  --repo id=baseurl[=gpgkey-url]       {}", t("extra RPM repository (e.g. Google Chrome's)"));
        println!("  --unit name.socket                   {}", t("systemd unit the layer starts at every boot"));
        println!("  --overlay name                       {}", t("extra files of the image that go in with the layer"));
        exit(code)
    }

    fn refuse(&self, why: Refused) -> ! {
        match why {
            Refused::Usage => self.usage(2),
            Refused::Text(msg) => self.die(&msg),
            Refused::Key(key, rest) => self.die(&format!("{}{rest}", self.t(key))),
        }
    }

    fn image_version(&self) -> String {
        image_version_of(&fs::read_to_string("/usr/lib/os-release").unwrap_or_default())
    }

    /// A command that needs root: as it is when we are root, through sudo otherwise.
    fn sudo<S: AsRef<OsStr>>(&self, program: &str, args: &[S]) -> Command {
        let mut c = if self.root {
            Command::new(program)
        } else {
            let mut c = Command::new("sudo");
            c.arg(program);
            c
        };
        c.args(args);
        c
    }

    /// Runs it; a command that fails (or cannot start) gives its exit code.
    fn run(&self, mut c: Command) -> Step {
        match c.status() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(s.code().unwrap_or(1)),
            Err(_) => Err(127),
        }
    }

    /// Runs it with its complaints hidden; whether it worked does not matter.
    fn quietly(&self, mut c: Command) {
        let _ = c.stderr(Stdio::null()).status();
    }

    /// Writes `content` to a root-owned file (mode 644).
    fn write_as_root(&self, path: &Path, content: &str) -> Step {
        let mut c = self.sudo("install", &[OsStr::new("-m"), OsStr::new("644"), OsStr::new("/dev/stdin"), path.as_os_str()]);
        let mut child = c.stdin(Stdio::piped()).spawn().map_err(|_| 127)?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(content.as_bytes());
        }
        match child.wait() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(s.code().unwrap_or(1)),
            Err(_) => Err(1),
        }
    }

    fn raw_names(dir: &Path) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(dir)
            .map(|rd| rd.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".raw")).map(String::from)).collect())
            .unwrap_or_default();
        v.sort();
        v
    }

    /// Every layer, on or off, in name order.
    fn names(&self) -> Vec<String> {
        let mut v = Self::raw_names(&self.ext);
        v.extend(Self::raw_names(&self.off));
        v.sort();
        v.dedup();
        v
    }

    /// A bookkeeping file of a layer, without the trailing line break.
    fn meta(&self, name: &str, kind: &str) -> String {
        fs::read_to_string(self.meta.join(format!("{name}.{kind}"))).unwrap_or_default().trim_end_matches('\n').to_string()
    }

    /// "ligada", "desligada" or "desatualizada" (made for another image version).
    fn state_of(&self, name: &str) -> &'static str {
        if !self.ext.join(format!("{name}.raw")).exists() {
            "desligada"
        } else if self.meta(name, "image") == self.image_version() {
            "ligada"
        } else {
            "desatualizada"
        }
    }

    // ── Building ────────────────────────────────────────────────────────────

    /// Builds the layer of `r` into `out`. The exit code of the inner script
    /// is the error: 3 nothing to add, 4 would upgrade the image, 5 nothing in /usr.
    fn build_layer(&self, r: &Request, out: &Path) -> Step<Built> {
        if !in_path("mkfs.erofs") {
            self.die(self.t("mkfs.erofs missing (erofs-utils package)"));
        }
        let cache = env::var_os("XDG_CACHE_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".cache"))
            .join("tamagi");
        fs::create_dir_all(&cache).map_err(io_failed(&cache))?;
        let work = WorkDir(temp_dir(&cache, "layer.").map_err(io_failed(&cache))?);
        let w = &work.0;
        let put = |file: &str, text: String| fs::write(w.join(file), text).map_err(io_failed(w));
        put("request", lines_of(&r.pkgs))?;
        put("coprs", lines_of(&r.coprs))?;
        put("rpmfusion", format!("{}\n", u8::from(r.rpmfusion)))?;
        put("repos", lines_of(&r.repos))?;
        put("overlay", if r.overlay.is_empty() { String::new() } else { format!("{}\n", self.overlays.join(&r.overlay).display()) })?;
        put("inner.sh", INNER.to_string())?;

        let mut c = if self.root {
            Command::new("bash")
        } else {
            let mut c = Command::new("podman");
            c.args(["unshare", "bash"]);
            c
        };
        c.arg(w.join("inner.sh")).arg(w).arg(&r.name).arg(if r.allow_upgrade { "1" } else { "0" });
        c.env("MSG_ALREADY", self.t("nothing to do: all requested packages are already in the image"))
            .env("MSG_UPGRADED", self.t("these base image packages would be upgraded by the layer:"))
            .env("MSG_UPGRADE_HINT", self.t("update the system first (tamagi-update) or retry with --allow-upgrade"))
            .env("MSG_NOUSR", self.t("packages install nothing into /usr"));
        self.run(c)?;

        fs::copy(w.join("layer.raw"), out).map_err(io_failed(out))?;
        let packages = fs::read_to_string(w.join("packages")).unwrap_or_default().trim_end_matches('\n').to_string();
        let mut etc_tar = None;
        if fs::metadata(w.join("etc.list")).map(|m| m.len() > 0).unwrap_or(false) {
            let mut name = out.as_os_str().to_os_string();
            name.push(".etc.tar");
            let tar = PathBuf::from(name);
            fs::copy(w.join("etc.tar"), &tar).map_err(io_failed(&tar))?;
            etc_tar = Some(tar);
        }
        Ok(Built { packages, etc_tar })
    }

    /// Puts a built layer in place and records what it was made from.
    fn install_layer(&self, r: &Request, raw: &Path, built: &Built) -> Step {
        let name = &r.name;
        println!("{} '{name}' ({})…", self.t("Enabling layer"), self.t("needs password"));
        let o = OsStr::new;
        self.run(self.sudo("install", &[o("-d"), o("-m"), o("755"), self.ext.as_os_str(), self.meta.as_os_str()]))?;
        self.run(self.sudo("install", &[o("-m"), o("644"), raw.as_os_str(), self.ext.join(format!("{name}.raw")).as_os_str()]))?;
        self.run(self.sudo("rm", &[o("-f"), self.off.join(format!("{name}.raw")).as_os_str()]))?;
        let record = |kind: &str, text: String| self.write_as_root(&self.meta.join(format!("{name}.{kind}")), &text);
        record("request", lines_of(&r.pkgs))?;
        record("coprs", lines_of(&r.coprs))?;
        record("rpmfusion", format!("{}\n", u8::from(r.rpmfusion)))?;
        record("repos", lines_of(&r.repos))?;
        record("units", lines_of(&r.units))?;
        record("overlay", format!("{}\n", r.overlay))?;
        record("packages", format!("{}\n", built.packages))?;
        record("image", format!("{}\n", self.image_version()))?;
        if let Some(tar) = &built.etc_tar {
            // Configuration the packages bring: only what is not there yet.
            self.run(self.sudo("tar", &[o("-C"), o("/"), o("--skip-old-files"), o("--no-same-owner"), o("-xf"), tar.as_os_str()]))?;
            self.quietly(self.sudo("restorecon", &["-R", "/etc"]));
            let _ = fs::remove_file(tar);
        }
        self.refresh()
    }

    fn refresh(&self) -> Step {
        if in_path("restorecon") {
            self.quietly(self.sudo("restorecon", &[o("-R"), self.ext.as_os_str()]));
        }
        self.run(self.sudo("systemd-sysext", &["refresh"]))?;
        self.settle();
        Ok(())
    }

    /// What the packages' scriptlets would have done (the layer installs
    /// without them): users and groups, folders in /var and /opt links,
    /// library cache, udev rules — and the units each layer asked for (--unit).
    fn settle(&self) {
        if in_path("ldconfig") {
            self.quietly(self.sudo::<&str>("ldconfig", &[]));
        }
        self.quietly(self.sudo::<&str>("systemd-sysusers", &[]));
        self.quietly(self.sudo("systemd-tmpfiles", &["--create"]));
        self.quietly(self.sudo("systemctl", &["daemon-reload"]));
        self.quietly(self.sudo("udevadm", &["control", "--reload"]));
        let mut unit_files: Vec<String> = fs::read_dir(&self.meta)
            .map(|rd| rd.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".units")).map(String::from)).collect())
            .unwrap_or_default();
        unit_files.sort();
        for name in unit_files {
            if !self.ext.join(format!("{name}.raw")).exists() {
                continue;
            }
            for unit in list_of(&self.meta(&name, "units")) {
                self.quietly(self.sudo("systemctl", &["start", "--no-block", &unit]));
            }
        }
    }

    // ── Commands ────────────────────────────────────────────────────────────

    fn cmd_add(&self, args: &[String]) -> Step {
        let r = parse_build_args(args, &self.overlays).unwrap_or_else(|e| self.refuse(e));
        let tmp = temp_dir(&env::temp_dir(), "tmp.").map_err(io_failed(&env::temp_dir()))?;
        let raw = tmp.join(format!("{}.raw", r.name));
        let result = self.build_layer(&r, &raw).and_then(|built| self.install_layer(&r, &raw, &built).map(|()| built));
        let _ = fs::remove_dir_all(&tmp);
        let built = result?;
        println!(
            "{}: '{}' ({} {}, {}).",
            self.t("Layer enabled"),
            r.name,
            built.packages.lines().count().max(1),
            self.t("package(s)"),
            self.image_version()
        );
        println!("{}: tamagi-layer remove {}", self.t("To undo"), r.name);
        Ok(())
    }

    fn cmd_build(&self, args: &[String]) -> Step {
        let r = parse_build_args(args, &self.overlays).unwrap_or_else(|e| self.refuse(e));
        let out = if r.out.is_empty() {
            env::current_dir().unwrap_or_default().join(format!("{}.raw", r.name))
        } else {
            PathBuf::from(&r.out)
        };
        let built = self.build_layer(&r, &out)?;
        println!("{}: {} ({} {})", self.t("Generated"), out.display(), built.packages.lines().count().max(1), self.t("package(s)"));
        if let Some(tar) = &built.etc_tar {
            println!("{}: {}", self.t("Package /etc files"), tar.display());
        }
        Ok(())
    }

    fn cmd_remove(&self, args: &[String]) -> Step {
        let name = match args.first().filter(|n| !n.is_empty()) {
            Some(n) => n,
            None => self.die(&format!("{}: tamagi-layer remove <nome>", self.t("usage"))),
        };
        if !valid_name(name) {
            self.die(&format!("{}: '{name}'", self.t("invalid name")));
        }
        let on = self.ext.join(format!("{name}.raw"));
        let off = self.off.join(format!("{name}.raw"));
        if !on.exists() && !off.exists() {
            self.die(&format!("{} '{name}'", self.t("layer does not exist")));
        }
        for unit in list_of(&self.meta(name, "units")) {
            self.quietly(self.sudo("systemctl", &["stop", &unit]));
        }
        let mut gone = vec![OsStr::new("-f").to_os_string(), on.into_os_string(), off.into_os_string()];
        gone.extend(META_KINDS.iter().map(|kind| self.meta.join(format!("{name}.{kind}")).into_os_string()));
        self.run(self.sudo("rm", &gone))?;
        self.refresh()?;
        println!("{}: '{name}'.", self.t("Layer removed"));
        Ok(())
    }

    fn cmd_list(&self) -> Step {
        let names = self.names();
        for name in &names {
            let request: String = self.meta(name, "request").lines().map(|l| format!("{l} ")).collect();
            println!("{name:<20} {:<14} {request}", self.t(self.state_of(name)));
        }
        if names.is_empty() {
            println!("{}: tamagi-layer add ferramentas strace ltrace", self.t("No layers yet. Example"));
        }
        Ok(())
    }

    /// Rebuilds the layers that are on (all, or the one named) for the image
    /// in use. A layer that cannot be rebuilt keeps its previous file.
    fn cmd_rebuild(&self, args: &[String]) -> Step {
        let only = args.first().map(String::as_str).unwrap_or("");
        let mut rc = 0;
        for name in self.names() {
            if (!only.is_empty() && only != name) || !self.ext.join(format!("{name}.raw")).exists() {
                continue;
            }
            let r = Request {
                pkgs: list_of(&self.meta(&name, "request")),
                coprs: list_of(&self.meta(&name, "coprs")),
                repos: list_of(&self.meta(&name, "repos")),
                units: list_of(&self.meta(&name, "units")),
                rpmfusion: self.meta(&name, "rpmfusion") == "1",
                overlay: self.meta(&name, "overlay"),
                name: name.clone(),
                ..Request::default()
            };
            if r.pkgs.is_empty() {
                eprintln!("{} '{name}': {}", self.t("layer"), self.t("missing package list, cannot rebuild"));
                rc = 1;
                continue;
            }
            println!("── {} '{name}' ({})", self.t("Rebuilding layer"), self.image_version());
            let Ok(tmp) = temp_dir(&env::temp_dir(), "tmp.") else {
                rc = 1;
                continue;
            };
            let raw = tmp.join(format!("{name}.raw"));
            let done = self.build_layer(&r, &raw).and_then(|built| self.install_layer(&r, &raw, &built));
            if done.is_err() {
                eprintln!("{} '{name}': {}", self.t("layer"), self.t("could not rebuild; kept previous version"));
                rc = 1;
            }
            let _ = fs::remove_dir_all(&tmp);
        }
        if rc == 0 {
            Ok(())
        } else {
            Err(rc)
        }
    }

    fn cmd_stale(&self) -> Step {
        for name in self.names() {
            if self.state_of(&name) == "desatualizada" {
                println!("{name}");
            }
        }
        Ok(())
    }

    /// Moves every layer file from one folder to the other (off ↔ on).
    fn move_all(&self, from: &Path, to: &Path, none: &str) -> Step<bool> {
        if Self::raw_names(from).is_empty() {
            println!("{}", self.t(none));
            return Ok(false);
        }
        self.run(self.sudo("install", &[o("-d"), o("-m"), o("755"), to.as_os_str()]))?;
        let script = format!("mv '{}'/*.raw '{}'/", from.display(), to.display());
        self.run(self.sudo("sh", &["-c", &script]))?;
        self.refresh()?;
        Ok(true)
    }

    fn cmd_off(&self) -> Step {
        if self.move_all(&self.ext, &self.off, "No active layers.")? {
            println!("{}: tamagi-layer on", self.t("Layers disabled. To re-enable"));
        }
        Ok(())
    }

    fn cmd_on(&self) -> Step {
        if self.move_all(&self.off, &self.ext, "No disabled layers.")? {
            println!("{}", self.t("Layers re-enabled."));
        }
        Ok(())
    }

    fn cmd_temp(&self) -> Step {
        println!("{}", self.t("/usr is writable until the next reboot; all changes vanish on reboot."));
        let _ = std::io::stdout().flush();
        let err = self.sudo("bootc", &["usr-overlay"]).exec();
        eprintln!("tamagi-layer: bootc: {err}");
        Err(127)
    }
}

fn o(s: &str) -> &OsStr {
    OsStr::new(s)
}

fn main() {
    let argv: Vec<String> = env::args().skip(1).collect();
    let app = App::new();
    let rest = argv.get(1..).unwrap_or(&[]);
    let done = match argv.first().map(String::as_str).unwrap_or("") {
        "add" => app.cmd_add(rest),
        "build" => app.cmd_build(rest),
        "remove" | "rm" => app.cmd_remove(rest),
        "list" | "ls" => app.cmd_list(),
        "rebuild" => app.cmd_rebuild(rest),
        "stale" => app.cmd_stale(),
        "off" => app.cmd_off(),
        "on" => app.cmd_on(),
        "temp" => app.cmd_temp(),
        // tamagi-layer-units.service, after systemd-sysext at boot
        "boot" => {
            app.settle();
            Ok(())
        }
        "-h" | "--help" | "help" => app.usage(0),
        _ => app.usage(2),
    };
    if let Err(code) = done {
        exit(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    fn parse(v: &[&str]) -> Result<Request, Refused> {
        parse_build_args(&strs(v), Path::new("/nonexistent-overlays"))
    }

    #[test]
    fn names() {
        assert!(valid_name("virt") && valid_name("browser-chrome") && valid_name("a1"));
        assert!(!valid_name("") && !valid_name("Ruim Nome") && !valid_name("1a") && !valid_name("-a") && !valid_name("a_b"));
        assert!(valid_name(&format!("a{}", "b".repeat(39))));
        assert!(!valid_name(&format!("a{}", "b".repeat(40))));
    }

    #[test]
    fn coprs() {
        assert!(valid_copr("owner/project") && valid_copr("@group/pro.ject-1"));
        assert!(!valid_copr("owner") && !valid_copr("owner/") && !valid_copr("/project") && !valid_copr("a/b/c") && !valid_copr("a/b c"));
        assert!(!valid_copr("owner/@project"));
    }

    #[test]
    fn repos() {
        assert!(valid_repo("docker-ce=https://download.docker.com/linux/fedora/45/x86_64/stable=https://download.docker.com/linux/fedora/gpg"));
        assert!(valid_repo("chrome=https://dl.google.com/linux/chrome/rpm/stable/$basearch"));
        assert!(!valid_repo("ruim"));
        assert!(!valid_repo("x=http://insecure.example/repo"));
        assert!(!valid_repo("X=https://example.org/r"));
        assert!(!valid_repo("x=https://example.org/r,--setopt=evil"));
        assert!(!valid_repo("x=https://example.org/r=https://k=https://extra"));
        assert!(!valid_repo("x=https://example.org/a b"));
        assert!(!valid_repo("x=https://"));
        assert!(!valid_repo("=https://example.org/r"));
    }

    #[test]
    fn units() {
        assert!(valid_unit("docker.socket") && valid_unit("virtqemud.service") && valid_unit("a@b.timer") && valid_unit("x.path"));
        assert!(!valid_unit("ruim") && !valid_unit(".service") && !valid_unit("a b.service") && !valid_unit("x.mount") && !valid_unit("x.service;y"));
    }

    #[test]
    fn packages() {
        assert!(valid_package("cowsay") && valid_package("mesa-vulkan-drivers.i686") && valid_package("@virtualization") && valid_package("gcc-c++"));
        assert!(!valid_package("x;rm") && !valid_package("-x") && !valid_package("a b") && !valid_package("") && !valid_package("$(x)"));
    }

    #[test]
    fn parses_a_full_request() {
        let r = parse(&["virt", "--rpmfusion", "--copr", "o/p", "--repo", "a=https://e.org/r", "--unit", "x.socket", "--allow-upgrade", "-o", "/tmp/x.raw", "qemu-kvm", "virt-manager"]).unwrap();
        assert_eq!(r.name, "virt");
        assert_eq!(r.pkgs, strs(&["qemu-kvm", "virt-manager"]));
        assert_eq!(r.coprs, strs(&["o/p"]));
        assert_eq!(r.repos, strs(&["a=https://e.org/r"]));
        assert_eq!(r.units, strs(&["x.socket"]));
        assert!(r.rpmfusion && r.allow_upgrade);
        assert_eq!(r.out, "/tmp/x.raw");
        assert_eq!(r.overlay, "");
    }

    #[test]
    fn refuses_what_the_shell_version_refused() {
        assert_eq!(parse(&[]), Err(Refused::Usage));
        assert_eq!(parse(&["--rpmfusion"]), Err(Refused::Usage));
        assert_eq!(parse(&["ok"]), Err(Refused::Key("specify at least one package", String::new())));
        assert_eq!(parse(&["Ruim Nome", "cowsay"]), Err(Refused::Key("invalid name", ": 'Ruim Nome'".into())));
        assert_eq!(parse(&["ok", "x;rm"]), Err(Refused::Key("invalid package", ": x;rm".into())));
        assert_eq!(parse(&["x", "--repo", "ruim", "cowsay"]), Err(Refused::Text("--repo id=https://baseurl[=https://gpgkey]".into())));
        assert_eq!(parse(&["x", "--unit", "ruim", "cowsay"]), Err(Refused::Text("--unit name.service|socket|timer|path".into())));
        assert_eq!(parse(&["x", "--copr", "ruim", "cowsay"]), Err(Refused::Text("--copr dono/projeto".into())));
        assert_eq!(parse(&["x", "--nope", "cowsay"]), Err(Refused::Key("unknown option", ": --nope".into())));
        assert_eq!(parse(&["x", "cowsay", "--overlay", "gaming"]), Err(Refused::Text("--overlay <name in /nonexistent-overlays>".into())));
        assert_eq!(parse(&["x", "cowsay", "--repo"]), Err(Refused::Text("--repo id=https://baseurl[=https://gpgkey]".into())));
        assert_eq!(parse(&["x", "cowsay", "-o"]), Err(Refused::Text("-o <arquivo>".into())));
    }

    #[test]
    fn overlay_must_exist() {
        let dir = env::temp_dir().join(format!("tamagi-layer-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("gaming")).unwrap();
        let r = parse_build_args(&strs(&["gaming", "--overlay", "gaming", "steam"]), &dir).unwrap();
        assert_eq!(r.overlay, "gaming");
        assert!(parse_build_args(&strs(&["gaming", "--overlay", "other", "steam"]), &dir).is_err());
        assert!(parse_build_args(&strs(&["gaming", "--overlay", "../gaming", "steam"]), &dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn bookkeeping_text() {
        assert_eq!(lines_of(&[]), "\n");
        assert_eq!(lines_of(&strs(&["a", "b"])), "a\nb\n");
        assert_eq!(list_of("a\n\nb\n"), strs(&["a", "b"]));
        assert!(list_of("\n").is_empty());
    }

    #[test]
    fn image_version_from_os_release() {
        assert_eq!(image_version_of("NAME=Tamagi\nIMAGE_VERSION=\"45.20261122\"\nOSTREE_VERSION='45.1'\n"), "45.20261122");
        assert_eq!(image_version_of("NAME=Tamagi\nOSTREE_VERSION='45.1'\n"), "45.1");
        assert_eq!(image_version_of("IMAGE_VERSION=\nOSTREE_VERSION=45.2\n"), "45.2");
        assert_eq!(image_version_of("NAME=Fedora\n"), "");
        assert_eq!(os_release_value("XIMAGE_VERSION=1\nIMAGE_VERSION_ID=2\n", "IMAGE_VERSION"), "");
    }

    #[test]
    fn the_inner_script_is_the_shell_one() {
        assert!(INNER.starts_with("set -euo pipefail\n"));
        assert!(INNER.contains("usr/lib/opt") && INNER.contains("tamagi-layer-$name-var.conf") && INNER.contains("%{NAME}.%{ARCH}"));
        assert!(INNER.trim_end().ends_with("chmod 0644 \"$work/layer.raw\""));
    }
}
