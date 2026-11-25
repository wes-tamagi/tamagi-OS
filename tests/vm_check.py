#!/usr/bin/python3
"""Confere o sistema instalado dentro da VM: entra pelo console serial, roda
comandos e compara as respostas; pelo QMP, tira uma captura da tela de login.
Só biblioteca padrão."""
import argparse
import json
import re
import socket
import sys
import time

ANSI = re.compile(rb"\x1b\[[0-9;?]*[a-zA-Z]|\x1b\][^\x07]*\x07|\r")


class Serial:
    def __init__(self, path):
        self.sock = socket.socket(socket.AF_UNIX)
        for _ in range(50):
            try:
                self.sock.connect(path)
                break
            except OSError:
                time.sleep(0.2)
        else:
            raise SystemExit(f"não consegui abrir o console serial {path}")
        self.sock.settimeout(1.0)
        self.buf = b""
        self.log = open(path + ".log", "wb")

    def read_until(self, patterns, timeout):
        """Lê até casar um dos padrões (regex em bytes). Devolve (índice, texto)."""
        end = time.time() + timeout
        compiled = [re.compile(p) for p in patterns]
        while time.time() < end:
            clean = ANSI.sub(b"", self.buf)
            for i, rx in enumerate(compiled):
                m = rx.search(clean)
                if m:
                    text = clean[: m.end()]
                    self.buf = b""
                    return i, text.decode("utf-8", "replace")
            try:
                chunk = self.sock.recv(65536)
            except (socket.timeout, TimeoutError):
                continue
            if not chunk:
                break
            self.log.write(chunk)
            self.log.flush()
            self.buf += chunk
        tail = ANSI.sub(b"", self.buf)[-1500:].decode("utf-8", "replace")
        raise SystemExit(f"tempo esgotado esperando {patterns}. Fim do console:\n{tail}")

    def send(self, text):
        self.sock.sendall(text.encode())


