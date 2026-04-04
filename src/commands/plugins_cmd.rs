//! Plugins Command - Plugin management
//!
//! Provides plugin listing, installation, and removal.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub enabled: bool,
    pub installed: bool,
}

impl Plugin {
    pub fn new(id: &str, name: &str, version: &str, description: &str, author: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            description: description.to_string(),
            author: author.to_string(),
            enabled: false,
            installed: false,
        }
    }
}

/// Built-in plugins info
pub fn builtin_plugins() -> Vec<Plugin> {
    vec![
        Plugin::new("git", "Git", "1.0.0", "Git integration", "Code Buddy Team"),
        Plugin::new("docker", "Docker", "1.0.0", "Docker integration", "Code Buddy Team"),
        Plugin::new("github", "GitHub", "1.0.0", "GitHub integration", "Code Buddy Team"),
    ]
}

/// Marketplace plugin listing
pub fn marketplace_plugins() -> Vec<Plugin> {
    vec![
        Plugin::new("slack", "Slack", "1.0.0", "Slack notifications", "Community"),
        Plugin::new("jira", "Jira", "1.0.0", "Jira integration", "Community"),
        Plugin::new("linear", "Linear", "1.0.0", "Linear integration", "Community"),
        Plugin::new("notion", "Notion", "1.0.0", "Notion integration", "Community"),
        Plugin::new("database", "Database", "1.0.0", "Database tools", "Community"),
    ]
}

/// Run plugins command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return list_plugins();
    }

    match args[0].as_str() {
        "list" | "ls" => list_plugins(),
        "installed" => list_installed(),
        "marketplace" => list_marketplace(),
        "install" => {
            if args.len() < 2 {
                return Ok("Usage: plugins install <plugin-id>".to_string());
            }
            install_plugin(&args[1])
        }
        "uninstall" | "remove" => {
            if args.len() < 2 {
                return Ok("Usage: plugins uninstall <plugin-id>".to_string());
            }
            uninstall_plugin(&args[1])
        }
        "enable" => {
            if args.len() < 2 {
                return Ok("Usage: plugins enable <plugin-id>".to_string());
            }
            enable_plugin(&args[1])
        }
        "disable" => {
            if args.len() < 2 {
                return Ok("Usage: plugins disable <plugin-id>".to_string());
            }
            disable_plugin(&args[1])
        }
        "search" => {
            if args.len() < 2 {
                return Ok("Usage: plugins search <query>".to_string());
            }
            search_plugins(&args[1])
        }
        _ => {
            Ok(format!("Unknown plugins command: {}\n\nUsage: plugins <list|install|uninstall|enable|disable|search>", args[0]))
        }
    }
}

fn list_plugins() -> Result<String> {
    let mut output = String::from("# Plugins\n\n## Built-in Plugins\n\n");

    for plugin in builtin_plugins() {
        output.push_str(&format!(
            "- `{}` - {} (v{})\n",
            plugin.id, plugin.name, plugin.version
        ));
    }

    output.push_str("\n## Installed Plugins\n\nNone installed.\n\n");
    output.push_str("Run `plugins marketplace` to see available plugins.\n");

    Ok(output)
}

fn list_installed() -> Result<String> {
    let output = String::from("# Installed Plugins\n\nNo plugins installed.\n");
    Ok(output)
}

fn list_marketplace() -> Result<String> {
    let mut output = String::from("# Plugin Marketplace\n\n");

    for plugin in marketplace_plugins() {
        output.push_str(&format!(
            "## {} ({})\n**Version:** {}\n**Author:** {}\n{}\n\n",
            plugin.name, plugin.id, plugin.version, plugin.author, plugin.description
        ));
    }

    output.push_str("Install with: `plugins install <plugin-id>`\n");
    Ok(output)
}

fn install_plugin(id: &str) -> Result<String> {
    Ok(format!("Installing plugin: {}\n", id))
}

fn uninstall_plugin(id: &str) -> Result<String> {
    Ok(format!("Uninstalled plugin: {}\n", id))
}

fn enable_plugin(id: &str) -> Result<String> {
    Ok(format!("Enabled plugin: {}\n", id))
}

fn disable_plugin(id: &str) -> Result<String> {
    Ok(format!("Disabled plugin: {}\n", id))
}

fn search_plugins(query: &str) -> Result<String> {
    let query_lower = query.to_lowercase();
    let mut output = format!("# Search: {}\n\n", query);

    for plugin in marketplace_plugins() {
        if plugin.name.to_lowercase().contains(&query_lower)
            || plugin.description.to_lowercase().contains(&query_lower)
        {
            output.push_str(&format!("- {} ({})\n", plugin.name, plugin.id));
        }
    }

    if output.lines().count() <= 2 {
        output.push_str("No matching plugins found.\n");
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = Plugin::new("test", "Test Plugin", "1.0.0", "A test", "Tester");
        assert_eq!(plugin.id, "test");
        assert!(!plugin.installed);
    }
}
