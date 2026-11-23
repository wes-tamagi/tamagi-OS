// tamagi-doctor — autonomous health, integrity, and performance diagnostics.
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"tamagi/internal/i18n"
)

type checkItem struct {
	ID      string `json:"id"`
	Status  string `json:"status"`
	Summary string `json:"summary"`
	Fix     string `json:"fix"`
}

func t(k string) string { return i18n.T(k) }

func userHome() string {
	if h, err := os.UserHomeDir(); err == nil && h != "" {
		return h
	}
	return os.Getenv("HOME")
}

func xdgDataHome() string {
	if d := os.Getenv("XDG_DATA_HOME"); d != "" {
		return d
	}
	return filepath.Join(userHome(), ".local/share")
}

func runOut(timeout time.Duration, name string, args ...string) (int, string, string) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, name, args...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	if err == nil {
		return 0, strings.TrimSpace(stdout.String()), strings.TrimSpace(stderr.String())
	}
	if ee, ok := err.(*exec.ExitError); ok {
		return ee.ExitCode(), strings.TrimSpace(stdout.String()), strings.TrimSpace(stderr.String())
	}
	return 127, strings.TrimSpace(stdout.String()), strings.TrimSpace(stderr.String())
}

func fileExists(p string) bool {
	st, err := os.Stat(p)
	return err == nil && !st.IsDir()
}

func dirExists(p string) bool {
	st, err := os.Stat(p)
	return err == nil && st.IsDir()
}

func checkImage() checkItem {
	if b, err := os.ReadFile("/var/lib/tamagi/quarantine.list"); err == nil {
		qCount := 0
		for _, ln := range strings.Split(string(b), "\n") {
			s := strings.TrimSpace(ln)
			if s != "" && !strings.HasPrefix(s, "#") {
				qCount++
			}
		}
		if qCount > 0 {
			return checkItem{"image", "WARN", fmt.Sprintf("%s: %d", t("Quarantined digest"), qCount), "tamagi-update quarantine"}
		}
	}
	if matches, err := filepath.Glob("/var/lib/tamagi/boot-failures/*.txt"); err == nil && len(matches) > 0 {
		return checkItem{"image", "WARN", fmt.Sprintf("%s: %d", t("Pending boot report"), len(matches)), "tamagi-boot-report show"}
	}
	ver := "Tamagi"
	if b, err := os.ReadFile("/usr/lib/os-release"); err == nil {
		for _, ln := range strings.Split(string(b), "\n") {
			if strings.HasPrefix(ln, "PRETTY_NAME=") {
				ver = strings.Trim(strings.TrimPrefix(ln, "PRETTY_NAME="), "\"")
				break
			}
		}
	}
	return checkItem{"image", "OK", ver, ""}
}

func checkKernel() checkItem {
	var uts syscall.Utsname
	kver := "unknown"
	if err := syscall.Uname(&uts); err == nil {
		var buf []byte
		for _, c := range uts.Release {
			if c == 0 {
				break
			}
			buf = append(buf, byte(c))
		}
		kver = string(buf)
	}
	flav := ""
	if b, err := os.ReadFile("/usr/share/tamagi/kernel-flavor"); err == nil {
		flav = strings.TrimSpace(string(b))
	}
	hasSCX := dirExists("/sys/kernel/sched_ext")
	rc, sb, _ := runOut(5*time.Second, "mokutil", "--sb-state")
	if rc == 0 && strings.Contains(strings.ToLower(sb), "enabled") && flav == "ogc" {
		cert := "/etc/pki/akmods/certs/akmods-ublue.der"
		if fileExists(cert) {
			rc2, _, _ := runOut(5*time.Second, "mokutil", "--test-key", cert)
			if rc2 == 0 {
				return checkItem{"kernel", "WARN", fmt.Sprintf("%s (%s): %s", kver, flav, t("Universal Blue Secure Boot key missing")), "tamagi setup secure-boot-key"}
			}
		}
	}
	detail := kver
	if flav != "" {
		detail += " (" + flav + ")"
	}
	if hasSCX {
		detail += " · sched_ext"
	}
	return checkItem{"kernel", "OK", detail, ""}
}

