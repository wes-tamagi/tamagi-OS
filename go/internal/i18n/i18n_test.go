package i18n

import "testing"

func TestNormalizeLang(t *testing.T) {
	cases := []struct {
		in, want string
	}{
		{"en_US.UTF-8", "en"},
		{"pt_BR.UTF-8", "pt"},
		{"es", "es"},
		{"fr_FR@euro", "fr"},
		{"de_DE.UTF-8", "en"},
		{"C", ""},
		{"", ""},
	}
	for _, tc := range cases {
		if got := normalizeLang(tc.in); got != tc.want {
			t.Fatalf("normalizeLang(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}
