//! How an item is installed: the badge shown next to everything offered.

/// The install method of a catalog action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Method {
    /// Flatpak app, in the user, sandboxed.
    Flatpak,
    /// RPM layer over the image: the only one that asks for the password.
    Native,
    /// Inside a distrobox box.
    Box,
    /// A Podman container of its own (Windows).
    Container,
    /// Files in the home folder of the user (agents, small tools).
    User,
    /// A `run:` command this program does not know.
    Command,
}

impl Method {
    /// Every badge, in the order of the legend and of the review page.
    pub const ALL: [Method; 6] = [Method::Flatpak, Method::Native, Method::Box, Method::Container, Method::User, Method::Command];
    /// The badges explained in the legend (`Command` only shows up for
    /// catalog lines newer than this program).
    pub const LEGEND: [Method; 5] = [Method::Flatpak, Method::Native, Method::Box, Method::Container, Method::User];

    /// The text of the badge (English: a translation key).
    pub fn label(self) -> &'static str {
        match self {
            Method::Flatpak => "Flatpak",
            Method::Native => "Native",
            Method::Box => "Box",
            Method::Container => "Container",
            Method::User => "In your user",
            Method::Command => "Command",
        }
    }

    /// The short explanation of the legend (English: a translation key).
    pub fn explanation(self) -> &'static str {
        match self {
            Method::Flatpak => "sandboxed app from Flathub",
            Method::Native => "system layer, asks for the password",
            Method::Box => "inside a distrobox",
            Method::Container => "its own Podman container",
            Method::User => "in your home folder",
            Method::Command => "runs a setup command",
        }
    }

    /// The same in two or three words, for the one-line legend.
    pub fn short(self) -> &'static str {
        match self {
            Method::Flatpak => "Flathub app",
            Method::Native => "asks for the password",
            Method::Box => "distrobox",
            Method::Container => "Podman",
            Method::User => "home folder",
            Method::Command => "setup command",
        }
    }

    /// Native layers are built with sudo: the password is asked for.
    pub fn asks_password(self) -> bool {
        self == Method::Native
    }
}

/// The badge of a catalog action (`flatpak:…`, `gaming:…`, `agent:…`, `run:…`).
pub fn classify(action: &str) -> Method {
    if action.starts_with("flatpak:") {
        return Method::Flatpak;
    }
    if action.starts_with("gaming:") {
        return Method::Native;
    }
    if action.starts_with("agent:") {
        return Method::User;
    }
    let Some(command) = action.strip_prefix("run:") else {
        return Method::Command;
    };
    let mut words = command.split_whitespace();
    let program = words.next().unwrap_or("");
    let name = program.rsplit('/').next().unwrap_or(program);
    match name {
        "tamagi-browser" => {
            // run:tamagi-browser install <browser> --flatpak|--native|--box
            for word in words {
                match word {
                    "--flatpak" => return Method::Flatpak,
                    "--native" => return Method::Native,
                    "--box" => return Method::Box,
                    _ => {}
                }
            }
            // tamagi-browser's own default.
            Method::Flatpak
        }
        "tamagi-gaming" => match words.next() {
            Some("ge-proton" | "proton-ge" | "mangohud" | "flatpak") => Method::User,
            _ => Method::Native,
        },
        // A Fedora package in a layer over the image.
        "tamagi-vm" | "tamagi-layer" => Method::Native,
        "tamagi-devbox" => Method::Box,
        "tamagi-windows" => Method::Container,
        // Before docker-setup: "lazydocker-setup" also ends in "docker-setup".
        // A web app is a menu entry in the home folder.
        "lazydocker-setup" | "llm-scripts-setup" | "tamagi-webapp" => Method::User,
        "docker-setup" | "vscode-setup" | "tailscale-setup" | "nvidia-setup" | "lark-setup" | "sniffnet-setup" => Method::Native,
        _ => Method::Command,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes() {
        assert_eq!(classify("flatpak:flathub:org.gimp.GIMP"), Method::Flatpak);
        assert_eq!(classify("flatpak:cosmic:io.github.cosmic_utils.Examine"), Method::Flatpak);
        assert_eq!(classify("gaming:steam"), Method::Native);
        assert_eq!(classify("gaming:base"), Method::Native);
        assert_eq!(classify("agent:claude"), Method::User);
        assert_eq!(classify("agent:local"), Method::User);
    }

    #[test]
    fn run_commands() {
        assert_eq!(classify("run:tamagi-vm install"), Method::Native);
        assert_eq!(classify("run:tamagi-layer add rclone rclone"), Method::Native);
        assert_eq!(classify("run:/usr/libexec/tamagi/docker-setup"), Method::Native);
        assert_eq!(classify("run:/usr/libexec/tamagi/vscode-setup"), Method::Native);
        assert_eq!(classify("run:/usr/libexec/tamagi/tailscale-setup"), Method::Native);
        assert_eq!(classify("run:tamagi-devbox create fedora"), Method::Box);
        assert_eq!(classify("run:tamagi-devbox create arch"), Method::Box);
        assert_eq!(classify("run:tamagi-windows install"), Method::Container);
        assert_eq!(classify("run:/usr/libexec/tamagi/lazydocker-setup"), Method::User);
        assert_eq!(classify("run:/usr/libexec/tamagi/llm-scripts-setup"), Method::User);
        assert_eq!(classify("run:tamagi-webapp install WhatsApp https://web.whatsapp.com"), Method::User);
    }

    #[test]
    fn lazydocker_is_not_docker() {
        assert_ne!(classify("run:/usr/libexec/tamagi/lazydocker-setup"), classify("run:/usr/libexec/tamagi/docker-setup"));
        assert_eq!(classify("run:docker-setup"), Method::Native);
    }

    #[test]
    fn browsers() {
        assert_eq!(classify("run:tamagi-browser install firefox --flatpak"), Method::Flatpak);
        assert_eq!(classify("run:tamagi-browser install chrome --native"), Method::Native);
        assert_eq!(classify("run:tamagi-browser install vivaldi --box"), Method::Box);
        assert_eq!(classify("run:tamagi-browser install chromium"), Method::Flatpak);
    }

    #[test]
    fn unknown() {
        assert_eq!(classify("run:something-new --flag"), Method::Command);
        assert_eq!(classify("whatever"), Method::Command);
        assert_eq!(classify(""), Method::Command);
    }

    #[test]
    fn only_native_asks_for_the_password() {
        for m in Method::ALL {
            assert_eq!(m.asks_password(), m == Method::Native, "{m:?}");
        }
    }

    /// Every action of the real catalog gets one of the five documented badges.
    #[test]
    fn real_catalog_has_no_unknown_action() {
        let Some(share) = crate::test_share_dir() else {
            return; // built outside the repository
        };
        let text = std::fs::read_to_string(share.join("catalog.tsv")).unwrap();
        for item in crate::catalog::parse_catalog(&text) {
            assert_ne!(classify(&item.action), Method::Command, "{}", item.action);
        }
    }
}
