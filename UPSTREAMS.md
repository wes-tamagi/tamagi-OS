# Upstreams, forks and contribution plan

Tamagi stands on other projects. This file says which ones we fork, which ones
we only reference, and what we intend to send back upstream.

A fork here is a **contribution fork**: work happens on a branch of the fork
and goes upstream as a pull request. A fork does not change where the image
gets its software: the build keeps using the upstream sources and Fedora
packages, at the versions pinned in the `Containerfile`, until a fork carries
a change we need that upstream has not merged.

## Forks (github.com/wes-tamagi)

| Fork | Upstream | License | Role in Tamagi |
|---|---|---|---|
| [cosmic-epoch](https://github.com/wes-tamagi/cosmic-epoch) | pop-os/cosmic-epoch | GPL-3.0 | The COSMIC desktop meta-repository; entry point for our COSMIC contributions |
| [cosmic-comp](https://github.com/wes-tamagi/cosmic-comp) | pop-os/cosmic-comp | GPL-3.0 | Compositor; reference for following the HDR work (see "HDR") |
| [cosmic-panel](https://github.com/wes-tamagi/cosmic-panel) | pop-os/cosmic-panel | GPL-3.0 | Panel and dock; improvements to send upstream |
| [cosmic-edit](https://github.com/wes-tamagi/cosmic-edit) | pop-os/cosmic-edit | GPL-3.0 | Text editor; improvements to send upstream |
| [cosmic-term](https://github.com/wes-tamagi/cosmic-term) | pop-os/cosmic-term | GPL-3.0 | Terminal; bring over what Ptyxis does with containers (see "Terminal") |
| [cosmic-ext-classic-menu](https://github.com/wes-tamagi/cosmic-ext-classic-menu) | championpeak87/cosmic-ext-classic-menu | GPL-3.0 | The start menu, compiled into the image |
| [tik](https://github.com/wes-tamagi/tik) | sysrich/tik | MIT | The installer (Tamagi's module lives in `live/tik`) |
| [llama.cpp](https://github.com/wes-tamagi/llama.cpp) | ggml-org/llama.cpp | MIT | Local AI, compiled into the image with Vulkan |
| [distrobox](https://github.com/wes-tamagi/distrobox) | 89luca89/distrobox | GPL-3.0 | Dev boxes; the image patches `distrobox-create` for podman 6 |
| [CLIO](https://github.com/wes-tamagi/CLIO) | SyntheticAutonomicMind/CLIO | GPL-3.0 | Coding agent offered in the catalog |
| [llm-scripts](https://github.com/wes-tamagi/llm-scripts) | fewtarius/llm-scripts | GPL-3.0 | Run profiles for local models |
| [lazydocker](https://github.com/wes-tamagi/lazydocker) | jesseduffield/lazydocker | MIT | Terminal view of containers (optional) |
| [main](https://github.com/wes-tamagi/main) | ublue-os/main | Apache-2.0 | The base image (`base-main`) |
| [akmods](https://github.com/wes-tamagi/akmods) | ublue-os/akmods | Apache-2.0 | OGC kernel and signed modules |
| [bluefin](https://github.com/wes-tamagi/bluefin) | ublue-os/bluefin | Apache-2.0 | Reference for the developer setup (VS Code + Dev Containers) |
| [bazzite](https://github.com/wes-tamagi/bazzite) | ublue-os/bazzite | Apache-2.0 | Reference for gaming: Valve's gamescope, game session, HDR |

## Referenced only

- **Ptyxis** ([gitlab.gnome.org/chergert/ptyxis](https://gitlab.gnome.org/chergert/ptyxis)),
  GPL-3.0 — the system terminal today, from the Fedora package. Hosted on
  GNOME's GitLab, not GitHub.
- **foot** ([codeberg.org/dnkl/foot](https://codeberg.org/dnkl/foot)), MIT —
  the terminal behind Tamagi's own windows, from the Fedora package.
- **cosmic-utils** ([github.com/cosmic-utils](https://github.com/cosmic-utils)) —
  community apps written for COSMIC (Tasks, Forecast, Examine, Calculator,
  Camera). Candidates for an optional "COSMIC apps" catalog category,
  installed from Flathub.
- **Docker CE**, **VS Code**, **Tailscale**, the browsers and third-party AI
  agents — optional, downloaded from their vendors on the user's machine.
- **openSUSE Aeon/MicroOS** (btrfs maintenance, the installer idea),
  **Omarchy** (keyboard-driven use, themes), **Fuduntu** (automatic power
  profiles, not done yet).

## Contribution plan for COSMIC

Apart from the theme submission, nothing below has been
written yet; this is the order of work.

### Terminal: cosmic-term with what Ptyxis does

Today Ptyxis is the system terminal (`Super+Enter`) because it knows distrobox
boxes and podman containers, and foot runs Tamagi's own windows because it
starts instantly and takes command, title and window id on the command line.
The goal is one native COSMIC terminal that makes both unnecessary. In the
cosmic-term fork, in this order:

1. Container menu: list distrobox boxes and podman containers and open a tab
   inside one, as Ptyxis does.
2. Per-container profiles (colour and title of the tab follow the container).
3. Command-line options foot has and cosmic-term lacks: command to run, window
   title, application id.
4. Fast start (server/client mode, as `footclient`).

Each item goes upstream as its own pull request. When the fork covers items
1 to 3, cosmic-term replaces Ptyxis and foot in the image.

### cosmic-panel and cosmic-edit

Improvements come from what Tamagi already works around:

- **cosmic-panel:** group windows under the right dock icon when the window's
  application id differs from the desktop file name (we had to rename ours);
  let a distribution ship a default panel layout without replacing the user's.
- **cosmic-edit:** changes are to be chosen from daily use; no list yet. The
  editor in the image stays Fedora's cosmic-edit; there will be no parallel
  editor (a Tauri rewrite was evaluated and dropped).

### Theme: Tamagi Ipê on cosmic-themes.org

[cosmic-themes.org](https://cosmic-themes.org/) takes themes through a web
form (`/create/`): name, author, link and the `.ron` file exported from COSMIC
Settings. No pull request is involved. The image already generates the file
(`/usr/share/cosmic-themes/tamagi-ipe.ron`). Submitted on 2026-11-22 as
"Tamagi Ipê" by wes-tamagi; the site reviews submissions before listing them.

### HDR

Tracking issue: [pop-os/cosmic-comp#1384](https://github.com/pop-os/cosmic-comp/issues/1384)
(open; nothing merged upstream).

What exists, according to the issue thread (not verified by us):

- **franzjeger/cosmic-hdr-reference** — a snapshot with HDR10/BT.2020/PQ
  output, `wp_color_management_v1`, and scRGB passthrough for Wine/Proton.
  Its author reports it validated on an NVIDIA RTX 5090 with two HDR displays.
- **Liquidmasl/cosmic-comp, branch `hdr`** (plus Liquidmasl/smithay, branch
  `hdr`) — the same work rebased onto current cosmic-comp: about +2.7k lines
  in cosmic-comp and +3.8k in Smithay. Reported working on an RTX 4090 with a
  Proton game in HDR; its README says the original was also validated on an
  AMD 680M and that Intel is untested. It runs as a separate "COSMIC (HDR)"
  login session, so the stock session stays as the fallback.
- An earlier implementation by jibsta210 stalled on colour tuning and failed
  on an AMD two-display setup.

In Tamagi: **desktop HDR is not shipped and not offered as an option.** An
opt-in experimental session was evaluated and dropped (2026-11-22). The
desktop waits for HDR in the official cosmic-comp; when the COSMIC team picks
an implementation we can help with testing and review.

HDR in games does not depend on this: the gaming layer runs gamescope with HDR
on displays that announce it.

## Fedora Hummingbird and Bluefin "utah"

[Fedora Hummingbird](https://fedoraproject.org/wiki/Hummingbird) is a
container-native, rolling variant built by a Fedora SIG: minimal, distroless
OCI images built from Fedora sources, aiming at near-zero CVEs.
[projectbluefin/utah](https://github.com/projectbluefin/utah) is Bluefin
rebuilt on the Hummingbird bootable base
(`quay.io/hummingbird-community/os-minimal`), with GNOME compiled from source
in a second repository. Utah calls itself experimental pre-alpha and does not
boot with Secure Boot yet.

**Moving Tamagi's base to Hummingbird now is not recommended:** the base ships
no desktop, so COSMIC would have to be compiled and maintained by us, and we
would lose Secure Boot and the signed OGC kernel modules we get from Universal
Blue today. Worth revisiting when utah leaves pre-alpha.

What we can adopt from utah without changing base:

| Practice in utah | Status in Tamagi |
|---|---|
| Every input image pinned by digest, updated by Renovate | Partly: versions pinned in the `Containerfile`; no Renovate |
| `:testing` and `:stable` streams; `:stable` moves only after the ISO test passes on the same digest | Not done: one `latest` tag, published from `main` |
| Install test in CI on a LUKS disk, with a published verification record | Partly: `tests/iso.sh` covers plain, LUKS and TPM installs, run locally, not in CI |
| Masking `bootc-fetch-apply-updates.timer`, which `/etc` can carry over from another bootc image | To check: matters for the Bluefin/Aurora migration in `tamagiguide.md` |
| Hardware flavors (`nvidia`, `gaming`) from one config file | Not done: one image; gaming is an optional layer; no NVIDIA variant |
| Minimal base with a manifest of everything added on top | Close: `build_files/packages.txt` |

## The Bazzite fork

`wes-tamagi/bazzite` is an unmodified fork of ublue-os/bazzite, kept as a
read-only reference for gamescope, the game session and HDR settings. Tamagi
gets what it needs from Bazzite without it: the OGC kernel and modules come
from ublue-os/akmods (the version Bazzite's `stable` ships), and the gaming
layer installs the packages at run time. Nothing from Bazzite's `testing` or
`unstable` streams goes into the image.