func checkStorage() checkItem {
	var fs syscall.Statfs_t
	home := userHome()
	if err := syscall.Statfs(home, &fs); err != nil || fs.Blocks == 0 {
		return checkItem{"storage", "WARN", t("could not measure disk space"), ""}
	}
	total := float64(fs.Blocks) * float64(fs.Bsize)
	freeBytes := float64(fs.Bavail) * float64(fs.Bsize)
	freePct := (freeBytes * 100.0) / total
	freeGB := freeBytes / (1024 * 1024 * 1024)

	if freePct < 5 || freeGB < 5 {
		return checkItem{"storage", "FAIL", fmt.Sprintf("%s: %.1f GiB (%.0f%%)", t("critical disk space"), freeGB, freePct), "flatpak uninstall --unused && podman system prune"}
	}
	if freePct < 12 || freeGB < 15 {
		return checkItem{"storage", "WARN", fmt.Sprintf("%s: %.1f GiB (%.0f%%)", t("low disk space"), freeGB, freePct), "flatpak uninstall --unused"}
	}

	uid := uint32(os.Getuid())
	for _, rel := range []string{".local", ".local/state", ".local/share"} {
		d := filepath.Join(home, rel)
		if info, err := os.Stat(d); err == nil {
			if st, ok := info.Sys().(*syscall.Stat_t); ok && st.Uid != uid {
				user := os.Getenv("USER")
				if user == "" {
					user = "$USER"
				}
				return checkItem{"storage", "FAIL", fmt.Sprintf("%s %s", d, t("is owned by another user (root?)")), fmt.Sprintf("sudo chown %s: %s %s", user, filepath.Join(home, ".local"), filepath.Join(home, ".local/state"))}
			}
		}
	}

	if fileExists("/etc/snapper/configs/root") {
		return checkItem{"storage", "WARN", fmt.Sprintf("%.1f GiB · %s", freeGB, t("snapper configured on / (conflicts with bootc)")), "sudo /usr/libexec/tamagi/home-snapshots prune"}
	}

	for _, vmDir := range []string{"/var/lib/libvirt/images", filepath.Join(xdgDataHome(), "libvirt/images")} {
		if dirExists(vmDir) {
			rc, out, _ := runOut(5*time.Second, "lsattr", "-d", vmDir)
			if rc == 0 {
				fields := strings.Fields(out)
				if len(fields) > 0 && !strings.Contains(fields[0], "C") {
					return checkItem{"storage", "WARN", fmt.Sprintf("%.1f GiB · %s", freeGB, t("VM directory missing NoCoW (+C) attribute")), "chattr +C " + vmDir}
				}
			}
		}
	}
	return checkItem{"storage", "OK", fmt.Sprintf("%.1f GiB %s (%.0f%%)", freeGB, t("free"), freePct), ""}
}

func memTotalKB() int64 {
	b, err := os.ReadFile("/proc/meminfo")
	if err != nil {
		return 0
	}
	for _, ln := range strings.Split(string(b), "\n") {
		if strings.HasPrefix(ln, "MemTotal:") {
			fields := strings.Fields(ln)
			if len(fields) >= 2 {
				v, _ := strconv.ParseInt(fields[1], 10, 64)
				return v
			}
		}
	}
	return 0
}

func checkPerformance() checkItem {
	var issues []string
	if !fileExists("/sys/block/zram0/disksize") {
		issues = append(issues, t("no zram0"))
	}
	rc, db, _ := runOut(5*time.Second, "sysctl", "-n", "vm.dirty_bytes")
	if rc == 0 && db == "0" && memTotalKB() >= 16*1024*1024 {
		issues = append(issues, "vm.dirty_bytes=0")
	}
	if len(issues) > 0 {
		return checkItem{"performance", "WARN", strings.Join(issues, ", "), "tamagi-perf status"}
	}
	_, cc, _ := runOut(5*time.Second, "sysctl", "-n", "net.ipv4.tcp_congestion_control")
	if cc == "" {
		cc = "-"
	}
	return checkItem{"performance", "OK", "zram · TCP " + cc, ""}
}

