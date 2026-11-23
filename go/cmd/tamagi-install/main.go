// tamagi-install — everything optional, in one catalog (+ live Flathub).
package main

import (
	"bufio"
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"
	"syscall"
	"time"

	"tamagi/internal/i18n"
)

type catalogItem struct {
	ID     string
	Cat    string
	Size   string
	Action string
	Desc   string
}

type flathubHit struct {
	ID       string
	Name     string
	Summary  string
	Verified bool
}

func t(k string) string { return i18n.T(k) }
func tl(k string)       { i18n.TL(k) }

func envOr(k, def string) string {
	if v := os.Getenv(k); v != "" {
		return v
	}
	return def
}

func userHome() string {
	if h, err := os.UserHomeDir(); err == nil && h != "" {
		return h
	}
	return os.Getenv("HOME")
}

func cacheDir() string {
	base := os.Getenv("XDG_CACHE_HOME")
	if base == "" {
		base = filepath.Join(userHome(), ".cache")
	}
	return filepath.Join(base, "tamagi")
}

func runtimeDir() string {
	if d := os.Getenv("XDG_RUNTIME_DIR"); d != "" {
		return d
	}
	return "/tmp"
}

func isTTY(f *os.File) bool {
	st, err := f.Stat()
	return err == nil && (st.Mode()&os.ModeCharDevice) != 0
}

func loadCatalog() []catalogItem {
	var items []catalogItem
	catPath := envOr("TAMAGI_CATALOG", "/usr/share/tamagi/catalog.tsv")
	if f, err := os.Open(catPath); err == nil {
		sc := bufio.NewScanner(f)
		for sc.Scan() {
			ln := sc.Text()
			if ln == "" || strings.HasPrefix(ln, "#") {
				continue
			}
			parts := strings.SplitN(ln, "\t", 5)
			if len(parts) < 5 {
				continue
			}
			items = append(items, catalogItem{
				ID:     parts[0],
				Cat:    parts[1],
				Size:   parts[2],
				Action: parts[3],
				Desc:   parts[4],
			})
		}
		f.Close()
	}

	fpPath := envOr("TAMAGI_FLATPAKS", "/usr/share/tamagi/flatpaks.list")
	if f, err := os.Open(fpPath); err == nil {
		sc := bufio.NewScanner(f)
		for sc.Scan() {
			ln := strings.TrimSpace(sc.Text())
			if ln == "" || strings.HasPrefix(ln, "#") {
				continue
			}
			fields := strings.Fields(ln)
			if len(fields) < 6 || fields[2] != "opcional" {
				continue
			}
			remote, app, cat, size := fields[0], fields[1], fields[3], fields[4]
			// Reconstruct desc starting from the 6th field in original spacing.
			idx := 0
			pos := 0
			for i := 0; i < 5; i++ {
				for pos < len(ln) && (ln[pos] == ' ' || ln[pos] == '\t') {
					pos++
				}
				for pos < len(ln) && ln[pos] != ' ' && ln[pos] != '\t' {
					pos++
				}
				idx = pos
			}
			desc := strings.TrimSpace(ln[idx:])
			baseName := strings.SplitN(desc, " — ", 2)[0]
			id := strings.ReplaceAll(strings.ToLower(baseName), " ", "-")
			items = append(items, catalogItem{
				ID:     id,
				Cat:    cat,
				Size:   size,
				Action: "flatpak:" + remote + ":" + app,
				Desc:   desc,
			})
		}
		f.Close()
	}
	return items
}

func (it catalogItem) label() string {
	return fmt.Sprintf("[%s] %s (≈ %s)", t(it.Cat), t(it.Desc), it.Size)
}

func cmdList(items []catalogItem) {
	for _, it := range items {
		fmt.Printf("  %-18s %s\n", it.ID, it.label())
	}
}

func runQuiet(name string, args ...string) bool {
	cmd := exec.Command(name, args...)
	return cmd.Run() == nil
}

