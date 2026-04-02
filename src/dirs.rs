//! Directory utilities for Code Buddy
//!
//! Provides cross-platform home directory and config path resolution.
//! Respects CODE_BUDDY_HOME environment variable for profile support.

use std::path::PathBuf;

/// Get the Code Buddy home directory
pub fn code_buddy_home() -> Option<PathBuf> {
    std::env::var("CODE_BUDDY_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|p| p.join(".code-buddy")))
}

/// Get the config directory
pub fn config_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("config"))
}

/// Get the data directory
pub fn data_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("data"))
}

/// Get the cache directory
pub fn cache_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("cache"))
}

/// Get the memory directory
pub fn memory_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("memory"))
}

/// Get the skills directory
pub fn skills_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("skills"))
}

/// Get the skins directory
pub fn skins_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("skins"))
}

/// Get the profiles directory
pub fn profiles_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("profiles"))
}

/// Get the cron directory
pub fn cron_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("cron"))
}

/// Get the sessions directory
pub fn sessions_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("sessions"))
}

/// Get the plugins directory
pub fn plugins_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("plugins"))
}

/// Get the MCP directory
pub fn mcp_dir() -> Option<PathBuf> {
    code_buddy_home().map(|p| p.join("mcp"))
}

/// Ensure a directory exists, creating it if needed
pub fn ensure_dir(path: &PathBuf) -> std::io::Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Display-friendly home path (e.g. ~/.code-buddy or ~/.code-buddy/profiles/name)
pub fn display_home() -> String {
    if let Ok(home) = std::env::var("CODE_BUDDY_HOME") {
        home
    } else {
        "~/.code-buddy".to_string()
    }
}
