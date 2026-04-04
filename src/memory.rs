//! Advanced Memory System with SQLite FTS5 Full-Text Search
//!
//! Provides persistent memory with:
//! - FTS5 full-text search across all memories
//! - Session-based context
//! - User profiles
//! - Dialectic Q&A (Honcho-style)
//! - Semantic search with LLM summarization

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use chrono::Local;

/// Safely lock mutex, handling poisoned state
/// Returns the guard, recovering from poison if necessary
macro_rules! safe_lock {
    ($mutex:expr) => {
        match $mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    };
}

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub session_id: Option<String>,
    pub key: String,
    pub value: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub importance: i32,
}

/// User profile entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: i64,
    pub category: String,  // "fact", "preference", "context", "habit"
    pub key: String,
    pub value: String,
    pub confidence: f32,
    pub source: String,  // "direct", "inferred", "session"
    pub created_at: String,
}

/// Session context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub project: Option<String>,
    pub working_dir: Option<String>,
    pub started_at: String,
    pub message_count: i32,
}

/// Memory search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchResult {
    pub id: i64,
    pub key: String,
    pub snippet: String,
    pub rank: f64,
    pub tags: Vec<String>,
}

/// Memory system
pub struct MemorySystem {
    conn: Mutex<Connection>,
}

