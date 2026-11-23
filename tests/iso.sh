#!/usr/bin/bash
# Testa a ISO de instalação numa VM (QEMU/KVM), sem root.
#
#   tests/iso.sh <iso>                 instala sozinho no disco output/vm-test.qcow2,
#                                      reinicia pelo disco e confere o sistema instalado
#   tests/iso.sh <iso> --luks          o mesmo, com o disco criptografado
#   tests/iso.sh <iso> --pt            o mesmo, instalando em português (pt_BR); também --es e --fr
#                                      (sem opção, instala em inglês, o idioma padrão)
#   tests/iso.sh <iso> --tpm           disco criptografado com a chave no TPM (um TPM virtual,
#                                      swtpm): o sistema tem de iniciar sem pedir a senha
#   tests/iso.sh <iso> --upgrade       atualização automática de ponta a ponta: instala apontando
#                                      para um registry local com uma imagem nova, dispara o serviço
#                                      do timer diário, reinicia na imagem nova e faz rollback
#   tests/iso.sh <iso> --rollback      no disco já instalado, simula uma atualização ruim e
#                                      confere que o sistema volta sozinho para a imagem boa
#   tests/iso.sh <iso> --verify        só confere o disco já instalado (--verify-luks se criptografado)
#   tests/iso.sh <iso> --interactive   abre a ISO numa janela, para usar o instalador
#   tests/iso.sh <iso> --boot-disk     abre numa janela o disco já instalado
#
# A instalação automática usa `tamagi.autoinstall=` na linha do kernel (veja
# live/tik/modules/pre/10-tamagi); o usuário criado é tamagi, senha tamagi.
set -euo pipefail

ISO=${1:?caminho da ISO}
MODE=${2:-auto}
OUT=$(cd "$(dirname "$ISO")" && pwd)
HERE=$(cd "$(dirname "$0")" && pwd)
DISK="$OUT/vm-test.qcow2"
VARS="$OUT/vm-test-vars.fd"
LUKS_PASS="disco-de-teste-123"
RAM=${TAMAGI_VM_RAM:-8G}
CPUS=${TAMAGI_VM_CPUS:-8}

[[ -f $ISO ]] || { echo "ISO não encontrada: $ISO (rode: just build-iso)" >&2; exit 1; }
[[ -w /dev/kvm ]] || { echo "sem acesso a /dev/kvm" >&2; exit 1; }

CODE="" VARS_TEMPLATE=""
for pair in "/usr/share/edk2/x64/OVMF_CODE.4m.fd:/usr/share/edk2/x64/OVMF_VARS.4m.fd" \
            "/usr/share/edk2/ovmf/OVMF_CODE.fd:/usr/share/edk2/ovmf/OVMF_VARS.fd" \
            "/usr/share/OVMF/OVMF_CODE_4M.fd:/usr/share/OVMF/OVMF_VARS_4M.fd" \
            "/usr/share/OVMF/OVMF_CODE.fd:/usr/share/OVMF/OVMF_VARS.fd"; do
    if [[ -f ${pair%%:*} && -f ${pair##*:} ]]; then CODE=${pair%%:*}; VARS_TEMPLATE=${pair##*:}; break; fi
done
[[ -n $CODE ]] || { echo "firmware UEFI (edk2-ovmf) não encontrado" >&2; exit 1; }

qemu() {
    qemu-system-x86_64 -enable-kvm -machine q35 -cpu host -smp "$CPUS" -m "$RAM" \
        -drive if=pflash,format=raw,readonly=on,file="$CODE" \
        -drive if=pflash,format=raw,file="$VARS" \
        -device virtio-net-pci,netdev=n0 -netdev user,id=n0 \
        -device qemu-xhci -device usb-tablet "$@"
}
disk_args=(-drive "file=$DISK,if=virtio,format=qcow2,discard=unmap")

# TPM virtual (swtpm) para o modo --tpm. O swtpm vem da imagem do Tamagi, num
# contêiner; ele encerra quando o QEMU desconecta, então sobe um por VM. O
# estado (as chaves seladas) fica em output/vm-tpm entre a instalação e o boot.
TPM_DIR="$OUT/vm-tpm"
tpm_args=()
start_tpm() {
    mkdir -p "$TPM_DIR"
    rm -f "$TPM_DIR/sock"
    podman rm -f tamagi-swtpm >/dev/null 2>&1 || true
    podman run -d --rm --name tamagi-swtpm -v "$TPM_DIR":/tpm "${TAMAGI_IMAGE:-localhost/tamagi}:${TAMAGI_TAG:-latest}" \
        swtpm socket --tpm2 --tpmstate dir=/tpm --ctrl type=unixio,path=/tpm/sock >/dev/null
    for _ in $(seq 50); do [[ -S $TPM_DIR/sock ]] && break; sleep 0.2; done
    [[ -S $TPM_DIR/sock ]] || { echo "o swtpm não subiu" >&2; exit 1; }
    # shellcheck disable=SC2054  # as vírgulas são dos argumentos do QEMU
    tpm_args=(-chardev "socket,id=chrtpm,path=$TPM_DIR/sock" -tpmdev emulator,id=tpm0,chardev=chrtpm -device tpm-tis,tpmdev=tpm0)
}
# shellcheck disable=SC2054  # a vírgula é do argumento do QEMU
window=(-device virtio-vga-gl -display gtk,gl=on)

case "$MODE" in
    --interactive)
        [[ -f $DISK ]] || qemu-img create -q -f qcow2 "$DISK" 64G
        [[ -f $VARS ]] || cp "$VARS_TEMPLATE" "$VARS"
        qemu "${disk_args[@]}" -cdrom "$ISO" -boot d "${window[@]}"
        exit ;;
    --boot-disk)
        [[ -f $DISK ]] || { echo "sem disco instalado: rode antes just test-iso" >&2; exit 1; }
        qemu "${disk_args[@]}" "${window[@]}"
        exit ;;
    auto|--luks|--tpm|--upgrade|--pt|--en|--es|--fr|--verify|--verify-luks|--verify-tpm|--rollback) ;;
    *) echo "modo desconhecido: $MODE" >&2; exit 2 ;;
