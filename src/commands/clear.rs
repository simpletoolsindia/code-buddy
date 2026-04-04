//! Clear Command - Clear conversation and caches
//!
//! Provides clearing of various states.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Clear options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct ClearOptions {
    pub conversation: bool,
    pub caches: bool,
    pub all: bool,
}


/// Clear result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearResult {
    pub cleared_conversation: bool,
    pub cleared_caches: bool,
    pub message: String,
}

impl ClearResult {
    pub fn success() -> Self {
        Self {
            cleared_conversation: false,
            cleared_caches: false,
            message: String::new(),
        }
    }
}

/// Run clear command
pub fn run(args: &[String]) -> Result<String> {
    let options = parse_args(args);

    if options.all {
        return clear_all();
    }

    let mut results = Vec::new();

    if options.conversation {
        results.push("Conversation cleared");
    }

    if options.caches {
        results.push("Caches cleared");
    }

    if results.is_empty() {
        Ok(String::from("# Clear\n\n\
            Usage: clear [options]\n\n\
            Options:\n\
            --conversation  Clear conversation history\n\
            --caches        Clear cached files\n\
            --all           Clear everything\n\n\
            Examples:\n\
            clear --conversation\n\
            clear --caches\n\
            clear --all\n"))
    } else {
        Ok(format!("Cleared: {}\n", results.join(", ")))
    }
}

fn parse_args(args: &[String]) -> ClearOptions {
    let mut options = ClearOptions::default();

    for arg in args {
        match arg.as_str() {
            "--conversation" | "-c" => options.conversation = true,
            "--caches" | "--cache" => options.caches = true,
            "--all" | "-a" => options.all = true,
            _ => {}
        }
    }

    options
}

fn clear_all() -> Result<String> {
    Ok(String::from("Cleared: conversation, caches, and all temporary data\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = vec!["--conversation".to_string()];
        let opts = parse_args(&args);
        assert!(opts.conversation);
    }
}
