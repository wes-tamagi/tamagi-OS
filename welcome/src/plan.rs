//! From the choices to the commands that install them. Nothing is installed
//! here: the back ends are `tamagi-install`, `tamagi apply` and `tamagi-setup`,
//! in the same order as the terminal assistant (`tamagi-firstrun`).

/// The browsers offered: (id for `tamagi-browser`, name shown).
pub const BROWSERS: [(&str, &str); 4] = [("firefox", "Firefox"), ("chrome", "Google Chrome"), ("vivaldi", "Vivaldi"), ("chromium", "Chromium")];

/// How a browser is installed: (flag for `tamagi-browser`, description key).
pub const BROWSER_METHODS: [(&str, &str); 3] = [
    ("flatpak", "Flatpak — Flathub app, sandboxed (recommended)"),
    ("native", "native — RPM in a layer over the image (asks for the password)"),
    ("box", "box — in a Fedora box, with hardware video decoding"),
];

/// `run:tamagi-browser install <browser> --<method>`
pub fn browser_action(browser: &str, method: &str) -> String {
    format!("run:tamagi-browser install {browser} --{method}")
}

/// Everything decided in the pages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choices {
    /// A ready-made profile (ai, dev, gamer, office), if one was chosen.
    pub profile: Option<String>,
    /// (browser, method), if a browser was chosen.
    pub browser: Option<(String, String)>,
    /// Catalog actions ticked one by one.
    pub picked: Vec<String>,
    /// Unlock the encrypted disk with the TPM chip.
    pub tpm: bool,
}

impl Choices {
    /// What goes to `tamagi-install --actions`: the browser first, then the
    /// catalog items, without repeats.
    pub fn actions(&self) -> Vec<String> {
        let mut actions: Vec<String> = Vec::new();
        if let Some((browser, method)) = &self.browser {
            actions.push(browser_action(browser, method));
        }
        for action in &self.picked {
            if !actions.contains(action) {
                actions.push(action.clone());
            }
        }
        actions
    }

    pub fn is_empty(&self) -> bool {
        self.profile.is_none() && self.browser.is_none() && self.picked.is_empty() && !self.tpm
    }
}

/// How the output of a command reports its progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stages {
    /// `── [n/total] title` lines (tamagi-install).
    Numbered,
    /// `── title` lines, one per step (tamagi apply).
    Counted,
    /// A single step.
    Single,
}

/// One command of the installation.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub argv: Vec<String>,
    /// English title shown while it runs (a translation key).
    pub title: &'static str,
    pub stages: Stages,
    /// Share of the overall progress bar: about how many stages it has.
    /// 0 = a wait that is not shown as a step.
    pub weight: u32,
}

/// How many `[n/total]` stages `tamagi-install --actions` will print: one for
/// all the Flatpaks, one for the native gaming layer, one per `run:` command
/// and one per agent (the same sum as its `run_actions`).
pub fn install_stage_count(actions: &[String]) -> u32 {
    let has = |prefix: &str| actions.iter().any(|a| a.starts_with(prefix)) as u32;
    let count = |prefix: &str| actions.iter().filter(|a| a.starts_with(prefix)).count() as u32;
    has("flatpak:") + has("gaming:") + count("run:") + count("agent:")
}

