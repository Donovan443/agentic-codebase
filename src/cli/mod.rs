//! Command-line interface for the `acb` binary.
//!
//! Wraps the public API into CLI commands. Stateless — every command
//! opens a file, does work, closes the file.

pub mod commands;
pub mod output;
pub mod repl;
pub mod repl_commands;
pub mod repl_complete;
