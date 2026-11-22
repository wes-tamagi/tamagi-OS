#!/usr/bin/python3
"""Lista os textos traduzíveis das ferramentas do Tamagi (os que passam por
t/tl e pelos auxiliares step/ask/confirm/input/pick) e confere os catálogos,
além de validar todas as traduções de actions.toml.

  i18n-strings.py <raiz>           lista os textos (um por linha)
  i18n-strings.py <raiz> --check   falha se faltar tradução em en/es/fr
<raiz> é a raiz do sistema ("/" na imagem, system_files no repositório)."""
import pathlib
import re
import sys
import tomllib

SCRIPTS = [
    "tamagi", "tamagi-firstrun", "tamagi-keys", "tamagi-assist", "tamagi-lang",
    "tamagi-panel", "tamagi-theme", "tamagi-ai", "tamagi-update",
    "tamagi-boot-report", "tamagi-windows", "tamagi-db", "tamagi-lembrete",
    "tamagi-dev", "tamagi-devbox", "tamagi-font", "tamagi-popos", "tamagi-antigravity",
    "tamagi-isolar", "tamagi-privacidade", "tamagi-convidado",
    "tamagi-browser", "tamagi-harden", "tamagi-layer", "tamagi-pkg", "tamagi-webapp",
    "tamagi-perf", "tamagi-workspace", "tamagi-doctor", "tamagi-gaming",
]
STR = r'"((?:[^"\\$]|\\.)*)"'
CALLS = re.compile(r'(?:\$\(|\b)(?:tl?|step|confirm|input|timer_line)\s+' + STR)
PY_CALLS_DQ = re.compile(r'\bt\(\s*"((?:[^"\\]|\\.)*)"\s*(?:,|\))')
PY_CALLS_SQ = re.compile(r"\bt\(\s*'((?:[^'\\]|\\.)*)'\s*(?:,|\))")
ASK = re.compile(r'\bask\s+(?:--no\s+)?' + STR)
PICK = re.compile(r'"[a-z]+=([^"]+)"')
HEADER = re.compile(r'\bpick\s+' + STR)


def strings(root):
    found = []
    for name in SCRIPTS:
        p = root / "usr/bin" / name
        if not p.is_file():
            continue
        text = p.read_text(encoding="utf-8")
        if text.startswith("#!/usr/bin/python"):
            for rx in (PY_CALLS_DQ, PY_CALLS_SQ):
                found += rx.findall(text)
        else:
            for rx in (CALLS, ASK, PICK, HEADER):
                found += rx.findall(text)
    seen = []
    for s in found:
        if s and s not in seen:
            seen.append(s)
    return seen


def check_actions_toml(root):
    bad = 0
    path = root / "usr/share/tamagi/actions.toml"
    if not path.is_file():
        print("falta usr/share/tamagi/actions.toml")
        return 1
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    for cid, meta in data.get("categories", {}).items():
        for lang in ("pt", "en", "es", "fr"):
            if not meta.get(lang):
                print(f"actions.toml: categoria '{cid}' sem tradução '{lang}'")
                bad += 1
    seen_ids = set()
    for a in data.get("action", []):
        aid = a.get("id", "?")
        if aid in seen_ids:
            print(f"actions.toml: ação duplicada '{aid}'")
            bad += 1
        seen_ids.add(aid)
        for field in ("title", "subtitle"):
            m = a.get(field, {})
            for lang in ("pt", "en", "es", "fr"):
                if not isinstance(m, dict) or not m.get(lang):
                    print(f"actions.toml: ação '{aid}' sem {field}.{lang}")
                    bad += 1
    return bad


def flatpak_catalog(root):
    """Categorias e descrições de flatpaks.list que o assistente mostra
    (tipos recomendado e opcional): passam por t() e precisam de tradução."""
    path = root / "usr/share/tamagi/flatpaks.list"
    found = []
    if path.is_file():
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#"):
                continue
            parts = line.split(None, 5)
            if len(parts) == 6 and parts[2] in ("recomendado", "opcional"):
                found += [parts[3], parts[5]]
    return found


def main():
    root = pathlib.Path(sys.argv[1])
    wanted = strings(root)
    wanted += [s for s in flatpak_catalog(root) if s not in wanted]
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
    bad += check_actions_toml(root)
    print(f"{len(wanted)} textos + actions.toml, {bad} problema(s)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
