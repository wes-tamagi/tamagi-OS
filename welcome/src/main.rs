//! tamagi-welcome — the first-login choices of TamagiOS, as a native COSMIC
//! window: a short list of basics to tick and a browser, like the first boot
//! of openSUSE Aeon. Every item says how it is installed; the rest of the
//! catalog is `tamagi install`.
//!
//! It only collects the choices. Installing is done by the same back ends the
//! terminal assistant (`tamagi-firstrun`) uses: `tamagi-install --actions`
//! and `tamagi-setup tpm-unlock on`.

mod app;
mod askpass;
mod catalog;
mod cli;
mod flow;
mod i18n;
mod m3;
mod method;
mod plan;
mod progress;
mod review;
mod setup;
mod setup_app;
mod setup_system;
mod system;
mod ui;

/// Set for the installers this program starts; see `cli`.
pub const ASKPASS_ENV: &str = "TAMAGI_WELCOME_ASKPASS";

/// The real sudo, for when this program is called as `sudo` (see
/// `system::sudo_shim`).
pub const REAL_SUDO_ENV: &str = "TAMAGI_WELCOME_REAL_SUDO";

fn main() -> std::process::ExitCode {
    use std::process::ExitCode;
    let mut argv = std::env::args();
    let program = argv.next().unwrap_or_default();
    let args: Vec<String> = argv.collect();
    // Called through the `sudo` link the installers find first in PATH:
    // hand over to the real sudo, asking for the password in the dialog.
    if std::path::Path::new(&program).file_name().is_some_and(|name| name == "sudo") {
        use std::os::unix::process::CommandExt;
        let real = std::env::var_os(REAL_SUDO_ENV).filter(|v| !v.is_empty()).unwrap_or_else(|| "/usr/bin/sudo".into());
        let error = std::process::Command::new(&real).args(cli::sudo_args(&args)).exec();
        eprintln!("tamagi-welcome: {}: {error}", real.to_string_lossy());
        return ExitCode::from(1);
    }
    let mode = match cli::parse(&args, std::env::var_os(ASKPASS_ENV).is_some()) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("tamagi-welcome: {message}\n{}", cli::USAGE);
            return ExitCode::from(2);
        }
    };
    match mode {
        cli::Mode::Help => {
            println!("{}", cli::USAGE);
            ExitCode::SUCCESS
        }
        cli::Mode::Install => match setup_app::run() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("tamagi-welcome: {error}");
                ExitCode::FAILURE
            }
        },
        cli::Mode::Version => {
            println!("tamagi-welcome {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        cli::Mode::Askpass(prompt) => match askpass::run(prompt) {
            Some(password) => {
                println!("{password}");
                ExitCode::SUCCESS
            }
            None => ExitCode::FAILURE,
        },
        cli::Mode::Welcome { force } => {
            if !force && system::stamp_path().is_file() {
                return ExitCode::SUCCESS;
            }
            // The big part of the first Flatpak app is the shared runtime: it
            // starts downloading now, while the choices are made. If nothing
            // ends up using it, it is removed at the end.
            system::spawn_quiet(&["tamagi-install", "prefetch"]);
            // Clock: 24 hours, except in English, which uses AM/PM.
            system::write_clock(i18n::lang());
            // The terminal apps start in step with the COSMIC theme.
            system::spawn_quiet(&["tamagi-theme", "sync"]);
            let result = app::run();
            system::remove_sudo_shim();
            match result {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("tamagi-welcome: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// `system_files/usr/share/tamagi` of the repository, for the tests that
/// check the real catalog and profiles (TAMAGI_TEST_SHARE overrides it).
#[cfg(test)]
pub(crate) fn test_share_dir() -> Option<std::path::PathBuf> {
    let dir = std::env::var_os("TAMAGI_TEST_SHARE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../system_files/usr/share/tamagi")));
    dir.join("catalog.tsv").is_file().then_some(dir)
}
