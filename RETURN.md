# RETURN.md — Tamagi OS Project State

Last updated: 2026-11-26 (**v1.6** built and validated locally for both `Tamagi-DxAI` and `Tamagi-SKL`).

## Current v1.6 Artifacts (`Tamagi-DxAI` + `Tamagi-SKL`)

- **Local multi-edition ISO (points to GHCR out of the box):**
  - `output/imagens_iso/tamagi_nova.iso` (**7.3 GB**, built 2026-11-26 19:21)
  - **SHA-256:** `a90993d445e53b8138477bb5eaa645c0ddae549a16c12d34f1e4cd91f15c46ea` (`output/imagens_iso/tamagi_nova.iso.sha256`)
- **OCI images built locally (`localhost/tamagi:latest`, `localhost/tamagi-skl:latest`):**
  - Published targets on merge to `main`: `ghcr.io/wes-tamagi/tamagi:latest` (`Tamagi-DxAI`) and `ghcr.io/wes-tamagi/tamagi-skl:latest` (`Tamagi-SKL`)
- **Documentation, PDFs, and infographics:**
  - [docs/tamagi-v1.6-guide.pdf](./docs/tamagi-v1.6-guide.pdf) — illustrated Tamagi OS v1.6 guide (architecture base, screenshots, infographics, changelog, and 5-command CLI reference)
  - [docs/tamagi-cli-guide.pdf](./docs/tamagi-cli-guide.pdf) — technical Tamagi CLI reference PDF
  - [rust/README.md](./rust/README.md) — standalone `tamagi-cli` repository documentation & CI workflow
  - [docs/atualizacao.md](./docs/atualizacao.md), [docs/universo-linux.md](./docs/universo-linux.md), `docs/infografico-tamagi-{dev,iniciantes}.{svg,png}`

> **Install with Secure Boot disabled in firmware.** The installer detects Secure Boot on screen 1 and blocks before touching the disk because installed systems boot unsigned `systemd-boot` (end-to-end Secure Boot is tracked in [ROADMAP-2.0.md](./ROADMAP-2.0.md)).

### Validation Summary (v1.6)

| Check | Result |
|---|---|
| `just lint` (ShellCheck + Python AST + anti-legacy CLI grep + i18n `pt/es/fr`) | **Passed** (`880 textos + actions.toml, 0 problema(s)`, `lint ok`) |
| Rust unit tests (`cargo test --locked`, Fedora 45 container) | **Passed** (`62 passed, 0 failed`: `agent-jail` 8, `tamagi` 22, `tamagi-boot` 3, `tamagi-doctor` 4, `tamagi-install` 7, `tamagi-why` 5, `tamagi-layer` 13) |
| `just build` (`Tamagi-DxAI` + `bootc container lint` + `rechunk`) | **Passed** (kernel `7.3.5-ogc3.1.fc45.x86_64`) |
| `just test` (`tests/image.sh` on `Tamagi-DxAI`) | **Passed** (`430 ok, 0 falha(s)`) |
| `just build-skl` (`Tamagi-SKL` + `bootc container lint` + `rechunk`) | **Passed** |
| `just test-skl` (`tests/image.sh` on `Tamagi-SKL`) | **Passed** (`393 ok, 0 falha(s)`) |
| `just build-iso` (`output/imagens_iso/tamagi_nova.iso`, SHA-256 `a90993d4…`) | **Passed** (`7.3 GB`, `DxAI` + `SKL` deduplicated) |
| `just test-iso` / `just test-iso-skl` / `just test-upgrade` on v1.6 ISO | **Not run yet** |
| Real hardware soak test (AMD machines, v1.5 baseline) | **Passed** (ran smoothly on AMD hardware; re-test v1.6 theme contrast, Wi-Fi powersave, and SKL tab guard on real hardware) |

---

## Summary of What Is New in v1.6

1. **5-command CLI + standalone `tamagi-cli` structure:** reduced `tamagi` from 11 to 5 commands (`update`, `pkg`, `ai`, `dev`, `sys`) while keeping `tamagi-layer` in `/usr/bin/` and `tamagi pkg layer`. Modernized UX between `dnf5` (aligned transaction tables) and `pacman` (`::` phase headers, progress bars, Braille TTY spinners) in `rust/tamagi/src/ui.rs` and `/usr/libexec/tamagi/ui-spinner`.
2. **Unified `tamagi-agent`:** removed duplicate `/usr/bin/tamagi-agents`; moved backends to `/usr/libexec/tamagi/{agent-local,agent-manager}`, added `~/.config/tamagi/ai.toml`, readable `flights show`, per-project `[egress]` in `tamagi-workspace.toml`, and anti-legacy CLI lint rules.
3. **Zero-fork Rust fast-boot (`tamagi-boot`):** replaced Bash `once-per-image` and `loader-conf` with a static multi-call Rust binary (`rust/tamagi/src/boot_bin.rs`), eliminating ~35 shell forks across boot units.
4. **UI/UX, Icons, Fonts, and Themes:** fixed Ipê dark accent contrast (`#f2c14e` 1.68:1 → `#9c660b` **4.86:1 WCAG AA**) and softened foreground (`#d8d3c0`), deepened Ipê Claro accent to `#734f00` (7.38:1 WCAG AAA), added `papirus-icon-theme`, `rsms-inter-fonts`, `google-noto-serif-vf-fonts`, `hintslight` fontconfig, unbranded COSMIC NASA/ESA wallpapers, and `TAMAGI` wordmark on 2 of 6 Ipê wallpapers (`ipe-mon`, `ipe-light-mon`).
5. **Storage, COSMIC Apps, and Cleanup:** replaced `cosmic-ext-storage` with `gnome-disk-utility`, removed `foot` (migrated `popup`/`term` to `ptyxis` + `cosmic-term` fallback), cleaned dead `btop` theme code, moved `Warehouse` to optional, removed `Super+Shift+W`, promoted native COSMIC Rust apps (`Popsicle`, `Cedilla`, `Calculator`, `CosmicTweaks`, `Examine`, `Tasks`, `Forecast`), updated local AI presets (`presets.tsv`, 12 models including `Phi-4 Mini`), and fixed `tamagi-update` so it no longer warns about missing Arch devboxes.
6. **Network & Diagnostics:** added `wifi.powersave = 2`, `tcp_slow_start_after_idle = 0`, tuned `tcp_rmem`/`tcp_wmem`/`netdev_max_backlog`, `systemd-resolved` caching + `DNSSEC=allow-downgrade`, and `tamagi sys doctor --report` (13 checks including `network` and NPU detection).
7. **`Tamagi-SKL` (School Edition):** installs **OnlyOffice**, **TurboWarp**, and **Papers** via `flatpak-setup`, and enforces max **2 tabs & 1 window** via offline MV3 extension `/usr/share/tamagi/skl-tab-guard` + hardened Chromium managed policy (`IncognitoModeAvailability: 1`, `DeveloperToolsAvailability: 2`, `URLBlocklist`).

---

## Next Cycle

See [ROADMAP-2.0.md](./ROADMAP-2.0.md) for **Tamagi 2.0 ("Tachico B")** — TunaOS-grade promote-what-you-gate CI, `/etc` drift journal, Bluefin AI + `tamagi-linux-mcp`, COSMIC Epoch 2/3, and end-to-end Secure Boot.
