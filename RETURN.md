# RETURN.md — Tamagi OS 1.7 ("Kazunobu M") Project State

Last updated: 2026-11-28 (branch `feat/v1.7-composefs-opencode-ux` — **Tamagi OS 1.7 "Kazunobu M"** built and validated end-to-end across `Tamagi-DxAI`, `Tamagi-SKL`, and standalone `SKL` fork, plus 1.7 infographics and 1080p60 promotional video).

## Current Artifacts (`Tamagi-DxAI 1.7` + `Tamagi-SKL 1.7` + `SKL`)

- **Local multi-edition ISO (`:dxai` + `:skl` + `:skl-os` deduplicated in `tamagi-oci/`):**
  - `output/imagens_iso/tamagi_nova.iso` (**6.7 GB**, built 2026-11-28, COSMIC `1.12.0` + `bootc 1.17.5` + kernel `7.3.5-ogc3.1.fc45.x86_64`)
  - **SHA-256:** `4c9e05358bbff03a055c615aa566eced7f86a63d8237e535bc3b54c67d9dabae` (`output/imagens_iso/tamagi_nova.iso.sha256`)
- **OCI images built locally (`localhost/tamagi:latest`, `localhost/tamagi-skl:latest`, `localhost/skl:latest`):**
  - Registry targets: `ghcr.io/wes-tamagi/tamagi:latest` (`Tamagi-DxAI 1.7`), `ghcr.io/wes-tamagi/tamagi-skl:latest` (`Tamagi-SKL 1.7`), and `ghcr.io/wes-tamagi/skl:latest` (`SKL` standalone fork in [`skl/`](./skl/README.md))
- **Infographics & Promotional Video (`1.7`):**
  - Technical & 1.7 release infographic: [`docs/infografico-tamagi-1.7.svg`](./docs/infografico-tamagi-1.7.svg), [`docs/infografico-tamagi-1.7.png`](./docs/infografico-tamagi-1.7.png) (also mirrored to `docs/infografico-tamagi-dev.{svg,png}`)
  - Beginner / Windows switcher infographic (`1.7`): [`docs/infografico-tamagi-iniciantes.svg`](./docs/infografico-tamagi-iniciantes.svg), [`docs/infografico-tamagi-iniciantes.png`](./docs/infografico-tamagi-iniciantes.png)
  - 1080p 60fps H.264 + AAC promotional video (36s): [`docs/tamagi-1.7-promo.mp4`](./docs/tamagi-1.7-promo.mp4) (generator: [`tools/make-promo-video.py`](./tools/make-promo-video.py))

> **Install with Secure Boot disabled in firmware.** The installer detects Secure Boot on screen 1 and blocks before touching the disk because installed systems boot unsigned `systemd-boot` (end-to-end Secure Boot is tracked in [ROADMAP-2.0.md](./ROADMAP-2.0.md)).

### Validation Summary

| Check | Result |
|---|---|
| `just lint` (ShellCheck + Python AST + anti-legacy CLI grep + i18n `pt/es/fr`) | **Passed** (`936 textos + actions.toml, 0 problema(s)`, `lint ok`) |
| Rust unit tests (`cargo test --release --locked` on `rust/` + `welcome/`) | **Passed** (all unit tests in `tamagi`, `tamagi-boot`, `tamagi-doctor`, `tamagi-install`, `tamagi-why`, `tamagi-layer`, `agent-jail`, and `tamagi-welcome`) |
| `just build` (`Tamagi-DxAI 1.7` + `bootc container lint` + `rechunk`) | **Passed** (COSMIC `1.12.0` + `bootc 1.17.5` + kernel `7.3.5-ogc3.1.fc45.x86_64`) |
| `just test` (`tests/image.sh` on `Tamagi-DxAI 1.7`) | **Passed** (`443 ok, 0 falha(s)`) |
| `just build-skl` + `just test-skl` (`Tamagi-SKL 1.7`) | **Passed** (`403 ok, 0 falha(s)`) |
| `just build-skl-fork` + `just test-skl-fork` (`SKL` fork, `skl-os`) | **Passed** (`403 ok, 0 falha(s)`) |
| `just test-installer-ui` (`tamagi-welcome --install`, dark + light themes) | **Passed** (`19 pages in dark + 19 pages in light`, `answers ok`) |
| `just build-iso` (`output/imagens_iso/tamagi_nova.iso`, SHA-256 `4c9e0535…`) | **Passed** (`6.7 GB`, `:dxai` + `:skl` + `:skl-os` deduplicated) |
| `just test-iso --extra-disks` (VM install on `1.7` + `/mnt/dados` secondary disk mount + `/var/home` SELinux `home_root_t` + Combustion) | **Passed** (`RESULTADO: ok`) |
| `just test-iso-skl`, `just test-iso-skl-fork`, `just test-preserve-home` | **Passed** (`RESULTADO: ok` on branch; core `1.7` ISO re-verified with `test-iso --extra-disks` after version bump & Fedora 45 COSMIC 1.12 / SELinux 45.26 fixes) |
| `just test-upgrade` / Real hardware soak test on `1.7` | **Not run yet** |

