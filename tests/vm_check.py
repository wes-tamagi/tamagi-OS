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
    ap.add_argument("--lang", default="en")
    ap.add_argument("--screenshot")
    args = ap.parse_args()

    con = Serial(args.serial)
    start = time.time()
    if args.luks:
        con.read_until([rb"[Pp]lease enter passphrase[^\n]*:"], 240)
        print(f"   pedido da senha do disco apareceu em {time.time() - start:.0f}s")
        time.sleep(1)
        con.send(args.luks + "\n")
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
    out, _ = run("findmnt -rno FSTYPE /boot")
    check("/boot separado (ext4)", out == "ext4", out)
    out, _ = run("lsblk -rno TYPE,FSTYPE | sort -u | tr '\\n' ' '")
    if args.luks or args.tpm:
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
        check("chave do TPM amarrada ao PCR 7 no primeiro boot", seal.endswith("feito") and "failed" not in seal, seal + " | " + detail)
        out, _ = sudo("sh -c 'cryptsetup luksDump $(blkid -t TYPE=crypto_LUKS -o device | head -1) | grep -A12 systemd-tpm2 | grep -i pcrs | tr -s \" \"'")
        check("o registro no TPM usa o PCR 7", bool(re.search(r"tpm2-hash-pcrs:\s*7\b", out)), out)
    for karg in ("init_on_alloc=1", "slab_nomerge", "vsyscall=none"):
        check(f"argumento do kernel {karg}", karg in cmdline.split(), cmdline)
    # O instalador acrescenta opções depois da compressão (",noatime"): vale o começo.
    check("argumento do kernel rootflags=compress=zstd:1",
          any(a.startswith("rootflags=compress=zstd:1") for a in cmdline.split()), cmdline)
    out, _ = sudo("bootc status --format json 2>/dev/null | python3 -c 'import json,sys; s=json.load(sys.stdin)[\"status\"]; print(s[\"booted\"][\"image\"][\"image\"][\"image\"], s[\"type\"])'")
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
    out, _ = run("bash -lc '. /usr/libexec/tamagi/i18n; t Sair'")
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
    out, _ = run("command -v virsh virt-manager | wc -l; systemctl is-enabled tamagi-layer-units.service")
    check("virt-manager fora da imagem; serviço das camadas ligado", out.split() == ["0", "enabled"], out)
    out, _ = run("systemctl is-enabled rpm-ostreed-automatic.timer flatpak-system-update.timer; systemctl --user is-enabled tamagi-autoupdate.timer flatpak-user-update.timer")
    check("atualização automática: os quatro timers ligados", out.split() == ["enabled"] * 4, out)
    out, _ = run("systemctl --user list-timers --all --no-legend 2>/dev/null | grep -c tamagi-autoupdate")
    check("timer de atualização do usuário agendado", out == "1", out)
    out, _ = run("grep -c 'tokyo-night-mon.png' /usr/share/cosmic/com.system76.CosmicBackground/v1/all")
    check("papel de parede padrão é o brasão", out == "1", out)
    out, _ = run("tamagi-harden status | head -1")
    check("tamagi-harden: modo equilibrado", "equilibrado" in out, out)
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
        check("segundo boot abre o disco pelo TPM (PCR 7), sem senha", i == 0)

    qmp(args.qmp, {"execute": "quit"})
    print()
    if failures:
        print(f"RESULTADO: {len(failures)} falha(s): " + "; ".join(failures))
        sys.exit(1)
    print("RESULTADO: ok — a ISO instala sozinha e o sistema instalado inicia como esperado")


if __name__ == "__main__":
    main()
