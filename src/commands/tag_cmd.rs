//! Tag Command - Session and file tagging
//!
//! Provides tagging functionality for sessions and files.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Tag
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Tag(pub String);

impl Tag {
    pub fn new(name: &str) -> Self {
        Self(name.to_lowercase().replace(' ', "-"))
    }
}

/// Tag manager
pub struct TagManager {
    tags: HashSet<Tag>,
}

impl TagManager {
    pub fn new() -> Self {
        Self { tags: HashSet::new() }
    }

    pub fn add(&mut self, tag: &str) {
        self.tags.insert(Tag::new(tag));
    }

    pub fn remove(&mut self, tag: &str) {
        self.tags.remove(&Tag::new(tag));
    }

    pub fn list(&self) -> Vec<&Tag> {
        self.tags.iter().collect()
    }

    pub fn has(&self, tag: &str) -> bool {
        self.tags.contains(&Tag::new(tag))
    }
}

impl Default for TagManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Run tag command
pub fn run(args: &[String]) -> Result<String> {
    let mut manager = TagManager::new();

    if args.is_empty() {
        return list_tags(&manager);
    }

    match args[0].as_str() {
        "list" | "ls" => list_tags(&manager),
        "add" => {
            if args.len() < 2 {
                return Ok("Usage: tag add <tag>".to_string());
            }
            add_tag(&mut manager, &args[1])
        }
        "remove" | "rm" => {
            if args.len() < 2 {
                return Ok("Usage: tag remove <tag>".to_string());
            }
            remove_tag(&mut manager, &args[1])
        }
        "search" => {
            if args.len() < 2 {
                return Ok("Usage: tag search <tag>".to_string());
            }
            search_by_tag(&args[1])
        }
        _ => {
            Ok(format!("Unknown tag command: {}\n\nUsage: tag <list|add|remove|search>", args[0]))
        }
    }
}

fn list_tags(manager: &TagManager) -> Result<String> {
    let mut output = String::from("# Tags\n\n");

    if manager.list().is_empty() {
        output.push_str("No tags configured.\n");
    } else {
        for tag in manager.list() {
            output.push_str(&format!("- `{}`\n", tag.0));
        }
    }

    Ok(output)
}

fn add_tag(manager: &mut TagManager, tag: &str) -> Result<String> {
    manager.add(tag);
    Ok(format!("Added tag: {}\n", tag))
}

fn remove_tag(manager: &mut TagManager, tag: &str) -> Result<String> {
    manager.remove(tag);
    Ok(format!("Removed tag: {}\n", tag))
}

fn search_by_tag(tag: &str) -> Result<String> {
    Ok(format!("Sessions tagged with '{}':\n\n[None found]\n", tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_creation() {
        let tag = Tag::new("My Tag");
        assert_eq!(tag.0, "my-tag");
    }

    #[test]
    fn test_tag_manager() {
        let mut manager = TagManager::new();
        manager.add("test");
        assert!(manager.has("test"));
        manager.remove("test");
        assert!(!manager.has("test"));
    }
}
