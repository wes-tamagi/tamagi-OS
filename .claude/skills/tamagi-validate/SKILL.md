---
name: tamagi-validate
description: Validate a change to the Tamagi image before saying it works — which checks to run, in which order, what each one proves and what it does not. Use before reporting a change as done, before a commit that touches system_files/, build_files/, live/, rust/, go/, welcome/ or tests/, and whenever asked "is the ISO good?".
---

# Validate a Tamagi change

"It builds" is the weakest claim this project can make. A change is validated
only by a check that ran, on the image that contains the change, and passed.
A check that was skipped, not run, or run on an older image counts as **not
validated** — say so instead of rounding it up.

(The rule is borrowed from TunaOS's "green criteria": never report green from
a build alone, and "never tested" is not "passed".)

## The chain

Run from the repository root, in this order. Each step needs the previous one.

| Step | Command | Proves | Does not prove |
|---|---|---|---|
| 1 | `just lint` | shell and Python parse; every UI string has pt/es/fr | anything at runtime |
| 2 | `just build` | the image builds; Rust and Go unit tests pass (they run inside the build) | that it boots |
| 3 | `just test` | files, services, commands and their logic inside the image (`tests/image.sh`) | a real boot: systemd is not running there |
| 4 | `just build-iso` | the live ISO is assembled from the image of step 2 | that it installs |
| 5 | `just test-iso` | the ISO installs unattended in a VM and the installed system boots and passes `tests/vm_check.py` | real hardware, Secure Boot keys, GPUs |

Optional, by what the change touches:

- installer, disk layout, `/var/home`: `just test-iso --preserve-home`
- update, rollback, boot-check, timers: `just test-upgrade`
- live boot, dracut, kernel command line: check `output/vm-install.log` for
  `dracut Warning` / `deprecated` after step 5
- encrypted disk, TPM: `bash tests/iso.sh output/tamagi.iso --tpm`

One loop that stops at the first failure and keeps the logs:

```bash
for step in lint build test build-iso test-iso; do
    just $step > "$LOGS/$step.log" 2>&1 || { echo "FAILED: $step"; break; }
done
```

## Rules learned the hard way

- **Do not edit the tree while the chain runs.** `just build` reads
  `system_files/`, `build_files/`, `rust/`, `go/` and `welcome/` during the
  build; `just test` reads `tests/`; `just build-iso` reads `live/`. An edit in
  the middle gives an ISO that is not the tree you tested. Wait, or state
  exactly which step had already finished reading.
- **A new check in `tests/image.sh` needs an image that has the feature.**
  Add the check, then rebuild; do not add it while an older image is being
  tested.
- **`tests/image.sh` runs in a container without systemd.** `systemctl
  is-enabled` and `systemd-analyze verify` work; `systemctl show`, `status`
  and `start` do not. Test unit *files* there, and behaviour in `vm_check.py`.
- **New text shown to the user** needs a line in each of
  `system_files/usr/share/tamagi/i18n/{pt,es,fr}.tsv`, or step 1 fails.
- **Never run podman or distrobox inside a bwrap overlay** on the development
  machine: it corrupts the user's rootless podman state.
- **Do not `pkill -f <pattern>` from the same shell line that contains the
  pattern**: it kills the validation itself. Kill by PID.
- **The ISO changed? So did its hash.** After step 4, rewrite
  `output/tamagi.iso.sha256`, and do not quote a hash while another run is
  writing `output/tamagi.iso`.
- **Destructive or security features** (leases, reset, harden) need a
  simulated mode for step 3 (an environment variable that prints instead of
  acting) *and* an honest note that the real path was not exercised, until
  `vm_check.py` covers it.

## Reporting

State, for each step, one of: passed (with the count: `430 ok, 0 falha(s)`),
failed (with the failing line), or **not run**. Then list separately what only
real hardware can confirm. Never write "validated" for the whole when any step
that the change needs was not run.
