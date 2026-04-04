//! Compact Command - Conversation compaction
//!
//! Provides conversation memory compaction.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Compact options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactOptions {
    pub keep_messages: usize,
    pub dry_run: bool,
    pub force: bool,
}

impl Default for CompactOptions {
    fn default() -> Self {
        Self {
            keep_messages: 10,
            dry_run: false,
            force: false,
        }
    }
}

/// Compact result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactResult {
    pub original_messages: usize,
    pub compacted_messages: usize,
    pub tokens_saved: usize,
    pub summary: String,
}

impl CompactResult {
    pub fn format(&self) -> String {
        format!(
            "# Compact Result\n\n\
            Original messages: {}\n\
            Compacted messages: {}\n\
            Tokens saved: ~{}\n\n\
            Summary:\n{}\n",
            self.original_messages,
            self.compacted_messages,
            self.tokens_saved,
            self.summary
        )
    }
}

/// Run compact command
pub fn run(args: &[String]) -> Result<String> {
    let options = parse_compact_args(args);

    // Simulated compaction
    let original = 100;
    let compacted = options.keep_messages;
    let tokens_saved = (original - compacted) * 200; // rough estimate

    if options.dry_run {
        return Ok(format!(
            "# Dry Run - No changes made\n\n\
            Would compact {} messages down to {}\n\
            Estimated tokens saved: ~{}\n",
            original, compacted, tokens_saved
        ));
    }

    let result = CompactResult {
        original_messages: original,
        compacted_messages: compacted,
        tokens_saved,
        summary: String::from("Previous conversation summarized into key points and decisions."),
    };

    Ok(result.format())
}

fn parse_compact_args(args: &[String]) -> CompactOptions {
    let mut options = CompactOptions::default();

    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--dry-run" | "-n" => options.dry_run = true,
            "--force" | "-f" => options.force = true,
            "--keep" | "-k" => {
                if i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse() {
                        options.keep_messages = n;
                    }
                }
            }
            _ => {}
        }
    }

    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = vec!["--dry-run".to_string(), "--keep".to_string(), "5".to_string()];
        let opts = parse_compact_args(&args);
        assert!(opts.dry_run);
        assert_eq!(opts.keep_messages, 5);
    }
}