def qmp(path, *commands):
    s = socket.socket(socket.AF_UNIX)
    s.connect(path)
    f = s.makefile("rw")
    f.readline()
    out = []
    for cmd in ({"execute": "qmp_capabilities"},) + commands:
        f.write(json.dumps(cmd) + "\n")
        f.flush()
        while True:
            reply = json.loads(f.readline())
            if "return" in reply or "error" in reply:
                out.append(reply)
                break
    s.close()
    return out[1:]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--serial", required=True)
    ap.add_argument("--qmp", required=True)
    ap.add_argument("--user", required=True)
    ap.add_argument("--password", required=True)
    ap.add_argument("--luks")
    ap.add_argument("--tpm", action="store_true", help="disco criptografado que abre sozinho pelo TPM")
    ap.add_argument("--tpm-pin", help="disco criptografado com TPM + PIN: o boot pede este PIN")
    ap.add_argument("--lang", default="en")
    ap.add_argument("--screenshot")
    ap.add_argument("--preserved-file", help="arquivo esperado vindo de /var/home preservada")
    args = ap.parse_args()

    con = Serial(args.serial)
    start = time.time()
    if args.luks:
        con.read_until([rb"[Pp]lease enter passphrase[^\n]*:"], 240)
        print(f"   pedido da senha do disco apareceu em {time.time() - start:.0f}s")
        time.sleep(1)
        con.send(args.luks + "\n")
    if args.tpm_pin:
        # O boot pede o PIN do TPM, não a senha longa do disco.
        i, _ = con.read_until([rb"PIN[^\n]*: ?", rb"[Pp]lease enter passphrase[^\n]*:"], 240)
        print(f"   pedido do {'PIN' if i == 0 else 'senha (errado)'} apareceu em {time.time() - start:.0f}s")
        if i != 0:
            print("FALHA: o boot pediu a senha do disco em vez do PIN do TPM")
            sys.exit(1)
        time.sleep(1)
        con.send(args.tpm_pin + "\n")
    con.read_until([rb"login: $"], 420)
    print(f"   tela de login no console em {time.time() - start:.0f}s")
    con.send(args.user + "\n")
    con.read_until([rb"[Pp]assword: ?$", rb"[Ss]enha: ?$", rb"[Cc]ontrase\xc3\xb1a: ?$", rb"[Mm]ot de passe ?: ?$"], 30)
    con.send(args.password + "\n")
    time.sleep(3)
    # Shell sem prompt enfeitado, para o console ficar fácil de ler.
    con.send("exec env TERM=dumb PS1='' bash --norc --noprofile\n")
    time.sleep(1)
    con.send("stty -echo; echo __PRONTO__\n")
    con.read_until([rb"__PRONTO__\n"], 30)

    counter = [0]

    def run(cmd, timeout=60):
        counter[0] += 1
        mark = f"__FIM_{counter[0]}__"
        con.send(f"{cmd}; echo {mark}$?\n")
        _, text = con.read_until([mark.encode() + rb"\d+\n"], timeout)
        body, _, rc = text.rpartition(mark)
        return body.strip(), int(rc.strip())

    def sudo(cmd, timeout=60):
        return run(f"echo '{args.password}' | sudo -S -p '' {cmd}", timeout)

    failures = []

    def check(desc, ok, detail=""):
        print(f"   {'ok   ' if ok else 'FALHA'} {desc}" + (f"  [{detail}]" if detail and not ok else ""))
        if not ok:
            failures.append(desc)

    # O greeter e os serviços de primeiro boot levam alguns segundos a mais.
    for _ in range(40):
        out, _ = run("systemctl is-active cosmic-greeter.service")
        if out == "active":
            break
        time.sleep(3)
    check("tela de login gráfica (cosmic-greeter) ativa", out == "active", out)

    out, _ = run("grep PRETTY_NAME /etc/os-release")
    check("sistema instalado é o Tamagi", "Tamagi" in out, out)
    out, _ = run("id -nG")
    check("usuário criado no primeiro boot, no grupo wheel", "wheel" in out.split(), out)
    out, _ = sudo("sh -c 'test -e /etc/tamagi/install.conf && echo existe || echo apagado'")
    check("arquivo de respostas do instalador foi apagado", out.endswith("apagado"), out)
    out, _ = run("findmnt -rno FSTYPE,OPTIONS /sysroot")
    check("raiz btrfs com compressão zstd", out.startswith("btrfs") and "compress=zstd" in out, out)
    check("raiz no subvolume root", "subvol=/root" in out, out)
    out, _ = run("findmnt -rno FSTYPE,OPTIONS /var/home")
    check("/var/home é o subvolume home, com compressão zstd", out.startswith("btrfs") and "subvol=/home" in out and "compress=zstd" in out, out)
    out, _ = run("for d in /var/lib/containers /var/cache /var/log /var/lib/libvirt/images /var/lib/flatpak; do findmnt -rno TARGET,OPTIONS \"$d\"; done | tr '\\n' ';'")
    check("subvolumes de estado (/var/lib/containers, cache, log, imagens de VM) isolados da raiz; sem var-flatpak",
          all(s in out for s in ("subvol=/var-containers", "subvol=/var-cache", "subvol=/var-log", "subvol=/var-libvirt")) and "var-flatpak" not in out, out)
    out, _ = run("stat -c %i \"$HOME\"; ls -Zd /var/home | cut -d' ' -f1")
    check("a pasta do usuário nasceu como subvolume btrfs (snapshots)", out.split()[0] == "256", out)
    check("rótulo SELinux de /var/home", "home_root_t" in out, out)
    out, _ = sudo("sh -c 'snapper list-configs 2>&1 | tail -n +3 | tr -s \" \" | tr \"\\n\" \";\"'")
    check("snapper: snapshots da pasta pessoal ligados no primeiro boot", "/var/home/" in out, out)
    out, _ = sudo("sh -c 'ls /efi/loader/entries >/dev/null 2>&1; findmnt -rno FSTYPE,SIZE /efi; lsblk -rno PARTLABEL,FSTYPE | grep -c ext4; bootctl is-installed 2>&1; bootctl status 2>/dev/null | grep -m1 -i \"Product:\"'")
    # ESP de 2 GiB em discos menores que 80 GiB (o disco de teste tem 64 GiB), 4 GiB nos maiores.
    efi_size = re.search(r"^vfat (\S+)$", out, re.M)
    efi_ok = (bool(efi_size) if args.preserved_file else bool(efi_size) and efi_size.group(1) in ("2G", "2.0G", "1.9G", "1.99G")) and "\n0\n" in out + "\n" and "yes" in out and "systemd-boot" in out
    efi_label = "partição EFI unificada em /efi (sem /boot ext4), com systemd-boot" if args.preserved_file else "partição EFI de 2 GiB (disco < 80 GiB) em /efi, sem partição /boot, com systemd-boot"
    check(efi_label, efi_ok, out)
    if args.preserved_file:
        out, _ = sudo(f"sh -c 'cat {args.preserved_file!r}; echo; stat -c \"%U:%G %C\" {args.preserved_file!r}'")
        check("arquivo preservado em /var/home com dono e contexto SELinux corretos", "TAMAGI-HOME-PRESERVED-OK" in out and f"{args.user}:{args.user}" in out and "user_home_t" in out, out)
        out, _ = sudo("sh -c 'snapper -c home-tamagi list 2>&1 || tamagi-snap list 2>&1'")
        check("snapshot preservado (#1) aparece após reinstalação", "1 " in out or "preserv" in out, out)
    out, _ = run("tr ' ' '\\n' < /proc/cmdline | grep -c '^composefs.digest='")
    check("sistema iniciado pelo backend composefs do bootc", out.strip().endswith("1"), out[-200:])
    out, _ = sudo("sh -c 'cat /var/lib/tamagi/combustion-done 2>&1; test -e /etc/tamagi/first-boot && echo marca-ficou || echo marca-apagada'")
    check("provisionamento do primeiro boot rodou uma vez e apagou a marca", "marca-apagada" in out and "No such file" not in out, out)
    teste, _ = sudo("sh -c 'cat /etc/tamagi-combustion-teste 2>/dev/null | tr \"\\n\" \" \"'")
    if "ok (" in out or teste:
        check("combustion/script do disco de rótulo combustion rodou como root, com os arquivos ao lado",
              "uid=0" in teste and "dado=arquivo ao lado do script" in teste and "usr=somente-leitura" in teste, teste + " | " + out)
    out, _ = run("lsblk -rno TYPE,FSTYPE | sort -u | tr '\\n' ' '")
    if args.luks or args.tpm or args.tpm_pin:
        check("disco criptografado (LUKS)", "crypt btrfs" in out and "crypto_LUKS" in out, out)
    else:
        check("disco sem criptografia (como pedido)", "crypt" not in out, out)
    cmdline, _ = run("cat /proc/cmdline")
    if args.tpm:
        # Chegar até aqui sem digitar a senha do disco já é o teste principal.
        check("disco aberto pelo TPM, sem senha (rd.luks.options=...tpm2-device=auto)", "tpm2-device=auto" in cmdline, cmdline)
        out, _ = sudo("sh -c 'systemd-cryptenroll $(blkid -t TYPE=crypto_LUKS -o device | head -1) 2>&1 | tr -s \" \" | tr \"\\n\" \";\"'")
        check("LUKS com dois acessos: senha (reserva) e TPM", "password" in out and out.count("tpm2") == 1, out)
        for _ in range(20):
            seal, _ = run("systemctl is-active tamagi-tpm-seal.service; test -e /etc/tamagi/tpm-seal && echo pendente || echo feito")
            if "activating" not in seal:
                break
            time.sleep(3)
        detail, _ = sudo("journalctl -b -u tamagi-tpm-seal -o cat --no-pager | tail -5")
        sb, _ = run("mokutil --sb-state 2>&1 | head -1")
        check("primeiro boot decide o PCR do TPM (modo automático)", seal.endswith("feito") and "failed" not in seal, seal + " | " + detail)
        out, _ = sudo("sh -c 'cryptsetup luksDump --dump-json-metadata $(blkid -t TYPE=crypto_LUKS -o device | head -1)'")
        pcrs = re.findall(r'"tpm2-pcrs"\s*:\s*\[([^\]]*)\]', out)
        if "SecureBoot enabled" in sb:
            check("Secure Boot ligado: chave do TPM amarrada ao PCR 7", [p.strip() for p in pcrs] == ["7"], f"{sb} | {pcrs}")
        else:
            # Secure Boot desligado: o PCR 7 não protege nada e só travaria o
            # disco depois de mudar firmware, shim ou MOK (o erro do Aeon).
            check("Secure Boot desligado: chave do TPM sem PCR (não trava após atualizações)", [p.strip() for p in pcrs] == [""], f"{sb} | {pcrs}")
    if args.tpm_pin:
        out, _ = sudo("sh -c 'cryptsetup luksDump --dump-json-metadata $(blkid -t TYPE=crypto_LUKS -o device | head -1)'")
        pcrs = re.findall(r'"tpm2-pcrs"\s*:\s*\[([^\]]*)\]', out)
        check("TPM + PIN: chave com PIN e sem PCR (não trava depois de atualizações)", '"tpm2-pin":true' in out.replace(" ", "") and [p.strip() for p in pcrs] == [""], out[-400:])
        seal, _ = run("test -e /etc/tamagi/tpm-seal && echo pendente || echo nada")
        check("TPM + PIN: nada a selar no primeiro boot", seal.endswith("nada"), seal)
    out, _ = sudo("cat /efi/loader/loader.conf")
    check("menu de boot sem edição da linha do kernel (editor no)", re.search(r"^\s*editor\s+no\b", out, re.M) is not None, out)
    for karg in ("init_on_alloc=1", "slab_nomerge", "vsyscall=none", "plymouth.use-simpledrm=1"):
        check(f"argumento do kernel {karg}", karg in cmdline.split(), cmdline)
    # O instalador acrescenta opções depois da compressão (",noatime"): vale o começo.
    check("argumento do kernel rootflags com subvol=root e compress=zstd:1",
          any(a.startswith("rootflags=") and "subvol=root" in a and "compress=zstd:1" in a for a in cmdline.split()), cmdline)
    out, _ = sudo("bootc status --format json 2>/dev/null | python3 -c 'import json,sys; s=json.load(sys.stdin)[\"status\"]; print(s[\"booted\"][\"image\"][\"image\"][\"image\"], \"bootcHost\" if s[\"booted\"][\"composefs\"] else s[\"type\"])'")
    check("bootc reconhece o sistema (bootcHost)", "bootcHost" in out, out)
    # O console serial mistura mensagens do kernel na saída: procura a referência.
    ref = re.search(r"^(\S+/\S+) bootcHost", out, re.M)
    ref = ref.group(1) if ref else ""
    check("origem das atualizações identificada", bool(ref), out)
    print("   info  origem das atualizações: " + ref)
    if ref and not ref.startswith("localhost/"):
        up, _ = sudo("timeout 180 bootc upgrade --check 2>&1 | tail -4")
        print("   info  bootc upgrade --check:\n      " + up.replace("\n", "\n      "))
        check("bootc consulta o registry de atualizações", bool(re.search(r"No changes|Update available|Digest|Total", up)) and "rror" not in up, up)
    out, _ = run("getenforce")
    check("SELinux em modo Enforcing", out == "Enforcing", out)
    out, _ = run("sysctl -n kernel.kptr_restrict kernel.yama.ptrace_scope vm.max_map_count | tr '\\n' ' '")
    check("sysctls de endurecimento e desempenho aplicados", out.split() == ["2", "1", "2147483642"], out)
    out, _ = run("localectl status | tr -s ' ' | tr '\\n' ';'")
    want = {"pt": ("pt_BR.UTF-8", "br-abnt2", "America/Sao_Paulo"), "en": ("en_US.UTF-8", "us", "UTC"),
            "es": ("es_ES.UTF-8", "es", "Europe/Madrid"), "fr": ("fr_FR.UTF-8", "fr", "Europe/Paris")}[args.lang]
    check(f"idioma {want[0]} e teclado {want[1]}", f"LANG={want[0]}" in out and f"Keymap: {want[1]}" in out, out)
    out, _ = run("bash -lc '. /usr/libexec/tamagi/i18n; t Quit'")
    check("ferramentas do Tamagi no idioma instalado", out.splitlines()[-1:] == [{"pt": "Sair", "en": "Quit", "es": "Salir", "fr": "Quitter"}[args.lang]], out)
    out, _ = run("timedatectl show -p Timezone --value")
    zone = want[2]
    check(f"fuso {zone}", out == zone, out)
    out, _ = run("hostnamectl hostname")
    check("nome do computador", out == "tamagi", out)
    out, _ = run("swapon --noheadings --show=NAME | head -1")
    check("swap em zram", "zram" in out, out)
    out, _ = run("systemctl is-enabled firewalld; systemctl is-active firewalld")
    check("firewall ativo", out.split() == ["enabled", "active"], out)
    out, _ = run("test -s /etc/ld.so.cache && systemctl show ldconfig.service -p Result --value")
    check("ldconfig: /etc/ld.so.cache íntegro e sem falha no boot", out.strip() == "success", out)
    out, _ = run("tamagi-mcp list | grep -c memory; /usr/libexec/tamagi/egress-guard --check-host api.anthropic.com:443")
    check("tamagi-mcp e egress-guard prontos no sistema instalado", out.splitlines() == ["1", "ALLOW"], out)
    out, _ = run("systemctl --failed --no-legend --plain | awk '{print $1}' | tr '\\n' ' '")
    if out.strip():
        why, _ = sudo("sh -c 'for u in %s; do journalctl -b -u $u -o cat --no-pager | tail -6; done'" % out.strip())
        out = f"{out.strip()} | {why}"
    check("nenhum serviço falhou no boot", out == "", out)
    out, _ = run("systemctl is-active tuned.service tuned-ppd.service | tr '\\n' ' '")
    check("perfis de energia (tuned + tuned-ppd) ativos", out.split() == ["active", "active"], out)
    out, _ = run("stat -c %C /var/log/tuned")
    check("rótulo SELinux de /var/log/tuned", "tuned_log_t" in out, out)
    out, _ = sudo("find /boot /sysroot -xdev -maxdepth 2 -context '*unlabeled_t*' 2>/dev/null | tr '\\n' ' '")
    check("nenhuma pasta sem rótulo SELinux em /boot e /sysroot", out == "", out)
    out, _ = sudo("ausearch -m avc -ts boot 2>/dev/null | grep -c unlabeled_t")
    check("nenhuma negação do SELinux por arquivo sem rótulo", out.splitlines()[-1:] == ["0"], out)
    # Máquinas virtuais são opcionais (tamagi vm install): nada de libvirt na imagem.
    out, _ = run("command -v virsh qemu-kvm cockpit-bridge >/dev/null && systemctl is-enabled cockpit.socket virtqemud.socket tamagi-layer-units.service | tr '\\n' ' '")
    check("DX nativo (Cockpit/libvirt/QEMU) e camadas ligadas", out.strip() == "enabled enabled enabled", out)
    # Uma camada de verdade, de ponta a ponta (tamagi-layer em Rust): instala um
    # pacote pequeno por cima da imagem, confere que aparece e desfaz.
    sudo("-v")
    out, rc = run("tamagi-layer add teste cowsay 2>&1 | tail -3; command -v cowsay; tamagi-layer list | grep -c '^teste '", 600)
    check("camada nativa: tamagi-layer add instala e ativa um pacote", "/usr/bin/cowsay" in out.split() and out.split()[-1:] == ["1"], out[-400:])
    sudo("-v")
    out, rc = run("tamagi-layer remove teste 2>&1 | tail -1; command -v cowsay | wc -l; tamagi-layer list | grep -c '^teste '")
    check("camada nativa: tamagi-layer remove desfaz", out.split()[-2:] == ["0", "0"], out[-300:])
    out, _ = run("systemctl is-enabled tamagi-system-update.timer; systemctl --user is-enabled tamagi-autoupdate.timer flatpak-user-update.timer")
    check("atualização automática: os três timers ligados", out.split() == ["enabled"] * 3, out)
    out, _ = run("systemctl --user list-timers --all --no-legend 2>/dev/null | grep -c tamagi-autoupdate")
    check("timer de atualização do usuário agendado", out == "1", out)
    out, _ = run("grep -c 'ipe-mon.png' /usr/share/cosmic/com.system76.CosmicBackground/v1/all")
    check("papel de parede padrão é o brasão", out == "1", out)
    out, _ = run("tamagi-harden status | head -1")
    check("tamagi-harden: modo equilibrado", "equilibrado" in out or "balanced" in out, out)
    out, _ = run("chromium-browser --version; chromedriver --version; tamagi-browser installed; grep -c '/etc/chromium/tamagi.conf' /etc/chromium/chromium.conf")
    check("Chromium nativo + ChromeDriver + conta Google prontos",
          "Chromium" in out and "ChromeDriver" in out and "chromium:native" in out and out.splitlines()[-1:] == ["1"], out)
    # Caixas distrobox: no host somente-leitura o comando gerado tem de ser aceito
    # pelo podman (o distrobox 1.8.2.5 gerava ":ro:rslave", que o podman 6 recusa).
    out, _ = run("distrobox create --yes --dry-run --name teste --image docker.io/library/ubuntu:26.04 2>&1 | grep -c ':ro:rslave'")
    check("distrobox gera volumes que o podman aceita", out.splitlines()[-1:] == ["0"], out)
    out, _ = run("cat /sys/block/vda/queue/scheduler")
    print(f"   info  escalonador de E/S do disco virtual: {out}")
    out, _ = sudo("bootc status 2>&1 | head -6")
    print("   info  bootc status:\n      " + out.replace("\n", "\n      "))

    if args.screenshot:
        time.sleep(8)
        r = qmp(args.qmp, {"execute": "screendump", "arguments": {"filename": args.screenshot, "format": "png"}})
        print(f"   captura da tela: {args.screenshot}" if "return" in r[0] else f"   captura falhou: {r[0]}")

    if args.tpm:
        # Com a chave já presa ao PCR 7, o segundo boot também tem de abrir sozinho.
        con.send(f"echo '{args.password}' | sudo -S -p '' systemctl reboot\n")
        time.sleep(8)
        i, _ = con.read_until([rb"login: $", rb"[Pp]lease enter passphrase[^\n]*:"], 420)
        check("segundo boot abre o disco pelo TPM, sem senha", i == 0)

    qmp(args.qmp, {"execute": "quit"})
    print()
    if failures:
        print(f"RESULTADO: {len(failures)} falha(s): " + "; ".join(failures))
        sys.exit(1)
    print("RESULTADO: ok — a ISO instala sozinha e o sistema instalado inicia como esperado")


if __name__ == "__main__":
    main()
