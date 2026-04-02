//! Task Tools - TaskCreateTool, TodoWriteTool
//!
//! Provides task and todo management tools.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: Priority,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl Task {
    pub fn new(title: &str) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.to_string(),
            description: None,
            status: TaskStatus::Pending,
            priority: Priority::Medium,
            created_at: now,
            updated_at: now,
            completed_at: None,
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }

    pub fn complete(&mut self) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
        self.updated_at = chrono::Utc::now();
    }

    pub fn cancel(&mut self) {
        self.status = TaskStatus::Cancelled;
        self.updated_at = chrono::Utc::now();
    }
}

/// Task list manager
pub struct TaskList {
    tasks: Vec<Task>,
}

impl TaskList {
    pub fn new() -> Self {
        Self { tasks: Vec::new() }
    }

    pub fn add(&mut self, task: Task) -> &Task {
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }

    pub fn get(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    pub fn list(&self) -> &[Task] {
        &self.tasks
    }

    pub fn list_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    pub fn list_by_priority(&self, priority: Priority) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.priority == priority).collect()
    }

    pub fn search(&self, query: &str) -> Vec<&Task> {
        let query_lower = query.to_lowercase();
        self.tasks.iter()
            .filter(|t| {
                t.title.to_lowercase().contains(&query_lower)
                    || t.description.as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || t.tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn complete(&mut self, id: &str) -> bool {
        if let Some(task) = self.get_mut(id) {
            task.complete();
            true
        } else {
            false
        }
    }

    pub fn cancel(&mut self, id: &str) -> bool {
        if let Some(task) = self.get_mut(id) {
            task.cancel();
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: &str) -> bool {
        let len = self.tasks.len();
        self.tasks.retain(|t| t.id != id);
        self.tasks.len() < len
    }

    pub fn stats(&self) -> TaskStats {
        TaskStats {
            total: self.tasks.len(),
            pending: self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count(),
            in_progress: self.tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count(),
            completed: self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count(),
            cancelled: self.tasks.iter().filter(|t| t.status == TaskStatus::Cancelled).count(),
        }
    }

    pub fn clear_completed(&mut self) {
        self.tasks.retain(|t| t.status != TaskStatus::Completed);
    }

    pub fn clear_all(&mut self) {
        self.tasks.clear();
    }
}

impl Default for TaskList {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskStats {
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub cancelled: usize,
}

impl TaskStats {
    pub fn summary(&self) -> String {
        format!(
            "Tasks: {} total ({} pending, {} in progress, {} completed, {} cancelled)",
            self.total, self.pending, self.in_progress, self.completed, self.cancelled
        )
    }
}

/// Format task list as markdown
pub fn format_task_list(tasks: &[&Task], show_stats: bool) -> String {
    let mut md = String::new();

    if show_stats {
        let stats = tasks.iter().fold(
            TaskStats { total: 0, pending: 0, in_progress: 0, completed: 0, cancelled: 0 },
            |mut acc, t| {
                acc.total += 1;
                match t.status {
                    TaskStatus::Pending => acc.pending += 1,
                    TaskStatus::InProgress => acc.in_progress += 1,
                    TaskStatus::Completed => acc.completed += 1,
                    TaskStatus::Cancelled => acc.cancelled += 1,
                }
                acc
            }
        );
        md.push_str(&format!("# Task List ({})\n\n", stats.summary()));
    } else {
        md.push_str("# Task List\n\n");
    }

    // Group by status
    let pending: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::Pending).collect();
    let in_progress: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::InProgress).collect();
    let completed: Vec<_> = tasks.iter().filter(|t| t.status == TaskStatus::Completed).collect();

    if !pending.is_empty() {
        md.push_str("## Pending\n\n");
        for task in pending {
            md.push_str(&format_task_line(task));
        }
        md.push('\n');
    }

    if !in_progress.is_empty() {
        md.push_str("## In Progress\n\n");
        for task in in_progress {
            md.push_str(&format_task_line(task));
        }
        md.push('\n');
    }

    if !completed.is_empty() {
        md.push_str("## Completed\n\n");
        for task in completed {
            md.push_str(&format_task_line(task));
        }
        md.push('\n');
    }

    md
}

fn format_task_line(task: &Task) -> String {
    let checkbox = match task.status {
        TaskStatus::Completed => "[x]",
        TaskStatus::InProgress => "[~]",
        _ => "[ ]",
    };
    let priority_icon = match task.priority {
        Priority::Critical => "🔴",
        Priority::High => "🟠",
        Priority::Medium => "🟡",
        Priority::Low => "⚪",
    };
    let tags = if task.tags.is_empty() {
        String::new()
    } else {
        format!(" [{}]", task.tags.join(", "))
    };
    format!("- {} {} {}{}\n", checkbox, priority_icon, task.title, tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("Test task");
        assert_eq!(task.title, "Test task");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_task_complete() {
        let mut task = Task::new("Test");
        task.complete();
        assert_eq!(task.status, TaskStatus::Completed);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_task_list() {
        let mut list = TaskList::new();
        let task = list.add(Task::new("Task 1"));
        let id = task.id.clone();
        assert_eq!(list.list().len(), 1);
        list.complete(&id);
        assert_eq!(list.stats().completed, 1);
    }

    #[test]
    fn test_task_search() {
        let mut list = TaskList::new();
        list.add(Task::new("Fix bug in login").with_tags(vec!["bug", "urgent"]));
        list.add(Task::new("Add feature").with_tags(vec!["feature"]));
        assert_eq!(list.search("bug").len(), 1);
        assert_eq!(list.search("feature").len(), 1);
        assert_eq!(list.search("urgent").len(), 1);
    }
}
