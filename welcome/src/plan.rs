//! From the choices to the commands that install them. Nothing is installed
//! here: the back ends are `tamagi-install` and `tamagi-setup`, in the same
//! order as the terminal assistant (`tamagi-firstrun`).

/// The browsers offered: (id for `tamagi-browser`, name shown).
pub const BROWSERS: [(&str, &str); 4] = [("opera-gx", "Opera GX"), ("chrome", "Google Chrome"), ("chromium", "Chromium"), ("vivaldi", "Vivaldi")];

/// How a browser is installed: (flag for `tamagi-browser`, description key).
pub const BROWSER_METHODS: [(&str, &str); 3] = [
    ("flatpak", "Flatpak — Flathub app, sandboxed (recommended)"),
    ("native", "native — RPM in a layer over the image (asks for the password)"),
    ("box", "box — in a Fedora box, with hardware video decoding"),
];

/// The ways `browser` can be installed: Opera GX exists for Linux only as a
/// Flatpak; the others in all three.
pub fn methods_for(browser: &str) -> &'static [(&'static str, &'static str)] {
    if browser == "opera-gx" {
        &BROWSER_METHODS[..1]
    } else {
        &BROWSER_METHODS
    }
}

/// `run:tamagi-browser install <browser> --<method>`
pub fn browser_action(browser: &str, method: &str) -> String {
    format!("run:tamagi-browser install {browser} --{method}")
}

/// Everything decided in the pages.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Choices {
    /// (browser, method), if a browser was chosen.
    pub browser: Option<(String, String)>,
    /// Catalog actions of the basics ticked.
    pub picked: Vec<String>,
    /// Unlock the encrypted disk with the TPM chip.
    pub tpm: bool,
}

impl Choices {
    /// What goes to `tamagi-install --actions`: the browser first (a web app
    /// needs it), then the ticked items, without repeats.
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
        self.browser.is_none() && self.picked.is_empty() && !self.tpm
    }
}

/// One command of the installation.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub argv: Vec<String>,
    /// English title shown while it runs (a translation key). Its output
    /// reports the stages as `── [n/total] title` lines.
    pub title: &'static str,
    /// Share of the overall progress bar: about how many stages it has.
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

/// The commands to run, in order.
pub fn steps(choices: &Choices) -> Vec<Step> {
    let mut steps = Vec::new();
    let argv = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
    if choices.tpm {
        steps.push(Step { argv: argv(&["tamagi-setup", "tpm-unlock", "on"]), title: "Encrypted disk", weight: 1 });
    }
    let actions = choices.actions();
    if !actions.is_empty() {
        let weight = install_stage_count(&actions).max(1);
        let mut command = argv(&["tamagi-install", "--actions"]);
        command.extend(actions);
        steps.push(Step { argv: command, title: "Browser and apps you chose", weight });
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
        assert_eq!(browser_action("opera-gx", "flatpak"), "run:tamagi-browser install opera-gx --flatpak");
        assert_eq!(browser_action("chrome", "native"), "run:tamagi-browser install chrome --native");
        assert_eq!(browser_action("vivaldi", "box"), "run:tamagi-browser install vivaldi --box");
        for (browser, _) in BROWSERS {
            for (method, _) in methods_for(browser) {
                let action = browser_action(browser, method);
                let expected = match *method {
                    "flatpak" => crate::method::Method::Flatpak,
                    "native" => crate::method::Method::Native,
                    _ => crate::method::Method::Box,
                };
                assert_eq!(crate::method::classify(&action), expected, "{action}");
            }
        }
    }

    #[test]
    fn opera_gx_is_flatpak_only() {
        assert_eq!(methods_for("opera-gx"), [BROWSER_METHODS[0]]);
        assert_eq!(methods_for("chrome").len(), 3);
        assert_eq!(BROWSERS[0].0, "opera-gx");
    }

    /// Every browser offered is a line of the real catalog (its size comes from there).
    #[test]
    fn browsers_are_in_the_catalog() {
        let Some(share) = crate::test_share_dir() else {
            return; // built outside the repository
        };
        let catalog = crate::catalog::parse_catalog(&std::fs::read_to_string(share.join("catalog.tsv")).unwrap());
        for (id, _) in BROWSERS {
            let item = catalog.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("{id} is not in catalog.tsv"));
            assert_eq!(item.action, browser_action(id, "flatpak"));
        }
    }

    #[test]
    fn actions_put_the_browser_first_and_drop_repeats() {
        let choices = Choices {
            browser: Some(("chrome".into(), "flatpak".into())),
            picked: s(&["flatpak:flathub:a.b.C", "run:tamagi-webapp install WhatsApp https://web.whatsapp.com", "flatpak:flathub:a.b.C", "run:tamagi-browser install chrome --flatpak"]),
            tpm: false,
        };
        assert_eq!(choices.actions(), s(&["run:tamagi-browser install chrome --flatpak", "flatpak:flathub:a.b.C", "run:tamagi-webapp install WhatsApp https://web.whatsapp.com"]));
    }

    #[test]
    fn nothing_chosen_runs_nothing() {
        let choices = Choices::default();
        assert!(choices.is_empty());
        assert!(steps(&choices).is_empty());
    }

    #[test]
    fn tpm_goes_first() {
        let choices = Choices { browser: None, picked: s(&["flatpak:flathub:org.gimp.GIMP"]), tpm: true };
        let all = steps(&choices);
        assert_eq!(all[0].argv, s(&["tamagi-setup", "tpm-unlock", "on"]));
        assert_eq!(all[1].argv, s(&["tamagi-install", "--actions", "flatpak:flathub:org.gimp.GIMP"]));
        assert_eq!(all.len(), 2);
        assert!(!choices.is_empty());
        let only_tpm = Choices { tpm: true, ..Choices::default() };
        assert_eq!(steps(&only_tpm).len(), 1);
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
        let choices = Choices { browser: Some(("chrome".into(), "flatpak".into())), picked: s(&["flatpak:flathub:a.b.C", "flatpak:flathub:d.e.F"]), tpm: true };
        let all = steps(&choices);
        let weights: Vec<u32> = all.iter().map(|s| s.weight).collect();
        assert_eq!(weights, [1, 2]);
    }
}
