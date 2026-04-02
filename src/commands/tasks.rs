//! Tasks Command - Task tracking and management
//!
//! Provides task creation, listing, and management.

use crate::tools::task::{Task, TaskList, TaskStats, TaskStatus, Priority, format_task_list};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Task list storage
pub struct TaskStore {
    tasks: TaskList,
    storage_path: Option<PathBuf>,
}

impl TaskStore {
    pub fn new() -> Self {
        Self {
            tasks: TaskList::new(),
            storage_path: None,
        }
    }

    pub fn with_storage(mut self, path: PathBuf) -> Self {
        self.storage_path = Some(path);
        self.load().ok();
        self
    }

    /// Create a new task
    pub fn create(&mut self, title: &str) -> Task {
        let task = Task::new(title);
        self.tasks.add(task.clone());
        self.save().ok();
        task
    }

    /// Create a task with full options
    pub fn create_with_options(
        &mut self,
        title: &str,
        description: Option<&str>,
        priority: Priority,
        tags: Vec<&str>,
    ) -> Task {
        let mut task = Task::new(title);
        if let Some(desc) = description {
            task = task.with_description(desc);
        }
        task = task.with_priority(priority);
        task = task.with_tags(tags);
        self.tasks.add(task.clone());
        self.save().ok();
        task
    }

    /// List all tasks
    pub fn list(&self) -> &[Task] {
        self.tasks.list()
    }

    /// List pending tasks
    pub fn pending(&self) -> Vec<&Task> {
        self.tasks.list_by_status(TaskStatus::Pending)
    }

    /// List in-progress tasks
    pub fn in_progress(&self) -> Vec<&Task> {
        self.tasks.list_by_status(TaskStatus::InProgress)
    }

    /// List completed tasks
    pub fn completed(&self) -> Vec<&Task> {
        self.tasks.list_by_status(TaskStatus::Completed)
    }

    /// Search tasks
    pub fn search(&self, query: &str) -> Vec<&Task> {
        self.tasks.search(query)
    }

    /// Complete a task
    pub fn complete(&mut self, id: &str) -> bool {
        let result = self.tasks.complete(id);
        self.save().ok();
        result
    }

    /// Cancel a task
    pub fn cancel(&mut self, id: &str) -> bool {
        let result = self.tasks.cancel(id);
        self.save().ok();
        result
    }

    /// Delete a task
    pub fn delete(&mut self, id: &str) -> bool {
        let result = self.tasks.remove(id);
        self.save().ok();
        result
    }

    /// Get task statistics
    pub fn stats(&self) -> TaskStats {
        self.tasks.stats()
    }

    /// Clear completed tasks
    pub fn clear_completed(&mut self) {
        self.tasks.clear_completed();
        self.save().ok();
    }

    /// Clear all tasks
    pub fn clear_all(&mut self) {
        self.tasks.clear_all();
        self.save().ok();
    }

    /// Save tasks to disk
    fn save(&self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            let tasks = self.tasks.list();
            let json = serde_json::to_string_pretty(tasks)?;
            std::fs::write(path, json)?;
        }
        Ok(())
    }

    /// Load tasks from disk
    fn load(&mut self) -> Result<()> {
        if let Some(ref path) = self.storage_path {
            if path.exists() {
                let json = std::fs::read_to_string(path)?;
                let tasks: Vec<Task> = serde_json::from_str(&json)?;
                for task in tasks {
                    self.tasks.add(task);
                }
            }
        }
        Ok(())
    }
}

impl Default for TaskStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Task output modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutputMode {
    Summary,
    Detailed,
    Json,
    Markdown,
}

impl Default for TaskOutputMode {
    fn default() -> Self {
        Self::Summary
    }
}

/// Run tasks command
pub fn run(args: &[String]) -> Result<String> {
    let mut store = TaskStore::new();

    if args.is_empty() {
        // List all tasks
        let tasks: Vec<&Task> = store.list().iter().collect();
        return Ok(format_task_list(&tasks, true));
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let tasks: Vec<&Task> = store.list().iter().collect();
            Ok(format_task_list(&tasks, true))
        }
        "pending" => {
            let tasks: Vec<&Task> = store.pending();
            Ok(format_task_list(&tasks, false))
        }
        "completed" | "done" => {
            let tasks: Vec<&Task> = store.completed();
            Ok(format_task_list(&tasks, false))
        }
        "stats" => {
            let stats = store.stats();
            Ok(stats.summary())
        }
        "add" | "create" => {
            if args.len() < 2 {
                return Ok("Usage: tasks add <title> [--priority high|medium|low] [--tags tag1,tag2]".to_string());
            }
            let title = &args[1];
            let task = store.create(title);
            Ok(format!("Created task: {} [{}]", task.title, task.id))
        }
        "complete" | "done" | "finish" => {
            if args.len() < 2 {
                return Ok("Usage: tasks complete <id>".to_string());
            }
            if store.complete(&args[1]) {
                Ok(format!("Task {} completed", args[1]))
            } else {
                Ok(format!("Task {} not found", args[1]))
            }
        }
        "cancel" => {
            if args.len() < 2 {
                return Ok("Usage: tasks cancel <id>".to_string());
            }
            if store.cancel(&args[1]) {
                Ok(format!("Task {} cancelled", args[1]))
            } else {
                Ok(format!("Task {} not found", args[1]))
            }
        }
        "delete" | "remove" => {
            if args.len() < 2 {
                return Ok("Usage: tasks delete <id>".to_string());
            }
            if store.delete(&args[1]) {
                Ok(format!("Task {} deleted", args[1]))
            } else {
                Ok(format!("Task {} not found", args[1]))
            }
        }
        "clear" => {
            store.clear_completed();
            Ok("Cleared completed tasks".to_string())
        }
        "search" | "find" => {
            if args.len() < 2 {
                return Ok("Usage: tasks search <query>".to_string());
            }
            let tasks: Vec<&Task> = store.search(&args[1]);
            Ok(format_task_list(&tasks, false))
        }
        _ => {
            Ok(format!("Unknown command: {}\n\nUsage: tasks <list|add|complete|cancel|delete|search>", args[0]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_store() {
        let mut store = TaskStore::new();
        let task = store.create("Test task");
        assert_eq!(store.list().len(), 1);
        assert!(store.complete(&task.id));
    }

    #[test]
    fn test_task_search() {
        let mut store = TaskStore::new();
        store.create("Fix login bug");
        store.create("Add new feature");
        assert_eq!(store.search("bug").len(), 1);
        assert_eq!(store.search("feature").len(), 1);
    }

    #[test]
    fn test_task_stats() {
        let mut store = TaskStore::new();
        store.create("Task 1");
        store.create("Task 2");
        let task = store.create("Task 3");
        store.complete(&task.id);
        let stats = store.stats();
        assert_eq!(stats.total, 3);
        assert_eq!(stats.completed, 1);
    }
}
