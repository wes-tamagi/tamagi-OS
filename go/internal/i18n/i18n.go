// Package i18n resolves Tamagi's active language and loads /usr/share/tamagi/i18n TSV catalogs.
package i18n

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
)

var (
	once sync.Once
	lang string
	dict map[string]string
)

func normalizeLang(raw string) string {
	s := strings.TrimSpace(strings.ToLower(raw))
	if s == "" || s == "c" || s == "posix" {
		return ""
	}
	if i := strings.IndexAny(s, "._@"); i >= 0 {
		s = s[:i]
	}
	switch s {
	case "en", "pt", "es", "fr":
		return s
	default:
		return "en"
	}
}

// DetectLang returns the active 2-letter language code ("en", "pt", "es", or "fr").
func DetectLang() string {
	for _, k := range []string{"TAMAGI_LANG"} {
		if l := normalizeLang(os.Getenv(k)); l != "" {
			return l
		}
	}
	if home, err := os.UserHomeDir(); err == nil && home != "" {
		if b, err := os.ReadFile(filepath.Join(home, ".config/tamagi/lang")); err == nil {
			if l := normalizeLang(string(b)); l != "" {
				return l
			}
		}
	}
	for _, k := range []string{"LC_ALL", "LC_MESSAGES", "LANG"} {
		if l := normalizeLang(os.Getenv(k)); l != "" {
			return l
		}
	}
	return "en"
}

func load() {
	once.Do(func() {
		lang = DetectLang()
		dict = make(map[string]string)
		if lang == "en" {
			return
		}
		dir := os.Getenv("TAMAGI_I18N_DIR")
		if dir == "" {
			dir = "/usr/share/tamagi/i18n"
		}
		f, err := os.Open(filepath.Join(dir, lang+".tsv"))
		if err != nil {
			return
		}
		defer f.Close()
		sc := bufio.NewScanner(f)
		for sc.Scan() {
			line := sc.Text()
			if line == "" || strings.HasPrefix(line, "#") {
				continue
			}
			k, v, ok := strings.Cut(line, "\t")
			if ok && k != "" && v != "" {
				dict[k] = v
			}
		}
	})
}

// T translates an English key into the active language.
func T(key string) string {
	load()
	if v, ok := dict[key]; ok {
		return v
	}
	return key
}

// TL prints the translated key followed by a newline.
func TL(key string) {
	fmt.Println(T(key))
}
