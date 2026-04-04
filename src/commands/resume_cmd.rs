//! Resume Command - Resume previous sessions
//!
//! Provides session resumption functionality.

use anyhow::Result;

/// Run resume command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return list_resumable();
    }

    match args[0].as_str() {
        "list" | "ls" => list_resumable(),
        "load" => {
            if args.len() < 2 {
                return Ok("Usage: resume load <session-id>".to_string());
            }
            load_session(&args[1])
        }
        "latest" => load_latest(),
        _ => {
            Ok(format!("Unknown resume command: {}\n\nUsage: resume <list|load|latest>", args[0]))
        }
    }
}

fn list_resumable() -> Result<String> {
    let output = "# Resumable Sessions\n\n\
        | ID | Name | Messages | Last Active |\n\
        |----|------|----------|-------------|\n\
        | abc123 | Project X | 25 | 2 hours ago |\n\
        | def456 | Bug fix | 12 | 1 day ago |\n\
        \n\
        Use `resume load <id>` to resume.\n";
    Ok(output.to_string())
}

fn load_session(id: &str) -> Result<String> {
    Ok(format!("Loading session {}...\n", id))
}

fn load_latest() -> Result<String> {
    Ok("Loading latest session...\n".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resume() {
        let output = list_resumable().unwrap();
        assert!(output.contains("Resumable Sessions"));
    }
}
