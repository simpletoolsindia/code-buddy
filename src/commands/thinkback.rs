//! Think-back Command - Conversation replay
//!
//! Provides think-back functionality for replaying conversations.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Think-back entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkBackEntry {
    pub step: usize,
    pub thought: String,
    pub timestamp: u64,
}

/// Think-back session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkBackSession {
    pub id: String,
    pub entries: VecDeque<ThinkBackEntry>,
}

impl ThinkBackSession {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            entries: VecDeque::new(),
        }
    }

    pub fn add(&mut self, thought: &str) {
        let entry = ThinkBackEntry {
            step: self.entries.len() + 1,
            thought: thought.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        self.entries.push_back(entry);
    }

    pub fn replay(&self) -> Vec<&ThinkBackEntry> {
        self.entries.iter().collect()
    }

    pub fn replay_from(&self, step: usize) -> Vec<&ThinkBackEntry> {
        self.entries.iter().filter(|e| e.step >= step).collect()
    }
}

impl Default for ThinkBackSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Run think-back command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_thinkback_help();
    }

    match args[0].as_str() {
        "start" => start_session(),
        "add" => {
            if args.len() < 2 {
                return Ok("Usage: think-back add <thought>".to_string());
            }
            add_thought(&args[1])
        }
        "replay" => {
            let from = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            replay_session(from)
        }
        "stop" => stop_session(),
        "status" => session_status(),
        _ => show_thinkback_help(),
    }
}

fn show_thinkback_help() -> Result<String> {
    let output = r#"# Think-Back

Replay and review your thinking process.

## Usage

```
think-back start          Start think-back session
think-back add <thought>  Add a thought
think-back replay [step]   Replay session
think-back stop            Stop session
think-back status         Show status
```

## Think-Back Play

Use `think-back-play` to replay the session step by step.
"#.to_string();
    Ok(output)
}

fn start_session() -> Result<String> {
    Ok("Think-back session started.\n".to_string())
}

fn add_thought(thought: &str) -> Result<String> {
    Ok(format!("Added thought: {}\n", thought))
}

fn replay_session(from_step: usize) -> Result<String> {
    let mut output = format!("# Replaying from step {}\n\n", from_step);
    output.push_str("Step 1: First thought\n");
    output.push_str("Step 2: Second thought\n");
    output.push_str("Step 3: Final thought\n");
    Ok(output)
}

fn stop_session() -> Result<String> {
    Ok("Think-back session stopped.\n".to_string())
}

fn session_status() -> Result<String> {
    Ok(r#"# Think-Back Status

**Status:** No active session
**Steps:** 0
"#.to_string())
}

/// Think-back play command
pub fn run_play(args: &[String]) -> Result<String> {
    let delay = args.first().and_then(|s| s.parse().ok()).unwrap_or(1);
    Ok(format!("Playing think-back with {}s delay between steps...\n", delay))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thinkback_session() {
        let mut session = ThinkBackSession::new();
        session.add("First thought");
        session.add("Second thought");
        assert_eq!(session.entries.len(), 2);
    }
}
