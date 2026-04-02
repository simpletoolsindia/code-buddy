//! Hooks Command - Automation hooks management
//!
//! Provides hooks listing, adding, and removal.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Hook definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub id: String,
    pub event: String,
    pub command: String,
    pub enabled: bool,
}

impl Hook {
    pub fn new(event: &str, command: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            event: event.to_string(),
            command: command.to_string(),
            enabled: true,
        }
    }
}

/// Hook manager
pub struct HooksManager {
    hooks: Vec<Hook>,
}

impl HooksManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add(&mut self, event: &str, command: &str) -> &Hook {
        self.hooks.push(Hook::new(event, command));
        self.hooks.last().unwrap()
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let len = self.hooks.len();
        self.hooks.retain(|h| h.id != id);
        self.hooks.len() < len
    }

    pub fn list(&self) -> &[Hook] {
        &self.hooks
    }

    pub fn get_by_event(&self, event: &str) -> Vec<&Hook> {
        self.hooks.iter().filter(|h| h.event == event && h.enabled).collect()
    }

    pub fn toggle(&mut self, id: &str) -> bool {
        if let Some(hook) = self.hooks.iter_mut().find(|h| h.id == id) {
            hook.enabled = !hook.enabled;
            true
        } else {
            false
        }
    }
}

impl Default for HooksManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Available hook events
pub fn available_events() -> Vec<(&'static str, &'static str)> {
    vec![
        ("before_write", "Before file write"),
        ("after_write", "After file write"),
        ("before_submit", "Before message submit"),
        ("after_submit", "After message submit"),
        ("on_error", "On error occurrence"),
        ("on_tool_use", "On tool execution"),
        ("on_compact", "On conversation compact"),
        ("on_start", "On session start"),
        ("on_exit", "On session exit"),
    ]
}

/// Run hooks command
pub fn run(args: &[String]) -> Result<String> {
    let mut manager = HooksManager::new();

    if args.is_empty() {
        return list_hooks(&manager);
    }

    match args[0].as_str() {
        "list" | "ls" => list_hooks(&manager),
        "add" => {
            if args.len() < 3 {
                return Ok("Usage: hooks add <event> <command>".to_string());
            }
            add_hook(&mut manager, &args[1], &args[2..].join(" "))
        }
        "remove" | "rm" => {
            if args.len() < 2 {
                return Ok("Usage: hooks remove <id>".to_string());
            }
            remove_hook(&mut manager, &args[1])
        }
        "toggle" => {
            if args.len() < 2 {
                return Ok("Usage: hooks toggle <id>".to_string());
            }
            toggle_hook(&mut manager, &args[1])
        }
        "events" => list_events(),
        _ => {
            Ok(format!("Unknown hooks command: {}\n\nUsage: hooks <list|add|remove|toggle|events>", args[0]))
        }
    }
}

fn list_hooks(manager: &HooksManager) -> Result<String> {
    let mut output = String::from("# Hooks\n\n## Available Events\n\n");

    for (event, desc) in available_events() {
        output.push_str(&format!("- `{}` - {}\n", event, desc));
    }

    output.push_str("\n## Configured Hooks\n\n");

    if manager.list().is_empty() {
        output.push_str("No hooks configured.\n");
    } else {
        output.push_str("| ID | Event | Command | Enabled |\n");
        output.push_str("|----|-------|---------|--------|\n");
        for hook in manager.list() {
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                &hook.id[..8],
                hook.event,
                hook.command,
                if hook.enabled { "Yes" } else { "No" }
            ));
        }
    }

    Ok(output)
}

fn add_hook(manager: &mut HooksManager, event: &str, command: &str) -> Result<String> {
    let hook = manager.add(event, command);
    Ok(format!(
        "Added hook: {} -> {}\nHook ID: {}",
        event, command, &hook.id[..8]
    ))
}

fn remove_hook(manager: &mut HooksManager, id: &str) -> Result<String> {
    if manager.remove(id) {
        Ok(format!("Removed hook: {}\n", id))
    } else {
        Ok(format!("Hook not found: {}\n", id))
    }
}

fn toggle_hook(manager: &mut HooksManager, id: &str) -> Result<String> {
    if manager.toggle(id) {
        Ok(format!("Toggled hook: {}\n", id))
    } else {
        Ok(format!("Hook not found: {}\n", id))
    }
}

fn list_events() -> Result<String> {
    let mut output = String::from("# Available Hook Events\n\n");
    output.push_str("| Event | Description |\n");
    output.push_str("|-------|-------------|\n");

    for (event, desc) in available_events() {
        output.push_str(&format!("| `{}` | {} |\n", event, desc));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_creation() {
        let hook = Hook::new("before_write", "echo 'test'");
        assert_eq!(hook.event, "before_write");
        assert!(hook.enabled);
    }

    #[test]
    fn test_hooks_manager() {
        let mut manager = HooksManager::new();
        manager.add("before_write", "echo 'test'");
        assert_eq!(manager.list().len(), 1);
    }
}