/// The commands to run, in order. `profile_items` is how many things the
/// chosen profile installs (only to size its share of the progress bar).
pub fn steps(choices: &Choices, profile_items: usize) -> Vec<Step> {
    let mut steps = Vec::new();
    let argv = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
    if choices.tpm {
        steps.push(Step { argv: argv(&["tamagi-setup", "tpm-unlock", "on"]), title: "Encrypted disk", stages: Stages::Single, weight: 1 });
    }
    let actions = choices.actions();
    if !actions.is_empty() {
        let weight = install_stage_count(&actions).max(1);
        let mut command = argv(&["tamagi-install", "--actions"]);
        command.extend(actions);
        steps.push(Step { argv: command, title: "Browser and items you chose", stages: Stages::Numbered, weight });
    }
    if let Some(profile) = &choices.profile {
        // Two Flatpak operations at once would collide: the profile waits for
        // the runtime that started downloading when the window opened.
        steps.push(Step { argv: argv(&["tamagi-install", "prefetch-wait"]), title: "Waiting for the shared Flatpak runtime", stages: Stages::Single, weight: 0 });
        steps.push(Step {
            argv: argv(&["tamagi", "apply", "-y", profile]),
            title: "Profile",
            stages: Stages::Counted,
            weight: (profile_items as u32).max(1),
        });
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn browser_actions() {
        assert_eq!(browser_action("firefox", "flatpak"), "run:tamagi-browser install firefox --flatpak");
        assert_eq!(browser_action("chrome", "native"), "run:tamagi-browser install chrome --native");
        assert_eq!(browser_action("vivaldi", "box"), "run:tamagi-browser install vivaldi --box");
        for (browser, _) in BROWSERS {
            for (method, _) in BROWSER_METHODS {
                let action = browser_action(browser, method);
                let expected = match method {
                    "flatpak" => crate::method::Method::Flatpak,
                    "native" => crate::method::Method::Native,
                    _ => crate::method::Method::Box,
                };
                assert_eq!(crate::method::classify(&action), expected, "{action}");
            }
        }
    }

    #[test]
    fn actions_put_the_browser_first_and_drop_repeats() {
        let choices = Choices {
            profile: None,
            browser: Some(("firefox".into(), "flatpak".into())),
            picked: s(&["gaming:steam", "agent:claude", "gaming:steam", "run:tamagi-browser install firefox --flatpak"]),
            tpm: false,
        };
        assert_eq!(choices.actions(), s(&["run:tamagi-browser install firefox --flatpak", "gaming:steam", "agent:claude"]));
    }

    #[test]
    fn nothing_chosen_runs_nothing() {
        let choices = Choices::default();
        assert!(choices.is_empty());
        assert!(steps(&choices, 0).is_empty());
    }

    #[test]
    fn profile_with_browser_matches_the_terminal_assistant() {
        let choices = Choices { profile: Some("ai".into()), browser: Some(("chrome".into(), "native".into())), picked: vec![], tpm: false };
        let commands: Vec<Vec<String>> = steps(&choices, 5).into_iter().map(|s| s.argv).collect();
        assert_eq!(
            commands,
            [
                s(&["tamagi-install", "--actions", "run:tamagi-browser install chrome --native"]),
                s(&["tamagi-install", "prefetch-wait"]),
                s(&["tamagi", "apply", "-y", "ai"]),
            ]
        );
    }

    #[test]
    fn tpm_goes_first() {
        let choices = Choices { profile: None, browser: None, picked: s(&["flatpak:flathub:org.gimp.GIMP"]), tpm: true };
        let all = steps(&choices, 0);
        assert_eq!(all[0].argv, s(&["tamagi-setup", "tpm-unlock", "on"]));
        assert_eq!(all[1].argv, s(&["tamagi-install", "--actions", "flatpak:flathub:org.gimp.GIMP"]));
        assert_eq!(all.len(), 2);
        assert!(!choices.is_empty());
        let only_tpm = Choices { tpm: true, ..Choices::default() };
        assert_eq!(steps(&only_tpm, 0).len(), 1);
    }

    #[test]
    fn stage_count_is_the_one_tamagi_install_prints() {
        assert_eq!(install_stage_count(&[]), 0);
        // Flatpaks share one stage, gaming packages another.
        assert_eq!(install_stage_count(&s(&["flatpak:flathub:a.b.C", "flatpak:cosmic:d.e.F", "gaming:steam", "gaming:base"])), 2);
        // Each run: and each agent: is a stage.
        assert_eq!(install_stage_count(&s(&["run:tamagi-vm install", "run:tamagi-devbox create fedora", "agent:claude", "agent:codex", "flatpak:flathub:a.b.C"])), 5);
    }

    #[test]
    fn weights() {
        let choices = Choices { profile: Some("dev".into()), browser: Some(("firefox".into(), "flatpak".into())), picked: s(&["agent:claude", "gaming:steam"]), tpm: true };
        let all = steps(&choices, 6);
        let weights: Vec<u32> = all.iter().map(|s| s.weight).collect();
        assert_eq!(weights, [1, 3, 0, 6]);
        assert_eq!(all[1].stages, Stages::Numbered);
        assert_eq!(all[3].stages, Stages::Counted);
    }
}
