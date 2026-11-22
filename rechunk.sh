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
    mkdir -p "$WORK/storage"
    [[ ! -e $WORK/image.oci.tar ]] || { echo "rechunk: $WORK/image.oci.tar already exists; remove it first" >&2; exit 1; }
    # As root (CI) the image can be mounted directly; rootless needs podman's namespace.
    if (( EUID == 0 )); then RECHUNK_INNER=1 exec bash "$0" "$SRC" "$OUT" "$WORK"; fi
    RECHUNK_INNER=1 exec podman unshare bash "$0" "$SRC" "$OUT" "$WORK"
fi

mnt=$(podman image mount "$SRC")
trap 'podman image umount "$SRC" >/dev/null 2>&1 || true' EXIT
echo "rechunk: $SRC mounted at $mnt"
podman run --rm --privileged --security-opt label=disable \
    -v "$mnt:/rootfs:ro" -v "$WORK:/work" -v "$WORK/storage:/var/lib/containers" \
    -e LAYERS="${RECHUNK_LAYERS:-96}" \
    "$SRC" bash -c '
        set -euo pipefail
        rpm-ostree compose build-chunked-oci --bootc --format-version 2 \
            --max-layers "$LAYERS" --rootfs /rootfs \
            --output containers-storage:localhost/rechunked:latest
        skopeo copy --quiet containers-storage:localhost/rechunked:latest oci-archive:/work/image.oci.tar
    '
podman image umount "$SRC" >/dev/null
trap - EXIT
echo "rechunk: importing as $OUT"
skopeo copy --quiet "oci-archive:$WORK/image.oci.tar" "containers-storage:$OUT"
podman images --format '{{.Repository}}:{{.Tag}}  {{.Size}}' "$OUT"
