#!/usr/bin/python3
"""Lista os textos traduzíveis das ferramentas do Tamagi (os que passam por
t/tl e pelos auxiliares step/ask/confirm/input/pick) e confere os catálogos,
além de validar todas as traduções de actions.toml.

  i18n-strings.py <raiz>           lista os textos (um por linha)
  i18n-strings.py <raiz> --check   falha se faltar tradução em pt/es/fr
As mensagens são escritas em inglês (a chave dos catálogos).
<raiz> é a raiz do sistema ("/" na imagem, system_files no repositório)."""
import pathlib
import re
import sys
import tomllib

SCRIPTS = [
    "tamagi", "tamagi-firstrun", "tamagi-keys", "tamagi-lang", "tamagi-install",
    "tamagi-panel", "tamagi-theme", "tamagi-ai", "tamagi-update", "tamagi-agent",
    "tamagi-mcp", "tamagi-boot-report", "tamagi-windows", "tamagi-db", "tamagi-vm",
    "tamagi-devbox", "tamagi-font", "tamagi-antigravity", "tamagi-try",
    "tamagi-isolate", "tamagi-privacy",
    "tamagi-browser", "tamagi-harden", "tamagi-layer", "tamagi-webapp",
    "tamagi-perf", "tamagi-workspace", "tamagi-doctor", "tamagi-gaming",
    "tamagi-cfs", "tamagi-skl", "opencode",
]
LIBEXEC_SCRIPTS = ["agent-manager", "flatpak-setup", "skl-browser-launch", "nvidia-setup"]
STR = r'"((?:[^"\\$]|\\.)*)"'
CALLS = re.compile(r'(?:\$\(|\b)(?:tl?|step|confirm|input|timer_line)\s+' + STR)
PY_CALLS_DQ = re.compile(r'\bt\(\s*"((?:[^"\\]|\\.)*)"\s*(?:,|\))')
PY_CALLS_SQ = re.compile(r"\bt\(\s*'((?:[^'\\]|\\.)*)'\s*(?:,|\))")
ASK = re.compile(r'\bask\s+(?:--no\s+)?' + STR)
PICK = re.compile(r'"[a-z]+=([^"]+)"')
HEADER = re.compile(r'\bpick\s+' + STR)


def strings(root):
    found = []
    paths = [root / "usr/bin" / name for name in SCRIPTS] + [root / "usr/libexec/tamagi" / name for name in LIBEXEC_SCRIPTS]
    for p in paths:
        if not p.is_file() or p.read_bytes()[:4] == b"\x7fELF":   # a Rust program: see below
            continue
        text = p.read_text(encoding="utf-8")
        if text.startswith("#!/usr/bin/python"):
            for rx in (PY_CALLS_DQ, PY_CALLS_SQ):
                found += rx.findall(text)
        else:
            for rx in (CALLS, ASK, PICK, HEADER):
                found += rx.findall(text)
    # The tools written in Rust (rust/, next to system_files in the repository;
    # the image carries only the binaries): t("…") calls, same English keys.
    for src in sorted((root.parent / "rust").glob("*/src/*.rs")):
        found += [m.replace('\\"', '"') for m in PY_CALLS_DQ.findall(src.read_text(encoding="utf-8"))]
    # The first-login window (welcome/, Rust + libcosmic): its list of strings.
    welcome = root.parent / "welcome/NEW_STRINGS.txt"
    if welcome.is_file():
        found += welcome.read_text(encoding="utf-8").splitlines()
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


def command_list(root):
    """Groups and descriptions of the COMMANDS table of `tamagi` (shown by
    `tamagi` with no arguments): they go through t() as variables."""
    # `tamagi` is a Rust program: the table is in its source, in the repository.
    src = root.parent / "rust/tamagi/src/main.rs"
    if not src.is_file():
        return []
    text = src.read_text(encoding="utf-8")
    found = []
    for group, desc in re.findall(r'^    \("([^"]+)", "[a-z-]+", "([^"]+)"\),$', text, re.M):
        found += [group, desc]
    return found


def devbox_labels(root):
    """Distro labels of tamagi-devbox (LABEL table), shown through t()."""
    text = (root / "usr/bin/tamagi-devbox").read_text(encoding="utf-8")
    block = re.search(r"declare -A LABEL=\((.*?)\n\)", text, re.S)
    return re.findall(r'\]="([^"]+)"', block.group(1)) if block else []


def mcp_labels(root):
    """Server summaries of tamagi-mcp (SUMMARY table), shown through t()."""
    p = root / "usr/bin/tamagi-mcp"
    if not p.is_file():
        return []
    text = p.read_text(encoding="utf-8")
    block = re.search(r"declare -A SUMMARY=\((.*?)\n\)", text, re.S)
    return re.findall(r'\]="([^"]+)"', block.group(1)) if block else []


def catalog(root):
    """Categories and descriptions of catalog.tsv (tamagi install, first login)."""
    path = root / "usr/share/tamagi/catalog.tsv"
    found = []
    if path.is_file():
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) == 5:
                found += [parts[1], parts[4]]
    return found


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
            if len(parts) == 6 and parts[2] == "opcional":
                found += [parts[3], parts[5]]
    return found


def main():
    root = pathlib.Path(sys.argv[1])
    wanted = strings(root)
    for extra in (flatpak_catalog(root), catalog(root), command_list(root), devbox_labels(root), mcp_labels(root)):
        wanted += [s for s in extra if s not in wanted]
    if "--check" not in sys.argv:
        print("\n".join(wanted))
        return
    bad = 0
    for lang in ("pt", "es", "fr"):
        cat = {}
        for line in (root / f"usr/share/tamagi/i18n/{lang}.tsv").read_text(encoding="utf-8").splitlines():
            if line and not line.startswith("#"):
                key, _, value = line.partition("\t")
                cat[key] = value
        missing = [s for s in wanted if not cat.get(s)]
        for s in missing:
            print(f"{lang}: missing translation: {s}")
        bad += len(missing)
        for extra in ("keys",):
            if not (root / f"usr/share/tamagi/{extra}.{lang}.txt").is_file():
                print(f"{lang}: falta {extra}.{lang}.txt")
                bad += 1
    bad += check_actions_toml(root)
    print(f"{len(wanted)} textos + actions.toml, {bad} problema(s)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
