#!/usr/bin/bash
# Kernel e módulos de fora do kernel, do Universal Blue (ghcr.io/ublue-os/akmods,
# montado em /akmods pelo Containerfile). O sabor vem de KERNEL_FLAVOR:
#
#   ogc   (padrão) o kernel Open Gaming Collective
#         (github.com/OpenGamingCollective/linux): patches de agendamento,
#         HDR, controles e portáteis.
#   main  o kernel da base, sem troca. Assinado pelo Fedora e pelo Universal
#         Blue: inicia com o Secure Boot ligado em qualquer máquina.
#   ark   o kernel-ark (gitlab.com/cki-project/kernel-ark): a árvore de onde
#         sai o kernel do Fedora Rawhide, que é o que se instala aqui — o mais
#         novo do Linux com a configuração do Fedora, muitas vezes uma -rc.
#         Pacotes com GPG do Fedora e vmlinuz assinado para o Secure Boot pelo
#         Fedora (inicia em qualquer máquina, como o main). O Universal Blue
#         não compila módulos para ele: sem xone, xpadneo e v4l2loopback.
#         O build recusa um kernel com depuração pesada (lento) ou sem a
#         assinatura de Secure Boot do Fedora.
#
# SECURE BOOT. O "ogc" é assinado SÓ pelo Universal Blue: com o Secure Boot ligado ele só
# inicia depois que a chave do Universal Blue for registrada na máquina, uma
# vez:  ujust enroll-secure-boot-key   (senha: universalblue) e reiniciar.
# Conferir antes de trocar de imagem:
#   mokutil --test-key /etc/pki/akmods/certs/akmods-ublue.der
# Com o Secure Boot desligado não há nada a fazer.
#
# Módulos instalados (compilados e assinados para o kernel escolhido):
#   xone, xpadneo    controles Xbox (cabo, adaptador sem fio e Bluetooth)
#   v4l2loopback     câmera virtual (OBS, compartilhar a tela como webcam)
# Existem na origem e ficam de fora por padrão: openrazer, wl (Wi-Fi Broadcom).
KMODS=(xone xpadneo v4l2loopback)
FLAVOR=${KERNEL_FLAVOR:-ogc}

base_kver=$(ls /usr/lib/modules)
[[ $(wc -w <<<"$base_kver") -eq 1 ]] || { echo "esperava um kernel na base, achei: $base_kver" >&2; exit 1; }
ARK_REPO=/etc/yum.repos.d/tamagi-ark.repo
if [[ $FLAVOR == ark ]]; then
    # Rawhide, só para o kernel e só durante o build.
    cat > "$ARK_REPO" <<'REPO'
[tamagi-ark]
name=Fedora Rawhide (kernel-ark)
baseurl=https://dl.fedoraproject.org/pub/fedora/linux/development/rawhide/Everything/$basearch/os/
enabled=0
gpgcheck=1
gpgkey=file:///usr/share/pki/rpm-gpg/RPM-GPG-KEY-fedora-rawhide-primary
REPO
    kver=$(dnf5 -q repoquery --disablerepo='*' --enablerepo=tamagi-ark --latest-limit 1 \
        --qf '%{version}-%{release}.%{arch}' kernel-core | tail -1)
    [[ $kver =~ ^[0-9]+\.[0-9]+ ]] || { echo "kernel-ark: não achei o kernel no Rawhide" >&2; exit 1; }
    new=()
    for p in kernel kernel-core kernel-modules-core kernel-modules kernel-modules-extra; do new+=("$p-$kver"); done
    install_new=(dnf5 -y install --setopt=install_weak_deps=False --enablerepo=tamagi-ark "${new[@]}")
else
    core=$(compgen -G '/akmods/kernel-rpms/kernel-core-*.rpm' | head -1)
    [[ -n $core ]] || { echo "akmods ($FLAVOR): sem kernel-rpms" >&2; exit 1; }
    kver=$(rpm -qp --qf '%{VERSION}-%{RELEASE}.%{ARCH}' "$core")
    mapfile -t new < <(compgen -G '/akmods/kernel-rpms/kernel-*.rpm' | grep -vE 'kernel-(devel|devel-matched|uki-virt)-[0-9]')
    install_new=(dnf5 -y install --setopt=install_weak_deps=False "${new[@]}")
fi

