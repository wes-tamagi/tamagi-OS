#!/usr/bin/python3
"""Prova a atualização automática de ponta a ponta (tests/iso.sh --upgrade).

O sistema foi instalado pela ISO apontando para um registry local do teste
(10.0.2.2:5000, o computador que roda a VM), onde está uma imagem "v2": a mesma
do Tamagi com um arquivo a mais. Aqui, dentro do sistema instalado:

  1. dispara o MESMO serviço que o timer diário dispara (rpm-ostreed-automatic);
  2. espera a imagem nova ficar preparada (staged), sem ninguém pedir;
  3. reinicia e confere: a imagem nova está em uso, a verificação de saúde
     passou e a imagem anterior ficou guardada;
  4. volta para a anterior (bootc rollback) e confere que ela inicia: a troca é
     atômica nos dois sentidos.

Uso: vm_upgrade.py <serial.sock> <qmp.sock> <usuario> <senha> <digest-da-v2>"""
import json
import os
import re
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from vm_check import Serial, qmp  # noqa: E402

serial, qmp_sock, user, pw, want_digest = sys.argv[1:6]
MARKER = "/usr/share/tamagi/teste-atualizacao"
con = Serial(serial)
count = [0]
failures = []


def login(timeout=600):
    con.read_until([rb"login: $"], timeout)
    con.send(user + "\n")
    con.read_until([rb"[Pp]assword: ?$", rb"[Ss]enha: ?$", rb"[Cc]ontrase\xc3\xb1a: ?$", rb"[Mm]ot de passe ?: ?$"], 30)
    con.send(pw + "\n")
    time.sleep(3)
    con.send("exec env TERM=dumb PS1='' LANG=C.UTF-8 bash --norc --noprofile\n")
    time.sleep(1)
    con.send("stty -echo cols 250; echo __PRONTO__\n")
    con.read_until([rb"__PRONTO__\n"], 30)
    # O teste dura mais que o cache de senha do sudo: na VM de teste, sudo sem senha.
    run(f"echo '{pw}' | sudo -S sh -c 'echo \"{user} ALL=(ALL) NOPASSWD: ALL\" > /etc/sudoers.d/99-tamagi-teste; chmod 440 /etc/sudoers.d/99-tamagi-teste'", show=False)


def run(cmd, timeout=120, show=False):
    count[0] += 1
    mark = f"__FIM_{count[0]}__"
    con.send(f"{cmd}; echo {mark}$?\n")
    _, text = con.read_until([mark.encode() + rb"\d+\n"], timeout)
    body = text.rpartition(mark)[0].strip()
    if show:
        print(f"$ {cmd}\n{body}\n", flush=True)
    return body


def check(desc, ok, detail=""):
    print(f"   {'ok   ' if ok else 'FALHA'} {desc}" + (f"  [{detail[-400:]}]" if detail and not ok else ""), flush=True)
    if not ok:
        failures.append(desc)


def status():
    """bootc status em JSON. O console serial mistura mensagens do kernel: pega só o objeto."""
    raw = run("sudo bootc status --format json 2>/dev/null | tr -d '\\n'; echo")
    m = re.search(r"\{.*\}", raw)
    return json.loads(m.group(0))["status"] if m else {}


def digest(slot):
    try:
        return slot["image"]["imageDigest"]
    except (KeyError, TypeError):
        return ""


def reboot():
    con.send("sudo systemctl reboot\n")
    time.sleep(5)
    login()


login()
st = status()
old = digest(st.get("booted"))
ref = (st.get("booted") or {}).get("image", {}).get("image", {}).get("image", "")
print(f"   info  em uso: {old[:19]}…  origem: {ref}", flush=True)
check("sistema instalado é um host bootc, sem atualização pendente", st.get("type") == "bootcHost" and not st.get("staged"), str(st)[:300])
check("a origem das atualizações é o registry do teste", ref.startswith("10.0.2.2:5000/"), ref)
check("a imagem em uso ainda não é a v2", bool(old) and old != want_digest, old)
check("o arquivo da v2 ainda não existe", "nao" in run(f"test -e {MARKER} && echo sim || echo nao"))