func checkSystemd() checkItem {
	rc1, sysF, _ := runOut(5*time.Second, "systemctl", "--failed", "--no-legend", "--plain")
	rc2, usrF, _ := runOut(5*time.Second, "systemctl", "--user", "--failed", "--no-legend", "--plain")
	if rc1 != 0 && rc2 != 0 {
		return checkItem{"systemd", "OK", t("systemd unavailable in this environment"), ""}
	}
	var failed []string
	for _, ln := range strings.Split(sysF+"\n"+usrF, "\n") {
		fields := strings.Fields(ln)
		if len(fields) > 0 {
			failed = append(failed, fields[0])
		}
	}
	if len(failed) > 0 {
		if len(failed) > 4 {
			failed = failed[:4]
		}
		return checkItem{"systemd", "WARN", fmt.Sprintf("%s: %s", t("failed units"), strings.Join(failed, ", ")), "systemctl --failed && systemctl --user --failed"}
	}
	return checkItem{"systemd", "OK", t("no failed units"), ""}
}

func checkLayers() checkItem {
	rc, out, _ := runOut(5*time.Second, "tamagi-layer", "stale")
	var stale []string
	if rc == 0 {
		for _, ln := range strings.Split(out, "\n") {
			if s := strings.TrimSpace(ln); s != "" {
				stale = append(stale, s)
			}
		}
	}
	if len(stale) > 0 {
		return checkItem{"layers", "WARN", fmt.Sprintf("%s: %s", t("stale layers"), strings.Join(stale, ", ")), "tamagi-layer rebuild"}
	}
	return checkItem{"layers", "OK", t("layers up to date"), ""}
}

func checkContainers() checkItem {
	rc, out, _ := runOut(5*time.Second, "podman", "ps", "-a", "--filter", "label=manager=distrobox", "--format", "{{.Names}}")
	if rc != 0 {
		return checkItem{"containers", "OK", t("podman unavailable in this environment"), ""}
	}
	var boxes []string
	for _, ln := range strings.Split(out, "\n") {
		if s := strings.TrimSpace(ln); s != "" {
			boxes = append(boxes, s)
		}
	}
	summary := fmt.Sprintf("%d %s", len(boxes), t("distrobox containers"))
	if len(boxes) > 0 {
		show := boxes
		if len(show) > 5 {
			show = show[:5]
		}
		summary += " (" + strings.Join(show, ", ") + ")"
	}
	return checkItem{"containers", "OK", summary, ""}
}

func checkGPU() checkItem {
	matches, _ := filepath.Glob("/dev/dri/renderD*")
	sort.Strings(matches)
	if len(matches) == 0 {
		return checkItem{"gpu", "WARN", t("no DRM render node found"), ""}
	}
	names := make([]string, len(matches))
	for i, m := range matches {
		names[i] = filepath.Base(m)
	}
	return checkItem{"gpu", "OK", fmt.Sprintf("%d GPU DRM (%s)", len(names), strings.Join(names, ", ")), ""}
}

func checkKVM() checkItem {
	f, err := os.OpenFile("/dev/kvm", os.O_RDWR, 0)
	if err != nil {
		if os.IsNotExist(err) {
			return checkItem{"virtualization", "WARN", t("/dev/kvm missing (virtualization disabled in BIOS?)"), ""}
		}
		return checkItem{"virtualization", "WARN", t("no read/write permission on /dev/kvm"), ""}
	}
	f.Close()
	if _, err := exec.LookPath("virsh"); err == nil {
		return checkItem{"virtualization", "OK", "/dev/kvm · libvirt · Cockpit Machines OK", ""}
	}
	return checkItem{"virtualization", "OK", "/dev/kvm OK", ""}
}

func checkSecurity() checkItem {
	rc, enf, _ := runOut(5*time.Second, "getenforce")
	if rc == 0 && strings.ToLower(enf) != "enforcing" {
		return checkItem{"security", "WARN", "SELinux: " + enf, "sudo setenforce 1"}
	}
	mode := t("balanced")
	if fileExists("/etc/sysctl.d/90-tamagi-strict.conf") {
		mode = t("strict")
	}
	if enf == "" {
		enf = "-"
	}
	return checkItem{"security", "OK", fmt.Sprintf("SELinux %s · %s %s", enf, t("mode"), mode), ""}
}

