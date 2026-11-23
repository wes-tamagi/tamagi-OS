#!/usr/bin/python3
"""Prova a atualização automática e a volta automática, de ponta a ponta
(tests/iso.sh --upgrade), no esquema systemd-boot + composefs do bootc.

O sistema foi instalado pela ISO apontando para um registry local do teste
(10.0.2.2:5000, o computador que roda a VM), onde está uma imagem "v2": a mesma
do Tamagi com um arquivo a mais. Aqui, dentro do sistema instalado:

  1. dispara o MESMO serviço que o timer dispara (tamagi-system-update);
  2. espera a imagem nova ficar preparada, sem ninguém pedir;
  3. reinicia e confere: a imagem nova está em uso, a verificação de saúde
     passou, a anterior ficou guardada, e um parâmetro do kernel posto nesta
     máquina passou para a imagem nova;
  4. publica uma imagem RUIM (uma verificação obrigatória que sempre falha),
     deixa o serviço prepará-la e reinicia: o sistema tem de tentar duas
     vezes, pôr a imagem em quarentena e voltar sozinho para a v2;
  5. o serviço recusa baixar de novo a imagem em quarentena;

Uso: vm_upgrade.py <serial.sock> <qmp.sock> <usuario> <senha> <digest-da-v2>"""
import json
import os
import re
import subprocess
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


def push_bad():
    """Troca a imagem do registry do teste pela ruim; devolve o digest dela."""
    out = os.path.join(os.path.dirname(serial), "vm-upgrade-bad.digest")
    subprocess.run(["podman", "push", "-q", "--tls-verify=false", "--compression-format", "zstd", "--compression-level", "3",
                    "--digestfile", out, "localhost/tamagi-teste-bad", "localhost:5000/tamagi:latest"], check=True)
    with open(out, encoding="utf-8") as fh:
        return fh.read().strip()


def staged_ready(st):
    slot = st.get("staged") or {}
    return "" if slot.get("downloadOnly") else digest(slot)


login()
st = status()
old = digest(st.get("booted"))
ref = (st.get("booted") or {}).get("image", {}).get("image", {}).get("image", "")
print(f"   info  em uso: {old[:19]}…  origem: {ref}", flush=True)
check("sistema instalado é um host bootc (composefs), sem atualização pendente",
      bool((st.get("booted") or {}).get("composefs")) and not st.get("staged"), str(st)[:300])
check("a origem das atualizações é o registry do teste", ref.startswith("10.0.2.2:5000/"), ref)
check("a imagem em uso ainda não é a v2", bool(old) and old != want_digest, old)
check("o arquivo da v2 ainda não existe", "nao" in run(f"test -e {MARKER} && echo sim || echo nao"))

# O registry do teste não tem TLS: só por isso este arquivo existe.
run("printf '[[registry]]\\nlocation = \"10.0.2.2:5000\"\\ninsecure = true\\n' | sudo tee /etc/containers/registries.conf.d/99-tamagi-teste.conf >/dev/null")
out = run("systemctl is-enabled tamagi-system-update.timer tamagi-boot-check.service; systemctl is-enabled rpm-ostreed-automatic.timer bootc-fetch-apply-updates.timer 2>&1 | grep -c '^enabled'")
check("atualização automática ligada (timer do Tamagi; os do rpm-ostree e do bootc desligados)", out.split() == ["enabled", "enabled", "0"], out)
out = run("stat -c %a /run/tamagi/bootc-status.json; tamagi-update status | grep -c tamagi-system-update; tamagi-update status | head -2 | tail -1")
check("estado da imagem publicado para o desktop (sem root)", out.split()[0] == "644", out)
run("sudo /usr/libexec/tamagi/kargs --append-if-missing=tamagi.teste=1")

print("=== disparando o serviço da atualização automática (o que o timer faz a cada 6 h)", flush=True)
start = time.time()
run("sudo systemctl start tamagi-system-update.service", 1500, show=True)
staged = staged_ready(status())
check(f"imagem nova preparada sozinha em {time.time() - start:.0f}s", bool(staged), run("sudo journalctl -b -u tamagi-system-update -o cat --no-pager | tail -12"))
check("a imagem preparada é a v2 do registry", staged == want_digest, f"{staged} != {want_digest}")
check("nada mudou no sistema em uso (a troca só vale no boot)", "nao" in run(f"test -e {MARKER} && echo sim || echo nao"))
out = run("tamagi-update status | tail -1; tamagi-update changes --sem-ia 2>&1 | head -4")
check("o desktop sabe da imagem pronta e do que muda nela", "staged" in out and "Traceback" not in out and "não encontrada" not in out, out)
out = run("sudo /usr/libexec/tamagi/system-update --now 2>&1 | tail -1")
check("rodar de novo não baixa outra vez", "already up to date" in out, out)
out = run("tamagi-update history 2>&1")
check("histórico registra a atualização preparada (staged)", "staged" in out, out)
out = run("tamagi-update check 2>&1; echo rc=$?")
check("tamagi-update check funciona sem erro", "Traceback" not in out and ("rc=0" in out or "rc=10" in out), out)
out = run("sudo tamagi-update drop 2>&1")
check("tamagi-update drop descarta o deployment staged", not staged_ready(status()), out)
out = run("tamagi-update history 2>&1")
check("histórico registra drop", "drop" in out, out)
out = run("tamagi-update check 2>&1; echo rc=$?")
check("após drop, tamagi-update check volta a detectar atualização", "New image available" in out and "rc=0" in out, out)
run("sudo systemctl start tamagi-system-update.service", 1500, show=True)
check("v2 re-preparada após drop antes do reboot", staged_ready(status()) == want_digest, str(staged_ready(status())))

