# Tamagi recipes

Small chores that are a few commands each. Run one with `tamagi recipe <name>`;
see what it would do with `tamagi recipe show <name>`. Write your own, in the
same format, with `tamagi recipe edit`: a recipe of yours with the same name
replaces the one here.

Only the ```just blocks run. The text around them is the explanation.

## Disk space

Shows what is taking space in your home folder, largest first.

```just
# what is taking space in your home folder
space:
    @dust -d 2 -n 20 "$HOME"
```

Removes what can be downloaded or built again: Flatpak runtimes nothing uses
any more, and container images and layers no container uses. Your files, apps
and boxes are not touched.

```just
# frees disk space: unused Flatpak runtimes and container images
clean:
    flatpak uninstall --user --unused --noninteractive
    podman image prune --force
```

## What changed

The last boots, and whether each one passed the health check.

```just
# the last boots and their health checks
boots:
    @journalctl --no-pager --unit tamagi-boot-check.service --output short --lines 20
```

The errors of this boot, without the noise.

```just
# the errors logged since this boot
errors:
    @journalctl --no-pager --boot --priority err --lines 60
```

## Network

Your addresses: on the local network and as the internet sees you.

```just
# local and public IP addresses
ip:
    @ip -brief -4 address show scope global
    @printf 'public  '; curl --silent --max-time 5 https://ifconfig.me || echo "(offline)"; echo
```