func checkDesktop() checkItem {
	apps := filepath.Join(xdgDataHome(), "applications")
	var bad []string
	if _, err := exec.LookPath("desktop-file-validate"); err == nil && dirExists(apps) {
		entries, _ := filepath.Glob(filepath.Join(apps, "*.desktop"))
		sort.Strings(entries)
		for _, f := range entries {
			if rc, _, _ := runOut(5*time.Second, "desktop-file-validate", f); rc != 0 {
				bad = append(bad, filepath.Base(f))
			}
		}
	}
	if len(bad) > 0 {
		show := bad
		if len(show) > 3 {
			show = show[:3]
		}
		return checkItem{"desktop", "WARN", fmt.Sprintf("%s: %s", t("invalid .desktop entries"), strings.Join(show, ", ")), fmt.Sprintf("desktop-file-validate %s/%s", apps, bad[0])}
	}
	if _, err := exec.LookPath("agy"); err == nil || fileExists(filepath.Join(userHome(), ".local/bin/agy")) {
		if !fileExists("/usr/share/applications/agy.desktop") && !fileExists(filepath.Join(apps, "agy.desktop")) {
			return checkItem{"desktop", "WARN", t("agy is installed but missing from the menu"), "tamagi-doctor --fix"}
		}
	}
	return checkItem{"desktop", "OK", t("menu and .desktop entries are valid"), ""}
}

func checkAI() checkItem {
	if !fileExists("/usr/lib64/ggml/libggml-vulkan.so") {
		return checkItem{"ai", "WARN", t("llama.cpp Vulkan backend missing"), ""}
	}
	rc, state, _ := runOut(5*time.Second, "systemctl", "--user", "is-active", "llama-server.service")
	if rc != 0 || state == "" {
		state = t("stopped")
	}
	return checkItem{"ai", "OK", "Vulkan · llama-server: " + state, ""}
}

func runSafeFixes() []string {
	var actions []string
	apps := filepath.Join(xdgDataHome(), "applications")
	if dirExists(apps) {
		if _, err := exec.LookPath("update-desktop-database"); err == nil {
			runOut(5*time.Second, "update-desktop-database", apps)
			actions = append(actions, "update-desktop-database")
		}
	}
	if _, err := exec.LookPath("agy"); err == nil || fileExists(filepath.Join(userHome(), ".local/bin/agy")) {
		if !fileExists("/usr/share/applications/agy.desktop") && !fileExists(filepath.Join(apps, "agy.desktop")) {
			if err := os.MkdirAll(apps, 0o755); err == nil {
				desktop := "[Desktop Entry]\nType=Application\nName=Antigravity (agy)\nTryExec=agy\n" +
					"Exec=/usr/libexec/tamagi/popup --class agy --title \"Antigravity (agy)\" agy\n" +
					"StartupWMClass=agy\nIcon=utilities-terminal\nTerminal=false\nCategories=Development;\n" +
					"Keywords=agy;antigravity;google;gemini;ia;ai;agent;\n"
				if os.WriteFile(filepath.Join(apps, "agy.desktop"), []byte(desktop), 0o644) == nil {
					runOut(5*time.Second, "update-desktop-database", apps)
					actions = append(actions, "agy.desktop")
				}
			}
		}
	}
	if rc, usrF, _ := runOut(5*time.Second, "systemctl", "--user", "--failed", "--no-legend", "--plain"); rc == 0 && strings.TrimSpace(usrF) != "" {
		runOut(5*time.Second, "systemctl", "--user", "reset-failed")
		actions = append(actions, "systemctl --user reset-failed")
	}
	return actions
}

