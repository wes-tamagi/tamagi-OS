//! Reads the output of the installers: the stage lines (`── [2/5] title`),
//! and the overall progress across the commands of the plan.

use crate::plan::{Stages, Step};

/// The text without ANSI escape sequences (gum colours the stage lines) and
/// without other control characters.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                // CSI: ESC [ parameters, ended by a byte in @..~
                Some('[') => {
                    chars.next();
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                // OSC: ESC ] text, ended by BEL or ESC \
                Some(']') => {
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' {
                            break;
                        }
                        if c == '\u{1b}' {
                            chars.next();
                            break;
                        }
                    }
                }
                // Two-character sequences (ESC ( B, ESC =, ...)
                Some('(') | Some(')') => {
                    chars.next();
                    chars.next();
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else if c == '\t' {
            out.push(' ');
        } else if !c.is_control() {
            out.push(c);
        }
    }
    out
}

/// What a line of output means for the progress.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// `── [n/total] title`
    Stage { n: u32, total: u32, title: String },
    /// `── title`: a stage without a number (tamagi apply)
    Heading(String),
    /// Anything else: goes to the log.
    Text(String),
    /// Nothing worth showing.
    Blank,
}

/// Classifies one line of output (colours and padding removed first).
pub fn parse_line(raw: &str) -> Line {
    let clean = strip_ansi(raw);
    let line = clean.trim();
    if line.is_empty() {
        return Line::Blank;
    }
    let Some(rest) = line.strip_prefix("──") else {
        return Line::Text(clean.trim_end().to_string());
    };
    let rest = rest.trim_start_matches('─').trim();
    if let Some(inner) = rest.strip_prefix('[') {
        if let Some((counter, title)) = inner.split_once(']') {
            if let Some((n, total)) = counter.split_once('/') {
                if let (Ok(n), Ok(total)) = (n.trim().parse::<u32>(), total.trim().parse::<u32>()) {
                    if n >= 1 && total >= 1 {
                        return Line::Stage { n: n.min(total), total, title: title.trim().to_string() };
                    }
                }
            }
        }
    }
    if rest.is_empty() {
        return Line::Blank;
    }
    Line::Heading(rest.to_string())
}

/// Splits a chunk of output into lines: `\n` ends a line and `\r` (used by
/// progress meters to redraw in place) does too.
pub fn split_lines(buffer: &mut String, chunk: &str) -> Vec<String> {
    buffer.push_str(chunk);
    let mut lines = Vec::new();
    while let Some(pos) = buffer.find(['\n', '\r']) {
        let line: String = buffer.drain(..=pos).collect();
        lines.push(line.trim_end_matches(['\n', '\r']).to_string());
    }
    lines
}

/// Follows the progress of the whole plan, one command after another.
#[derive(Debug, Clone)]
pub struct Tracker {
    weights: Vec<u32>,
    kinds: Vec<Stages>,
    /// The command running (index in the plan), once one has started.
    current: Option<usize>,
    /// Stages finished inside the current command.
    done_in_current: f32,
    /// Stages the current command has (known or estimated).
    total_in_current: f32,
    /// Stage lines seen in the current command (for `Stages::Counted`).
    seen: u32,
    finished: bool,
}

impl Tracker {
    pub fn new(steps: &[Step]) -> Self {
        Self {
            weights: steps.iter().map(|s| s.weight).collect(),
            kinds: steps.iter().map(|s| s.stages).collect(),
            current: None,
            done_in_current: 0.0,
            total_in_current: 1.0,
            seen: 0,
            finished: false,
        }
    }

    /// A command of the plan started.
    pub fn start(&mut self, index: usize) {
        self.current = Some(index);
        self.done_in_current = 0.0;
        self.total_in_current = self.weights.get(index).copied().unwrap_or(1).max(1) as f32;
        self.seen = 0;
    }

    /// A line of the current command. Returns the stage title to show, when
    /// the line starts a stage.
    pub fn line(&mut self, line: &Line) -> Option<String> {
        let kind = self.current.and_then(|i| self.kinds.get(i)).copied()?;
        match (line, kind) {
            (Line::Stage { n, total, title }, _) => {
                self.total_in_current = *total as f32;
                self.done_in_current = (*n - 1) as f32;
                Some(format!("[{n}/{total}] {title}"))
            }
            (Line::Heading(title), Stages::Counted) => {
                self.seen += 1;
                // More stages than expected: the estimate grows with them.
                if self.seen as f32 > self.total_in_current {
                    self.total_in_current = self.seen as f32;
                }
                self.done_in_current = (self.seen - 1) as f32;
                Some(title.clone())
            }
            (Line::Heading(title), _) => Some(title.clone()),
            _ => None,
        }
    }

    /// Everything ran.
    pub fn finish(&mut self) {
        self.finished = true;
    }

