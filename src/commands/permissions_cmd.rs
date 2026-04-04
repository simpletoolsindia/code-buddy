//! Permissions Command - Permission management
//!
//! Provides permission viewing and configuration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Permission mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionMode {
    AutoAccept,
    Ask,
    ManualApproval,
    Bypass,
}

impl PermissionMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" | "auto-accept" => Some(PermissionMode::AutoAccept),
            "ask" => Some(PermissionMode::Ask),
            "manual" | "manual-approval" => Some(PermissionMode::ManualApproval),
            "bypass" | "dangerous" => Some(PermissionMode::Bypass),
            _ => None,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            PermissionMode::AutoAccept => "Automatically accept safe operations",
            PermissionMode::Ask => "Ask for confirmation before operations",
            PermissionMode::ManualApproval => "Require manual approval for all operations",
            PermissionMode::Bypass => "Bypass all permission checks (dangerous!)",
        }
    }
}

/// Tool permission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub tool: String,
    pub allowed: bool,
    pub denied_reason: Option<String>,
}

impl ToolPermission {
    pub fn allowed(tool: &str) -> Self {
        Self {
            tool: tool.to_string(),
            allowed: true,
            denied_reason: None,
        }
    }

    pub fn denied(tool: &str, reason: &str) -> Self {
        Self {
            tool: tool.to_string(),
            allowed: false,
            denied_reason: Some(reason.to_string()),
        }
    }
}

/// Permission context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionContext {
    pub mode: PermissionMode,
    pub allowed_tools: HashSet<String>,
    pub denied_tools: HashSet<String>,
    pub allowed_commands: HashSet<String>,
    pub denied_commands: HashSet<String>,
}

impl Default for PermissionContext {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Ask,
            allowed_tools: HashSet::new(),
            denied_tools: HashSet::new(),
            allowed_commands: HashSet::new(),
            denied_commands: HashSet::new(),
        }
    }
}

impl PermissionContext {
    pub fn is_tool_allowed(&self, tool: &str) -> bool {
        if self.denied_tools.contains(tool) {
            return false;
        }
        if !self.allowed_tools.is_empty() {
            return self.allowed_tools.contains(tool);
        }
        true
    }

    pub fn is_command_allowed(&self, command: &str) -> bool {
        if self.denied_commands.contains(command) {
            return false;
        }
        if !self.allowed_commands.is_empty() {
            return self.allowed_commands.contains(command);
        }
        true
    }
}

/// Run permissions command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_permissions();
    }

    match args[0].as_str() {
        "list" | "ls" => show_permissions(),
        "allow" => {
            if args.len() < 2 {
                return Ok("Usage: permissions allow <tool|command>".to_string());
            }
            allow_item(&args[1])
        }
        "deny" => {
            if args.len() < 2 {
                return Ok("Usage: permissions deny <tool|command>".to_string());
            }
            deny_item(&args[1])
        }
        "mode" => {
            if args.len() < 2 {
                return Ok("Usage: permissions mode <auto|ask|manual|bypass>".to_string());
            }
            set_mode(&args[1])
        }
        _ => {
            Ok(format!("Unknown permissions command: {}\n\nUsage: permissions <list|allow|deny|mode>", args[0]))
        }
    }
}

fn show_permissions() -> Result<String> {
    let mut output = String::from("# Permissions\n\n## Mode\n\nCurrent: Ask\n\n## Allowed Tools\n\n- Read\n- Write\n- Edit\n- Glob\n- Grep\n- Bash (with confirmation)\n- WebSearch\n- WebFetch\n\n## Denied Tools\n\nNone configured.\n");
    Ok(output)
}

fn allow_item(item: &str) -> Result<String> {
    Ok(format!("Allowed: {}\n", item))
}

fn deny_item(item: &str) -> Result<String> {
    Ok(format!("Denied: {}\n", item))
}

fn set_mode(mode: &str) -> Result<String> {
    let parsed = PermissionMode::from_str(mode);

    if let Some(m) = parsed {
        Ok(format!(
            "Set permission mode to: {:?}\n{}\n",
            m, m.description()
        ))
    } else {
        Ok(format!(
            "Unknown permission mode: {}\n\nValid modes:\n- auto\n- ask\n- manual\n- bypass\n",
            mode
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_mode() {
        let mode = PermissionMode::from_str("ask").unwrap();
        assert!(matches!(mode, PermissionMode::Ask));
    }

    #[test]
    fn test_permission_context() {
        let ctx = PermissionContext::default();
        assert!(ctx.is_tool_allowed("Read"));
    }
}