func runOut(timeout time.Duration, envExtra []string, name string, args ...string) (int, string) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, name, args...)
	if len(envExtra) > 0 {
		cmd.Env = append(os.Environ(), envExtra...)
	}
	var out bytes.Buffer
	cmd.Stdout = &out
	err := cmd.Run()
	if err == nil {
		return 0, out.String()
	}
	if ee, ok := err.(*exec.ExitError); ok {
		return ee.ExitCode(), out.String()
	}
	return 127, out.String()
}

func isInstalled(it catalogItem) bool {
	a := it.Action
	switch {
	case strings.HasPrefix(a, "flatpak:"):
		parts := strings.Split(a, ":")
		app := parts[len(parts)-1]
		return runQuiet("flatpak", "info", "--user", app) || runQuiet("flatpak", "info", app)
	case strings.HasPrefix(a, "gaming:"):
		key := strings.TrimPrefix(a, "gaming:")
		pkg := key
		switch key {
		case "base":
			pkg = "gamescope"
		case "overlay":
			pkg = "goverlay"
		}
		b, err := os.ReadFile("/var/lib/tamagi/layers/gaming.request")
		if err != nil {
			return false
		}
		for _, ln := range strings.Split(string(b), "\n") {
			if strings.TrimSpace(ln) == pkg {
				return true
			}
		}
		return false
	case a == "agent:local":
		return false
	case strings.HasPrefix(a, "agent:"):
		want := strings.TrimPrefix(a, "agent:")
		rc, out := runOut(5*time.Second, nil, "tamagi-agents", "installed")
		if rc != 0 {
			return false
		}
		for _, ln := range strings.Split(out, "\n") {
			if strings.TrimSpace(ln) == want {
				return true
			}
		}
		return false
	case strings.HasPrefix(a, "run:tamagi-browser install "):
		key := strings.Fields(strings.TrimPrefix(a, "run:tamagi-browser install "))[0]
		rc, out := runOut(5*time.Second, nil, "tamagi-browser", "installed")
		if rc != 0 {
			return false
		}
		for _, ln := range strings.Split(out, "\n") {
			if strings.HasPrefix(ln, key+":") {
				return true
			}
		}
		return false
	case strings.HasPrefix(a, "run:tamagi-vm "):
		_, err1 := exec.LookPath("virsh")
		_, err2 := exec.LookPath("cockpit-bridge")
		return err1 == nil && err2 == nil
	case strings.HasPrefix(a, "run:tamagi-windows "):
		cfg := envOr("XDG_CONFIG_HOME", filepath.Join(userHome(), ".config"))
		_, err := os.Stat(filepath.Join(cfg, "containers/systemd/tamagi-windows.container"))
		return err == nil
	case strings.HasPrefix(a, "run:tamagi-devbox create "):
		fields := strings.Fields(a)
		distro := fields[len(fields)-1]
		cname := "dev-" + distro
		if distro == "fedora" {
			cname = "dev"
		}
		return runQuiet("podman", "container", "exists", cname)
	case strings.Contains(a, "tailscale-setup"):
		_, err := exec.LookPath("tailscale")
		return err == nil
	case strings.HasPrefix(a, "run:tamagi-layer add "):
		fields := strings.Fields(a)
		_, err := exec.LookPath(fields[len(fields)-1])
		return err == nil
	case strings.Contains(a, "llm-scripts-setup"):
		st, err := os.Stat(filepath.Join(userHome(), ".local/bin/llm"))
		return err == nil && (st.Mode()&0o111) != 0
	case strings.Contains(a, "vscode-setup"):
		_, err := exec.LookPath("code")
		return err == nil
	case strings.Contains(a, "lazydocker-setup"):
		st, err := os.Stat(filepath.Join(userHome(), ".local/bin/lazydocker"))
		return err == nil && (st.Mode()&0o111) != 0
	case strings.Contains(a, "docker-setup"):
		_, err := exec.LookPath("dockerd")
		return err == nil
	default:
		return false
	}
}

