//! Source location types for tracking where code units appear in source files.

use serde::{Deserialize, Serialize};

/// A location range in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// Starting line (1-indexed).
    pub start_line: u32,
    /// Starting column (0-indexed, byte offset).
    pub start_col: u32,
    /// Ending line (1-indexed).
    pub end_line: u32,
    /// Ending column (0-indexed, byte offset).
    pub end_col: u32,
}

impl Span {
    /// Creates a new span from start and end positions.
    pub fn new(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Creates a single-point span (zero-width).
    pub fn point(line: u32, col: u32) -> Self {
        Self::new(line, col, line, col)
    }

    /// Returns the number of lines this span covers.
    pub fn line_count(&self) -> u32 {
        self.end_line.saturating_sub(self.start_line) + 1
    }

    /// Returns true if the given position is within this span.
    pub fn contains(&self, line: u32, col: u32) -> bool {
        if line < self.start_line || line > self.end_line {
            return false;
        }
        if line == self.start_line && col < self.start_col {
            return false;
        }
        if line == self.end_line && col > self.end_col {
            return false;
        }
        true
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}-{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col
        )
    }
}