    /// Overall progress, 0.0 to 1.0. The stage running counts as half done,
    /// so the bar moves as soon as something starts.
    pub fn fraction(&self) -> f32 {
        if self.finished {
            return 1.0;
        }
        let total: u32 = self.weights.iter().sum();
        let Some(current) = self.current else {
            return 0.0;
        };
        if total == 0 {
            return 0.0;
        }
        let before: u32 = self.weights.iter().take(current).sum();
        let weight = self.weights.get(current).copied().unwrap_or(0) as f32;
        let inside = ((self.done_in_current + 0.5) / self.total_in_current.max(1.0)).clamp(0.0, 1.0);
        ((before as f32 + weight * inside) / total as f32).clamp(0.0, 0.99)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{steps, Choices};

    #[test]
    fn ansi() {
        assert_eq!(strip_ansi("\u{1b}[1;34m── [2/5] Flatpak: a b\u{1b}[0m"), "── [2/5] Flatpak: a b");
        assert_eq!(strip_ansi("\u{1b}[38;5;4mx\u{1b}[m y"), "x y");
        assert_eq!(strip_ansi("\u{1b}]0;title\u{7}text"), "text");
        assert_eq!(strip_ansi("\u{1b}(Bplain"), "plain");
        assert_eq!(strip_ansi("a\tb\u{8}"), "a b");
        assert_eq!(strip_ansi("ação — ok"), "ação — ok");
        assert_eq!(strip_ansi("cut\u{1b}"), "cut");
    }

    #[test]
    fn stage_lines() {
        assert_eq!(parse_line("── [2/5] Flatpak: org.gimp.GIMP"), Line::Stage { n: 2, total: 5, title: "Flatpak: org.gimp.GIMP".into() });
        // As gum prints it: colour codes and padding.
        assert_eq!(parse_line("\u{1b}[1;34m── [1/3] Native games: steam \u{1b}[0m  "), Line::Stage { n: 1, total: 3, title: "Native games: steam".into() });
        assert_eq!(parse_line("  ── [10/12] AI agent: claude"), Line::Stage { n: 10, total: 12, title: "AI agent: claude".into() });
        // A counter past the total is clamped.
        assert_eq!(parse_line("── [7/5] x"), Line::Stage { n: 5, total: 5, title: "x".into() });
    }

    #[test]
    fn headings_and_text() {
        assert_eq!(parse_line("── Flatpak: dev.zed.Zed"), Line::Heading("Flatpak: dev.zed.Zed".into()));
        assert_eq!(parse_line("── [x/y] odd"), Line::Heading("[x/y] odd".into()));
        assert_eq!(parse_line("── [0/0] odd"), Line::Heading("[0/0] odd".into()));
        assert_eq!(parse_line("Everything installed."), Line::Text("Everything installed.".into()));
        assert_eq!(parse_line("  indented text  "), Line::Text("  indented text".into()));
        assert_eq!(parse_line(""), Line::Blank);
        assert_eq!(parse_line("\u{1b}[0m   "), Line::Blank);
        assert_eq!(parse_line("────"), Line::Blank);
    }

    #[test]
    fn lines_from_chunks() {
        let mut buffer = String::new();
        assert_eq!(split_lines(&mut buffer, "one\ntw"), ["one"]);
        assert_eq!(split_lines(&mut buffer, "o\r\n 10%\r 20%\rthree"), ["two", "", " 10%", " 20%"]);
        assert_eq!(buffer, "three");
        assert_eq!(split_lines(&mut buffer, "\n"), ["three"]);
        assert!(buffer.is_empty());
    }

    fn plan() -> Vec<Step> {
        let choices = Choices {
            profile: Some("ai".into()),
            browser: Some(("firefox".into(), "flatpak".into())),
            picked: vec!["agent:claude".into()],
            tpm: false,
        };
        // tamagi-install (2 stages), prefetch-wait (0), tamagi apply (2 items)
        steps(&choices, 2)
    }

    #[test]
    fn progress_grows_and_ends_at_one() {
        let mut t = Tracker::new(&plan());
        assert_eq!(t.fraction(), 0.0);
        let mut last = 0.0;
        let mut check = |t: &Tracker| {
            let now = t.fraction();
            assert!(now >= last, "{now} < {last}");
            assert!(now < 1.0);
            last = now;
        };
        t.start(0);
        check(&t);
        assert_eq!(t.line(&parse_line("── [1/2] tamagi-browser install firefox --flatpak")).as_deref(), Some("[1/2] tamagi-browser install firefox --flatpak"));
        check(&t);
        assert_eq!(t.line(&parse_line("some output")), None);
        t.line(&parse_line("── [2/2] AI agent: claude"));
        check(&t);
        // 2 of 4 weights, the second stage half done: (1.5/2 * 2) / 4
        assert!((t.fraction() - 0.375).abs() < 1e-6);
        t.start(1);
        check(&t);
        t.start(2);
        check(&t);
        assert_eq!(t.line(&parse_line("── agent: claude")).as_deref(), Some("agent: claude"));
        check(&t);
        t.line(&parse_line("── box: dev"));
        check(&t);
        // A third, unexpected stage does not push the bar backwards or past the end.
        t.line(&parse_line("── services: autoupdate"));
        let now = t.fraction();
        assert!(now < 1.0);
        t.finish();
        assert_eq!(t.fraction(), 1.0);
    }

    #[test]
    fn numbered_total_replaces_the_estimate() {
        let mut t = Tracker::new(&plan());
        t.start(0);
        // The plan expected 2 stages; the installer says 4.
        t.line(&parse_line("── [1/4] a"));
        assert!((t.fraction() - (0.5 / 4.0 * 2.0) / 4.0).abs() < 1e-6);
    }

    #[test]
    fn lines_before_any_command_are_ignored() {
        let mut t = Tracker::new(&plan());
        assert_eq!(t.line(&parse_line("── [1/2] x")), None);
        assert_eq!(t.fraction(), 0.0);
    }

    #[test]
    fn empty_plan() {
        let mut t = Tracker::new(&[]);
        assert_eq!(t.fraction(), 0.0);
        t.finish();
        assert_eq!(t.fraction(), 1.0);
    }
}
