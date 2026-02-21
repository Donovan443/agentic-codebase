//! A simple Rust library for testing the parser.

use std::collections::HashMap;
use std::io::{self, Read};

/// A configuration struct.
pub struct Config {
    pub name: String,
    pub values: HashMap<String, String>,
}

/// A private helper struct.
struct InternalState {
    count: u32,
}

/// Trait for things that can be processed.
pub trait Processor {
    /// Process an item and return a result.
    fn process(&self, input: &str) -> Result<String, Box<dyn std::error::Error>>;

    /// Check if the processor is ready.
    fn is_ready(&self) -> bool;
}

/// An enum representing status.
pub enum Status {
    Active,
    Inactive,
    Error(String),
}

impl Config {
    /// Create a new config.
    pub fn new(name: String) -> Self {
        Self {
            name,
            values: HashMap::new(),
        }
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    fn internal_method(&self) -> bool {
        true
    }
}

impl Processor for Config {
    fn process(&self, input: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(format!("{}: {}", self.name, input))
    }

    fn is_ready(&self) -> bool {
        !self.name.is_empty()
    }
}

pub(crate) fn crate_visible_func() -> u32 {
    42
}

pub(super) fn super_visible_func() -> u32 {
    43
}

/// An async function.
pub async fn fetch_remote(url: &str) -> io::Result<String> {
    Ok(format!("fetched: {}", url))
}

/// Complex function with many branches.
pub fn calculate(x: i32, y: i32) -> i32 {
    if x > 0 {
        if y > 0 {
            x + y
        } else {
            x - y
        }
    } else {
        match y {
            0 => 0,
            1..=10 => y * 2,
            _ => y,
        }
    }
}

mod inner {
    /// A function inside a module.
    pub fn inner_func() -> &'static str {
        "inner"
    }
}

/// A macro for testing.
macro_rules! my_macro {
    ($x:expr) => {
        $x + 1
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = Config::new("test".into());
        assert_eq!(config.name, "test");
    }

    #[test]
    fn test_calculate() {
        assert_eq!(calculate(1, 2), 3);
        assert_eq!(calculate(-1, 5), 10);
    }
}