func cmdInstalled(items []catalogItem) {
	for _, it := range items {
		if isInstalled(it) {
			fmt.Printf("%s\t%s\n", it.ID, it.Action)
		}
	}
}

func flathubRemote() {
	if !runQuiet("flatpak", "remote-add", "--user", "--if-not-exists", "flathub", "/etc/flatpak/remotes.d/flathub.flatpakrepo") {
		_ = runQuiet("flatpak", "remote-add", "--user", "--if-not-exists", "flathub", "https://dl.flathub.org/repo/flathub.flatpakrepo")
	}
	_ = runQuiet("flatpak", "remote-add", "--user", "--if-not-exists", "cosmic", "/etc/flatpak/remotes.d/cosmic.flatpakrepo")
}

func runtimeOf(remote, app string) string {
	rc, out := runOut(20*time.Second, []string{"LC_ALL=C"}, "flatpak", "remote-info", "--user", remote, app)
	if rc != 0 {
		return ""
	}
	for _, ln := range strings.Split(out, "\n") {
		trimmed := strings.TrimSpace(ln)
		if strings.HasPrefix(trimmed, "Runtime:") {
			return strings.TrimSpace(strings.TrimPrefix(trimmed, "Runtime:"))
		}
	}
	return ""
}

func runtimeHere(ref string) bool {
	return runQuiet("flatpak", "info", "--user", "runtime/"+ref)
}

func prefetchPIDPath() string { return filepath.Join(runtimeDir(), "tamagi-prefetch.pid") }
func prefetchMarkPath() string {
	return filepath.Join(cacheDir(), "prefetched-runtime")
}

func cmdPrefetch() {
	_ = os.MkdirAll(cacheDir(), 0o755)
	if rc, out := runOut(5*time.Second, nil, "nmcli", "-t", "-f", "GENERAL.METERED", "dev", "show"); rc == 0 && strings.Contains(out, ":yes") {
		return
	}
	pidPath := prefetchPIDPath()
	_ = os.WriteFile(pidPath, []byte(strconv.Itoa(os.Getpid())+"\n"), 0o644)
	defer os.Remove(pidPath)

	flathubRemote()
	ref := runtimeOf("flathub", envOr("TAMAGI_PREFETCH_APP", "org.mozilla.firefox"))
	if ref == "" || runtimeHere(ref) {
		return
	}
	logFile, err := os.Create(filepath.Join(cacheDir(), "prefetch.log"))
	cmd := exec.Command("flatpak", "install", "--user", "-y", "--noninteractive", "flathub", "runtime/"+ref)
	if err == nil {
		cmd.Stdout = logFile
		cmd.Stderr = logFile
	}
	errRun := cmd.Run()
	if logFile != nil {
		logFile.Close()
	}
	if errRun == nil {
		_ = os.WriteFile(prefetchMarkPath(), []byte(ref+"\n"), 0o644)
	}
}

func prefetchWait() {
	b, err := os.ReadFile(prefetchPIDPath())
	if err != nil {
		return
	}
	pid, err := strconv.Atoi(strings.TrimSpace(string(b)))
	if err != nil || pid <= 0 {
		return
	}
	if syscall.Kill(pid, 0) != nil {
		return
	}
	tl("Finishing the download of the shared Flatpak runtime (started while you were choosing)…")
	for syscall.Kill(pid, 0) == nil {
		time.Sleep(1 * time.Second)
	}
}

func cmdPrefetchCleanup() {
	prefetchWait()
	b, err := os.ReadFile(prefetchMarkPath())
	if err != nil {
		return
	}
	ref := strings.TrimSpace(string(b))
	if ref != "" {
		_, out := runOut(10*time.Second, nil, "flatpak", "list", "--user", "--app", "--columns=application")
		if strings.TrimSpace(out) == "" {
			_ = runQuiet("flatpak", "uninstall", "--user", "-y", "--noninteractive", "runtime/"+ref)
		}
	}
	_ = os.Remove(prefetchMarkPath())
}

