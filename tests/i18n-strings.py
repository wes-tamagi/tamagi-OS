#!/usr/bin/python3
"""Lista os textos traduzíveis das ferramentas do Tamagi (os que passam por
t/tl e pelos auxiliares step/ask/confirm/input/pick) e confere os catálogos.

  i18n-strings.py <raiz>           lista os textos (um por linha)
  i18n-strings.py <raiz> --check   falha se faltar tradução em en/es/fr
<raiz> é a raiz do sistema ("/" na imagem, system_files no repositório)."""
import pathlib
import re
import sys

SCRIPTS = ["tamagi-firstrun", "tamagi-menu", "tamagi-keys", "tamagi-assist", "tamagi-lang", "tamagi-panel", "tamagi-theme"]
STR = r'"((?:[^"\\$]|\\.)*)"'
CALLS = re.compile(r'(?:\$\(|\b)(?:tl?|step|confirm|input)\s+' + STR)
ASK = re.compile(r'\bask\s+(?:--no\s+)?' + STR)
PICK = re.compile(r'"[a-z]+=([^"]+)"')
HEADER = re.compile(r'\bpick\s+' + STR)


def strings(root):
    found = []
    for name in SCRIPTS:
        text = (root / "usr/bin" / name).read_text(encoding="utf-8")
        for rx in (CALLS, ASK, PICK, HEADER):
            found += rx.findall(text)
    seen = []
    for s in found:
        if s and s not in seen:
            seen.append(s)
    return seen


def main():
    root = pathlib.Path(sys.argv[1])
    wanted = strings(root)
    if "--check" not in sys.argv:
        print("\n".join(wanted))
        return
    bad = 0
    for lang in ("en", "es", "fr"):
        cat = {}
        for line in (root / f"usr/share/tamagi/i18n/{lang}.tsv").read_text(encoding="utf-8").splitlines():
            if line and not line.startswith("#"):
                key, _, value = line.partition("\t")
                cat[key] = value
        missing = [s for s in wanted if not cat.get(s)]
        for s in missing:
            print(f"{lang}: falta traduzir: {s}")
        bad += len(missing)
        for extra in ("keys", "assist"):
            if not (root / f"usr/share/tamagi/{extra}.{lang}.txt").is_file():
                print(f"{lang}: falta {extra}.{lang}.txt")
                bad += 1
    print(f"{len(wanted)} textos, {bad} sem tradução")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
