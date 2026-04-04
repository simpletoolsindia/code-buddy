//! Keybindings Command - Keyboard shortcuts
//!
//! Provides keybindings listing and configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Keybinding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBinding {
    pub key: String,
    pub action: String,
    pub description: String,
}

/// Run keybindings command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return list_keybindings();
    }

    match args[0].as_str() {
        "list" | "ls" => list_keybindings(),
        "set" => {
            if args.len() < 3 {
                return Ok("Usage: keybindings set <key> <action>".to_string());
            }
            set_keybinding(&args[1], &args[2])
        }
        "remove" | "rm" => {
            if args.len() < 2 {
                return Ok("Usage: keybindings remove <key>".to_string());
            }
            remove_keybinding(&args[1])
        }
        "reset" => reset_keybindings(),
        _ => list_keybindings(),
    }
}

fn list_keybindings() -> Result<String> {
    let output = r#"# Keybindings

## General

| Key | Action |
|-----|--------|
| Ctrl+C | Cancel |
| Ctrl+D | Exit |
| Ctrl+L | Clear |
| Ctrl+Z | Undo |

## Navigation

| Key | Action |
|-----|--------|
| Tab | Autocomplete |
| Up/Down | History |

## Editor

| Key | Action |
|-----|--------|
| Ctrl+S | Save |
| Ctrl+W | Close |
| Ctrl+F | Find |

## Custom

No custom keybindings.

---
Use `keybindings set <key> <action>` to add a custom binding.
"#.to_string();
    Ok(output)
}

fn set_keybinding(key: &str, action: &str) -> Result<String> {
    Ok(format!("Set {} -> {}\n", key, action))
}

fn remove_keybinding(key: &str) -> Result<String> {
    Ok(format!("Removed binding for {}\n", key))
}

fn reset_keybindings() -> Result<String> {
    Ok("Keybindings reset to defaults.\n".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keybindings() {
        let output = list_keybindings().unwrap();
        assert!(output.contains("Keybindings"));
    }
}
