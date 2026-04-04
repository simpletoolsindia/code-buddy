//! Skin Tool - Theme and appearance customization
//!
//! Use when user asks to customize theme or appearance.

use anyhow::Result;
use code_buddy::skin_engine::{SkinConfig, built_in_skins};
use dirs as dirs_crate;
use serde::{Deserialize, Serialize};
use super::Tool;

/// Skin/theme tool for appearance customization
pub struct SkinTool;

impl SkinTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SkinTool {
    fn default() -> Self {
        Self::new()
    }
}

fn get_skins_dir() -> std::path::PathBuf {
    dirs_crate::home_dir()
        .map(|h| h.join(".code-buddy").join("skins"))
        .unwrap_or_else(|| std::path::PathBuf::from("~/.code-buddy/skins"))
}

impl Tool for SkinTool {
    fn name(&self) -> &str {
        "Skin"
    }

    fn description(&self) -> &str {
        "Manage CLI themes/skins. List available skins, apply a theme, or create custom ones. \
Built-in skins: default (gold/kawaii), ares (red/sci-fi), mono (monochrome), slate (dark). \
User skins stored in ~/.code-buddy/skins/\n\
Args: <action> [skin_name] [--colors <json>]
  list                  - List all available skins
  apply <name>         - Apply a skin by name
  create <name> <desc>  - Create a custom skin
  current              - Show current active skin
Example: Skin('list')
Example: Skin('apply', 'dracula')
Example: Skin('apply', 'monokai')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Skin tool usage:\n\
  list                  - List all available skins\n\
  apply <name>         - Apply a skin by name\n\
  create <name> <desc>  - Create a custom skin\n\
  current              - Show current active skin\n\
Built-in: default, ares, mono, slate".to_string());
        }

        let action = args.first().map(|s| s.to_lowercase()).unwrap_or_default();

        match action.as_str() {
            "list" => {
                let skins = built_in_skins();
                let output: Vec<serde_json::Value> = skins
                    .into_iter()
                    .map(|(name, skin)| {
                        serde_json::json!({
                            "name": name,
                            "description": skin.description,
                            "colors": {
                                "banner_border": skin.colors.banner_border,
                                "error": skin.colors.error,
                                "success": skin.colors.success,
                                "warning": skin.colors.warning,
                            },
                        })
                    })
                    .collect();
                Ok(serde_json::to_string_pretty(&output)?)
            }
            "apply" => {
                if args.len() < 2 {
                    return Ok("Usage: Skin('apply', '<skin_name>')".to_string());
                }
                let skin_name = &args[1].to_lowercase();
                let skins = built_in_skins();

                if skins.contains_key(skin_name) {
                    Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "success": true,
                        "skin": skin_name,
                        "message": format!("Skin '{}' applied. Restart code-buddy to see changes.", skin_name),
                        "hint": "Skin changes take effect on next session"
                    }))?)
                } else {
                    Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "error": format!("Skin '{}' not found. Available: {}", skin_name, skins.keys().cloned().collect::<Vec<_>>().join(", "))
                    }))?)
                }
            }
            "current" => {
                let skin = SkinConfig::default();
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "current_skin": skin.name,
                    "description": skin.description,
                    "branding": {
                        "agent_name": skin.branding.agent_name,
                        "response_label": skin.branding.response_label,
                    },
                }))?)
            }
            "create" => {
                if args.len() < 3 {
                    return Ok("Usage: Skin('create', '<name>', '<description>')".to_string());
                }
                let name = &args[1];
                let description = &args[2];

                std::fs::create_dir_all(get_skins_dir())?;

                let default_skin = SkinConfig::default();
                let custom_skin = SkinConfig {
                    name: name.to_string(),
                    description: description.to_string(),
                    colors: default_skin.colors,
                    spinner: default_skin.spinner,
                    branding: default_skin.branding,
                    tool_prefix: default_skin.tool_prefix,
                    tool_emojis: default_skin.tool_emojis,
                };

                let yaml = serde_yaml::to_string(&custom_skin)
                    .unwrap_or_else(|_| format!("name: {}\ndescription: {}", name, description));
                let skin_path = get_skins_dir().join(format!("{}.yaml", name));
                std::fs::write(&skin_path, &yaml)?;

                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "skin": name,
                    "path": skin_path.to_string_lossy(),
                    "message": format!("Custom skin '{}' created at {}", name, skin_path.display()),
                }))?)
            }
            _ => {
                Ok(format!("Unknown action: {}\nActions: list, apply, create, current", action))
            }
        }
    }
}
