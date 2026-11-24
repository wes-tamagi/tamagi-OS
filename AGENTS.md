# AGENTS.md — Tamagi

Guide for every AI agent working in this repository (Claude, Gemini, Codex…).
Adapted from the model of Project Bluefin's Dakota
(`projectbluefin/dakota`, branch `testing`, `AGENTS.md`) to how Tamagi is
built.

Tamagi is an atomic desktop: a bootc OCI image on top of Universal Blue
`base-main` (Fedora stable) with the COSMIC desktop and the OGC kernel. It is
built with `podman build` from the `Containerfile` and `build_files/*.sh`;
files that go into the image as they are live in `system_files/`. The
installer of the live media is `tamagi-welcome --install` (Rust + libcosmic,
`welcome/`) over the `/usr/libexec/tamagi-install` engine (`live/`).

## Roles

| Who | Role | Decides |
|---|---|---|
| Wes | Owner | everything, last word |
| Gemini | CEO | product, design, research, priorities (`gemini-serv.md` → `gemini-retorno.md`) |
| Claude | CTO | architecture, code, build, tests (`claude-serve.md` holds the CEO's orders) |

A CEO order becomes work only after the owner agrees with it. Product or
design questions that are still open go to `gemini-serv.md`; they are not
decided in code.

## Skill routing

| Task | Load |
|---|---|
| Saying a change works, before a commit that touches `system_files/`, `build_files/`, `live/`, `rust/`, `go/`, `welcome/` or `tests/` | `.claude/skills/tamagi-validate` |

Load only the skill the task needs.

## Non-negotiable safety

- Never write to `ublue-os/*`, `pop-os/*`, `fedora/*` or any other upstream:
  no issues, comments, pull requests or forks. Ask the owner to report it.
- Never post GitHub comments, reviews or releases unless asked.
- Never expose secrets, and never weaken signing (cosign), Secure Boot,
  SELinux or the checks of the build.
- Never push to `main`. Work on a branch; the owner merges.
- Never run podman or distrobox inside a `bwrap` sandbox on the development
  machine: it breaks the user's rootless podman.

## Sources of truth

1. Read the file you change and what calls it before editing.
2. Executable configuration wins over prose: the `Containerfile`,
   `build_files/`, the `Justfile` and `tests/` are the truth; a README or
   doc that disagrees with them is stale and must be fixed.
3. Check external syntax and behaviour (bootc, composefs, COSMIC, systemd,
   dnf5, cosign) in the current upstream docs before changing how it is used.
4. Keep knowledge by correcting the existing doc or skill. Do not append
   dated session logs, changelogs or "lessons learned" lists.

## Development workflow

- `just` lists every task. Run it before inventing a command; add a
  `Justfile` recipe when a repeatable operation has none.
- The validation chain, in order: `just lint` → `just build` → `just test`
  → `just test-installer-ui` (installer window) → `just build-iso` →
  `just test-iso` (+ `--preserve-home`, `just test-upgrade` when the change
  touches them). Use the narrowest check that exercises the change, and say
  which ones ran.
- Never report green from a build alone. A check that was skipped, not run,
  or run on an older image is **not validated**. Hardware results are not
  interchangeable with VM results.
- Before saying it is done, read the diff and list the checks that ran.
- New text shown to users needs its line in
  `system_files/usr/share/tamagi/i18n/{pt,es,fr}.tsv` (`just lint` fails
  otherwise). Interfaces and commands are in English.

## Architecture invariants

- Packages enter only through `build_files/` (dnf5 at build time). Nothing is
  installed on the host at runtime except through `tamagi-layer`
  (systemd-sysext) or Flatpak in the user.
- Fedora stable plus the stable OGC kernel. No Bazzite unstable, no rawhide
  base, no Arch/Hummingbird/Liquorix.
- The image gets smaller, not bigger: whatever is optional goes to the
  catalog (`system_files/usr/share/tamagi/catalog.tsv`). Exceptions fixed by
  the owner: virtualization (QEMU/libvirt/Cockpit Machines), the AI and
  container stack.
- Build steps are deterministic: no network access outside package
  repositories and pinned sources, no timestamps, no host identity.
- Every new third-party source (a GitHub download, a COPR, a new registry)
  is pinned to a commit or digest with a checksum, and needs the owner's
  approval first.
- Boot work that only depends on the image runs once per image
  (`/usr/libexec/tamagi/once-per-image`), never on every boot.

## Human decision gates

Stop and ask the owner before:

- architecture or new-subsystem decisions;
- user-visible behaviour changes without agreed acceptance criteria;
- signing, secrets or third-party supply-chain changes;
- removing something users rely on;
- committing, pushing, releasing or publishing an ISO.

## Commits

`<type>(<scope>): <description>` in English (e.g.
`fix(boot): run ldconfig once per image`), with the attribution trailer the
agent's tool requires.