func explainWithAI(results []checkItem) string {
	var problems []checkItem
	for _, r := range results {
		if r.Status != "OK" {
			problems = append(problems, r)
		}
	}
	if len(problems) == 0 {
		return t("No issues found to explain.")
	}
	langNames := map[string]string{"pt": "português", "en": "English", "es": "español", "fr": "français"}
	langName := langNames[i18n.DetectLang()]
	if langName == "" {
		langName = "English"
	}
	probJSON, _ := json.Marshal(problems)
	reqBody, _ := json.Marshal(map[string]any{
		"messages": []map[string]string{
			{"role": "system", "content": fmt.Sprintf("You are a concise Linux systems architect. Reply in %s in at most 4 short bullet points explaining the root cause and exact next step based ONLY on the JSON diagnostics.", langName)},
			{"role": "user", "content": string(probJSON)},
		},
		"temperature": 0.2,
		"max_tokens":  300,
	})
	client := &http.Client{Timeout: 20 * time.Second}
	resp, err := client.Post("http://127.0.0.1:9931/v1/chat/completions", "application/json", bytes.NewReader(reqBody))
	if err != nil {
		return t("Local AI unavailable (start with: tamagi-ai start).")
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return t("Local AI unavailable (start with: tamagi-ai start).")
	}
	var parsed struct {
		Choices []struct {
			Message struct {
				Content string `json:"content"`
			} `json:"message"`
		} `json:"choices"`
	}
	if json.Unmarshal(body, &parsed) == nil && len(parsed.Choices) > 0 {
		return strings.TrimSpace(parsed.Choices[0].Message.Content)
	}
	return t("Local AI unavailable (start with: tamagi-ai start).")
}

func runAllChecks() []checkItem {
	fns := []func() checkItem{
		checkImage, checkKernel, checkStorage, checkPerformance, checkSystemd,
		checkLayers, checkContainers, checkGPU, checkKVM, checkSecurity,
		checkDesktop, checkAI,
	}
	results := make([]checkItem, len(fns))
	var wg sync.WaitGroup
	for i, fn := range fns {
		wg.Add(1)
		go func(idx int, f func() checkItem) {
			defer wg.Done()
			results[idx] = f()
		}(i, fn)
	}
	wg.Wait()
	return results
}

func isTTY() bool {
	fi, err := os.Stdout.Stat()
	return err == nil && (fi.Mode()&os.ModeCharDevice) != 0
}

func main() {
	var jsonOut, doFix, doExplain bool
	fs := flag.NewFlagSet("tamagi-doctor", flag.ExitOnError)
	fs.SetOutput(os.Stdout)
	fs.BoolVar(&jsonOut, "json", false, t("structured JSON output"))
	fs.BoolVar(&doFix, "fix", false, t("apply safe user-level fixes"))
	fs.BoolVar(&doExplain, "explain", false, t("ask local AI (llama.cpp) for an explanation"))
	fs.Usage = func() {
		fmt.Fprintf(os.Stdout, "tamagi-doctor — %s\n\n", t("Autonomous health, integrity, and performance diagnostics for Tamagi"))
		fs.PrintDefaults()
	}
	_ = fs.Parse(os.Args[1:])

	fixed := []string{}
	if doFix {
		fixed = runSafeFixes()
	}
	results := runAllChecks()

	if jsonOut {
		enc := json.NewEncoder(os.Stdout)
		enc.SetEscapeHTML(false)
		enc.SetIndent("", "  ")
		_ = enc.Encode(map[string]any{"checks": results, "fixed": fixed})
		return
	}

	if len(fixed) > 0 {
		fmt.Printf("%s: %s\n\n", t("Applied fixes"), strings.Join(fixed, ", "))
	}

	okN, warnN, failN := 0, 0, 0
	colors := map[string]string{"OK": "\033[32m", "WARN": "\033[33m", "FAIL": "\033[31m"}
	tty := isTTY()
	for _, r := range results {
		switch r.Status {
		case "OK":
			okN++
		case "WARN":
			warnN++
		case "FAIL":
			failN++
		}
		tag := fmt.Sprintf("%-4s", r.Status)
		if tty {
			tag = colors[r.Status] + tag + "\033[0m"
		}
		fmt.Printf("  [%s] %-15s %s\n", tag, r.ID, r.Summary)
		if r.Fix != "" {
			fmt.Printf("           → %s\n", r.Fix)
		}
	}

	fmt.Printf("\n%s: %d OK · %d WARN · %d FAIL\n", t("Summary"), okN, warnN, failN)
	if doExplain {
		fmt.Printf("\n── %s\n", t("Local AI analysis"))
		fmt.Println(explainWithAI(results))
	}
	if failN > 0 {
		os.Exit(1)
	}
}
