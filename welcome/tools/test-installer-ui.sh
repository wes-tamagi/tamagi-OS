#!/usr/bin/bash
# Walks the installer window (tamagi-welcome --install) without a monitor, in
# the dark and in the light theme, with the stub back ends: every page must
# render and the answers file must hold what was typed. The PNGs stay in
# output/installer-shots/{dark,light}/ for a visual review.
#   just test-installer-ui            (needs the image: just build)
# Runs tamagi-welcome in a Fedora container; the COSMIC theme files come from
# the image, so the screenshots show the theme and the interface font the
# live media really has.
set -euo pipefail
cd "$(dirname "$0")/../.."
IMAGE=${1:-localhost/tamagi:latest}
FEDORA=${FEDORA_VERSION:-45}
OUT=output/installer-shots
CACHE=.cache/installer-ui
PAGES=17
mkdir -p "$OUT" "$CACHE/cargo" "$CACHE/target"
rm -rf "$CACHE/cosmic" "$CACHE/fonts" "${OUT:?}/dark" "${OUT:?}/light"

cid=$(podman create "$IMAGE")
trap 'podman rm -f "$cid" >/dev/null 2>&1 || true' EXIT
podman cp "$cid:/usr/share/cosmic" "$CACHE/cosmic"
# A fonte da interface (Roboto Flex) também vem da imagem.
podman cp "$cid:/usr/share/fonts/roboto-flex" "$CACHE/fonts"

podman run --rm \
    -v .:/src:ro,z -v "./$CACHE/cargo:/cargo:z" -v "./$CACHE/target:/target:z" \
    -v "./$CACHE/cosmic:/usr/share/cosmic:ro,z" -v "./$CACHE/fonts:/usr/share/fonts/roboto-flex:ro,z" -v "./$OUT:/shots:z" \
    -e CARGO_HOME=/cargo -e CARGO_TARGET_DIR=/target -e CARGO_NET_GIT_FETCH_WITH_CLI=true \
    -w /src/welcome "quay.io/fedora/fedora:$FEDORA" bash -c '
set -euo pipefail
dnf -y -q install --setopt=install_weak_deps=False git-core cargo rust gcc pkgconf-pkg-config \
    libxkbcommon-devel wayland-devel fontconfig-devel freetype-devel sway grim wtype \
    wlr-protocols-devel dbus-daemon cosmic-icon-theme google-noto-sans-fonts \
    google-noto-sans-mono-fonts openssl >/dev/null
cargo build --release --locked 2>&1 | tail -1
for theme in dark light; do
    mkdir -p /shots/$theme
    log=$(OUT=/shots/$theme THEME=$theme BIN=/target/release/tamagi-welcome SHARE=/src/system_files/usr/share/tamagi \
        APP_WAIT=3 dbus-run-session tools/screenshots.sh installer 2>&1)
    for want in LOCALE=pt_BR.UTF-8 KEYMAP=br-abnt2 TIMEZONE=America/Sao_Paulo USER=ana "FULLNAME=Ana Lima" LUKS_KEYFILE= LUKS_TPM=pin LUKS_TPM_PINFILE= PRESERVE_LUKS=1; do
        grep -q "^$want" <<<"$log" || { echo "$theme: the answers lack $want"; echo "$log" | tail -30; exit 1; }
    done
    echo "$theme: answers ok"
done'

for theme in dark light; do
    n=$(find "$OUT/$theme" -name 'i*.png' -size +10k | wc -l)
    [[ $n -eq $PAGES ]] || { echo "FAILED: $theme has $n of $PAGES screenshots"; exit 1; }
done
echo "installer ui ok: $PAGES pages in the dark and in the light theme ($OUT)"
