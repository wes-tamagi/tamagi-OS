//! Shared modern CLI UI helpers (between `dnf5` and `pacman`):
//! - `pacman`-style `::` phase headers, `[1/N]` counters, and compact output
//! - `dnf5`-style aligned transaction tables and semantic colors
//! - TTY-aware Braille spinner and animated progress bar (disabled when piped or `NO_COLOR`)
#![allow(dead_code)]

use std::env;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn is_tty() -> bool {
    io::stdout().is_terminal()
}

pub fn use_color() -> bool {
    is_tty() && env::var_os("NO_COLOR").is_none()
}

pub fn paint(code: &str, text: &str) -> String {
    if use_color() && !code.is_empty() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn bold(s: &str) -> String {
    paint("1", s)
}

pub fn dim(s: &str) -> String {
    paint("2", s)
}

pub fn cyan(s: &str) -> String {
    paint("1;36", s)
}

pub fn green(s: &str) -> String {
    paint("1;32", s)
}

pub fn yellow(s: &str) -> String {
    paint("1;33", s)
}

pub fn red(s: &str) -> String {
    paint("1;31", s)
}

/// `pacman`-style phase header: `:: Synchronizing package databases...`
pub fn phase(msg: &str) {
    println!("{} {}", cyan("::"), bold(msg));
}

/// Compact step counter: `  [1/3] Installing...`
pub fn step_progress(idx: usize, total: usize, msg: &str) {
    let tag = format!("[{idx}/{total}]");
    println!("  {} {}", cyan(&tag), msg);
}

/// `dnf5`-style transaction table row with aligned columns.
pub fn print_tx_table(title: &str, rows: &[(String, String, String, String)]) {
    if rows.is_empty() {
        return;
    }
    phase(title);
    let w0 = rows.iter().map(|r| r.0.len()).max().unwrap_or(8).clamp(8, 28);
    let w1 = rows.iter().map(|r| r.1.len()).max().unwrap_or(6).clamp(6, 16);
    let w2 = rows.iter().map(|r| r.2.len()).max().unwrap_or(4).clamp(4, 10);
    let header = format!("  {:<w0$}  {:<w1$}  {:>w2$}  {}", "Package", "Source", "Size", "Action");
    println!("{}", dim(&header));
    let sep = format!("  {}", "─".repeat(w0 + w1 + w2 + 16));
    println!("{}", dim(&sep));
    for (pkg, src, size, act) in rows {
        let colored_act = match act.as_str() {
            "install" | "add" => green(act),
            "remove" | "rm" => red(act),
            "update" | "upgrade" => yellow(act),
            _ => cyan(act),
        };
        println!("  {:<w0$}  {:<w1$}  {:>w2$}  {}", bold(pkg), dim(src), size, colored_act);
    }
    println!();
}

/// Render a compact progress bar `[######----] 60%`.
pub fn progress_bar(done: usize, total: usize, width: usize) -> String {
    let pct = if total == 0 { 100 } else { (done.min(total) * 100) / total };
    let filled = (pct * width) / 100;
    let empty = width.saturating_sub(filled);
    let bar = format!("{}{}", "━".repeat(filled), "─".repeat(empty));
    if use_color() {
        format!("[{}{}] {:>3}%", paint("36", &"━".repeat(filled)), dim(&"─".repeat(empty)), pct)
    } else {
        format!("[{bar}] {:>3}%", pct)
    }
}

/// Animated Braille spinner for long operations in interactive terminals.
pub struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    started: Instant,
    label: String,
    active: bool,
}

impl Spinner {
    pub fn start(label: impl Into<String>) -> Self {
        let label = label.into();
        let started = Instant::now();
        if !is_tty() {
            return Self {
                stop: Arc::new(AtomicBool::new(true)),
                handle: None,
                started,
                label,
                active: false,
            };
        }
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = Arc::clone(&stop);
        let text = label.clone();
        let colored = use_color();
        let handle = thread::spawn(move || {
            let mut i = 0usize;
            while !stop_flag.load(Ordering::Relaxed) {
                let frame = FRAMES[i % FRAMES.len()];
                let head = if colored { format!("\x1b[1;36m{frame}\x1b[0m") } else { frame.to_string() };
                print!("\r  {head} {text}   ");
                let _ = io::stdout().flush();
                i = i.wrapping_add(1);
                thread::sleep(Duration::from_millis(80));
            }
        });
        Self { stop, handle: Some(handle), started, label, active: true }
    }

    pub fn finish_ok(mut self, detail: Option<&str>) {
        self.stop_inner();
        let secs = self.started.elapsed().as_secs_f32();
        let extra = detail.map(|d| format!(" · {d}")).unwrap_or_default();
        println!("  {} {}{} {}", green("✓"), self.label, extra, dim(&format!("({secs:.1}s)")));
    }

    pub fn finish_err(mut self, detail: Option<&str>) {
        self.stop_inner();
        let extra = detail.map(|d| format!(" · {d}")).unwrap_or_default();
        println!("  {} {}{}", red("✗"), self.label, extra);
    }

    fn stop_inner(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        if self.active {
            print!("\r{:80}\r", "");
            let _ = io::stdout().flush();
            self.active = false;
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_bounds() {
        let b0 = progress_bar(0, 10, 10);
        let b5 = progress_bar(5, 10, 10);
        let b10 = progress_bar(10, 10, 10);
        assert!(b0.contains("  0%"));
        assert!(b5.contains(" 50%"));
        assert!(b10.contains("100%"));
    }
}
