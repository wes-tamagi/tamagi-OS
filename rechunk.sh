#!/usr/bin/bash
# rechunk.sh — rewrites the built image as a fresh, chunked bootc image.
#
#   rechunk.sh <source image> <output image> [work dir]
#
# Why: the image is built on top of an ostree-encapsulated base. What the
# build replaces or removes from the base (the swapped kernel, the trimmed
# locales) stays in the lower layers and in the base's object store
# (/sysroot/ostree): about 9.3 GB for 6.5 GB of real content. A plain squash
# would break the ostree labels that bootc relies on. `rpm-ostree compose
# build-chunked-oci` is the supported way: it builds a new image from the root
# filesystem alone, split in content-based layers (so an update downloads only
# the layers that changed).
#
# Rootless: runs inside `podman unshare`, where the image can be mounted, and
# hands the mounted root (read-only) to a container of the image itself. That
# container has a storage of its own (in the work dir), where the tool writes
# the result; it leaves as an OCI archive, which is then loaded here.
set -euo pipefail

SRC=${1:?usage: rechunk.sh <source image> <output image> [work dir]}
OUT=${2:?usage: rechunk.sh <source image> <output image> [work dir]}
WORK=${3:-$(pwd)/output/rechunk}

if [[ ${RECHUNK_INNER:-} != 1 ]]; then
    mkdir -p "$WORK"
    # As root (CI) the image can be mounted directly; rootless needs podman's namespace.
    if (( EUID == 0 )); then RECHUNK_INNER=1 exec bash "$0" "$SRC" "$OUT" "$WORK"; fi
    RECHUNK_INNER=1 exec podman unshare bash "$0" "$SRC" "$OUT" "$WORK"
fi

# The new image is made from the files alone and starts with no labels. bootc
# records the image version at install time, and bootupd refuses a record
# without it (bootloader-update.service fails at every boot): the version
# label is carried over. The other labels of the source describe the base
# image (its title, its kernel, an expiry date for its registry) and stay out.
version=$(podman inspect "$SRC" --format '{{index .Config.Labels "org.opencontainers.image.version"}}')
labels=(--label "org.opencontainers.image.title=Tamagi"
        --label "org.opencontainers.image.description=Tamagi — atomic COSMIC desktop for AI and agents (bootc)")
[[ -n $version && $version != "<no value>" ]] || version=$(date -u +%Y%m%d)
labels+=(--label "org.opencontainers.image.version=$version")

# The work dir is this script's own (a private container storage and the
# archive that leaves it): emptied before and after.
clean_work() { rm -rf -- "${WORK:?}/storage" "${WORK:?}/image.oci.tar"; }
clean_work
mkdir -p "$WORK/storage"

mnt=$(podman image mount "$SRC")
trap 'podman image umount "$SRC" >/dev/null 2>&1 || true' EXIT
echo "rechunk: $SRC mounted at $mnt"
podman run --rm --privileged --security-opt label=disable \
    -v "$mnt:/rootfs:ro" -v "$WORK:/work" -v "$WORK/storage:/var/lib/containers" \
    -e LAYERS="${RECHUNK_LAYERS:-96}" \
    "$SRC" bash -c '
        set -euo pipefail
        rpm-ostree compose build-chunked-oci --bootc --format-version 2 \
            --max-layers "$LAYERS" --rootfs /rootfs "$@" \
            --output containers-storage:localhost/rechunked:latest
        skopeo copy --quiet containers-storage:localhost/rechunked:latest oci-archive:/work/image.oci.tar
    ' rechunk "${labels[@]}"
podman image umount "$SRC" >/dev/null
trap - EXIT
echo "rechunk: importing as $OUT"
skopeo copy --quiet "oci-archive:$WORK/image.oci.tar" "containers-storage:$OUT"
clean_work
podman images --format '{{.Repository}}:{{.Tag}}  {{.Size}}' "$OUT"