func runtimesNote(remote string, apps []string) {
	appsOf := make(map[string][]string)
	var order []string
	for _, app := range apps {
		if runQuiet("flatpak", "info", "--user", app) {
			continue
		}
		ref := runtimeOf(remote, app)
		if ref == "" {
			continue
		}
		if _, ok := appsOf[ref]; !ok {
			order = append(order, ref)
		}
		short := app
		if i := strings.LastIndex(app, "."); i >= 0 {
			short = app[i+1:]
		}
		appsOf[ref] = append(appsOf[ref], short)
	}
	if len(order) == 0 {
		return
	}
	fmt.Printf("  %s:\n", t("Shared runtimes these apps use"))
	missing := 0
	for _, ref := range order {
		displayRef := strings.Replace(ref, "/x86_64/", " ", 1)
		appList := strings.Join(appsOf[ref], " ")
		if runtimeHere(ref) {
			fmt.Printf("    %-46s %s  (%s)\n", displayRef, t("already here"), appList)
		} else {
			missing++
			fmt.Printf("    %-46s %s  (%s)\n", displayRef, t("to download"), appList)
		}
	}
	if missing > 1 {
		tl("  More than one runtime to download: each is a large download (about 1 GB).")
	}
}

func flathubSearch(query string) []flathubHit {
	reqBody, _ := json.Marshal(map[string]any{
		"query":         query,
		"hits_per_page": 12,
	})
	client := &http.Client{Timeout: 15 * time.Second}
	resp, err := client.Post("https://flathub.org/api/v2/search", "application/json", bytes.NewReader(reqBody))
	if err == nil {
		defer resp.Body.Close()
		var parsed struct {
			Hits []struct {
				AppID    string `json:"app_id"`
				Name     string `json:"name"`
				Summary  string `json:"summary"`
				Verified bool   `json:"verification_verified"`
			} `json:"hits"`
		}
		if json.NewDecoder(resp.Body).Decode(&parsed) == nil && len(parsed.Hits) > 0 {
			hits := make([]flathubHit, 0, len(parsed.Hits))
			for _, h := range parsed.Hits {
				if h.AppID == "" {
					continue
				}
				hits = append(hits, flathubHit{
					ID:       h.AppID,
					Name:     h.Name,
					Summary:  h.Summary,
					Verified: h.Verified,
				})
			}
			if len(hits) > 0 {
				return hits
			}
		}
	}

	flathubRemote()
	rc, out := runOut(15*time.Second, []string{"LC_ALL=C"}, "flatpak", "search", "--user", "--columns=application,name,description", query)
	if rc != 0 {
		return nil
	}
	var hits []flathubHit
	for _, ln := range strings.Split(out, "\n") {
		cols := strings.Split(ln, "\t")
		if len(cols) >= 2 && strings.Contains(cols[0], ".") {
			summary := ""
			if len(cols) >= 3 {
				summary = cols[2]
			}
			hits = append(hits, flathubHit{
				ID:      cols[0],
				Name:    cols[1],
				Summary: summary,
			})
		}
	}
	return hits
}

func cmdSearch(query string) int {
	if strings.TrimSpace(query) == "" {
		fmt.Fprintln(os.Stderr, "usage: tamagi install search <text>")
		return 2
	}
	hits := flathubSearch(query)
	if len(hits) == 0 {
		fmt.Printf("%s: %s\n", t("Nothing on Flathub for"), query)
		return 1
	}
	for _, h := range hits {
		mark := ""
		if h.Verified {
			mark = " ✓"
		}
		fmt.Printf("  %-44s %s — %s%s\n", h.ID, h.Name, h.Summary, mark)
	}
	fmt.Println()
	tl("✓ = published by the app's own developer. Install with: tamagi install <app id>")
	return 0
}

var appIDRx = regexp.MustCompile(`^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+){2,}$`)

