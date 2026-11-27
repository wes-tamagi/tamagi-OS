# Tamagi CLI (`tamagi-cli`)

Standalone static Rust CLI suite for **[Tamagi OS](https://github.com/wes-tamagi/tamagi-OS)** — an atomic, `bootc`-based Linux desktop designed for AI coding agents, containers, and reproducible workstations.

> **Upstream OS:** [github.com/wes-tamagi/tamagi-OS](https://github.com/wes-tamagi/tamagi-OS)

## Binaries in this workspace

| Binary | Crate | Purpose |
|---|---|---|
| `tamagi` | `tamagi` | Primary command router, declarative state (`tamagi.toml`), and `dnf5`/`pacman`-style UI (`ui.rs`) |
| `tamagi-install` | `tamagi` | Unified catalog + Flathub + reversible native `systemd-sysext` package manager with transaction tables and TTY animations |
| `tamagi-doctor` | `tamagi` | Autonomous health, integrity, network (`BBR` / Wi-Fi / NPU), and `--report` soak-test diagnostics |
| `tamagi-why` | `tamagi` | Instant provenance inspector for any file, command, port, or service |
| `tamagi-layer` | `tamagi-layer` | Reversible `systemd-sysext` native Fedora package layers over `/usr` |
| `agent-jail` | `agent-jail` | Project-jailed sandbox with Copy-on-Write Shadow Workspaces and kernel-enforced Egress Guard |

## Command structure

```bash
tamagi                                  # list all commands
tamagi update                           # atomic OS, layers, Flatpaks, boxes, and AI agents
tamagi pkg install <id|app>             # catalog, Flathub, or --native (sysext)
tamagi pkg remove <id|app>              # unified removal (Flatpak, browser, agent, layer)
tamagi pkg search <query>               # search curated catalog + Flathub
tamagi pkg layer [add|remove|list]      # manage reversible systemd-sysext layers
tamagi ai ask [prompt]                  # local llama.cpp system agent
tamagi ai run --shadow <agent>          # run coding agent in CoW shadow + Egress Guard
tamagi ai flights [list|show|clear]     # agent flight recorder (audit)
tamagi dev box create fedora            # create a distrobox dev environment
tamagi dev db add postgres              # start a local Podman Quadlet database
tamagi sys doctor [--report|--fix]      # system diagnostics / Markdown soak-test report
tamagi sys why /usr/bin/bash            # explain where any file, port, or command came from
```

## Build & test

```bash
cargo test --locked
RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --locked --target x86_64-unknown-linux-gnu
```

## Integration with Tamagi OS

Tamagi OS compiles this workspace statically during its container build ([`Containerfile`](https://github.com/wes-tamagi/tamagi-OS/blob/main/Containerfile)) and installs the binaries into `/usr/bin/` and `/usr/libexec/tamagi/`.

To publish or sync this directory to its standalone repository (`wes-tamagi/tamagi-cli`):

```bash
git subtree split --prefix=rust -b tamagi-cli
git push https://github.com/wes-tamagi/tamagi-cli.git tamagi-cli:main
```