---

## Summary of Architectural & UX Upgrades

1. **`composefs-rs` + `bootc 1.17.5` + `anomalyco/opencode`:**
   - Added `/usr/bin/cfsctl` (`bootc internals cfs` from `containers/composefs-rs`) and `tamagi sys cfs` (`/usr/bin/tamagi-cfs`: `status [--json]`, `fsck` with `--require-verity`, `images`, `measure` via `fs-verity`, `gc`), upgraded `system-update` to mount staged deployments via `bootc internals cfs --system mount`, fixed `composefs.digest=` parsing in `tamagi-boot`, added `composefs` as the 14th check in `tamagi-doctor`, and tagged `/usr` provenance with `[composefs + fs-verity]` in `tamagi-why`.
   - Added `/usr/bin/opencode` verified on-demand bootstrapper (AVX2/baseline auto-detection, SHA-256 verified), localized `opencode.desktop`, COSMIC-to-OpenCode JSON theme sync, `opencode.ai` allowlist in `egress-guard`, and the built-in read-only `mcp-tamagi-os` (`tamagi ai mcp add os`, `0 MB` idle RAM).
2. **Terminals (`Ptyxis` + `foot`, no `cosmic-term`, single-tab fix):**
   - Fixed `Super+Enter` opening two tabs in Ptyxis (`restore-session=false` in dconf + `gsettings`, cold-start `ptyxis -d "$PWD"` without `--new-window`), added lightweight `foot` (`/etc/xdg/foot/foot.ini` + theme sync) for instant popups and fallback, and removed `cosmic-term`.
3. **Icons & UI/UX Polish:**
   - Fixed warning-triangle (`!`) missing icons by creating complete `Papirus`, `Papirus-Dark`, and `Papirus-Light` inheritance chains (`Cosmic,Pop,Adwaita,hicolor`), replacing 4 non-existent symbolic icon names in `actions.toml`, adding `width`/`height` attributes to all custom SVGs, designing 6 new custom vector icons (`opencode`, `tamagi-agent`, `tamagi-menu`, `tamagi-vm`, `tamagi-lark`, `skl`), syncing `tamagi-menu` highlight color to the active theme accent, and adding live app search in `tamagi-welcome`.
4. **First Boot — LibreOffice & Expanded GPU Drivers:**
   - Added `org.libreoffice.LibreOffice`, `ge-proton`, and `opencode` to `basics.list`, created `drivers.list` (NVIDIA GeForce/RTX, NVIDIA CUDA, AMD Radeon VA-API/VDPAU, AMD ROCm, Intel Arc/Iris Xe media, Intel oneAPI Level Zero/OpenCL compute, LACT, Vulkan tools, extra codecs), upgraded `nvidia-setup` (`install`, `cuda`, `remove`, `remove-cuda`), and added a hardware-aware **GPU & hardware drivers** step (`detect_gpus()`) in both `tamagi-welcome` and `tamagi-firstrun`.
5. **Native Steam + GE-Proton + `gamescope` + MangoHud:**
   - Pre-installed native `steam` + 32-bit PipeWire/ALSA/`gamescope` libs in `Tamagi-DxAI`, enabled the **Tamagi Gaming** login session out of the box, configured `STEAM_EXTRA_COMPAT_TOOLS_PATHS` (`/usr/lib/environment.d/60-tamagi-gaming.conf`), and added `tamagi pkg gaming ge-proton [install|update|list|remove]` (SHA-512 verified) and `tamagi pkg gaming mangohud [preset|status|reset]`.
6. **Standalone `SKL` Fork (`skl-os`):**
   - Added standalone `SKL` school fork (`skl/Containerfile`, `skl/Justfile`, `skl/README.md`, `NAME="SKL"`, `LOGO=skl`, `/usr/bin/skl` CLI, GRUB boot entry, GUI edition selector in `tamagi-welcome --install`, and deduplicated `:skl-os` tag in `tamagi_nova.iso`).
7. **Secondary Disks at Installation (`/run/media` vs `/mnt/…`):**
   - Added `ExtraDisksMount` (`RunMedia` vs `Mnt`) to `tamagi-welcome --install` and `configure_extra_disks()` in `/usr/libexec/tamagi-install` (`blkid` superblock detection, `/var/mnt/<slug>` + `/mnt/<slug>` with SELinux `mnt_t` labels and `nofail,x-systemd.device-timeout=5s` in `/etc/fstab`, never erasing user data).