func flathubResolve(want string) (string, int) {
	if os.Getenv("TAMAGI_INSTALL_NO_FLATHUB") == "1" {
		return "", 1
	}
	if appIDRx.MatchString(want) {
		flathubRemote()
		if runQuiet("flatpak", "remote-info", "--user", "flathub", want) {
			return "flatpak:flathub:" + want, 0
		}
	}
	hits := flathubSearch(want)
	if len(hits) == 0 {
		return "", 1
	}
	low := strings.ToLower(want)
	var exactID, exactName string
	labels := make([]string, len(hits))
	for i, h := range hits {
		mark := ""
		if h.Verified {
			mark = " ✓"
		}
		labels[i] = fmt.Sprintf("%s  %s — %s%s", h.ID, h.Name, h.Summary, mark)
		if exactID == "" {
			pick := h.ID
			if idx := strings.LastIndex(pick, "."); idx >= 0 {
				pick = pick[idx+1:]
			}
			if strings.ToLower(h.Name) == low || strings.ToLower(pick) == low || strings.ToLower(h.ID) == low {
				exactID = h.ID
				exactName = h.Name
			}
		}
	}
	if exactID == "" && len(hits) == 1 {
		exactID = hits[0].ID
		exactName = hits[0].Name
	}
	if exactID == "" {
		if isTTY(os.Stdin) && isTTY(os.Stdout) {
			cmd := exec.Command("gum", "choose", "--height", "14", "--header", t("Not in the catalog. On Flathub")+": "+want)
			cmd.Stdin = strings.NewReader(strings.Join(labels, "\n") + "\n")
			cmd.Stderr = os.Stderr
			out, err := cmd.Output()
			if err != nil {
				return "", 2
			}
			pick := strings.TrimSpace(string(out))
			if i := strings.Index(pick, "  "); i >= 0 {
				exactID = pick[:i]
				rest := strings.TrimSpace(pick[i+2:])
				exactName = strings.SplitN(rest, " — ", 2)[0]
			}
		} else {
			fmt.Fprintf(os.Stderr, "%s: %s\n", t("Not in the catalog. On Flathub"), want)
			for _, lbl := range labels {
				fmt.Fprintf(os.Stderr, "  %s\n", lbl)
			}
			fmt.Fprintf(os.Stderr, "%s: tamagi install <app id>\n", t("Install one with"))
			return "", 2
		}
	}
	fmt.Printf("%s: %s (%s)\n", t("From Flathub"), exactName, exactID)
	return "flatpak:flathub:" + exactID, 0
}

