# SKL (Temporary Codename — Fork of Tamagi-SKL)

`SKL` (`EDITION=skl-os`, `NAME="SKL"`) is the standalone educational and institutional operating system forked from `Tamagi-SKL`.

## Architecture

- **Immutable Atomic Core:** Fedora stable (`ublue-os/base-main`) + OGC kernel + `bootc` + `composefs-rs` (`fs-verity` read-only `/usr`) + `systemd-boot`.
- **Two-Account Classroom Model:**
  - **Administrator (`wheel`):** Teacher / IT account with SSH (`sshd.service`) and Cockpit Web (`:9090`) enabled out of the box.
  - **Student (`aluno`):** Passwordless auto-login into COSMIC with an immutable panel (`chattr +i`), managed Chromium policy (zero saved passwords, cookies cleared on exit, 15-day history retention), Google Workspace + Gemini web apps, OnlyOffice, TurboWarp, and Okular.
- **Zero Heavy Dev/AI Bloat:** No graphical app store (`cosmic-store` removed), no `llama.cpp`, no coding agents, and no native Steam.
- **Management CLI:** `sudo skl status`, `sudo skl history [days]`, `sudo skl homepage <url>`, `sudo skl open <url>`, `sudo skl lock|unlock`, `sudo skl reset-session`, `sudo skl panel-lock|panel-unlock`.

## Building & Testing

From the repository root:

```bash
just build-skl-fork     # builds localhost/skl:latest
just test-skl-fork      # runs tests/image.sh against SKL
just build-iso          # embeds :dxai, :skl, and :skl-os deduplicated into the ISO
just test-iso-skl-fork  # installs and boots SKL in a UEFI + TPM2 VM
```
