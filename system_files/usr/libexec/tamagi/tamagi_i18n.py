"""Tamagi — shared i18n library for the Python tools (en/pt/es/fr).

English is the source language: the key of a message is its English text.
Uses the same TSV catalogs (/usr/share/tamagi/i18n/<lang>.tsv) and the same
priority order as /usr/libexec/tamagi/i18n:
  TAMAGI_LANG > LC_ALL > LC_MESSAGES > ~/.config/tamagi/lang > LANG
"""
import os
import pathlib

I18N_DIR = pathlib.Path(os.environ.get("TAMAGI_I18N_DIR", "/usr/share/tamagi/i18n"))
_CACHE = {}


def detect_lang(default="en"):
    raw = os.environ.get("TAMAGI_LANG")
    if not raw:
        raw = os.environ.get("LC_ALL") or os.environ.get("LC_MESSAGES")
    if not raw:
        cfg = pathlib.Path(os.environ.get("XDG_CONFIG_HOME", pathlib.Path.home() / ".config")) / "tamagi" / "lang"
        try:
            raw = cfg.read_text(encoding="utf-8").strip().splitlines()[0]
        except (OSError, IndexError):
            raw = ""
    if not raw:
        raw = os.environ.get("LANG") or default
    code = raw.split("_")[0].split(".")[0].lower()
    if code in ("pt", "en", "es", "fr"):
        return code
    return "en"


def _catalog(lang):
    if lang == "en":
        return {}
    if lang not in _CACHE:
        table = {}
        tsv = pathlib.Path(os.environ.get("TAMAGI_I18N_DIR", str(I18N_DIR))) / f"{lang}.tsv"
        try:
            for line in tsv.read_text(encoding="utf-8").splitlines():
                if not line or line.startswith("#") or "\t" not in line:
                    continue
                k, v = line.split("\t", 1)
                if k and v:
                    table[k] = v
        except OSError:
            pass
        _CACHE[lang] = table
    return _CACHE[lang]


def t(text, lang=None):
    lang = lang or detect_lang()
    if lang == "en":
        return text
    return _catalog(lang).get(text, text)
