//! The command line.
//!
//!   tamagi-welcome                    the first-login choices (once)
//!   tamagi-welcome --force            shows them again
//!   tamagi-welcome --askpass PROMPT   password dialog; prints it to stdout
//!
//! `sudo` runs its askpass helper as `$SUDO_ASKPASS "<prompt>"`, with no room
//! for a flag. The installers are started with SUDO_ASKPASS pointing at this
//! program and TAMAGI_WELCOME_ASKPASS=1: with that variable set, a lone
//! argument is the prompt.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Welcome { force: bool },
    Askpass(String),
    Help,
    Version,
}

pub const USAGE: &str = "tamagi-welcome [--force] | --askpass <prompt> | --help | --version";

/// `args` without the program name; `askpass_env` is whether
/// TAMAGI_WELCOME_ASKPASS is set.
pub fn parse(args: &[String], askpass_env: bool) -> Result<Mode, String> {
    let mut force = false;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--askpass" => return Ok(Mode::Askpass(iter.next().cloned().unwrap_or_default())),
            "--force" => force = true,
            "-h" | "--help" => return Ok(Mode::Help),
            "-V" | "--version" => return Ok(Mode::Version),
            prompt if askpass_env && !prompt.starts_with("--") => return Ok(Mode::Askpass(prompt.to_string())),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(Mode::Welcome { force })
}

/// The arguments for the real `sudo` when this program stands in for it (see
/// `system::sudo_shim`): `-A` first, so the password is always asked for
/// through the dialog. sudo only does that by itself when DISPLAY is set
/// (WAYLAND_DISPLAY is not enough). `-A` and `-S` cannot go together, so
/// anything that looks like `-S` leaves the arguments alone.
pub fn sudo_args(args: &[String]) -> Vec<String> {
    let reads_stdin_or_asks = args.iter().take_while(|a| *a != "--").any(|a| {
        a == "--stdin" || a == "--askpass" || (a.starts_with('-') && !a.starts_with("--") && (a.contains('S') || a.contains('A')))
    });
    let mut out = Vec::with_capacity(args.len() + 1);
    if !reads_stdin_or_asks {
        out.push("-A".to_string());
    }
    out.extend(args.iter().cloned());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sudo_gets_askpass() {
        assert_eq!(sudo_args(&args(&["true"])), args(&["-A", "true"]));
        assert_eq!(sudo_args(&args(&["-k"])), args(&["-A", "-k"]));
        assert_eq!(sudo_args(&args(&["-n", "true"])), args(&["-A", "-n", "true"]));
        assert_eq!(sudo_args(&args(&["-u", "root", "sh", "-c", "id"])), args(&["-A", "-u", "root", "sh", "-c", "id"]));
        assert_eq!(sudo_args(&args(&[])), args(&["-A"]));
        // Already asking through askpass, or reading the password from stdin.
        assert_eq!(sudo_args(&args(&["-A", "true"])), args(&["-A", "true"]));
        assert_eq!(sudo_args(&args(&["-S", "true"])), args(&["-S", "true"]));
        assert_eq!(sudo_args(&args(&["-kS", "true"])), args(&["-kS", "true"]));
        assert_eq!(sudo_args(&args(&["-u", "root", "--stdin", "true"])), args(&["-u", "root", "--stdin", "true"]));
        // After "--" everything belongs to the command.
        assert_eq!(sudo_args(&args(&["--", "pacman", "-S", "x"])), args(&["-A", "--", "pacman", "-S", "x"]));
    }

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn welcome() {
        assert_eq!(parse(&args(&[]), false), Ok(Mode::Welcome { force: false }));
        assert_eq!(parse(&args(&["--force"]), false), Ok(Mode::Welcome { force: true }));
        // The variable alone changes nothing: flags still mean what they say.
        assert_eq!(parse(&args(&["--force"]), true), Ok(Mode::Welcome { force: true }));
        assert_eq!(parse(&args(&[]), true), Ok(Mode::Welcome { force: false }));
    }

    #[test]
    fn askpass_flag() {
        assert_eq!(parse(&args(&["--askpass", "Password:"]), false), Ok(Mode::Askpass("Password:".into())));
        assert_eq!(parse(&args(&["--askpass"]), false), Ok(Mode::Askpass(String::new())));
    }

    #[test]
    fn askpass_as_sudo_calls_it() {
        assert_eq!(parse(&args(&["[sudo] password for ana: "]), true), Ok(Mode::Askpass("[sudo] password for ana: ".into())));
        // Without the variable a stray word is a mistake, not a prompt.
        assert!(parse(&args(&["[sudo] password for ana: "]), false).is_err());
    }

    #[test]
    fn others() {
        assert_eq!(parse(&args(&["--help"]), false), Ok(Mode::Help));
        assert_eq!(parse(&args(&["--version"]), false), Ok(Mode::Version));
        assert!(parse(&args(&["--nope"]), true).is_err());
    }
}
