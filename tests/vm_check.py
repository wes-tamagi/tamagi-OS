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
    ap.add_argument("--lang", default="pt")
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
    con.read_until([rb"[Pp]assword: ?$", rb"[Ss]enha: ?$"], 30)
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
    out, _ = run("ls /etc/tamagi/install.conf 2>&1")
    check("arquivo de respostas do instalador foi apagado", "No such file" in out or "não" in out, out)
    out, _ = run("findmnt -rno FSTYPE,OPTIONS /sysroot")
    check("raiz btrfs com compressão zstd", out.startswith("btrfs") and "compress=zstd" in out, out)
    out, _ = run("findmnt -rno FSTYPE /boot")
    check("/boot separado (ext4)", out == "ext4", out)
    out, _ = run("lsblk -rno TYPE,FSTYPE | sort -u | tr '\\n' ' '")
    if args.luks:
        check("disco criptografado (LUKS)", "crypt btrfs" in out and "crypto_LUKS" in out, out)
    else:
        check("disco sem criptografia (como pedido)", "crypt" not in out, out)
    cmdline, _ = run("cat /proc/cmdline")
    for karg in ("init_on_alloc=1", "slab_nomerge", "vsyscall=none", "rootflags=compress=zstd:1"):
        check(f"argumento do kernel {karg}", karg in cmdline.split(), cmdline)
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
    out, _ = run("grep -cw binder /proc/filesystems")
    check("binderfs no kernel (Waydroid)", out == "1", out)
    out, _ = run("sysctl -n kernel.kptr_restrict kernel.yama.ptrace_scope vm.max_map_count | tr '\\n' ' '")
    check("sysctls de endurecimento e desempenho aplicados", out.split() == ["2", "1", "2147483642"], out)
    out, _ = run("localectl status | tr -s ' ' | tr '\\n' ';'")
    if args.lang == "en":
        check("idioma en_US e teclado us", "LANG=en_US.UTF-8" in out and "Keymap: us" in out, out)
    else:
        check("idioma pt_BR e teclado br-abnt2", "LANG=pt_BR.UTF-8" in out and "br-abnt2" in out, out)
    out, _ = run("timedatectl show -p Timezone --value")
    zone = "UTC" if args.lang == "en" else "America/Sao_Paulo"
    check(f"fuso {zone}", out == zone, out)
    out, _ = run("hostnamectl hostname")
    check("nome do computador", out == "tamagi", out)
    out, _ = run("swapon --noheadings --show=NAME | head -1")
    check("swap em zram", "zram" in out, out)
    out, _ = run("systemctl is-enabled firewalld; systemctl is-active firewalld")
    check("firewall ativo", out.split() == ["enabled", "active"], out)
    out, _ = run("systemctl --failed --no-legend --plain | awk '{print $1}' | tr '\\n' ' '")
    check("nenhum serviço falhou no boot", out == "", out)
    out, _ = run("systemctl is-active tuned.service tuned-ppd.service | tr '\\n' ' '")
    check("perfis de energia (tuned + tuned-ppd) ativos", out.split() == ["active", "active"], out)
    out, _ = run("stat -c %C /var/log/tuned")
    check("rótulo SELinux de /var/log/tuned", "tuned_log_t" in out, out)
    out, _ = sudo("find /boot /sysroot -xdev -maxdepth 2 -context '*unlabeled_t*' 2>/dev/null | tr '\\n' ' '")
    check("nenhuma pasta sem rótulo SELinux em /boot e /sysroot", out == "", out)
    out, _ = sudo("ausearch -m avc -ts boot 2>/dev/null | grep -c unlabeled_t")
    check("nenhuma negação do SELinux por arquivo sem rótulo", out.splitlines()[-1:] == ["0"], out)
    out, _ = run("systemctl is-active virtqemud.socket virtnetworkd.socket | tr '\\n' ' '; ls -dZ /var/lib/libvirt/images | cut -d' ' -f1")
    check("libvirt pronto (sockets ativos, pasta de imagens rotulada)", out.split()[:2] == ["active", "active"] and "virt_image_t" in out, out)
    out, _ = sudo("sh -c 'virsh -c qemu:///system list --all 2>&1 | head -1; virsh -c qemu:///system net-list --all 2>&1 | grep -c default'")
    check("virsh fala com o QEMU do sistema e a rede default existe", "Id" in out and out.splitlines()[-1] == "1", out)
    out, _ = run("systemctl is-enabled rpm-ostreed-automatic.timer flatpak-system-update.timer; systemctl --user is-enabled tamagi-autoupdate.timer flatpak-user-update.timer")
    check("atualização automática: os quatro timers ligados", out.split() == ["enabled"] * 4, out)
    out, _ = run("systemctl --user list-timers --all --no-legend 2>/dev/null | grep -c tamagi-autoupdate")
    check("timer de atualização do usuário agendado", out == "1", out)
    out, _ = run("grep -c 'tokyo-night-mon.png' /usr/share/cosmic/com.system76.CosmicBackground/v1/all")
    check("papel de parede padrão é o brasão", out == "1", out)
    out, _ = run("tamagi-harden status | head -1")
    check("tamagi-harden: modo equilibrado", "equilibrado" in out, out)
    out, _ = run("tamagi-waydroid status | head -1")
    check("tamagi-waydroid enxerga o Binder", "sim" in out, out)
    out, _ = run("cat /sys/block/vda/queue/scheduler")
    print(f"   info  escalonador de E/S do disco virtual: {out}")
    out, _ = sudo("bootc status 2>&1 | head -6")
    print("   info  bootc status:\n      " + out.replace("\n", "\n      "))

    if args.screenshot:
        time.sleep(8)
        r = qmp(args.qmp, {"execute": "screendump", "arguments": {"filename": args.screenshot, "format": "png"}})
        print(f"   captura da tela: {args.screenshot}" if "return" in r[0] else f"   captura falhou: {r[0]}")

    qmp(args.qmp, {"execute": "quit"})
    print()
    if failures:
        print(f"RESULTADO: {len(failures)} falha(s): " + "; ".join(failures))
        sys.exit(1)
    print("RESULTADO: ok — a ISO instala sozinha e o sistema instalado inicia como esperado")


if __name__ == "__main__":
    main()
