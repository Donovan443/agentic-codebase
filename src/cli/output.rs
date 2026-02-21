//! Output formatting and styling helpers.
//!
//! Provides colored output, progress indicators, and human-friendly formatting.
//! Respects the `NO_COLOR` environment variable and TTY detection so that
//! output is always readable when piped, captured, or run in CI.

use std::io::Write;

// ---------------------------------------------------------------------------
// TTY / color detection
// ---------------------------------------------------------------------------

/// Returns `true` when stdout is connected to an interactive terminal.
fn atty_stdout() -> bool {
    unsafe { libc_isatty(1) != 0 }
}

extern "C" {
    #[link_name = "isatty"]
    fn libc_isatty(fd: i32) -> i32;
}

/// Returns `true` when colored output should be used.
///
/// Color is enabled when ALL of the following are true:
/// - `NO_COLOR` environment variable is **not** set
/// - `ACB_NO_COLOR` environment variable is **not** set
/// - Stdout is a TTY (interactive terminal)
pub fn color_enabled() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if std::env::var("ACB_NO_COLOR").is_ok() {
        return false;
    }
    atty_stdout()
}

// ---------------------------------------------------------------------------
// Styled output helper
// ---------------------------------------------------------------------------

/// A thin helper that conditionally applies ANSI escape codes.
///
/// Create with [`Styled::auto()`] to respect environment / TTY, or
/// force a mode with [`Styled::plain()`] / [`Styled::colored()`].
#[derive(Clone, Copy)]
pub struct Styled {
    use_color: bool,
}

impl Styled {
    /// Auto-detect whether to use color based on environment and TTY.
    pub fn auto() -> Self {
        Self {
            use_color: color_enabled(),
        }
    }

    /// Force plain text output (no colors).
    pub fn plain() -> Self {
        Self { use_color: false }
    }

    /// Force colored output regardless of environment.
    #[allow(dead_code)]
    pub fn colored() -> Self {
        Self { use_color: true }
    }

    // -- Symbols --------------------------------------------------------

    /// Green check mark or "OK".
    pub fn ok(&self) -> &str {
        if self.use_color {
            "\x1b[32m\u{2713}\x1b[0m"
        } else {
            "OK"
        }
    }

    /// Red cross or "FAIL".
    pub fn fail(&self) -> &str {
        if self.use_color {
            "\x1b[31m\u{2717}\x1b[0m"
        } else {
            "FAIL"
        }
    }

    /// Yellow warning symbol or "!!".
    pub fn warn(&self) -> &str {
        if self.use_color {
            "\x1b[33m\u{26A0}\x1b[0m"
        } else {
            "!!"
        }
    }

    /// Blue info dot or "->".
    pub fn info(&self) -> &str {
        if self.use_color {
            "\x1b[34m\u{25CF}\x1b[0m"
        } else {
            "->"
        }
    }

    /// Dimmed right arrow or "  ".
    pub fn arrow(&self) -> &str {
        if self.use_color {
            "\x1b[90m\u{2192}\x1b[0m"
        } else {
            "->"
        }
    }

    // -- Text coloring --------------------------------------------------

    /// Bold text.
    pub fn bold(&self, text: &str) -> String {
        if self.use_color {
            format!("\x1b[1m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }

    /// Green text (for success values).
    pub fn green(&self, text: &str) -> String {
        if self.use_color {
            format!("\x1b[32m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }

    /// Yellow text (for warnings / numbers).
    pub fn yellow(&self, text: &str) -> String {
        if self.use_color {
            format!("\x1b[33m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }

    /// Red text (for errors).
    pub fn red(&self, text: &str) -> String {
        if self.use_color {
            format!("\x1b[31m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }

    /// Cyan text (for paths and identifiers).
    pub fn cyan(&self, text: &str) -> String {
        if self.use_color {
            format!("\x1b[36m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }

    /// Dim / grey text (for secondary information).
    pub fn dim(&self, text: &str) -> String {
        if self.use_color {
            format!("\x1b[90m{}\x1b[0m", text)
        } else {
            text.to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

/// Format a byte count into a human-readable string (e.g., "4.2 MB").
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Print a simple inline progress indicator to stderr.
///
/// Call repeatedly with increasing `current` values; each call
/// overwrites the previous line. Call [`progress_done`] when finished.
pub fn progress(label: &str, current: usize, total: usize) {
    if total == 0 || !color_enabled() {
        return;
    }
    let pct = (current as f64 / total as f64 * 100.0).min(100.0);
    let bar_width = 20;
    let filled = (pct / 100.0 * bar_width as f64) as usize;
    let empty = bar_width - filled;
    eprint!(
        "\r  {} [{}{}] {:>3.0}%",
        label,
        "\u{2588}".repeat(filled),
        "\u{2591}".repeat(empty),
        pct,
    );
    let _ = std::io::stderr().flush();
}

/// Finish a progress indicator with a newline on stderr.
pub fn progress_done() {
    if color_enabled() {
        eprintln!();
    }
}