# O registry do teste não tem TLS: só por isso este arquivo existe.
run("printf '[[registry]]\\nlocation = \"10.0.2.2:5000\"\\ninsecure = true\\n' | sudo tee /etc/containers/registries.conf.d/99-tamagi-teste.conf >/dev/null")
out = run("systemctl is-enabled rpm-ostreed-automatic.timer; grep -c '^AutomaticUpdatePolicy=stage' /etc/rpm-ostreed.conf")
check("atualização automática ligada (timer diário, política stage)", out.split() == ["enabled", "1"], out)

print("=== disparando o serviço da atualização automática (o que o timer faz todo dia)", flush=True)
start = time.time()
run("sudo systemctl start rpm-ostreed-automatic.service", show=True)
staged = ""
while time.time() - start < 1500:
    time.sleep(15)
    staged = digest(status().get("staged"))
    if staged:
        break
    state = run("systemctl is-active rpm-ostreed-automatic.service; rpm-ostree status 2>/dev/null | grep -m1 -i -E 'Transaction|State'").replace("\n", " · ")
    print(f"   …{time.time() - start:4.0f}s  {state[:140]}", flush=True)
check(f"imagem nova preparada sozinha (staged) em {time.time() - start:.0f}s", bool(staged), run("sudo journalctl -b -u rpm-ostreed-automatic -u rpm-ostreed -o cat --no-pager | tail -12"))
check("a imagem preparada é a v2 do registry", staged == want_digest, f"{staged} != {want_digest}")
check("nada mudou no sistema em uso (a troca só vale no boot)", "nao" in run(f"test -e {MARKER} && echo sim || echo nao"))

print("=== reiniciando para a imagem nova", flush=True)
reboot()
for _ in range(40):          # a verificação de saúde leva alguns segundos
    if run("systemctl is-active boot-complete.target").endswith("active"):
        break
    time.sleep(3)
st = status()
check("depois do reinício a imagem em uso é a v2", digest(st.get("booted")) == want_digest, digest(st.get("booted")))
check("o arquivo da v2 existe", run(f"cat {MARKER} 2>&1").endswith("v2"))
check("a imagem anterior ficou guardada para rollback", digest(st.get("rollback")) == old, digest(st.get("rollback")))
check("verificação de saúde do boot passou (greenboot)", run("systemctl is-active boot-complete.target greenboot-healthcheck.service | tr '\\n' ' '").split() == ["active", "active"])
check("nenhum serviço falhou", run("systemctl --failed --no-legend --plain | awk '{print $1}' | tr '\\n' ' '") == "")
check("/usr continua somente-leitura", "recusado" in run("sudo touch /usr/teste-escrita 2>/dev/null && echo gravou || echo recusado"))
check("dados do usuário preservados (/var/home)", run(f"test -d /var/home/{user} && echo sim").endswith("sim"))

print("=== voltando para a imagem anterior (bootc rollback)", flush=True)
run("sudo bootc rollback", 120, show=True)
reboot()
st = status()
check("rollback: a imagem anterior voltou a ser a em uso", digest(st.get("booted")) == old, digest(st.get("booted")))
check("rollback: o arquivo da v2 sumiu", "nao" in run(f"test -e {MARKER} && echo sim || echo nao"))
check("rollback: a v2 continua guardada (dá para voltar a ela)", digest(st.get("rollback")) == want_digest, digest(st.get("rollback")))

qmp(qmp_sock, {"execute": "quit"})
print()
if failures:
    print(f"RESULTADO: {len(failures)} falha(s): " + "; ".join(failures))
    sys.exit(1)
print("RESULTADO: ok — a atualização automática prepara a imagem nova, o reinício troca de forma atômica e o rollback volta")
