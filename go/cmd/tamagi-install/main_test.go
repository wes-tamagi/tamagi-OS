package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadCatalog(t *testing.T) {
	dir := t.TempDir()
	cat := filepath.Join(dir, "catalog.tsv")
	fp := filepath.Join(dir, "flatpaks.list")
	if err := os.WriteFile(cat, []byte("# comment\nsteam\tGames\t1 GB\tgaming:base\tSteam — games\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(fp, []byte("flathub org.videolan.VLC opcional Media 100MB VLC — video player\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Setenv("TAMAGI_CATALOG", cat)
	t.Setenv("TAMAGI_FLATPAKS", fp)

	items := loadCatalog()
	if len(items) != 2 {
		t.Fatalf("got %d items, want 2", len(items))
	}
	if items[0].ID != "steam" || items[1].ID != "vlc" || items[1].Action != "flatpak:flathub:org.videolan.VLC" {
		t.Fatalf("unexpected items: %+v", items)
	}
}
