#!/usr/bin/bash
# Testa a ISO de instalação numa VM (QEMU/KVM), sem root.
#
#   tests/iso.sh <iso>                 instala sozinho no disco output/vm-test.qcow2,
#                                      reinicia pelo disco e confere o sistema instalado
#   tests/iso.sh <iso> --luks          o mesmo, com o disco criptografado
#   tests/iso.sh <iso> --en            o mesmo, instalando em inglês (en_US)
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
    auto|--luks|--en|--verify|--verify-luks|--rollback) ;;
    *) echo "modo desconhecido: $MODE" >&2; exit 2 ;;
esac

if [[ $MODE != --verify* && $MODE != --rollback ]]; then
echo "── 1/2  Instalação sem perguntas (${MODE#--})"
rm -f -- "$DISK" "$VARS" "$OUT/vm-install.log" "$OUT/vm-serial.sock" "$OUT/vm-qmp.sock"
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
[[ $MODE == --luks ]] && append+=" tamagi.autoinstall.luks=$LUKS_PASS"
[[ $MODE == --en ]] && append+=" tamagi.autoinstall.lang=en"
start=$SECONDS
timeout 3600 qemu-system-x86_64 -enable-kvm -machine q35 -cpu host -smp "$CPUS" -m "$RAM" \
    -drive if=pflash,format=raw,readonly=on,file="$CODE" \
    -drive if=pflash,format=raw,file="$VARS" \
    -device virtio-net-pci,netdev=n0 -netdev user,id=n0 \
    "${disk_args[@]}" -cdrom "$ISO" \
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

echo "── 2/2  Primeiro boot do sistema instalado"
qemu "${disk_args[@]}" -device virtio-vga -display none \
    -serial "unix:$OUT/vm-serial.sock,server=on,wait=off" \
    -qmp "unix:$OUT/vm-qmp.sock,server=on,wait=off" &
vm=$!
trap 'kill $vm 2>/dev/null || true' EXIT
sleep 2
if [[ $MODE == --rollback ]]; then
    echo "   (rollback automático: simula uma atualização ruim no disco já instalado)"
    python3 "$HERE/vm_rollback.py" "$OUT/vm-serial.sock" "$OUT/vm-qmp.sock" tamagi tamagi
    exit
fi
luks=()
[[ $MODE == *luks ]] && luks=(--luks "$LUKS_PASS")
[[ $MODE == --en ]] && luks+=(--lang en)
python3 "$HERE/vm_check.py" --serial "$OUT/vm-serial.sock" --qmp "$OUT/vm-qmp.sock" \
    --user tamagi --password tamagi --screenshot "$OUT/vm-greeter.png" "${luks[@]}"