if [[ $kver != "$base_kver" ]]; then
    echo "kernel: trocando $base_kver por $kver ($FLAVOR)"
    # Uma imagem bootc tem exatamente um kernel: sai o da base, entra o do sabor.
    mapfile -t old < <(rpm -qa --qf '%{NAME}\n' | grep -E '^kernel(-core|-modules|-modules-core|-modules-extra|-uki-virt|-tools|-tools-libs)?$' | sort -u)
    rpm -e --nodeps "${old[@]}"
    rm -rf "/usr/lib/modules/$base_kver"
    # Os ganchos de instalação de kernel (rpm-ostree e dracut) não funcionam
    # dentro do build do contêiner: ficam inertes durante a troca, e o
    # initramfs é gerado logo abaixo, à mão.
    hooks=(/usr/lib/kernel/install.d/05-rpmostree.install /usr/lib/kernel/install.d/50-dracut.install)
    for h in "${hooks[@]}"; do
        [[ -f $h ]] || continue
        mv "$h" "$h.tamagi-bak"
        printf '#!/bin/sh\nexit 0\n' > "$h"; chmod +x "$h"
    done
    "${install_new[@]}"
    for h in "${hooks[@]}"; do [[ -f $h.tamagi-bak ]] && mv -f "$h.tamagi-bak" "$h"; done
    rm -rf /boot/* 2>/dev/null || true
    [[ $(ls /usr/lib/modules) == "$kver" ]] || { echo "depois da troca esperava só $kver, achei: $(ls /usr/lib/modules | tr '\n' ' ')" >&2; exit 1; }
    [[ -f /usr/lib/modules/$kver/vmlinuz ]] || { echo "sem vmlinuz em /usr/lib/modules/$kver" >&2; exit 1; }
    depmod -a "$kver"
    # initramfs genérico (a imagem serve para qualquer máquina), como o da base.
    dracut --force --no-hostonly --reproducible --zstd --add ostree \
        --kver "$kver" "/usr/lib/modules/$kver/initramfs.img"
    chmod 0600 "/usr/lib/modules/$kver/initramfs.img"
else
    echo "kernel: o da base ($kver, sabor $FLAVOR)"
fi

if [[ $FLAVOR == ark ]]; then
    rm -f "$ARK_REPO"
    conf=/usr/lib/modules/$kver/config
    # Rawhide às vezes publica kernels com depuração pesada (lockdep, KASAN):
    # lentos demais para uso diário. Nesse caso o build para.
    for opt in CONFIG_PROVE_LOCKING CONFIG_KASAN CONFIG_DEBUG_KMEMLEAK; do
        grep -qx "$opt=y" "$conf" && { echo "kernel-ark $kver veio com $opt (depuração): tente de novo mais tarde" >&2; exit 1; }
    done
    grep -aq 'Fedora Secure Boot Signer' "/usr/lib/modules/$kver/vmlinuz" \
        || { echo "kernel-ark $kver sem a assinatura de Secure Boot do Fedora" >&2; exit 1; }
    KMODS=()
    echo "kernel-ark: sem módulos do Universal Blue (xone, xpadneo, v4l2loopback não existem para ele)"
fi

rpms=()
for k in "${KMODS[@]}"; do
    # Só serve o módulo compilado exatamente para o kernel desta imagem.
    f=$(compgen -G "/akmods/rpms/kmods/kmod-$k-$kver-*.rpm" | head -1)
    [[ -n $f ]] || { echo "akmods: não há kmod-$k para o kernel $kver" >&2; ls /akmods/rpms/kmods >&2; exit 1; }
    rpms+=("$f")
    # Parte comum do módulo (regras udev, firmware, modprobe.d), quando existe.
    while read -r c; do [[ -n $c ]] && rpms+=("$c"); done < <(compgen -G "/akmods/rpms/common/$k-*.rpm" || true)
done
mapfile -t rpms < <(printf '%s\n' "${rpms[@]}" | sort -u)
if [[ ${#KMODS[@]} -gt 0 ]]; then

# Dependências (firmware dos controles etc.) vêm do COPR do próprio akmods, só no build.
repo=/etc/yum.repos.d/_copr_ublue-os-akmods.repo
[[ -f $repo ]] && sed -i 's/^enabled=0/enabled=1/' "$repo"
dnf5 -y install --setopt=install_weak_deps=False "${rpms[@]}"
[[ -f $repo ]] && sed -i 's/^enabled=1/enabled=0/' "$repo"

depmod -a "$kver"
for k in "${KMODS[@]}"; do
    rpm -q "kmod-$k" >/dev/null || { echo "kmod-$k não ficou instalado" >&2; exit 1; }
done
fi
# O que o Tamagi espera de qualquer kernel (os mesmos pontos de tests/image.sh).
conf=/usr/lib/modules/$kver/config
for opt in CONFIG_SCHED_CLASS_EXT=y CONFIG_SECURITY_SELINUX=y CONFIG_RUST=y CONFIG_DRM_ACCEL=y; do
    grep -qx "$opt" "$conf" || { echo "kernel $kver sem $opt" >&2; exit 1; }
done
# Binder (Android em contêiner): em C no kernel OGC, em Rust no da base.
grep -qxE 'CONFIG_ANDROID_BINDER_IPC(_RUST)?=y' "$conf" || { echo "kernel $kver sem binder" >&2; exit 1; }
mkdir -p /usr/share/tamagi
echo "$FLAVOR" > /usr/share/tamagi/kernel-flavor
echo "kernel $kver ($FLAVOR) com os módulos: ${KMODS[*]}"