func runActions(actions []string) int {
	byRemote := make(map[string][]string)
	var remoteOrder []string
	var flatpakApps, gaming, runs, agents []string

	for _, a := range actions {
		switch {
		case strings.HasPrefix(a, "flatpak:"):
			parts := strings.SplitN(a, ":", 3)
			if len(parts) == 3 {
				r, app := parts[1], parts[2]
				if _, ok := byRemote[r]; !ok {
					remoteOrder = append(remoteOrder, r)
				}
				byRemote[r] = append(byRemote[r], app)
				flatpakApps = append(flatpakApps, app)
			}
		case strings.HasPrefix(a, "gaming:"):
			gaming = append(gaming, strings.TrimPrefix(a, "gaming:"))
		case strings.HasPrefix(a, "agent:"):
			agents = append(agents, strings.TrimPrefix(a, "agent:"))
		case strings.HasPrefix(a, "run:"):
			runs = append(runs, strings.TrimPrefix(a, "run:"))
		}
	}

	total := 0
	if len(remoteOrder) > 0 {
		total++
	}
	if len(gaming) > 0 {
		total++
	}
	total += len(runs) + len(agents)
	n := 0
	step := func(msg string) {
		n++
		fmt.Println()
		banner := fmt.Sprintf("── [%d/%d] %s", n, total, msg)
		if _, err := exec.LookPath("gum"); err == nil {
			c := exec.Command("gum", "style", "--foreground", "4", "--bold", banner)
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if c.Run() == nil {
				return
			}
		}
		fmt.Println(banner)
	}

	var failed []string
	var fpCmd *exec.Cmd
	var fpDone chan error
	fpLogPath := filepath.Join(cacheDir(), "install-flatpak.log")

	installFlatpaksSync := func() bool {
		ok := true
		for _, r := range remoteOrder {
			args := append([]string{"install", "--user", "-y", "--noninteractive", "--or-update", r}, byRemote[r]...)
			c := exec.Command("flatpak", args...)
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if c.Run() != nil {
				ok = false
			}
		}
		return ok
	}

	if len(remoteOrder) > 0 {
		_ = os.MkdirAll(cacheDir(), 0o755)
		flathubRemote()
		step("Flatpak: " + strings.Join(flatpakApps, " "))
		for _, r := range remoteOrder {
			runtimesNote(r, byRemote[r])
		}
		prefetchWait()

		if total > 1 {
			var script strings.Builder
			for _, r := range remoteOrder {
				quoted := make([]string, len(byRemote[r]))
				for i, a := range byRemote[r] {
					quoted[i] = "'" + strings.ReplaceAll(a, "'", "'\\''") + "'"
				}
				script.WriteString(fmt.Sprintf("flatpak install --user -y --noninteractive --or-update '%s' %s || exit 1\n", r, strings.Join(quoted, " ")))
			}
			fpCmd = exec.Command("bash", "-c", script.String())
			if lf, err := os.Create(fpLogPath); err == nil {
				fpCmd.Stdout = lf
				fpCmd.Stderr = lf
			}
			if fpCmd.Start() == nil {
				fpDone = make(chan error, 1)
				go func() { fpDone <- fpCmd.Wait() }()
				tl("  Downloading in the background while the rest is installed.")
			} else {
				failed = append(failed, flatpakApps...)
			}
		} else if !installFlatpaksSync() {
			failed = append(failed, flatpakApps...)
		}
	}

	if len(gaming) > 0 {
		step(t("Native games") + ": " + strings.Join(gaming, " "))
		args := append([]string{"native"}, gaming...)
		c := exec.Command("tamagi-gaming", args...)
		c.Stdin = os.Stdin
		c.Stdout = os.Stdout
		c.Stderr = os.Stderr
		if c.Run() != nil {
			failed = append(failed, "gaming")
		}
	}

	for _, rcmd := range runs {
		step(rcmd)
		parts := strings.Fields(rcmd)
		if len(parts) > 0 {
			c := exec.Command(parts[0], parts[1:]...)
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if c.Run() != nil {
				failed = append(failed, rcmd)
			}
		}
	}

	for _, ag := range agents {
		step(t("AI agent") + ": " + ag)
		c := exec.Command("tamagi-agents", "install", ag)
		c.Stdin = os.Stdin
		c.Stdout = os.Stdout
		c.Stderr = os.Stderr
		if c.Run() != nil {
			failed = append(failed, ag)
		}
	}

	if fpDone != nil {
		select {
		case err := <-fpDone:
			if err != nil {
				failed = append(failed, flatpakApps...)
				printTail(fpLogPath, 5)
			}
		default:
			fmt.Println()
			tl("Waiting for the Flatpaks still downloading…")
			tty := isTTY(os.Stdout)
		waitLoop:
			for {
				select {
				case err := <-fpDone:
					if tty {
						fmt.Printf("\r%-80s\r", "")
					}
					if err != nil {
						failed = append(failed, flatpakApps...)
						printTail(fpLogPath, 5)
					}
					break waitLoop
				case <-time.After(1 * time.Second):
					if tty {
						last := lastLine(fpLogPath)
						if len(last) > 76 {
							last = last[:76]
						}
						fmt.Printf("\r  %-76s", last)
					}
				}
			}
		}
	}

	fmt.Println()
	if len(failed) > 0 {
		fmt.Printf("%s: %s\n", t("Could not install"), strings.Join(failed, " "))
		fmt.Printf("%s: tamagi install\n", t("Try again with"))
		return 1
	}
	tl("Everything installed.")
	return 0
}