impl MemorySystem {
    /// Create a new memory system
    pub fn new(home: Option<PathBuf>) -> Result<Self> {
        let db_path = home
            .unwrap_or_else(|| crate::dirs::memory_dir().unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".code-buddy").join("memory"))
                    .unwrap_or_else(|| PathBuf::from("~/.code-buddy/memory"))
            }))
            .join("memory.db");

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // Enable FTS5
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             -- Main memories table
             CREATE TABLE IF NOT EXISTS memories (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id TEXT,
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 tags TEXT DEFAULT '[]',
                 importance INTEGER DEFAULT 5,
                 created_at TEXT NOT NULL,
                 updated_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
             CREATE INDEX IF NOT EXISTS idx_memories_key ON memories(key);

             -- FTS5 virtual table for full-text search
             CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                 key, value, tags,
                 content='memories',
                 content_rowid='id',
                 tokenize='porter unicode61'
             );

             -- Triggers to keep FTS in sync
             CREATE TRIGGER IF NOT EXISTS memories_fts_insert AFTER INSERT ON memories BEGIN
                 INSERT INTO memories_fts(rowid, key, value, tags) VALUES (new.id, new.key, new.value, new.tags);
             END;
             CREATE TRIGGER IF NOT EXISTS memories_fts_delete AFTER DELETE ON memories BEGIN
                 INSERT INTO memories_fts(memories_fts, rowid, key, value, tags) VALUES('delete', old.id, old.key, old.value, old.tags);
             END;
             CREATE TRIGGER IF NOT EXISTS memories_fts_update AFTER UPDATE ON memories BEGIN
                 INSERT INTO memories_fts(memories_fts, rowid, key, value, tags) VALUES('delete', old.id, old.key, old.value, old.tags);
                 INSERT INTO memories_fts(rowid, key, value, tags) VALUES (new.id, new.key, new.value, new.tags);
             END;

             -- User profiles table
             CREATE TABLE IF NOT EXISTS user_profiles (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 category TEXT NOT NULL,
                 key TEXT NOT NULL,
                 value TEXT NOT NULL,
                 confidence REAL DEFAULT 1.0,
                 source TEXT DEFAULT 'direct',
                 created_at TEXT NOT NULL,
                 UNIQUE(category, key)
             );

             -- Session context table
             CREATE TABLE IF NOT EXISTS sessions (
                 session_id TEXT PRIMARY KEY,
                 project TEXT,
                 working_dir TEXT,
                 started_at TEXT NOT NULL,
                 message_count INTEGER DEFAULT 0
             );

             -- Trajectory/session search table
             CREATE TABLE IF NOT EXISTS session_messages (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id TEXT NOT NULL,
                 role TEXT NOT NULL,
                 content TEXT NOT NULL,
                 timestamp TEXT NOT NULL,
                 FOREIGN KEY(session_id) REFERENCES sessions(session_id)
             );
             CREATE INDEX IF NOT EXISTS idx_msg_session ON session_messages(session_id);

             -- FTS for session messages
             CREATE VIRTUAL TABLE IF NOT EXISTS session_fts USING fts5(
                 content, role,
                 content='session_messages',
                 content_rowid='id',
                 tokenize='porter unicode61'
             );

             CREATE TRIGGER IF NOT EXISTS session_fts_insert AFTER INSERT ON session_messages BEGIN
                 INSERT INTO session_fts(rowid, content, role) VALUES (new.id, new.content, new.role);
             END;
            "
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Store a memory entry
    pub fn store(&self, key: &str, value: &str, tags: Vec<String>, importance: i32, session_id: Option<&str>) -> Result<i64> {
        let now = Local::now().to_rfc3339();
        let tags_json = serde_json::to_string(&tags)?;

        let conn = safe_lock!(&self.conn);
        conn.execute(
            "INSERT INTO memories (session_id, key, value, tags, importance, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![session_id, key, value, tags_json, importance, now, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Search memories using FTS5
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>> {
        let conn = safe_lock!(&self.conn);

        // Sanitize FTS5 query to prevent injection attacks
        // FTS5 special characters: " * ( ) : ^ -
        let safe_query = query
            .replace('"', "\"\"")  // Escape double quotes
            .replace(['*', '(', ')', ':', '^'], " ")
            .trim()
            .to_string();

        if safe_query.is_empty() {
            return Ok(vec![]);
        }

        // Use FTS5 MATCH for full-text search with sanitized query
        let mut stmt = conn.prepare(
            "SELECT m.id, m.key, snippet(memories_fts, 1, '<mark>', '</mark>', '...', 32) as snippet,
                    bm25(memories_fts) as rank, m.tags
             FROM memories_fts
             JOIN memories m ON memories_fts.rowid = m.id
             WHERE memories_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![safe_query, limit as i64], |row| {
            let tags_str: String = row.get(4)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemorySearchResult {
                id: row.get(0)?,
                key: row.get(1)?,
                snippet: row.get(2)?,
                rank: row.get(3)?,
                tags,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(results)
    }

    /// Get memory by key
    pub fn get(&self, key: &str) -> Result<Option<MemoryEntry>> {
        let conn = safe_lock!(&self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, session_id, key, value, tags, created_at, updated_at, importance FROM memories WHERE key = ?1"
        )?;

        let result = stmt.query_row(params![key], |row| {
            let tags_str: String = row.get(4)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                tags,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                importance: row.get(7)?,
            })
        }).optional()?;

        Ok(result)
    }

    /// List all memories
    pub fn list(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        let conn = safe_lock!(&self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, session_id, key, value, tags, created_at, updated_at, importance
             FROM memories ORDER BY updated_at DESC LIMIT ?1"
        )?;

        let entries = stmt.query_map(params![limit as i64], |row| {
            let tags_str: String = row.get(4)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(MemoryEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                tags,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                importance: row.get(7)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(entries)
    }

    /// Delete a memory
    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = safe_lock!(&self.conn);
        conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Update a memory
    pub fn update(&self, id: i64, value: &str, tags: Vec<String>) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let tags_json = serde_json::to_string(&tags)?;

        let conn = safe_lock!(&self.conn);
        conn.execute(
            "UPDATE memories SET value = ?1, tags = ?2, updated_at = ?3 WHERE id = ?4",
            params![value, tags_json, now, id],
        )?;
        Ok(())
    }

    /// Store session message
    pub fn store_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let conn = safe_lock!(&self.conn);
        conn.execute(
            "INSERT INTO session_messages (session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now],
        )?;
        // Update message count
        conn.execute(
            "INSERT OR REPLACE INTO sessions (session_id, message_count) VALUES (?1, message_count + 1)
             ON CONFLICT(session_id) DO UPDATE SET message_count = message_count + 1",
            params![session_id],
        )?;
        Ok(())
    }

    /// Search session history
    pub fn search_sessions(&self, query: &str, limit: usize) -> Result<Vec<MemorySearchResult>> {
        let conn = safe_lock!(&self.conn);
        let mut stmt = conn.prepare(
            "SELECT m.id, s.session_id, snippet(session_fts, 0, '<mark>', '</mark>', '...', 32) as snippet,
                    bm25(session_fts) as rank, s.session_id
             FROM session_fts
             JOIN session_messages m ON session_fts.rowid = m.id
             JOIN sessions s ON m.session_id = s.session_id
             WHERE session_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2"
        )?;

        let results = stmt.query_map(params![query, limit as i64], |row| {
            Ok(MemorySearchResult {
                id: row.get(0)?,
                key: row.get(1)?,
                snippet: row.get(2)?,
                rank: row.get(3)?,
                tags: vec![],
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(results)
    }

    /// Store user profile fact
    pub fn store_profile(&self, category: &str, key: &str, value: &str, confidence: f32, source: &str) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let conn = safe_lock!(&self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO user_profiles (category, key, value, confidence, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![category, key, value, confidence, source, now],
        )?;
        Ok(())
    }

    /// Get user profile
    pub fn get_profile(&self, category: Option<&str>) -> Result<Vec<UserProfile>> {
        let conn = safe_lock!(&self.conn);
        let query = if category.is_some() {
            "SELECT id, category, key, value, confidence, source, created_at FROM user_profiles WHERE category = ?1"
        } else {
            "SELECT id, category, key, value, confidence, source, created_at FROM user_profiles"
        };

        let mut stmt = conn.prepare(query)?;
        let profiles = if let Some(cat) = category {
            stmt.query_map(params![cat], |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    category: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    confidence: row.get(4)?,
                    source: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?.filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map([], |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    category: row.get(1)?,
                    key: row.get(2)?,
                    value: row.get(3)?,
                    confidence: row.get(4)?,
                    source: row.get(5)?,
                    created_at: row.get(6)?,
                })
            })?.filter_map(|r| r.ok()).collect()
        };

        Ok(profiles)
    }

    /// Context query (dialectic Q&A - searches both memories and sessions)
    pub fn context_query(&self, query: &str) -> Result<String> {
        let mem_results = self.search(query, 5)?;
        let session_results = self.search_sessions(query, 5)?;

        let mut response = format!("# Context Query: {}\n\n", query);

        if !mem_results.is_empty() {
            response.push_str("## Relevant Memories\n\n");
            for r in &mem_results {
                response.push_str(&format!("- **[{}]({})**: {}\n", r.key, r.id, r.snippet));
            }
            response.push('\n');
        }

        if !session_results.is_empty() {
            response.push_str("## Relevant Sessions\n\n");
            for r in &session_results {
                response.push_str(&format!("- **Session {}**: {}\n", r.key, r.snippet));
            }
            response.push('\n');
        }

        if mem_results.is_empty() && session_results.is_empty() {
            response.push_str("No relevant context found.\n");
        }

        Ok(response)
    }
}

/// Extension trait for optional query results
trait OptionalExt<T> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalExt<T> for std::result::Result<T, rusqlite::Error> {
    fn optional(self) -> std::result::Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_system() {
        let mem = MemorySystem::new(Some(PathBuf::from("/tmp/test-code-buddy-memory"))).unwrap();

        // Store a memory
        let id = mem.store("test_key", "test value", vec!["test".to_string()], 5, None).unwrap();
        assert!(id > 0);

        // Search
        let results = mem.search("test", 10).unwrap();
        assert!(!results.is_empty());

        // List
        let entries = mem.list(10).unwrap();
        assert!(!entries.is_empty());

        // Get
        let entry = mem.get("test_key").unwrap();
        assert!(entry.is_some());

        // Clean up
        mem.delete(id).unwrap();
    }

    #[test]
    fn test_memory_update() {
        let mem = MemorySystem::new(Some(PathBuf::from("/tmp/test-code-buddy-memory-update"))).unwrap();

        // Store a memory
        let id = mem.store("update_key", "original value", vec!["tag1".to_string()], 5, None).unwrap();

        // Update the memory
        let update_result = mem.update(id, "updated value", vec!["tag2".to_string()]);
        assert!(update_result.is_ok());

        // Verify update
        let entry = mem.get("update_key").unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value, "updated value");

        // Clean up
        mem.delete(id).unwrap();
    }

    #[test]
    fn test_memory_search_with_limit() {
        let mem = MemorySystem::new(Some(PathBuf::from("/tmp/test-code-buddy-memory-limit"))).unwrap();

        // Store multiple memories
        mem.store("search_key_1", "content about rust programming", vec!["rust".to_string()], 5, None).unwrap();
        mem.store("search_key_2", "content about python programming", vec!["python".to_string()], 5, None).unwrap();

        // Search with limit
        let results = mem.search("programming", 1).unwrap();
        assert!(results.len() <= 1);

        // Search without explicit limit
        let results_all = mem.search("programming", 100).unwrap();
        assert!(!results_all.is_empty());
    }

    #[test]
    fn test_memory_nonexistent() {
        let mem = MemorySystem::new(Some(PathBuf::from("/tmp/test-code-buddy-memory-nonexistent"))).unwrap();

        // Get non-existent memory
        let entry = mem.get("nonexistent_key").unwrap();
        assert!(entry.is_none());
    }
}
