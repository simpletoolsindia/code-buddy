//! Memory System
//!
//! Provides persistent project memory storage that survives across sessions.
//! Memory is stored in .claude/memory/ directory within each project.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: String,
    pub category: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Project memory storage
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectMemory {
    pub entries: HashMap<String, MemoryEntry>,
    pub project_path: Option<String>,
}

impl ProjectMemory {
    /// Create new empty memory
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            project_path: None,
        }
    }

    /// Get memory directory for a project
    pub fn memory_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("code-buddy").join("memory"))
    }

    /// Get memory file path for a project
    fn memory_file_path(project_path: &std::path::Path) -> PathBuf {
        let project_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("default");

        let safe_name = project_name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>();

        Self::memory_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config/code-buddy/memory"))
            .join(format!("{}.json", safe_name))
    }

    /// Load memory for a project
    pub fn load(project_path: &std::path::Path) -> Result<Self> {
        let path = Self::memory_file_path(project_path);

        if !path.exists() {
            let mut mem = Self::new();
            mem.project_path = project_path.to_str().map(String::from);
            return Ok(mem);
        }

        let content = fs::read_to_string(&path)?;
        let mut mem: Self = serde_json::from_str(&content)?;
        mem.project_path = project_path.to_str().map(String::from);
        Ok(mem)
    }

    /// Save memory to disk
    pub fn save(&self, project_path: &std::path::Path) -> Result<()> {
        let path = Self::memory_file_path(project_path);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Set a memory entry
    pub fn set(&mut self, key: &str, value: &str, category: Option<&str>) {
        let entry = MemoryEntry {
            key: key.to_string(),
            value: value.to_string(),
            category: category.map(String::from),
            updated_at: chrono::Utc::now(),
        };
        self.entries.insert(key.to_string(), entry);
    }

    /// Get a memory entry
    pub fn get(&self, key: &str) -> Option<&MemoryEntry> {
        self.entries.get(key)
    }

    /// Delete a memory entry
    pub fn delete(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }

    /// List all keys (optionally filtered by category)
    pub fn keys(&self, category: Option<&str>) -> Vec<&String> {
        match category {
            Some(cat) => self
                .entries
                .values()
                .filter(|e| e.category.as_deref() == Some(cat))
                .map(|e| &e.key)
                .collect(),
            None => self.entries.keys().collect(),
        }
    }

    /// Search memory entries
    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .values()
            .filter(|e| {
                e.key.to_lowercase().contains(&query_lower)
                    || e.value.to_lowercase().contains(&query_lower)
                    || e.category
                        .as_ref()
                        .map(|c| c.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Export as markdown
    pub fn to_markdown(&self) -> String {
        if self.entries.is_empty() {
            return "# Memory\n\nNo memory entries.\n".to_string();
        }

        let mut md = String::from("# Memory\n\n");

        // Group by category
        let mut categories: HashMap<String, Vec<&MemoryEntry>> = HashMap::new();
        for entry in self.entries.values() {
            let cat = entry.category.clone().unwrap_or_else(|| "General".to_string());
            categories.entry(cat).or_default().push(entry);
        }

        for (cat, entries) in categories {
            md.push_str(&format!("## {}\n\n", cat));
            for entry in entries {
                md.push_str(&format!("### {}\n\n{}\n\n", entry.key, entry.value));
            }
        }

        md
    }
}

/// Run memory command
pub fn run(args: &[String]) -> Result<()> {
    let action = args.first().map(|s| s.as_str()).unwrap_or("list");

    let project_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    match action {
        "get" => {
            let key = args.get(1).ok_or_else(|| anyhow::anyhow!("Usage: memory get <key>"))?;
            let mut mem = ProjectMemory::load(&project_path)?;
            if let Some(entry) = mem.get(key) {
                println!("# {}", entry.key);
                if let Some(ref cat) = entry.category {
                    println!("Category: {}\n", cat);
                }
                println!("{}", entry.value);
            } else {
                println!("No memory entry found for key: {}", key);
            }
        }

        "set" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: memory set <key> <value> [--category <cat>]");
            }
            let key = &args[1];
            let value = &args[2];
            let category = args
                .iter()
                .position(|s| s == "--category")
                .and_then(|i| args.get(i + 1).map(String::from));

            let mut mem = ProjectMemory::load(&project_path)?;
            mem.set(key, value, category.as_deref());
            mem.save(&project_path)?;
            println!("Memory saved: {} = {}", key, value);
        }

        "delete" | "del" => {
            let key = args.get(1).ok_or_else(|| anyhow::anyhow!("Usage: memory delete <key>"))?;
            let mut mem = ProjectMemory::load(&project_path)?;
            if mem.delete(key) {
                mem.save(&project_path)?;
                println!("Deleted memory entry: {}", key);
            } else {
                println!("No memory entry found for key: {}", key);
            }
        }

        "list" | "ls" => {
            let category = args
                .iter()
                .position(|s| s == "--category")
                .and_then(|i| args.get(i + 1).map(String::from));

            let mem = ProjectMemory::load(&project_path)?;
            let keys = mem.keys(category.as_deref());

            if keys.is_empty() {
                println!("No memory entries.");
                return Ok(());
            }

            println!("Memory entries:\n");
            for key in keys {
                if let Some(entry) = mem.get(key) {
                    if let Some(ref cat) = entry.category {
                        println!("  [{}] {}", cat, key);
                    } else {
                        println!("  {}", key);
                    }
                }
            }
        }

        "search" => {
            let query = args.get(1).ok_or_else(|| anyhow::anyhow!("Usage: memory search <query>"))?;
            let mem = ProjectMemory::load(&project_path)?;
            let results = mem.search(query);

            if results.is_empty() {
                println!("No matching memory entries for: {}", query);
            } else {
                println!("Memory search results for '{}':\n", query);
                for entry in results {
                    println!("## {} {}\n", entry.key,
                        entry.category.as_ref().map(|c| format!("[{}]", c)).unwrap_or_default());
                    println!("{}\n", entry.value);
                }
            }
        }

        "clear" => {
            let mut mem = ProjectMemory::load(&project_path)?;
            let count = mem.entries.len();
            mem.clear();
            mem.save(&project_path)?;
            println!("Cleared {} memory entries.", count);
        }

        "export" => {
            let mem = ProjectMemory::load(&project_path)?;
            println!("{}", mem.to_markdown());
        }

        _ => {
            println!("Memory commands:");
            println!("  memory list              - List all memory entries");
            println!("  memory get <key>        - Get a specific memory entry");
            println!("  memory set <key> <value> [--category <cat>] - Set a memory entry");
            println!("  memory delete <key>     - Delete a memory entry");
            println!("  memory search <query>   - Search memory entries");
            println!("  memory clear            - Clear all memory entries");
            println!("  memory export           - Export as markdown");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_memory_operations() {
        let temp_dir = std::env::temp_dir().join("code-buddy-test-memory");
        std::fs::create_dir_all(&temp_dir).ok();

        let mut mem = ProjectMemory::new();
        mem.set("test_key", "test_value", Some("testing"));
        mem.set("another_key", "another_value", None);

        assert_eq!(mem.get("test_key").unwrap().value, "test_value");
        assert_eq!(mem.keys(None).len(), 2);
        assert_eq!(mem.keys(Some("testing")).len(), 1);

        assert!(mem.delete("test_key"));
        assert_eq!(mem.keys(None).len(), 1);

        // Cleanup
        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_memory_search() {
        let mut mem = ProjectMemory::new();
        mem.set("rust_project", "A Rust project", Some("projects"));
        mem.set("python_project", "A Python project", Some("projects"));
        mem.set("todo", "Remember to fix bug", None);

        let results = mem.search("rust");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "rust_project");

        let results = mem.search("project");
        assert_eq!(results.len(), 2);

        let results = mem.search("bug");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_memory_to_markdown() {
        let mut mem = ProjectMemory::new();
        mem.set("key1", "value1", Some("cat1"));
        mem.set("key2", "value2", Some("cat2"));

        let md = mem.to_markdown();
        assert!(md.contains("# Memory"));
        assert!(md.contains("## cat1"));
        assert!(md.contains("## cat2"));
    }
}