print("=== reiniciando para a imagem nova", flush=True)
reboot()
for _ in range(60):          # a verificação de saúde leva alguns segundos
    if run("systemctl is-active tamagi-boot-check.service").endswith("active"):
        break
    time.sleep(3)
st = status()
check("depois do reinício a imagem em uso é a v2", digest(st.get("booted")) == want_digest, digest(st.get("booted")))
check("o arquivo da v2 existe", run(f"cat {MARKER} 2>&1").endswith("v2"))
check("a imagem anterior ficou guardada para rollback", digest(st.get("rollback")) == old, digest(st.get("rollback")))
out = run("systemctl is-active tamagi-boot-check.service; sudo cat /var/lib/tamagi/boot-state/good | cut -c1-12; sudo jq -r '.status.booted.composefs.verity[0:12]' /run/tamagi/bootc-status.json")
check("verificação de saúde do boot passou e marcou a imagem como boa", out.split()[0] == "active" and len(out.split()) == 3 and out.split()[1] == out.split()[2], out)
check("nenhum serviço falhou", run("systemctl --failed --no-legend --plain | awk '{print $1}' | tr '\\n' ' '") == "")
check("parâmetro do kernel desta máquina passou para a imagem nova", run("grep -c 'tamagi.teste=1' /proc/cmdline").endswith("1"))
check("/usr continua somente-leitura", "recusado" in run("sudo touch /usr/teste-escrita 2>/dev/null && echo gravou || echo recusado"))
check("dados do usuário preservados (/var/home)", run(f"test -d /var/home/{user} && echo sim").endswith("sim"))
print("   info  " + run("systemd-analyze | head -1"), flush=True)

print("=== publicando uma imagem ruim e deixando o serviço prepará-la", flush=True)
bad = push_bad()
run("sudo systemctl start tamagi-system-update.service", 1500)
check("a imagem ruim foi preparada (nada a denuncia antes do boot)", staged_ready(status()) == bad, run("sudo journalctl -b -u tamagi-system-update -o cat --no-pager | tail -6"))
print("=== reiniciando na imagem ruim; ela tem de voltar sozinha", flush=True)
start = time.time()
con.send("sudo systemctl reboot\n")
back = False
while time.time() - start < 900 and not back:
    time.sleep(45)
    con.send("\n")
    try:
        login(120)
    except Exception:           # reiniciou no meio do login: tenta de novo
        continue
    back = digest(status().get("booted")) == want_digest
check(f"voltou sozinho para a v2 em {time.time() - start:.0f}s", back)
st = status()
check("a imagem ruim ficou como anterior, não em uso", digest(st.get("rollback")) == bad, digest(st.get("rollback")))
out = run("cat /var/lib/tamagi/quarantine.list; ls /var/lib/tamagi/boot-failures/ | wc -l")
check("a imagem ruim está em quarentena e há relatório da falha", bad in out and out.split()[-1] != "0", out)
out = run("sudo journalctl -b -2 -u tamagi-boot-check -o cat --no-pager | grep -c 'trying once more'; sudo journalctl -b -1 -u tamagi-boot-check -o cat --no-pager | grep -c 'going back'")
check("foram duas tentativas: repetir, depois voltar", out.split() == ["1", "1"], out)
out = run("sudo /usr/libexec/tamagi/system-update --now 2>&1 | tail -1")
check("o serviço recusa baixar de novo a imagem em quarentena", "quarantine" in out and not status().get("staged"), out)
check("tamagi boot-report mostra a falha", run("tamagi-boot-report status | wc -l").split()[-1] not in ("", "0"))
check("nenhum serviço falhou depois da volta", run("systemctl --failed --no-legend --plain | awk '{print $1}' | tr '\\n' ' '") == "")

qmp(qmp_sock, {"execute": "quit"})
print()
if failures:
    print(f"RESULTADO: {len(failures)} falha(s): " + "; ".join(failures))
    sys.exit(1)
print("RESULTADO: ok — a atualização automática prepara a imagem nova, o reinício troca de forma atômica e uma imagem ruim volta sozinha")
