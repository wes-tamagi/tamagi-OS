#!/usr/bin/python3
"""Prova o rollback automático (tests/iso.sh --rollback). No sistema instalado, cria uma
"atualização ruim" (mesma imagem + tamagi.failboot na linha do kernel, que a
verificação de saúde do Tamagi reprova) e reinicia. Esperado: o sistema
reinicia sozinho até esgotar as tentativas e volta para a imagem anterior."""
import sys, time
import os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vm_check import Serial, qmp
serial, qmp_sock, user, pw = sys.argv[1:5]
con = Serial(serial)
n = [0]

def login():
    con.read_until([rb"login: $"], 600)
    con.send(user + "\n"); con.read_until([rb"[Pp]assword: ?$", rb"[Ss]enha: ?$", rb"[Cc]ontrase\xc3\xb1a: ?$", rb"[Mm]ot de passe ?: ?$"], 30); con.send(pw + "\n"); time.sleep(3)
    con.send("exec env TERM=dumb PS1='' bash --norc --noprofile\n"); time.sleep(1)
    con.send("stty -echo cols 250; echo __PRONTO__\n"); con.read_until([rb"__PRONTO__\n"], 30)

def run(cmd, timeout=180, show=True):
    n[0] += 1; mark = f"__FIM_{n[0]}__"
    con.send(f"{cmd}; echo {mark}$?\n")
    _, text = con.read_until([mark.encode() + rb"\d+\n"], timeout)
    body = text.rpartition(mark)[0].strip()
    if show: print(f"$ {cmd}\n{body}\n", flush=True)
    return body

login()
run(f"echo '{pw}' | sudo -S -v; systemctl is-active greenboot-healthcheck.service boot-complete.target; sudo grub2-editenv - list")
run("tamagi-boot-report status")
run("sudo rpm-ostree kargs --append=tamagi.failboot 2>&1 | tail -2", 300)
run("sudo grub2-editenv - list; rpm-ostree status | grep -E '●|Version|Digest' | head -6")
print("=== reiniciando na 'atualização ruim'", flush=True)
start = time.time()
con.send("sudo systemctl reboot\n")
boots = 0
while True:
    # Em cada boot ruim o console chega a mostrar login antes de o greenboot reiniciar.
    try:
        login()
    except SystemExit as e:
        print("sem login:", str(e)[:300]); break
    boots += 1
    try:
        cmdline = run("cat /proc/cmdline | tr ' ' '\\n' | grep -c tamagi.failboot", 20, show=False)
    except SystemExit:
        print(f"  boot {boots}: reiniciou antes de responder (boot ruim)", flush=True); continue
    if cmdline.strip().endswith("0"):
        print(f"=== boot {boots}: voltou para a imagem boa em {time.time() - start:.0f}s", flush=True); break
    print(f"  boot {boots}: ainda na imagem ruim", flush=True)
    if boots > 8: print("desisti"); break
    time.sleep(1)
time.sleep(15)
run(f"echo '{pw}' | sudo -S -v; cat /proc/cmdline | tr ' ' '\\n' | grep -c failboot; sudo grub2-editenv - list; rpm-ostree status | grep -E '●|Version|kargs|failboot' | head -8")
run("systemctl is-active greenboot-healthcheck.service boot-complete.target cosmic-greeter.service; sudo journalctl -b -u greenboot-healthcheck -o cat --no-pager | tail -6")
run("sudo ls -la /var/lib/tamagi/boot-failures/; tamagi-boot-report status")
run("sudo -u " + user + " tamagi-boot-report show </dev/null | head -40")
good = run("cat /proc/cmdline | tr ' ' '\\n' | grep -c tamagi.failboot", show=False).strip().endswith("0")
reports = run("sudo sh -c 'ls /var/lib/tamagi/boot-failures/*.txt 2>/dev/null | wc -l'", show=False).strip().splitlines()[-1]
healthy = run("systemctl is-active boot-complete.target", show=False).strip().endswith("active")
qmp(qmp_sock, {"execute": "quit"})
print(f"voltou para a imagem boa: {good} · relatórios gravados: {reports} · boot atual saudável: {healthy}")
if not (good and healthy and reports.isdigit() and int(reports) > 0):
    sys.exit("RESULTADO: FALHA — o rollback automático não se comportou como esperado")
print("RESULTADO: ok — a imagem ruim foi desfeita sozinha e o motivo ficou registrado")
