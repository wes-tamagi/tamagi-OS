#!/usr/bin/bash
# Roda dentro de um contêiner da imagem live (chamado por build-iso.sh), com
# a própria imagem live montada em /live e a pasta de saída em /out.
set -euo pipefail

: "${LABEL:?}" "${OUT_NAME:?}"
TREE=/out/iso-work/tree
EFI_IMG=/out/iso-work/efiboot.img
kver=$(ls /live/usr/lib/modules | head -1)

mkdir -p "$TREE/LiveOS" "$TREE/boot"
cp "/live/usr/lib/modules/$kver/vmlinuz" "$TREE/boot/vmlinuz"
cp "/live/usr/lib/modules/$kver/initramfs-live.img" "$TREE/boot/initrd.img"

echo "squashfs do ambiente live (kernel $kver)…"
# /live é a raiz da imagem: sem /proc, /sys, /dev montados, nada a excluir.
if [[ ${SQUASH_COMP:-zstd} == xz ]]; then
    comp=(-comp xz -Xbcj x86 -Xdict-size 1M)
else
    comp=(-comp zstd -Xcompression-level 19)
fi
# Fica fora do ambiente live o que não serve para instalar nem para
# experimentar o desktop (o sistema INSTALADO vem da imagem embutida e
# continua completo): os dois initramfs (o live já está em /boot da ISO), a
# os agentes de IA de terceiros, documentação e manuais.
exclude=(
    'usr/lib/modules/*/initramfs.img' 'usr/lib/modules/*/initramfs-live.img'
    usr/bin/claude
    usr/share/doc usr/share/man
)
mksquashfs /live "$TREE/LiveOS/squashfs.img" -noappend -no-progress -b 1M "${comp[@]}" \
    -wildcards -e "${exclude[@]}"
ls -lh "$TREE/LiveOS/squashfs.img"

# ── GRUB (UEFI). O shim e o GRUB são os binários assinados do Fedora que
#    já vêm na imagem; o grub.cfg fica na partição EFI da ISO. ──
shim=$(find /live/usr/lib/efi/shim -name BOOTX64.EFI | head -1)
mm=$(find /live/usr/lib/efi/shim -name mmx64.efi | head -1)
grub=$(find /live/usr/lib/efi/grub2 -name grubx64.efi | head -1)
[[ -f $shim && -f $grub ]] || { echo "shim/GRUB não encontrados em /live/usr/lib/efi" >&2; exit 1; }

BASE="root=live:CDLABEL=$LABEL rd.live.image rd.live.overlay.overlayfs=1 rd.luks=0 rd.lvm=0 rd.md=0 rd.multipath=0 enforcing=0 rootdelay=5"
cat > /tmp/grub.cfg <<CFG
set default=0
set timeout=5
search --no-floppy --set=root --label $LABEL

menuentry "Tamagi — Install or Try" {
    linux /boot/vmlinuz $BASE quiet
    initrd /boot/initrd.img
}
menuentry "Tamagi — Safe Graphics Mode (nomodeset)" {
    linux /boot/vmlinuz $BASE nomodeset
    initrd /boot/initrd.img
}
menuentry "Tamagi — Boot Diagnostics (verbose)" {
    linux /boot/vmlinuz $BASE rd.live.debug=1
    initrd /boot/initrd.img
}
CFG

# Imagem FAT da partição EFI, montada sem root com o mtools.
truncate -s 16M "$EFI_IMG"
mkfs.vfat -n TAMAGI_EFI "$EFI_IMG" >/dev/null
mmd -i "$EFI_IMG" ::/EFI ::/EFI/BOOT ::/EFI/fedora
mcopy -i "$EFI_IMG" "$shim" ::/EFI/BOOT/BOOTX64.EFI
mcopy -i "$EFI_IMG" "$grub" ::/EFI/BOOT/grubx64.efi
[[ -f $mm ]] && mcopy -i "$EFI_IMG" "$mm" ::/EFI/BOOT/mmx64.efi
# Chave do Universal Blue, que assina o kernel do Tamagi (o do Bazzite): com o
# Secure Boot ligado, o shim recusa o kernel e abre o MokManager; ali,
# "Enroll key from disk" → TAMAGI_EFI → universalblue.der registra a chave, uma
# vez, e a ISO inicia. (Ou desligue o Secure Boot.)
key=/live/etc/pki/akmods/certs/akmods-ublue.der
if [[ -f $key ]]; then
    mcopy -i "$EFI_IMG" "$key" ::/universalblue.der
    cp "$key" "$TREE/universalblue.der"
fi
# O GRUB do Fedora procura a configuração em EFI/fedora; EFI/BOOT é a reserva.
mcopy -i "$EFI_IMG" /tmp/grub.cfg ::/EFI/fedora/grub.cfg
mcopy -i "$EFI_IMG" /tmp/grub.cfg ::/EFI/BOOT/grub.cfg
mkdir -p "$TREE/EFI/BOOT"
cp /tmp/grub.cfg "$TREE/EFI/BOOT/grub.cfg"

echo "montando a ISO…"
# ISO 9660 nível 3 (arquivos > 4 GB) com a partição EFI anexada: inicia por
# pendrive (GPT/MBR híbrido) e por CD/DVD ou VM (El Torito), sempre em UEFI.
# -partition_offset 16 grava um segundo conjunto de superblocos ISO 9660 na
# partição 1 (setor 64 = 32 KiB): assim tanto o disco inteiro (/dev/sdX) quanto
# a partição (/dev/sdX1) têm filesystem iso9660 válido com LABEL=TAMAGI_LIVE,
# evitando que o dracut-initqueue trave quando o udev escaneia as partições.
xorriso -as mkisofs \
    -o "/out/$OUT_NAME" \
    -V "$LABEL" -iso-level 3 -R -J -joliet-long \
    -partition_offset 16 \
    -append_partition 2 C12A7328-F81F-11D2-BA4B-00A0C93EC93B "$EFI_IMG" \
    -appended_part_as_gpt \
    -e --interval:appended_partition_2:all:: -no-emul-boot \
    "$TREE" 2>&1 | tail -4