esac

if [[ $MODE != --verify* && $MODE != --rollback ]]; then
echo "── 1/2  Instalação sem perguntas (${MODE#--})"
rm -f -- "$DISK" "$VARS" "$OUT/vm-install.log" "$OUT/vm-serial.sock" "$OUT/vm-qmp.sock"
if [[ $MODE == --tpm ]]; then podman rm -f tamagi-swtpm >/dev/null 2>&1 || true; rm -rf "$TPM_DIR"; start_tpm; fi
qemu-img create -q -f qcow2 "$DISK" 64G
cp "$VARS_TEMPLATE" "$VARS"
# Kernel e initramfs saem da própria ISO, para poder passar a linha do kernel.
boot=$(mktemp -d)
xorriso -osirrox on -indev "$ISO" -extract /boot/vmlinuz "$boot/vmlinuz" \
    -extract /boot/initrd.img "$boot/initrd.img" >/dev/null 2>&1
chmod u+w "$boot"/*
append="root=live:CDLABEL=TAMAGI_LIVE rd.live.image rd.live.overlay.overlayfs=1 enforcing=0"
append+=" console=ttyS0 systemd.unit=multi-user.target"
append+=" tamagi.autoinstall=/dev/vda tamagi.autoinstall.kargs=console=tty0,console=ttyS0"
[[ $MODE == --luks || $MODE == --tpm ]] && append+=" tamagi.autoinstall.luks=$LUKS_PASS"
[[ $MODE == --tpm ]] && append+=" tamagi.autoinstall.tpm=1"
if [[ $MODE == --upgrade ]]; then
    # Registry local (sem TLS) com a "v2": a imagem do Tamagi + um arquivo. Da VM,
    # o computador que a roda é 10.0.2.2.
    IMG="${TAMAGI_IMAGE:-localhost/tamagi}:${TAMAGI_TAG:-latest}"
    echo "   preparando o registry local com a imagem nova (v2)…"
    podman rm -f tamagi-registry >/dev/null 2>&1 || true
    podman run -d --rm --name tamagi-registry -p 127.0.0.1:5000:5000 docker.io/library/registry:2 >/dev/null
    ctx=$(mktemp -d)
    printf 'FROM %s\nRUN echo v2 > /usr/share/tamagi/teste-atualizacao\n' "$IMG" > "$ctx/Containerfile"
    podman build -q -t localhost/tamagi-teste-v2 "$ctx" >/dev/null
    # E a imagem RUIM, para a volta automática: uma verificação obrigatória que falha.
    printf 'FROM localhost/tamagi-teste-v2\nRUN printf "#!/bin/sh\\necho FALHOU: imagem ruim de teste\\nexit 1\\n" > /usr/lib/tamagi/boot-check/required.d/99-teste.sh && chmod +x /usr/lib/tamagi/boot-check/required.d/99-teste.sh\n' > "$ctx/Containerfile"
    podman build -q -t localhost/tamagi-teste-bad "$ctx" >/dev/null
    rm -rf "$ctx"
    podman push -q --tls-verify=false --compression-format zstd --compression-level 3 \
        --digestfile "$OUT/vm-upgrade.digest" localhost/tamagi-teste-v2 localhost:5000/tamagi:latest
    echo "   v2 publicada: $(cat "$OUT/vm-upgrade.digest")"
    append+=" tamagi.autoinstall.imgref=10.0.2.2:5000/tamagi:latest"
fi
[[ $MODE == --en || $MODE == --pt || $MODE == --es || $MODE == --fr ]] && append+=" tamagi.autoinstall.lang=${MODE#--}"
start=$SECONDS
timeout 3600 qemu-system-x86_64 -enable-kvm -machine q35 -cpu host -smp "$CPUS" -m "$RAM" \
    -drive if=pflash,format=raw,readonly=on,file="$CODE" \
    -drive if=pflash,format=raw,file="$VARS" \
    -device virtio-net-pci,netdev=n0 -netdev user,id=n0 \
    "${disk_args[@]}" "${tpm_args[@]}" -cdrom "$ISO" \
    -kernel "$boot/vmlinuz" -initrd "$boot/initrd.img" -append "$append" \
    -display none -serial "file:$OUT/vm-install.log" -no-reboot || true
rm -f -- "$boot/vmlinuz" "$boot/initrd.img"; rmdir "$boot"
echo "   levou $(( (SECONDS - start) / 60 )) min; registro em $OUT/vm-install.log"
if ! grep -aq 'TAMAGI-AUTOINSTALL: OK' "$OUT/vm-install.log"; then
    echo "   A INSTALAÇÃO FALHOU. Fim do registro:"
    tail -n 40 "$OUT/vm-install.log" | sed 's/\x1b\[[0-9;]*[a-zA-Z]//g' | cut -c1-220
    exit 1
fi
echo "   instalação concluída ($(qemu-img info --output=json "$DISK" | python3 -c 'import json,sys; print(round(json.load(sys.stdin)["actual-size"]/2**30, 1))') GiB usados no disco)"
fi
[[ -f $DISK ]] || { echo "sem disco instalado em $DISK" >&2; exit 1; }
rm -f -- "$OUT/vm-serial.sock" "$OUT/vm-qmp.sock"

# Provisionamento no primeiro boot (tamagi-combustion.service): um segundo
# disco com rótulo "combustion" e um script, como um pendrive do Aeon. Só vale
# no primeiro boot depois de instalar; os modos --verify conferem o que ficou.
combustion_args=()
if [[ $MODE != --verify* && $MODE != --rollback && $MODE != --upgrade ]]; then
    stick=$(mktemp -d)
    mkdir "$stick/combustion"
    echo "arquivo ao lado do script" > "$stick/combustion/dado.txt"
    cat > "$stick/combustion/script" <<'SCRIPT'
#!/bin/bash
# combustion: network
set -e
{ echo "uid=$(id -u)"; echo "dado=$(cat dado.txt)"; echo "usr=$(touch /usr/teste 2>/dev/null && echo gravavel || echo somente-leitura)"; } > /etc/tamagi-combustion-teste
SCRIPT
    rm -f "$OUT/vm-combustion.img"
    truncate -s 8M "$OUT/vm-combustion.img"
    mkfs.vfat -n combustion "$OUT/vm-combustion.img" >/dev/null
    mcopy -i "$OUT/vm-combustion.img" -s "$stick/combustion" ::/combustion
    rm -rf "$stick"
    combustion_args=(-drive "file=$OUT/vm-combustion.img,if=virtio,format=raw,readonly=on")
fi

echo "── 2/2  Primeiro boot do sistema instalado"
[[ $MODE == *tpm ]] && start_tpm
qemu "${disk_args[@]}" "${combustion_args[@]}" "${tpm_args[@]}" -device virtio-vga -display none \
    -serial "unix:$OUT/vm-serial.sock,server=on,wait=off" \
    -qmp "unix:$OUT/vm-qmp.sock,server=on,wait=off" &
vm=$!
trap 'kill $vm 2>/dev/null || true; podman rm -f tamagi-swtpm tamagi-registry >/dev/null 2>&1 || true; podman rmi localhost/tamagi-teste-v2 localhost/tamagi-teste-bad >/dev/null 2>&1 || true' EXIT
sleep 2
if [[ $MODE == --rollback ]]; then
    echo "a volta automática agora é provada junto com a atualização: tests/iso.sh <iso> --upgrade" >&2
    exit 2
fi
if [[ $MODE == --upgrade ]]; then
    echo "   (atualização automática: registry local -> imagem preparada -> reinício -> rollback)"
    python3 "$HERE/vm_upgrade.py" "$OUT/vm-serial.sock" "$OUT/vm-qmp.sock" tamagi tamagi "$(cat "$OUT/vm-upgrade.digest")"
    exit
fi
luks=()
[[ $MODE == *luks ]] && luks=(--luks "$LUKS_PASS")
[[ $MODE == *tpm ]] && luks=(--tpm)
[[ $MODE == --en || $MODE == --pt || $MODE == --es || $MODE == --fr ]] && luks+=(--lang "${MODE#--}")
# Sem opção de idioma, a ISO instala no idioma padrão da imagem (en; pt na variante :ptbr).
[[ $MODE == --en || $MODE == --pt || $MODE == --es || $MODE == --fr ]] || luks+=(--lang "${TAMAGI_DEFAULT_LANG:-en}")
python3 "$HERE/vm_check.py" --serial "$OUT/vm-serial.sock" --qmp "$OUT/vm-qmp.sock" \
    --user tamagi --password tamagi --screenshot "$OUT/vm-greeter.png" "${luks[@]}"