func lastLine(path string) string {
	b, err := os.ReadFile(path)
	if err != nil {
		return ""
	}
	s := strings.ReplaceAll(string(b), "\r", "\n")
	lines := strings.Split(strings.TrimSpace(s), "\n")
	if len(lines) == 0 {
		return ""
	}
	return lines[len(lines)-1]
}

func printTail(path string, n int) {
	b, err := os.ReadFile(path)
	if err != nil {
		return
	}
	lines := strings.Split(strings.TrimSpace(string(b)), "\n")
	if len(lines) > n {
		lines = lines[len(lines)-n:]
	}
	for _, ln := range lines {
		fmt.Println(ln)
	}
}

func printHelp() {
	fmt.Print(`tamagi-install — everything optional, in one catalog.

The image is lean. Optional desktop apps, development tools, games and AI
agents are your choice, here:

  tamagi install              choose from the catalog (or search Flathub)
  tamagi install list         the catalog, with what each item is
  tamagi install <id>…        installs by id (e.g.: tamagi install steam codex)
  tamagi install <name>…      not in the catalog? looked up on Flathub (--user)
  tamagi install search <text>   what Flathub has for a text
`)
}

func main() {
	items := loadCatalog()
	args := os.Args[1:]
	if len(args) == 0 {
		if !isTTY(os.Stdin) {
			cmdList(items)
			return
		}
		flathubPrompt := "[Flathub] " + t("Search Flathub online…")
		labels := make([]string, 0, len(items)+1)
		labels = append(labels, flathubPrompt)
		for _, it := range items {
			labels = append(labels, it.label())
		}
		cmd := exec.Command("/usr/libexec/tamagi/gum-multi", "--height", "20",
			"--header", t("Space toggles · Enter installs · nothing selected: installs nothing"))
		cmd.Stdin = strings.NewReader(strings.Join(labels, "\n") + "\n")
		cmd.Stderr = os.Stderr
		out, err := cmd.Output()
		if err != nil {
			return
		}
		chosen := strings.TrimSpace(string(out))
		if chosen == "" {
			tl("Nothing selected.")
			return
		}
		var picked []string
		for _, line := range strings.Split(chosen, "\n") {
			if line == flathubPrompt {
				inCmd := exec.Command("gum", "input", "--placeholder", t("App name or Flathub ID…"))
				inCmd.Stderr = os.Stderr
				if qOut, err := inCmd.Output(); err == nil {
					if q := strings.TrimSpace(string(qOut)); q != "" {
						if act, rc := flathubResolve(q); rc == 0 {
							picked = append(picked, act)
						}
					}
				}
				continue
			}
			for _, it := range items {
				if it.label() == line {
					picked = append(picked, it.Action)
					break
				}
			}
		}
		if len(picked) == 0 {
			tl("Nothing selected.")
			return
		}
		os.Exit(runActions(picked))
	}

	switch args[0] {
	case "list":
		cmdList(items)
	case "search":
		os.Exit(cmdSearch(strings.Join(args[1:], " ")))
	case "ids":
		for _, it := range items {
			fmt.Println(it.ID)
		}
	case "installed":
		cmdInstalled(items)
	case "--actions":
		os.Exit(runActions(args[1:]))
	case "prefetch":
		cmdPrefetch()
	case "prefetch-wait":
		prefetchWait()
	case "prefetch-cleanup":
		cmdPrefetchCleanup()
	case "-h", "--help", "help":
		printHelp()
	default:
		var picked []string
		for _, want := range args {
			found := false
			for _, it := range items {
				if it.ID == want {
					picked = append(picked, it.Action)
					found = true
					break
				}
			}
			if found {
				continue
			}
			act, rc := flathubResolve(want)
			if rc == 0 {
				picked = append(picked, act)
				continue
			}
			if rc == 2 {
				os.Exit(2)
			}
			fmt.Fprintf(os.Stderr, "%s: %s: %s  (tamagi install list · tamagi install search %s)\n",
				t("error"), t("not in the catalog and not on Flathub"), want, want)
			os.Exit(2)
		}
		os.Exit(runActions(picked))
	}
}
