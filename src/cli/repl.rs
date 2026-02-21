//! Interactive REPL for ACB — slash command interface.
//!
//! Launch with `acb` (no subcommand) to enter interactive mode.
//! Type `/help` for available commands, Tab for completion.

use crate::cli::output::Styled;
use crate::cli::repl_commands;
use crate::cli::repl_complete;
use rustyline::config::CompletionType;
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};

/// History file location.
fn history_path() -> std::path::PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home).join(".acb_history")
}

/// Print the welcome banner.
fn print_banner() {
    let s = Styled::auto();

    eprintln!();
    eprintln!(
        "  {} {} {}",
        s.green("\u{25c9}"),
        s.bold(&format!("acb v{}", env!("CARGO_PKG_VERSION"))),
        s.dim("\u{2014} Semantic Code Compiler for AI Agents")
    );
    eprintln!();
    eprintln!(
        "    Press {} to browse commands, {} to complete, {} to quit.",
        s.cyan("/"),
        s.dim("Tab"),
        s.dim("/exit")
    );
    eprintln!();
}

/// Run the interactive REPL.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    print_banner();

    // Configure rustyline with List completion
    let config = Config::builder()
        .history_ignore_space(true)
        .auto_add_history(true)
        .completion_type(CompletionType::List)
        .completion_prompt_limit(20)
        .build();

    let helper = repl_complete::AcbHelper::new();
    let mut rl: Editor<repl_complete::AcbHelper, rustyline::history::DefaultHistory> =
        Editor::with_config(config)?;
    rl.set_helper(Some(helper));

    // Bind custom keys
    repl_complete::bind_keys(&mut rl);

    // Load history
    let hist_path = history_path();
    if hist_path.exists() {
        let _ = rl.load_history(&hist_path);
    }

    // Session state
    let mut state = repl_commands::ReplState::new();

    // Prompt
    let prompt = if Styled::auto().ok() == "OK" {
        " acb> ".to_string()
    } else {
        " \x1b[36macb>\x1b[0m ".to_string()
    };

    // Main REPL loop
    loop {
        match rl.readline(&prompt) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                match repl_commands::execute(line, &mut state) {
                    Ok(true) => {
                        let s = Styled::auto();
                        eprintln!("  {} Goodbye!", s.dim("\u{2728}"));
                        break;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        let s = Styled::auto();
                        eprintln!("  {} {e}", s.fail());
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                let s = Styled::auto();
                eprintln!("  {} Type {} to quit.", s.dim("(Ctrl+C)"), s.bold("/exit"));
            }
            Err(ReadlineError::Eof) => {
                let s = Styled::auto();
                eprintln!("  {} Goodbye!", s.dim("\u{2728}"));
                break;
            }
            Err(err) => {
                eprintln!("  Error: {err}");
                break;
            }
        }
    }

    // Save history
    let _ = std::fs::create_dir_all(hist_path.parent().unwrap_or(std::path::Path::new(".")));
    let _ = rl.save_history(&hist_path);

    Ok(())
}
