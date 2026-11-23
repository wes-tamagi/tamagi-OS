# Tamagi

**Tamagi 1.0 "Kazunobu M"** is an atomic, immutable Linux distribution designed
for AI workflows and coding agents.

Coding agents (Claude Code, Codex, CLIO, opencode, Qwen Code, Gemini CLI,
OpenClaw) and local models run in isolated, reversible workspaces: each agent
is jailed to the current project directory, blocked from reading credentials,
and every filesystem or database change can be rolled back.

The host is a [bootc](https://github.com/bootc-dev/bootc) OCI image built on
[Universal Blue](https://universal-blue.org/) with the Rust-based COSMIC
desktop and the Open Gaming Collective (OGC) kernel used by Bazzite. The base
image contains no browsers, games, virtual machines, AI agents or development
runtimes: all of these are opt-in through Flatpak, reversible native layers or
distrobox containers. Local inference uses llama.cpp, compiled into the image.

The whole OS is an OCI container: `podman build` builds the image, updates
switch atomically, and the previous deployment stays on disk for rollback.

## Contents

- [Quick start](#quick-start)
- [One command: `tamagi`](#one-command-tamagi)
- [AI agents: project-jailed and reversible](#ai-agents-project-jailed-and-reversible)
- [First login and ready-made profiles](#first-login-and-ready-made-profiles)
- [Three core ideas](#three-core-ideas)
- [Install](#install)
- [Updates and signed images](#updates-and-signed-images)
- [Software](#software): [browsers](#browsers-flatpak-native-or-box) ·
  [layers](#layers-fedora-packages-on-top-of-the-image) ·
  [devboxes](#devboxes-and-workspaces) · [virtual machines](#virtual-machines) ·
  [Windows applications](#windows-applications) · [gaming](#gaming)
- [AI](#ai)
- [System tools](#system-tools)
- [Security](#security)
- [Desktop](#desktop)
- [Build and test](#build-and-test)
- [Kernel, modules and signing](#kernel-modules-and-signing)
- [Repository structure](#repository-structure)
- [Why another distro?](#why-another-distro)
- [Known limits](#known-limits)
- [Acknowledgments](#acknowledgments) · [License](#license)

Related documents: [tamagiguide.md](tamagiguide.md) (install, migrate, update),
[UPSTREAMS.md](UPSTREAMS.md) (forks and contribution plan),
[CONTRIBUTORS.md](CONTRIBUTORS.md).

---

## Quick start

```bash
tamagi                                       # list all commands
tamagi update                                # update system, Flatpaks and boxes
tamagi doctor                                # run system diagnostics
tamagi install                               # open the optional software catalog
tamagi browser install firefox --flatpak     # or --native, or --box
tamagi agents install claude codex qwen      # install AI coding agents
tamagi devbox create ubuntu                  # create a dev container
tamagi db add postgres                       # start a local development database
```

## One command: `tamagi`

`tamagi` replaces Universal Blue's `ujust`, which is removed from the image,
and puts all system administration behind one CLI.

- **Subcommands are always in English.** Output messages are English (the
  source language), Brazilian Portuguese, Spanish and French, from
  `/usr/share/tamagi/i18n`; other locales get English.
- **Dispatch:** `tamagi <command>` runs the program `tamagi-<command>`.
- **Languages used:** the entry point (`tamagi`), native layering
  (`tamagi layer`) and the agent jail are static Rust binaries (`rust/`, unit
  tested during the image build). Commands that only chain other commands are
  Bash; a few are Python.
- **Former `ujust` tasks** live in `tamagi update` and `tamagi setup` (Secure
  Boot key, TPM unlock, firmware, cleanup, LazyVim, image switching).

## AI agents: project-jailed and reversible

```bash
tamagi agents install claude codex opencode   # or: tamagi apply ai
cd ~/Projects/my-app
tamagi agents run claude                      # run the agent jailed to this directory
tamagi agents jail                            # open a shell inside the same jail
```

Inside the bubblewrap sandbox (the isolation layer Flatpak uses), an agent can
read and write only the project directory and its own configuration.

| | |
|---|---|
| **Hidden** | `~/.ssh`, `~/.gnupg`, API tokens, browser profiles, other repositories, removable drives |
| **Read-only** | the operating system; the project's `.git/hooks` and `.git/config` |
| **Network** | enabled for API calls; `--offline` cuts it |
| **Compilers** | if a devbox exists (`tamagi devbox`), the jail is built inside it (Node, Python, Rust, git, database clients); otherwise on the host (`--host` forces it) |
| **Rollback point** | before launch Tamagi records a git ref (`refs/tamagi/before-agent/<timestamp>`) and takes a home snapshot when snapshots are on; `--db` also snapshots running `tamagi db` instances |
| **Authentication** | log in once outside the jail (`claude`, `codex`…): there is no browser or desktop session inside |
| **Publishing** | without SSH keys in the jail, an agent cannot `git push` |

## First login and ready-made profiles

Like openSUSE Aeon, the installer puts only the base system on disk. On first
login a native COSMIC window (`tamagi-welcome`, Rust + libcosmic) offers only
the basics: a short list to tick (`/usr/share/tamagi/basics.list`: the COSMIC
apps, the Flatpak start-up manager Ignition, Mission Center, OnlyOffice and
WhatsApp as a web app) and a browser (Opera GX, Google Chrome, Chromium or
Vivaldi). Everything else, including the ready-made profiles, comes later
with `tamagi install`. **Every item shows how it is installed:**

| Badge | Meaning |
|---|---|
| **Native** | RPM in a reversible layer over the image; asks for the password |
| **Flatpak** | sandboxed app from Flathub |
| **Box** | inside a distrobox |
| **Container** | its own Podman container (Windows) |
| **In your user** | in your home folder (AI agents, small tools) |

A review page lists everything grouped by install method, with the
approximate download and what will ask for the password, before anything is
installed. The shared Flatpak runtime starts downloading in the background
while you choose. The same choices stay available later:

```bash
tamagi install                                    # the catalog
tamagi install wesnoth                            # not in the catalog? found on Flathub and installed
tamagi install search strategy                    # what Flathub has for a text
tamagi profiles                                   # ai, dev, gamer, office
tamagi apply ai                                   # AI agents + devbox (suggested)
tamagi apply gamer                                # native Steam, gamescope HDR, MangoHud, Heroic
tamagi apply https://example.org/my-machine.toml  # apply a shared configuration
```

Profiles are `tamagi.toml` files in `/usr/share/tamagi/profiles`. They are
strictly additive and never remove packages or settings. Remote files
(`https://` only) are always printed for review before execution, even with `-y`.

## Three core ideas

### 1. Declarative machine state

`~/.config/tamagi/tamagi.toml` declares themes, browsers, Flatpaks, languages
(mise inside devboxes), AI agents, boxes, layers, web apps and services. Keys
are in English (`[appearance]`, `[browsers]`, `[boxes]`); older Portuguese keys
are still accepted.

```bash
tamagi init      # generate tamagi.toml from the current machine
tamagi diff      # show what differs between the machine and the file
tamagi apply     # apply only what is missing (prompts when sudo is needed)
```

### 2. Non-destructive system layers

`tamagi layer add <name> <package>` overlays Fedora RPMs onto `/usr` as
read-only systemd-sysext images. The base image stays unmodified, rollback
stays intact, and removing the layer restores the upstream image.

### 3. Self-healing boot

On every boot, `tamagi-boot-check.service` runs the checks in
`/usr/lib/tamagi/boot-check`. bootc with systemd-boot does not count boot
attempts, so Tamagi does it: a new image that fails is retried once with the
native layers off, then `bootc rollback` brings the previous image back and
the failed one is quarantined so automatic updates do not download it again.
This covers boots that reach the system; a boot that stops in the kernel or
the initramfs needs the previous entry picked by hand in the boot menu. On the
next login, `tamagi boot-report` lists what failed and, if a local LLM is
configured, summarizes it in the user's language.

## Install

Tamagi uses [tik](https://github.com/sysrich/tik) (openSUSE Aeon's installer)
with a custom module that calls `bootc install`. The ISO builds rootless:

```bash
just build-iso      # output/tamagi.iso (about 5 GiB: fits an 8 GB USB drive)
just test-iso       # unattended VM install + post-install verification
just test-upgrade   # end-to-end: local registry, automatic update, reboot, then a bad image that must roll back by itself
just run-iso        # launch the ISO in a windowed VM
sudo bash live/write-usb.sh /dev/disk/by-id/usb-... --yes
```

The UEFI-only ISO boots a live COSMIC session and launches the installer.

- **Languages:** English is the default; Brazilian Portuguese is second, then
  Spanish and French. The chosen language applies to the installer, the OS and
  the CLI. Only one image is published (`latest`); a variant with
  Brazilian Portuguese as the default can be built locally from the same
  source (`TAMAGI_DEFAULT_LANG=pt just build`).
- **Offline:** installation copies the image embedded in the ISO.
- **Atomic installation:** before wiping the disk, the installer checks disk
  capacity and the SHA-256 digest of every embedded layer, and aborts on
  corrupted media before writing. The digest check starts when the installer
  window opens and runs while the questions are answered, so it adds no wait
  after the disk is chosen. The EFI partition is created with the GPT
  `NoBlockIOProtocol` flag so firmware ignores it during copying or a power
  loss; the flag is cleared in one final partition-table write after full
  verification. Failed installs clean up the partitions they created.
- **TPM 2.0 disk encryption:** with a TPM 2.0 chip the installer offers
  `systemd-cryptenroll`. On first boot `tamagi-tpm-seal.service` seals the key
  to PCR 7 (Secure Boot state). The recovery passphrase keeps working if
  hardware or firmware changes. Tested with `just test-iso --tpm`.

<details>
<summary>Internal installer flow</summary>

1. `live/Containerfile` layers tik (pinned by tag and commit),
   `live/tik/modules/pre/10-tamagi`, a live initramfs and the live user onto
   the base image.
2. `/usr/libexec/tamagi-install` partitions the disk and runs
   `bootc install to-filesystem --source-imgref oci:...`. The layout follows
   openSUSE Aeon: an 8 GiB ESP mounted at `/efi` (UEFI only), booted by
   systemd-boot, and one zstd-compressed Btrfs (optional LUKS2 + TPM) with the
   subvolumes `root` (the system) and `home` (`/var/home`). There is no
   `/boot` partition and no GRUB: kernels and initramfs live on the ESP
   (about 265 MiB per image). systemd-boot needs bootc's composefs backend
   (`--composefs-backend --bootloader systemd`), where `rpm-ostree` does not
   work: Tamagi's tools talk to `bootc`. A machine installed with the older
   GRUB layout cannot be converted in place; it has to be reinstalled.
3. `tamagi-firstboot.service` creates the user account on first boot; the
   home folder is a Btrfs subvolume with snapper snapshots on from the start.
   `tamagi-combustion.service` then looks, on that first boot only, for a
   drive labelled `combustion` (or `ignition`) holding `combustion/script`
   and runs it as root: the provisioning format of openSUSE
   [Combustion](https://github.com/openSUSE/combustion), run from the booted
   system instead of the initrd. CoreOS Ignition files are not interpreted.
4. `tamagi.autoinstall=/dev/sdX` on the kernel command line runs an unattended
   installation (used by `just test-iso`).

</details>

Migrating from Bluefin or Aurora, and the step-by-step for a fresh install:
see [tamagiguide.md](tamagiguide.md).

## Updates and signed images

| Component | Mechanism |
|---|---|
| OS image | `tamagi-system-update.timer`, every six hours: `bootc` prepares the new image beside the running one; it takes effect on the next reboot (never reboots by itself). Skipped on battery and on metered connections (`/etc/tamagi/update.conf`) |
| Flatpaks | system and user systemd timers |
| Devboxes and subsystems | `tamagi-autoupdate.timer` (skipped on battery) |

```bash
tamagi update             # everything now: image, layers, Flatpaks, boxes, agents
tamagi update status      # timers and pending reboots
tamagi update changes     # what the staged image changes (with an optional local-LLM summary)
tamagi update auto off    # or: on
```

**Signed images.** An installed system only accepts images from
`ghcr.io/wes-tamagi` signed with one of the project's two cosign keys
(`cosign.pub`, `cosign-2.pub`). An update without a valid signature is refused
and the system stays on the current image. Verify by hand with:

```bash
cosign verify --key cosign.pub ghcr.io/wes-tamagi/tamagi:latest
```

**Your own image.** Publish it to a registry (`.github/workflows/build.yml`
pushes to GHCR), then build the ISO with
`TAMAGI_TARGET_IMGREF=ghcr.io/YOUR_USER/tamagi:latest just build-iso`. On an
installed Tamagi: `tamagi setup switch-image ghcr.io/YOUR_USER/tamagi:latest`.

> **Publish before building an ISO.** Installed systems follow
> `TAMAGI_TARGET_IMGREF`. An ISO built from a local image newer than the
> registry makes the first automatic update go *back* to the registry image.
> Push (and let CI publish) before `just build-iso`.

## Software

### Browsers: Flatpak, native or box

No browser is preinstalled. Chrome, Firefox, Vivaldi and Chromium each support
three install methods:

```bash
tamagi browser install                     # interactive selector
tamagi browser install firefox             # Flatpak (default, Flathub)
tamagi browser install chromium --native   # Fedora RPM through tamagi layer (asks for the password)
tamagi browser install chrome --box        # Fedora distrobox with hardware video acceleration
tamagi browser list                        # available and installed browsers
tamagi browser default firefox             # set the default browser
tamagi browser remove chrome
```

- **Panel button:** "Web Browser" offers installation until a browser exists,
  then opens the default one.
- **Opera GX:** exists for Linux only as a Flatpak (`com.opera.opera-gx`).
- **Native:** Chrome and Vivaldi come from the vendors' repositories
  (key-verified); Firefox and Chromium from Fedora. Layers are rebuilt after
  OS updates.
- **Box:** exposes GPU, audio, webcam, USB security keys and `$HOME`.
  Chromium-based browsers start with VA-API decode/encode
  (`~/.config/tamagi/browser-flags.conf`). `tamagi browser status` shows VA-API
  and Vulkan.
- **Web apps** (`tamagi webapp`) use whichever Chromium-based browser is installed.
- **Isolated profiles:** each has its own data directory and launcher entry.

  ```bash
  tamagi browser profile add work
  tamagi browser chrome --profile work
  tamagi browser profile list | remove <name>
  ```

- **FIDO2 / passkeys:** plug the key in before opening a boxed browser (see
  [Known limits](#known-limits)); diagnose with `tamagi browser fido`.
- **Internet banking:** Tamagi does not bundle Brazilian banking modules
  (Warsaw). Use a VM (`tamagi vm install`).

Measured on development hardware (Ryzen 7 5700, RX 7600, browser box):
`vainfo` reports H.264, HEVC, VP9 and AV1 decode and H.264, HEVC and AV1
encode; Chrome 158 used `VaapiVideoDecoder` for H.264, VP9 and AV1 with WebGL,
WebGPU and the Chrome sandbox on.

### Layers: Fedora packages on top of the image

For drivers, services or low-level tools that must live in `/usr` without
rebuilding the image:

```bash
tamagi layer add tools strace ltrace   # build and activate an erofs layer (asks for the password)
tamagi layer list                      # layers, packages and state
tamagi layer remove tools              # back to the pure base image
tamagi layer off | on                  # disable / enable all layers
tamagi layer temp                      # writable /usr until the next reboot
```

Each layer is a read-only erofs image in `/var/lib/extensions`, merged by
systemd-sysext. Dependencies are resolved rootless with dnf against the host
RPM database, so only missing packages go in. `--copr owner/project` adds a
COPR repository.

**Constraints:** package scriptlets do not run (packages that create system
users or change the boot need an image rebuild); upgrades of base packages are
refused unless `--allow-upgrade`; stale layers are rebuilt by `tamagi update`;
layers are turned off before a rollback when boot health checks fail. Declare
layers under `[layers]` in `tamagi.toml`.

### Devboxes and workspaces

Compilers and toolchains live in containers. The default box (`dev`) is
Fedora 45; Ubuntu, openSUSE Tumbleweed and Arch are also supported. Each box
has gcc/g++/cmake/ninja, git, SQLite, Rust/Cargo, Node.js/npm, Python, mise,
database clients (psql, mariadb, valkey-cli/redis-cli, reaching `tamagi db` on
127.0.0.1), GPU access and host browser delegation (`BROWSER`).

```bash
tamagi devbox                           # interactive menu (default: fedora)
tamagi devbox create                    # create 'dev' (Fedora 45)
tamagi devbox create ubuntu             # also: opensuse, arch
tamagi devbox create fedora --extra go,zig,clang,browser
tamagi devbox enter | lang | browser | distros | list | update | remove <name>
```

**mise inside the box.** Language runtimes stay off the host:

```bash
tamagi devbox lang                      # web, bun, deno, python, ruby, go, rust, java, dotnet, zig
tamagi devbox lang node@lts python@3.13
tamagi devbox enter                     # inside: mise use, mise run, mise ls
```

Each box keeps its own binaries (`~/.local/share/mise-<box>`) and shares
`~/.config/mise/config.toml`. For automated tests, `tamagi devbox browser`
installs Chromium in the box and sets `CHROME_BIN`,
`PUPPETEER_EXECUTABLE_PATH` and `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH`.

**Terminal tools live in the box.** Neovim, lazygit, gh, uv, glow, hyperfine,
direnv, yq and ShellCheck are not in the image: `tamagi devbox create`
installs them in the box and exports a launcher for each to `~/.local/bin`,
so `nvim` or `gh` typed on the host runs the one of the box. `Super+Shift+N`
and `Super+Shift+G` offer to create the box when it does not exist yet.
rclone and COSMIC Player are optional native layers (`tamagi install rclone`,
`tamagi install cosmic-player`).

**Containers:** Podman is the engine. Docker CE (`tamagi install docker`, a
native layer) and lazydocker (`tamagi install lazydocker`) are optional.

**Local databases, reversible:**

```bash
tamagi db add postgres             # rootless Podman Quadlet on 127.0.0.1:5432
tamagi db url postgres | env       # connection string / export DATABASE_URL=...
tamagi db shell postgres           # psql, mysql, redis-cli, mongosh
tamagi db snapshot postgres        # save state before a migration
tamagi db restore postgres         # bring it back
```

**Disposable:** `tamagi try htop` starts a throwaway container with the package
and deletes it on exit (`--distro arch|ubuntu|opensuse`, `--here` mounts the
current folder at `/work`, `--gui`, `--gpu`).

**Workspaces:** `tamagi workspace` detects a repository's stack (`Cargo.toml`,
`package.json`, `pyproject.toml`, `go.mod`, SQLite, `docker-compose.yml`),
writes `tamagi-workspace.toml` and coordinates devboxes, layers, local
databases and tasks (`tamagi workspace init | status | up | enter | run <task>`).

### Virtual machines

`tamagi vm install` installs Fedora's native virt-manager in a reversible
layer: QEMU/KVM, UEFI + Secure Boot (edk2-ovmf), virtual TPM (swtpm), virgl 3D,
SPICE (clipboard, audio, USB passthrough), virtiofs, NAT and rootless
networking (passt), libvirt-nss, virt-viewer, virt-install, virsh, virt-top and
guestfs-tools. Libvirt daemons are socket-activated.

```bash
tamagi vm                              # launch virt-manager (installs on first run)
tamagi vm install | status | remove    # removing drops the layer; VMs and disks stay
tamagi vm access full                  # let VMs use USB drives and external disks
```

### Windows applications

- **Bottles (Wine):** in the catalog; runs `.exe` and `.msi` with GPU
  acceleration (DXVK/VKD3D) and no Windows license. Good for Win32 apps and
  games; not for Office 365 or Adobe Creative Cloud.
- **Windows 11 VM, on demand:** `tamagi windows` runs
  [dockur/windows](https://github.com/dockur/windows) (rootless Podman Quadlet
  + QEMU/KVM) through FreeRDP, for Office 365, Adobe, banking and smartcard
  software and Windows-only USB devices.

```bash
tamagi windows            # launch (downloads about 6 GB from Microsoft on first run)
tamagi windows stop       # shut down and free RAM
tamagi windows web        # follow the installation in the browser
tamagi windows status | remove
```

Needs 12 GB of RAM (4–8 GB while running), 40 GB of free disk and a valid
Microsoft license. `~/Windows` is shared with the VM. No GPU passthrough;
kernel-level anti-cheat is unsupported on both paths.

### Gaming

Only kernel-level parts ship in the base image: the OGC kernel, Xbox
controller drivers (xone/xpadneo) and `scx_rusty`. Steam, steam-devices,
gamescope, MangoHud, GameMode, 32-bit Mesa/Vulkan, Lutris, Wine, Winetricks,
GOverlay, vkBasalt and Protontricks install on demand, as a reversible native
layer or through Flatpak:

```bash
tamagi gaming                                              # interactive menu
tamagi gaming native steam                                 # Steam + gamescope, MangoHud, GameMode, 32-bit
tamagi gaming native lutris wine overlay protontricks      # added to the same layer
tamagi gaming flatpak                                      # Flatpak gaming stack
tamagi gaming latency                                      # tuned + scx_rusty low-latency profile
tamagi gaming status | remove
```

**gamescope with HDR.** HDR runs on DRM through gamescope, outside COSMIC (as
on Bazzite and SteamOS). After `tamagi gaming native`, pick the **Tamagi
Gaming** session at login for Steam Big Picture with HDR and VRR; leaving
Steam returns to the login screen. Inside COSMIC, windowed gamescope is SDR
until COSMIC has HDR upstream.

```bash
tamagi gaming hdr                  # gamescope, HDR displays and configuration
tamagi gaming hdr on | off         # session HDR (default: on)
tamagi gaming hdr nits 300         # SDR brightness inside HDR (default: 400)
tamagi gaming hdr itm on           # inverse tone mapping for SDR games
tamagi gaming hdr vrr on | off     # adaptive sync
tamagi gaming run -- %command%     # Steam launch option
```

Settings: `~/.config/tamagi/gamescope.conf` (`WIDTH`, `HEIGHT`, `REFRESH`, `CLIENT`).

## AI

```bash
tamagi ai setup                         # recommend a model for the hardware, download, start the server
agent "why is my boot slow?"            # 100% local system agent (tamagi agent)
tamagi ai chat | code                   # llama.cpp web UI | opencode on the local model
tamagi agents                           # coding agent catalog (none preinstalled)
tamagi agents install claude antigravity codex clio qwen opencode gemini openclaw
```

| Agent | How it is installed |
|---|---|
| Claude Code (`claude`) | official Anthropic installer, checksum-verified, in `~/.local/bin` |
| Codex CLI (`codex`), opencode | official GitHub binary, SHA-256 verified, in `~/.local/bin` |
| CLIO (`clio`) | pure Perl (GPL-3.0), GitHub release digest-verified; runs in the devbox against local llama.cpp |
| Antigravity CLI (`agy`) | Google's installer (SHA-512 checked by it), one binary in `~/.local/bin`; the desktop IDE is separate: `tamagi antigravity` |
| Qwen Code, Gemini CLI, OpenClaw | npm packages inside the devbox, with launchers in `~/.local/bin` |
| Local AI | `tamagi agent` backed by local llama.cpp (`tamagi ai setup`) |

- **llama.cpp v1.0.0:** compiled with Vulkan and runtime CPU dispatch; serves
  OpenAI- and Anthropic-compatible APIs on `127.0.0.1:9931`. Loads on demand
  and unloads after 15 idle minutes.
- **`tamagi agent`** (alias `agent`): local system agent using the declarative
  actions in `/usr/share/tamagi/actions.toml`.
- **llm-scripts** (`tamagi install llm-scripts`): GPL-3.0 toolkit that reads
  model metadata and VRAM to compute context and KV-cache profiles
  (`llm doctor`, `llm models`, `llm profile --model <model>`, `llm serve`),
  wired to Tamagi's llama.cpp.

| Preset | Profile | Model | Size | Target hardware |
|---|---|---|---:|---|
| `mini` | general | Gemma 4 E2B | 2.8 GB | any machine |
| `leve` | general | Gemma 4 E4B | 4.6 GB | 8–16 GB RAM |
| `mistral` | reasoning | Ministral 3 8B Reasoning | 5.2 GB | fast reasoning (16 GB RAM) |
| `agente` | agent | Ornith 1.5 9B | 5.8 GB | local agent / tool calling (16 GB RAM / 8 GB VRAM) |
| `medio` | general | Gemma 4 12B | 7.2 GB | 16 GB+ RAM or 8–12 GB GPU |
| `devstral` | code | Devstral Small 2 24B | 14.5 GB | multi-file software engineering |
| `qwen38` | general | Qwen 3.8 27B | 16.5 GB | hybrid reasoning |
| `codigo` | code | Qwen3 Coder 30B (MoE) | 18.0 GB | fast coding (32 GB+ RAM) |
| `ornith` | agent | Ornith 1.5 35B (MoE) | 20.5 GB | top autonomous agent (48 GB RAM / 24 GB VRAM) |
| `grande` | general | Qwen 3.6 35B (MoE) | 20.5 GB | 48 GB+ RAM or 24 GB GPU |
| `workstation` | workstation | Mistral Small 4 119B | 68.0 GB | 96 GB+ RAM / multi-GPU |

## System tools

| Command | What it does |
|---|---|
| `tamagi doctor` | 12 checks (bootc, kernel/Secure Boot, Btrfs, zram, systemd, sysext layers, containers, GPU, KVM, SELinux, `.desktop` entries, local AI); `--json`, `--fix` (safe user-level fixes), `--explain` (local-LLM summary) |
| `tamagi rewind` | one timeline of OS deployments, dated registry images (`tamagi:45.YYYYMMDD`) and home snapshots; restore a file, reapply a past `tamagi.toml`, or roll the OS back. The current deployment stays in the boot menu; `tamagi rewind follow` resumes updates |
| `tamagi snap` | history of `$HOME`: hourly Btrfs snapshots (24 hourly, 7 daily, 4 weekly), excluding `~/.cache` and container storage |
| `tamagi perf` | CPU/EPP profiles (`eco`, `balanced`, `performance`, `latency`) coordinated with `scx_rusty`; `tamagi perf status`, `tamagi perf bench` |
| `tamagi setup` | machine chores: `secure-boot-key`, `tpm-unlock on|off`, `firmware`, `bios`, `clean`, `lazyvim`, `switch-image <ref>` |
| `tamagi isolate <file>` | runs an untrusted script, AppImage or binary in a disposable jail: no network, empty `$HOME`, read-only OS, no D-Bus; display, audio and GPU kept. `--keep` keeps the jail, `--net` allows networking. Also in COSMIC Files → Open With → "Run Isolated" |
| `tamagi guest on` | passwordless "Guest" account whose home lives in RAM and resets on logout; no sudo, polkit blocks disks, VMs and package installs. Off by default |
| `tamagi privacy` | one-touch privacy mode (`Super+Ctrl+P`): camera drivers unbound, microphones muted, Bluetooth blocked, mDNS off, firewall zone `drop`, random Wi-Fi MAC, optional Tailscale exit node, Do Not Disturb |
| `tamagi capture` | `text` (region OCR to the clipboard), `record` (screen recording) |
| `tamagi font`, `tamagi remind 25m "Break"` | monospace font everywhere; a desktop reminder |

**Home history:**

```bash
tamagi snap
tamagi snap versions ~/Documents/contract.odt
tamagi snap restore ~/Documents/contract.odt       # latest version that differs
tamagi snap restore ~/Documents/contract.odt 42    # a specific snapshot
tamagi snap now "before refactor"
```

The current file is kept next to the restored one as `<file>.antes-<date>`.
Snapshots are browsable in `~/.snapshots/<N>/snapshot`.

**Disk maintenance:** the root is Btrfs with zstd compression. As on openSUSE
MicroOS/Aeon, `btrfsmaintenance` runs a monthly scrub and a weekly balance;
`fstrim` runs monthly and off battery (Btrfs already discards freed blocks
continuously).

## Security

The default profile is **balanced**: hardening sysctls, kernel boot parameters
(`init_on_alloc`, `slab_nomerge`, `randomize_kstack_offset`), per-network Wi-Fi
MAC randomization, active firewall and enforcing SELinux, keeping gaming,
distrobox, Flatpak and GPU workloads working.

```bash
tamagi harden status
tamagi harden strict               # no SMT on vulnerable CPUs, kernel lockdown, root-only ptrace
tamagi harden balanced             # back to the default
tamagi harden usb on               # USBGuard allowlist of the devices attached now
tamagi harden malloc on            # hardened_malloc globally
tamagi setup tpm-unlock on         # unlock the encrypted disk with the TPM
```

**Landlock sandbox of the local agent.** Unattended commands run by
`tamagi agent` are restricted in the kernel by Landlock
(`/usr/libexec/tamagi/sandbox`), without root or containers:

| Execution mode | Permissions |
|---|---|
| Read-only and `read_file` (auto-run) | reads the system except secrets; writes only to a private temp dir; no TCP sockets; no signals to other processes |
| Auto-approved (`-y`) | writes to `$HOME` subdirectories, `/tmp`, `/run/user`; secrets stay read-only |
| User-confirmed | runs normally after explicit review |

Protected secrets: `~/.ssh`, `~/.gnupg`, `~/.password-store`, CLI tokens (gh,
claude, aws, kube), keyrings, browser profiles and the paths listed in
`~/.config/tamagi/sandbox-segredos`. Disable with `TAMAGI_AGENT_SANDBOX=0`.
Limits: `-y` cannot create or delete files directly in the root of `$HOME`;
setuid binaries (sudo, rootless podman/distrobox) cannot run under Landlock
and always ask for confirmation.

## Desktop

- **COSMIC 1.11** with tiling on and one floating bottom panel (apps centred,
  tray and clock on the right). Other layouts: `tamagi panel set centro|classico|cosmic`.
- **Terminal:** Ptyxis is the system terminal (container-aware,
  theme-synced); foot runs Tamagi's own windows; cosmic-term stays installed.
- **Start menu:** `cosmic-ext-classic-menu` 1.1.0, compiled in the image, on
  `Super+Space`. Command palette on `Super+A`; COSMIC launcher on `Super+Alt+Space`.
- **Themes:** choose themes and wallpapers in COSMIC Settings. Tamagi's 11
  themes are in `/usr/share/cosmic-themes`; a user unit
  (`tamagi-theme-sync.path`) keeps foot, Ptyxis, btop, Neovim and Tamagi's
  windows in step with any COSMIC theme.
- **Wallpapers:** in Settings › Wallpaper.
  - *Generated at build time*, 8 Edo-period motifs (mon, seigaiha, asanoha,
    byobu, shippo, hinode, uroko, ondas) for each of the 11 themes, in
    `/usr/share/backgrounds/tamagi/`.
  - *Illustrated*, ten pictures in `/usr/share/backgrounds/tamagi/art/`
    (source in `system_files/usr/share/backgrounds/tamagi/art/`, produced by
    `tools/art-wallpapers.py`).
- **Crest and boot splash:** the Tamagi crest is a *tamaki* (環) ring with
  three *tama* (玉) jewels; the Plymouth splash is drawn by
  `build_files/make-plymouth.py`.
- **Fonts and locales:** Noto Sans (UI), Roboto Slab (serif), JetBrains Mono
  (terminal). Locales: en-US, pt-BR, es, fr (`tamagi lang pick`).
- **Minimal apps:** only Flatseal, Warehouse, Loupe, Papers and three COSMIC
  applets are installed automatically.
- **Networking:** WireGuard, OpenVPN, OpenConnect, L2TP, PPTP, SSTP,
  IKEv2/IPsec, SSH, SMB, NFS, SSHFS, MTP, iPhone, mDNS, driverless
  printing and scanning; Tailscale is opt-in (`tamagi install tailscale`).

### Shortcuts

`Super+K` lists all of them.

| Shortcut | Action |
|---|---|
| `Super+Enter` / `Super+Space` | terminal / start menu |
| `Super+A` / `Super+Alt+Space` | Tamagi command palette / COSMIC launcher |
| `Super+Shift+A` | local AI agent |
| `Super+W` / `F` / `M` | close / fullscreen / maximize |
| `Super+1…9`, with `Shift` | switch workspace / move window |
| `Super+Arrows`, with `Shift` | focus / move window |
| `Super+T` / `Super+Y` | float window / toggle tiling |
| `Super+B` / `E` | browser / files |
| `Super+Shift+G` / `D` / `M` / `W` | lazygit (from the dev box; offers to create it) / databases / Mission Center / Windows |
| `Super+Ctrl+P` / `E` / `R` | privacy mode / emojis / reminder |
| `Shift+Print` / `Alt+Print` | OCR of a region / screen recording |

## Build and test

Needs `podman` and `just`. Runs rootless.

```bash
just build          # build the OS image (about 20 min the first time) + rechunk (about 7 min)
just test           # about 410 assertions inside the built image
just test-desktop   # boot a headless COSMIC session and save output/desktop.png
just test-browser   # build and verify the browser box locally
just test-agent     # download the mini preset (2.8 GB) and test the local agent
just lint
```

- `just build` ends with rechunking (`rechunk.sh`, through
  `rpm-ostree compose build-chunked-oci`): dropping superseded base layers
  takes the image from about 9.3 GB to about 6.8 GB (2.9 GB compressed) and
  limits updates to the chunks that changed. `TAMAGI_RECHUNK=0 just build`
  skips it during development.
- `TAMAGI_DEFAULT_LANG=pt just build` builds the Brazilian Portuguese variant.
- The Rust tools (`rust/`, `welcome/`) are compiled in their own build stages;
  their unit tests run there and fail the build if they fail.

## Kernel, modules and signing

- **Default kernel:** OGC (`ogc` flavor, the version Bazzite's `stable` ships)
  with signed `ublue-os/akmods` (xone, xpadneo, v4l2loopback).
- **Secure Boot:** the OGC kernel is signed by Universal Blue, not Fedora.
  Enroll the key once with `tamagi setup secure-boot-key` (password
  `universalblue`). The updater downloads a new image held back, runs the
  Secure Boot guard, and releases it only if its kernel would boot (Secure
  Boot on without the key enrolled: it stays held). On the ISO, `universalblue.der`
  is on `TAMAGI_EFI` for MokManager.
- **Other flavors, for local builds:** `KERNEL_FLAVOR=main just build`
  (Fedora-signed base kernel) or `KERNEL_FLAVOR=ark just build` (Fedora
  Rawhide kernel, no Universal Blue modules).
- **Image signing:** see [Updates and signed images](#updates-and-signed-images).

## Repository structure

```text
Containerfile            stages: llama.cpp -> start menu + welcome -> Rust tools -> image
build_files/             numbered build scripts; packages.txt
rust/                    tamagi, tamagi-layer and agent-jail (static Rust binaries)
welcome/                 tamagi-welcome, the first-login window (Rust + libcosmic)
system_files/            files overlaid onto the image root
  usr/bin/tamagi-*       update, setup, install, browser, harden, ai, agents…
  usr/libexec/tamagi/    container provisioners, firstboot, popups
  usr/share/backgrounds/tamagi/art/   illustrated wallpapers
live/                    ISO: Containerfile, tik module, tamagi-install, build-iso.sh
tests/                   image.sh, desktop.sh, agent.sh, browser.sh, iso.sh, vm_check.py
tools/                   development helpers (art-wallpapers.py)
cosign.pub, cosign-2.pub public keys the installed system accepts
disk_config/             bootc-image-builder VM disk config (alternative, needs sudo)
```

## Why another distro?

*Design rationale, by way of Max Weber.*

Modern systems engineering follows formal rationalization: replacing ad-hoc
manual configuration with repeatable rules. Immutable Linux applies this
directly: the OS is a verified artifact, identical across machines, updated
atomically and reversible on failure.

Weber also noted the cost of unchecked formal rationality:

> "No one knows who will live in this cage in the future."
> — Max Weber, *The Protestant Ethic and the Spirit of Capitalism* (1905)

On immutable desktops that rigidity shows when banking software will not
install, browsers lack hardware acceleration, or AI agents are either blocked
from working or given the whole of `$HOME`. Tamagi keeps the base OS immutable
while leaving jails, devboxes, reversible layers and on-demand VMs under the
user's control.

Weber also separated the ethic of conviction (*Gesinnungsethik*) from the
ethic of responsibility (*Verantwortungsethik*):

> "One has to give an account of the foreseeable results of one's action."
> — Max Weber, *Politics as a Vocation* (1919)

An ethic of conviction tells users to give up proprietary banking modules or
Windows-only tax tools. An ethic of responsibility accepts that those
obligations exist today and isolates them in on-demand VMs, so security and
daily work do not conflict.

## Known limits

- **Architecture and boot:** x86_64 and UEFI only. The installer targets
  internal disks, wipes the chosen disk and has no guided dual boot.
- **Hardware validation:** installation (with and without LUKS, TPM), upgrade
  and rollback are tested in QEMU/KVM VMs, not on bare metal. Never exercised
  on a real machine: the large native layers (Docker, virt-manager, gaming),
  the gaming session with HDR, `tamagi apply`, and migration from
  Bluefin/Aurora. The browser box was measured on AMD (RX 7600); Intel and
  NVIDIA paths are untested.
- **NVIDIA proprietary driver:** not in the image; needs another `BASE_IMAGE`
  (untested).
- **Desktop HDR:** waits for upstream COSMIC
  ([pop-os/cosmic-comp#1384](https://github.com/pop-os/cosmic-comp/issues/1384));
  use the Tamagi Gaming session for HDR.
- **Login screen:** uses COSMIC's default wallpaper.
- **TPM unlock:** bound to PCR 7; toggling Secure Boot or updating its keys
  falls back to the passphrase (expected).
- **FIDO2 in boxed browsers:** works when the key is plugged in before the
  browser starts; hot-plugging fails inside distrobox's user namespace.
- **Layers:** package scriptlets do not run (see [Layers](#layers-fedora-packages-on-top-of-the-image)).
- **`tamagi isolate`:** X11-only apps still reach Xwayland and can inspect
  other X11 windows.
- **Localization gaps:** root boot services (home-snapshots, firstboot,
  secureboot-guard) log in Portuguese only; some option names are still
  Portuguese (`tamagi panel set centro|classico`, the AI presets).
- **Proprietary agents:** Claude Code downloads from Anthropic
  (checksum-verified); Antigravity CLI is installed by Google's own install
  script (HTTPS only; it checks the SHA-512 of the binary it downloads).

## Acknowledgments

Tamagi is developed with AI coding agents under the author's direction: Gemini
(Google, through Antigravity) and Claude (Anthropic) are listed contributors.
See [CONTRIBUTORS.md](CONTRIBUTORS.md).

Upstream projects: Universal Blue (`base-main`, Fedora 45 packages), Open
Gaming Collective (kernel), Bazzite (kernel, tuning, gaming stack), Omarchy
(keyboard workflow, themes, web apps), COSMIC (System76), tik and the
base-only install model (openSUSE Aeon), GrapheneOS / secureblue
(hardened_malloc and security model), bootc, distrobox, llama.cpp, mise and
Rust for Linux. None of these projects endorse Tamagi. Forks and the
contribution plan: [UPSTREAMS.md](UPSTREAMS.md).

## License

Repository code, including the Rust tools in `rust/` and `welcome/`, is
licensed under BSD-3-Clause ([LICENSE](LICENSE)).
Built images aggregate third-party software under its own licenses. Tamagi is
an independent project, not affiliated with or endorsed by Fedora, Red Hat,
System76, Canonical, Microsoft or Basecamp. Fedora logos are replaced with
generic-logos.

Tamagi does not redistribute Windows, proprietary browsers or proprietary AI
agents: `tamagi windows`, `tamagi browser` and `tamagi agents` download them
from their vendors on the user's machine. See [NOTICE](NOTICE)
(`/usr/share/doc/tamagi/NOTICE` in the image).
