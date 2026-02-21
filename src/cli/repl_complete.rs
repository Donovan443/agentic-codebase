//! Tab completion for the ACB interactive REPL.
//!
//! Provides context-aware completion for slash commands, query types,
//! and .acb file paths.

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{
    Cmd, ConditionalEventHandler, Event, EventContext, EventHandler, Helper, KeyEvent, RepeatCount,
};

/// All available REPL slash commands.
pub const COMMANDS: &[(&str, &str)] = &[
    ("/compile", "Compile a directory into an .acb graph"),
    ("/info", "Display summary of a loaded .acb file"),
    ("/query", "Run a query (symbol, deps, impact, ...)"),
    ("/get", "Get detailed info about a unit by ID"),
    ("/load", "Load an .acb file for querying"),
    ("/units", "List all units in the loaded graph"),
    ("/clear", "Clear the screen"),
    ("/help", "Show available commands"),
    ("/exit", "Quit the REPL"),
];

/// Query type names for completion.
const QUERY_TYPES: &[&str] = &[
    "symbol",
    "deps",
    "rdeps",
    "impact",
    "calls",
    "similar",
    "prophecy",
    "stability",
    "coupling",
];

/// ACB REPL helper providing tab completion.
pub struct AcbHelper;

impl Default for AcbHelper {
    fn default() -> Self {
        Self::new()
    }
}

impl AcbHelper {
    pub fn new() -> Self {
        Self
    }

    /// Get list of .acb files in the current directory.
    fn acb_files(&self) -> Vec<String> {
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(".") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "acb") {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        files.push(name.to_string());
                    }
                }
            }
        }
        files.sort();
        files
    }
}

impl Completer for AcbHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos];

        // Complete command names if input starts with /
        if !input.contains(' ') {
            let matches: Vec<Pair> = COMMANDS
                .iter()
                .filter(|(cmd, _)| cmd.starts_with(input))
                .map(|(cmd, desc)| Pair {
                    display: format!("{cmd:<16} {desc}"),
                    replacement: format!("{cmd} "),
                })
                .collect();
            return Ok((0, matches));
        }

        // Split into command and args
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0];
        let args = if parts.len() > 1 { parts[1] } else { "" };

        match cmd {
            // Query type completion
            "/query" => {
                if !args.contains(' ') {
                    let prefix_start = input.len() - args.len();
                    let matches: Vec<Pair> = QUERY_TYPES
                        .iter()
                        .filter(|t| t.starts_with(args.trim()))
                        .map(|t| Pair {
                            display: t.to_string(),
                            replacement: format!("{t} "),
                        })
                        .collect();
                    return Ok((prefix_start, matches));
                }
                Ok((pos, Vec::new()))
            }

            // .acb file completion for /load, /info
            "/load" | "/info" => {
                let files = self.acb_files();
                let prefix_start = input.len() - args.len();
                let matches: Vec<Pair> = files
                    .iter()
                    .filter(|f| f.starts_with(args.trim()))
                    .map(|f| Pair {
                        display: f.clone(),
                        replacement: format!("{f} "),
                    })
                    .collect();
                Ok((prefix_start, matches))
            }

            // Directory completion for /compile
            "/compile" => {
                let prefix_start = input.len() - args.len();
                let query = args.trim();
                let mut matches: Vec<Pair> = Vec::new();
                let search_dir = if query.is_empty() { "." } else { query };
                if let Ok(entries) = std::fs::read_dir(search_dir) {
                    for entry in entries.flatten() {
                        if entry.path().is_dir() {
                            if let Some(name) = entry.file_name().to_str() {
                                if name.starts_with(query) || query.is_empty() {
                                    matches.push(Pair {
                                        display: name.to_string(),
                                        replacement: format!("{name} "),
                                    });
                                }
                            }
                        }
                    }
                }
                Ok((prefix_start, matches))
            }

            _ => Ok((pos, Vec::new())),
        }
    }
}

impl Hinter for AcbHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if pos < line.len() || line.is_empty() {
            return None;
        }
        // Show first matching command as ghost text
        if line.starts_with('/') && !line.contains(' ') {
            for (cmd, _) in COMMANDS {
                if cmd.starts_with(line) && *cmd != line {
                    return Some(cmd[line.len()..].to_string());
                }
            }
        }
        None
    }
}

impl Highlighter for AcbHelper {}
impl Validator for AcbHelper {}
impl Helper for AcbHelper {}

/// Event handler: Tab accepts hint if present, else triggers completion.
pub struct TabCompleteOrAcceptHint;

impl ConditionalEventHandler for TabCompleteOrAcceptHint {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        if ctx.has_hint() {
            Some(Cmd::CompleteHint)
        } else {
            Some(Cmd::Complete)
        }
    }
}

/// Bind custom key sequences to the editor.
pub fn bind_keys(rl: &mut rustyline::Editor<AcbHelper, rustyline::history::DefaultHistory>) {
    rl.bind_sequence(
        KeyEvent::from('\t'),
        EventHandler::Conditional(Box::new(TabCompleteOrAcceptHint)),
    );
}

/// Find the closest matching command for a misspelled input (Levenshtein distance).
pub fn suggest_command(input: &str) -> Option<&'static str> {
    let input_lower = input.to_lowercase();
    let mut best: Option<(&str, usize)> = None;

    for (cmd, _) in COMMANDS {
        let cmd_name = &cmd[1..];
        let dist = levenshtein(&input_lower, cmd_name);
        if dist <= 3 && (best.is_none() || dist < best.unwrap().1) {
            best = Some((cmd, dist));
        }
    }

    best.map(|(cmd, _)| cmd)
}

/// Simple Levenshtein distance for fuzzy matching.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}
