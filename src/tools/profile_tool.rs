//! Profile Tool - Multi-instance isolated environments
//!
//! Use for isolating contexts, credentials, or environments.

use anyhow::Result;
use code_buddy::profiles::{ProfileManager, Profile};
use dirs as dirs_crate;
use serde::{Deserialize, Serialize};
use super::Tool;

/// Profile management tool for isolated environments
pub struct ProfileTool;

impl ProfileTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProfileTool {
    fn default() -> Self {
        Self::new()
    }
}

fn get_profiles_root() -> std::path::PathBuf {
    dirs_crate::home_dir()
        .map(|h| h.join(".code-buddy").join("profiles"))
        .unwrap_or_else(|| std::path::PathBuf::from("~/.code-buddy/profiles"))
}

impl Tool for ProfileTool {
    fn name(&self) -> &str {
        "Profile"
    }

    fn description(&self) -> &str {
        "Manage isolated environments (profiles). Each profile has its own config, \
memory, skills, sessions, and credentials. \
Use for work/personal separation, project isolation, multi-tenant workflows. \
Args: <action> [profile_name]
  list                  - List all profiles
  create <name>         - Create a new profile
  switch <name>         - Switch to a profile (writes CODE_BUDDY_HOME)
  delete <name>         - Delete a profile
  current               - Show current active profile
Example: Profile('list')
Example: Profile('create', 'work-project-x')
Example: Profile('switch', 'personal')
Example: Profile('current')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Profile tool usage:\n\
  list                  - List all profiles\n\
  create <name>         - Create a new profile\n\
  switch <name>         - Switch to a profile\n\
  delete <name>         - Delete a profile\n\
  current               - Show current active profile".to_string());
        }

        let action = args.first().map(|s| s.to_lowercase()).unwrap_or_default();
        let profiles_root = get_profiles_root();

        match action.as_str() {
            "list" => {
                let manager = ProfileManager::new(profiles_root)?;
                let profiles = manager.list()?;
                let output: Vec<serde_json::Value> = profiles
                    .into_iter()
                    .map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "home": p.home.to_string_lossy(),
                            "description": p.description,
                            "created_at": p.created_at,
                            "last_used": p.last_used,
                        })
                    })
                    .collect();
                Ok(serde_json::to_string_pretty(&output)?)
            }
            "create" => {
                if args.len() < 2 {
                    return Ok("Usage: Profile('create', '<name>')".to_string());
                }
                let name = &args[1];
                let manager = ProfileManager::new(profiles_root)?;
                let profile = manager.create(name, None)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "profile": profile.name,
                    "home": profile.home.to_string_lossy(),
                    "message": format!("Profile '{}' created. Switch with: Profile('switch', '{}')", name, name)
                }))?)
            }
            "switch" => {
                if args.len() < 2 {
                    return Ok("Usage: Profile('switch', '<name>')".to_string());
                }
                let name = &args[1];
                let manager = ProfileManager::new(profiles_root)?;
                let new_home = manager.activate(name)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "profile": name,
                    "home": new_home.to_string_lossy(),
                    "message": format!("Switched to profile '{}'. Set CODE_BUDDY_HOME={} for this session.", name, new_home.to_string_lossy()),
                    "env_hint": format!("export CODE_BUDDY_HOME=\"{}\"", new_home.to_string_lossy()),
                }))?)
            }
            "delete" => {
                if args.len() < 2 {
                    return Ok("Usage: Profile('delete', '<name>')".to_string());
                }
                let name = &args[1];
                if name == "default" {
                    return Ok(serde_json::to_string_pretty(&serde_json::json!({
                        "success": false,
                        "error": "Cannot delete the default profile"
                    }))?);
                }
                let manager = ProfileManager::new(profiles_root)?;
                manager.remove(name)?;
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": true,
                    "message": format!("Profile '{}' deleted", name)
                }))?)
            }
            "current" => {
                let current_home = std::env::var("CODE_BUDDY_HOME")
                    .unwrap_or_else(|_| "~/.code-buddy".to_string());
                let profile_name = std::path::Path::new(&current_home)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "default".to_string());
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "current_profile": profile_name,
                    "home": current_home,
                    "message": "Current active profile"
                }))?)
            }
            _ => {
                Ok(format!("Unknown action: {}\nActions: list, create, switch, delete, current", action))
            }
        }
    }
}
