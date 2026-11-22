set -euo pipefail
export LC_ALL=C
work=$1 name=$2 allow_upgrade=$3
R=$work/root
dbpath=$(rpm --eval '%_dbpath')
ver=$(. /usr/lib/os-release; echo "$VERSION_ID")
arch=$(uname -m)

mkdir -p "$R$dbpath"
cp -a "$(realpath "$dbpath")/." "$R$dbpath/"
rpm -qa --qf '%{NAME}.%{ARCH} %{NAME} %{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\n' | sort > "$work/before"

repo=()
while read -r copr; do
    [[ -n $copr ]] || continue
    id="copr-${copr//\//-}"
    base="https://download.copr.fedorainfracloud.org/results/$copr"
    repo+=("--repofrompath=$id,$base/fedora-$ver-$arch/"
           "--setopt=$id.gpgcheck=1" "--setopt=$id.gpgkey=$base/pubkey.gpg")
done < "$work/coprs"
while IFS='=' read -r id base key; do
    [[ -n $id && -n $base ]] || continue
    base=${base//\$basearch/$arch}
    repo+=("--repofrompath=$id,$base")
    if [[ -n $key ]]; then repo+=("--setopt=$id.gpgcheck=1" "--setopt=$id.gpgkey=$key"); else repo+=("--setopt=$id.gpgcheck=0"); fi
done < "$work/repos"
if [[ $(cat "$work/rpmfusion" 2>/dev/null || echo 0) == 1 ]]; then
    repo+=(
        "--repofrompath=rpmfusion-free,https://download1.rpmfusion.org/free/fedora/releases/$ver/Everything/$arch/os/"
        "--setopt=rpmfusion-free.gpgcheck=0"
        "--repofrompath=rpmfusion-free-updates,https://download1.rpmfusion.org/free/fedora/updates/$ver/$arch/"
        "--setopt=rpmfusion-free-updates.gpgcheck=0"
        "--repofrompath=rpmfusion-nonfree,https://download1.rpmfusion.org/nonfree/fedora/releases/$ver/Everything/$arch/os/"
        "--setopt=rpmfusion-nonfree.gpgcheck=0"
        "--repofrompath=rpmfusion-nonfree-updates,https://download1.rpmfusion.org/nonfree/fedora/updates/$ver/$arch/"
        "--setopt=rpmfusion-nonfree-updates.gpgcheck=0"
    )
fi

mapfile -t pkgs < "$work/request"
dnf5 -y --installroot="$R" --use-host-config --releasever="$ver" \
    --setopt=install_weak_deps=False --setopt=tsflags=noscripts \
    --setopt=cachedir="$work/cache" --setopt=keepcache=False \
    --setopt=persistdir="$work/persist" --setopt=logdir="$work/log" \
    "${repo[@]}" install "${pkgs[@]}"

rpm --root "$R" -qa --qf '%{NAME}.%{ARCH} %{NAME} %{EPOCHNUM}:%{VERSION}-%{RELEASE}.%{ARCH}\n' | sort > "$work/after"
join -v 2 -j 1 <(cut -d' ' -f1 "$work/before" | sort -u) <(cut -d' ' -f1 "$work/after" | sort -u) \
    | grep -v '^gpg-pubkey\.' > "$work/new-names" || true
awk 'NR == FNR { new[$1]; next } $1 in new { print $2, $3 }' "$work/new-names" "$work/after" > "$work/packages"
[[ -s $work/packages ]] || { echo "$MSG_ALREADY" >&2; exit 3; }
comm -13 "$work/before" "$work/after" | cut -d' ' -f1 | grep -v -x -F -f "$work/new-names" \
    | grep -v '^gpg-pubkey\.' > "$work/upgraded" || true
if [[ -s $work/upgraded && $allow_upgrade != 1 ]]; then
    echo "$MSG_UPGRADED" >&2
    sed 's/^/  /' "$work/upgraded" >&2
    echo "$MSG_UPGRADE_HINT" >&2
    exit 4
fi

# Folders the packages own under /var (libvirt's images and nvram, swtpm's
# state...): /var is the machine's, not the layer's, so they become tmpfiles
# lines and are created with the right owner and mode when the layer is on.
mapfile -t newpkgs < <(cut -d" " -f1 "$work/packages")
if (( ${#newpkgs[@]} )); then
    mkdir -p "$R/usr/lib/tmpfiles.d"
    rpm --root "$R" -q --qf '[%{FILEMODES:perms} %{FILEMODES:octal} %{FILEUSERNAME} %{FILEGROUPNAME} %{FILENAMES}\n]' "${newpkgs[@]}" 2>/dev/null \
        | awk '$1 ~ /^d/ && $5 ~ /^\/var\/(lib|log|cache|spool)\// { printf "d %s %s %s %s -\n", $5, substr($2, length($2) - 3), $3, $4 }' \
        | sort -u > "$R/usr/lib/tmpfiles.d/tamagi-layer-$name-var.conf"
    [[ -s $R/usr/lib/tmpfiles.d/tamagi-layer-$name-var.conf ]] || rm -f "$R/usr/lib/tmpfiles.d/tamagi-layer-$name-var.conf"
fi

if [[ -d $R/etc ]]; then
    ( cd "$R" && find etc -type f -o -type l | sort ) > "$work/etc.list"
    tar -C "$R" -cf "$work/etc.tar" etc
fi
rm -rf "$R$dbpath" "$R/usr/lib/sysimage" "$R/var" "$R/etc" "$R/run" "$R/tmp" "$R/dev" "$R/proc" "$R/sys"
# /opt is a link to /var/opt on this system (state, not image): packages that
# install there (Chrome, Vivaldi) go to /usr/lib/opt, and a tmpfiles line puts
# the /opt/<name> link back — the same thing rpm-ostree does.
if [[ -d $R/opt ]] && compgen -G "$R/opt/*" >/dev/null; then
    mkdir -p "$R/usr/lib/opt" "$R/usr/lib/tmpfiles.d"
    for d in "$R"/opt/*; do
        n=$(basename "$d")
        mv "$d" "$R/usr/lib/opt/$n"
        printf 'L+ /opt/%s - - - - /usr/lib/opt/%s\n' "$n" "$n" >> "$R/usr/lib/tmpfiles.d/tamagi-layer-$name.conf"
    done
fi
find "$R" -mindepth 1 -maxdepth 1 ! -name usr -exec rm -rf {} +
[[ -d $R/usr ]] || { echo "$MSG_NOUSR" >&2; exit 5; }

mkdir -p "$R/usr/lib/extension-release.d" "$R/usr/share/tamagi/layers"
printf 'ID=%s\nVERSION_ID=%s\n' "$(. /usr/lib/os-release; echo "$ID")" "$ver" \
    > "$R/usr/lib/extension-release.d/extension-release.$name"
cp "$work/packages" "$R/usr/share/tamagi/layers/$name.packages"
# Files of the image that only make sense with this layer on (e.g. the login
# session of the gaming layer).
overlay=$(cat "$work/overlay" 2>/dev/null || true)
if [[ -n $overlay && -d $overlay/usr ]]; then cp -a "$overlay/usr/." "$R/usr/"; fi

ctx=/etc/selinux/targeted/contexts/files/file_contexts
args=(-zlz4hc -T0 --quiet)
[[ -r $ctx ]] && args+=(--file-contexts="$ctx")
mkfs.erofs "${args[@]}" "$work/layer.raw" "$R"
chmod 0644 "$work/layer.raw"
