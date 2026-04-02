//! Session Command - Session management
//!
//! Provides session listing, loading, and management.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub last_active: u64,
    pub message_count: usize,
    pub size_bytes: usize,
}

impl Session {
    pub fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            created_at: now,
            last_active: now,
            message_count: 0,
            size_bytes: 0,
        }
    }

    pub fn age_days(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.last_active) / 86400
    }
}

/// Session manager
pub struct SessionManager {
    storage_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let storage_dir = PathBuf::from(home).join(".config/code-buddy/sessions");
        std::fs::create_dir_all(&storage_dir).ok();
        Self { storage_dir }
    }

    pub fn list(&self) -> Vec<Session> {
        let mut sessions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.storage_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(session) = serde_json::from_str::<Session>(&content) {
                            sessions.push(session);
                        }
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.last_active.cmp(&a.last_active));
        sessions
    }

    pub fn save(&self, session: &Session) -> Result<PathBuf> {
        let path = self.storage_dir.join(format!("{}.json", session.id));
        let json = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, json)?;
        Ok(path)
    }

    pub fn load(&self, id: &str) -> Result<Session> {
        let path = self.storage_dir.join(format!("{}.json", id));
        let content = std::fs::read_to_string(path)?;
        let session: Session = serde_json::from_str(&content)?;
        Ok(session)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let path = self.storage_dir.join(format!("{}.json", id));
        std::fs::remove_file(path)?;
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Run session command
pub fn run(args: &[String]) -> Result<String> {
    let manager = SessionManager::new();

    if args.is_empty() {
        return list_sessions(&manager);
    }

    match args[0].as_str() {
        "list" | "ls" => list_sessions(&manager),
        "load" => {
            if args.len() < 2 {
                return Ok("Usage: session load <id>".to_string());
            }
            load_session(&manager, &args[1])
        }
        "save" => {
            if args.len() < 2 {
                return Ok("Usage: session save <name>".to_string());
            }
            save_session(&manager, &args[1])
        }
        "delete" | "rm" => {
            if args.len() < 2 {
                return Ok("Usage: session delete <id>".to_string());
            }
            delete_session(&manager, &args[1])
        }
        "rename" => {
            if args.len() < 3 {
                return Ok("Usage: session rename <id> <new-name>".to_string());
            }
            rename_session(&manager, &args[1], &args[2])
        }
        _ => {
            Ok(format!("Unknown session command: {}\n\nUsage: session <list|load|save|delete|rename>", args[0]))
        }
    }
}

fn list_sessions(manager: &SessionManager) -> Result<String> {
    let sessions = manager.list();

    if sessions.is_empty() {
        return Ok("# Sessions\n\nNo saved sessions.\n".to_string());
    }

    let mut output = String::from("# Saved Sessions\n\n");
    output.push_str("| ID | Name | Messages | Age |\n");
    output.push_str("|----|------|----------|-----|\n");

    for session in sessions {
        output.push_str(&format!(
            "| {} | {} | {} | {}d |\n",
            &session.id[..8],
            session.name,
            session.message_count,
            session.age_days()
        ));
    }

    Ok(output)
}

fn load_session(manager: &SessionManager, id: &str) -> Result<String> {
    match manager.load(id) {
        Ok(session) => Ok(format!(
            "# Session: {}\n\nMessages: {}\nCreated: {}",
            session.name,
            session.message_count,
            session.created_at
        )),
        Err(_) => Ok(format!("Session not found: {}\n", id)),
    }
}

fn save_session(manager: &SessionManager, name: &str) -> Result<String> {
    let session = Session::new(name);
    let path = manager.save(&session)?;
    Ok(format!("Saved session as: {}\nPath: {}", name, path.display()))
}

fn delete_session(manager: &SessionManager, id: &str) -> Result<String> {
    manager.delete(id)?;
    Ok(format!("Deleted session: {}\n", id))
}

fn rename_session(manager: &SessionManager, id: &str, new_name: &str) -> Result<String> {
    let mut session = manager.load(id)?;
    session.name = new_name.to_string();
    session.last_active = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    manager.save(&session)?;
    Ok(format!("Renamed session to: {}\n", new_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("Test Session");
        assert_eq!(session.name, "Test Session");
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_session_age() {
        let session = Session::new("Test");
        assert_eq!(session.age_days(), 0);
    }
}
