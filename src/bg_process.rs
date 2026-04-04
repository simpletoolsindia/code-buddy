//! Background Process Management
//!
//! Run commands in background with status notifications.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};

/// Safely lock mutex, handling poisoned state
macro_rules! safe_lock {
    ($mutex:expr) => {
        match $mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    };
}

/// Background process status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProcessStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Background process info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundProcess {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub check_interval_secs: Option<u64>,
}

/// Process registry
pub struct ProcessRegistry {
    processes: Arc<Mutex<HashMap<String, BackgroundProcess>>>,
    children: Arc<Mutex<HashMap<String, Child>>>,
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start a background process
    pub async fn start(
        &self,
        command: &str,
        args: Vec<String>,
        working_dir: Option<&str>,
        check_interval: Option<u64>,
    ) -> Result<String> {
        let id = nanoid::nanoid!(8);

        let mut cmd = Command::new(command);
        cmd.args(&args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.kill_on_drop(true);

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let child = cmd.spawn()?;

        let pid = child.id();
        let now = chrono::Local::now().to_rfc3339();

        let process = BackgroundProcess {
            id: id.clone(),
            command: command.to_string(),
            args: args.clone(),
            working_dir: working_dir.map(String::from),
            status: ProcessStatus::Running,
            pid,
            exit_code: None,
            started_at: now,
            completed_at: None,
            check_interval_secs: check_interval,
        };

        safe_lock!(&self.processes).insert(id.clone(), process);
        safe_lock!(&self.children).insert(id.clone(), child);

        Ok(id)
    }

    /// Check process status
    pub fn check(&self, id: &str) -> Option<BackgroundProcess> {
        let mut processes = safe_lock!(&self.processes);

        if let Some(process) = processes.get_mut(id) {
            // Try to poll child
            if process.pid.is_some() {
                let mut children = safe_lock!(&self.children);
                if let Some(child) = children.get_mut(id) {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            process.status = if status.success() {
                                ProcessStatus::Completed
                            } else {
                                ProcessStatus::Failed
                            };
                            process.exit_code = status.code();
                            process.completed_at = Some(chrono::Local::now().to_rfc3339());
                        }
                        Ok(None) => {
                            process.status = ProcessStatus::Running;
                        }
                        Err(_) => {
                            process.status = ProcessStatus::Failed;
                        }
                    }
                }
            }
        }

        processes.get(id).cloned()
    }

    /// Cancel a process
    pub fn cancel(&self, id: &str) -> Result<()> {
        let mut children = safe_lock!(&self.children);
        if let Some(mut child) = children.remove(id) {
            child.start_kill()?;
        }

        let mut processes = safe_lock!(&self.processes);
        if let Some(process) = processes.get_mut(id) {
            process.status = ProcessStatus::Cancelled;
            process.completed_at = Some(chrono::Local::now().to_rfc3339());
        }

        Ok(())
    }

    /// Get output of a process
    pub async fn get_output(&self, id: &str) -> Result<(String, String)> {
        let (stdout, stderr) = {
            let mut children = safe_lock!(&self.children);
            if let Some(child) = children.get_mut(id) {
                (child.stdout.take(), child.stderr.take())
            } else {
                return Ok((String::new(), String::new()));
            }
        };

        let out = if let Some(mut stdout) = stdout {
            let mut buf = vec![];
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut stdout, &mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        } else {
            String::new()
        };

        let err = if let Some(mut stderr) = stderr {
            let mut buf = vec![];
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut stderr, &mut buf).await;
            String::from_utf8_lossy(&buf).to_string()
        } else {
            String::new()
        };

        Ok((out, err))
    }

    /// List all processes
    pub fn list(&self) -> Vec<BackgroundProcess> {
        let processes = safe_lock!(&self.processes);
        processes.values().cloned().collect()
    }

    /// Cleanup completed processes
    pub fn cleanup(&self) {
        let mut processes = safe_lock!(&self.processes);
        let mut children = safe_lock!(&self.children);

        processes.retain(|id, p| {
            if p.status != ProcessStatus::Running {
                children.remove(id);
                false
            } else {
                true
            }
        });
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Distributed Execution System
/// Worker node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerNode {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub status: WorkerStatus,
    pub current_tasks: usize,
    pub max_tasks: usize,
    pub last_heartbeat: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    Online,
    Busy,
    Offline,
    Unknown,
}

/// Distributed task queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTask {
    pub id: String,
    pub description: String,
    pub command: String,
    pub args: Vec<String>,
    pub priority: u8, // 0 = lowest, 255 = highest
    pub status: DistributedTaskStatus,
    pub assigned_to: Option<String>,
    pub result: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub worker_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistributedTaskStatus {
    Queued,
    Dispatched,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl DistributedTask {
    pub fn new(command: &str, args: Vec<String>) -> Self {
        Self {
            id: nanoid::nanoid!(12),
            description: format!("{} {:?}", command, args),
            command: command.to_string(),
            args,
            priority: 128,
            status: DistributedTaskStatus::Queued,
            assigned_to: None,
            result: None,
            created_at: chrono::Local::now().to_rfc3339(),
            started_at: None,
            completed_at: None,
            worker_id: None,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }
}

/// Task queue for distributed execution
#[derive(Debug, Clone, Default)]
pub struct TaskQueue {
    pub tasks: Vec<DistributedTask>,
    pub workers: Vec<WorkerNode>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a task to the queue
    pub fn enqueue(&mut self, task: DistributedTask) {
        self.tasks.push(task);
        // Sort by priority (highest first)
        self.tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Get next task for a worker
    pub fn dequeue(&mut self, worker_id: &str) -> Option<DistributedTask> {
        if let Some(pos) = self.tasks.iter().position(|t| t.status == DistributedTaskStatus::Queued) {
            let mut task = self.tasks.remove(pos);
            task.status = DistributedTaskStatus::Dispatched;
            task.assigned_to = Some(worker_id.to_string());
            task.started_at = Some(chrono::Local::now().to_rfc3339());
            Some(task)
        } else {
            None
        }
    }

    /// Register a worker
    pub fn register_worker(&mut self, worker: WorkerNode) {
        self.workers.push(worker);
    }

    /// Remove offline workers
    pub fn cleanup_workers(&mut self) {
        self.workers.retain(|w| w.status != WorkerStatus::Offline);
    }

    /// Get queue status
    pub fn status(&self) -> QueueStatus {
        let total = self.tasks.len();
        let queued = self.tasks.iter().filter(|t| t.status == DistributedTaskStatus::Queued).count();
        let running = self.tasks.iter().filter(|t| t.status == DistributedTaskStatus::Running).count();
        let completed = self.tasks.iter().filter(|t| t.status == DistributedTaskStatus::Completed).count();
        let failed = self.tasks.iter().filter(|t| t.status == DistributedTaskStatus::Failed).count();

        QueueStatus {
            total_tasks: total,
            queued,
            running,
            completed,
            failed,
            workers_online: self.workers.iter().filter(|w| w.status == WorkerStatus::Online).count(),
            workers_busy: self.workers.iter().filter(|w| w.status == WorkerStatus::Busy).count(),
        }
    }
}

/// Queue status report
#[derive(Debug, Clone, Serialize)]
pub struct QueueStatus {
    pub total_tasks: usize,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub workers_online: usize,
    pub workers_busy: usize,
}

impl QueueStatus {
    pub fn progress_percent(&self) -> f64 {
        if self.total_tasks == 0 {
            return 100.0;
        }
        (self.completed as f64 / self.total_tasks as f64) * 100.0
    }

    pub fn summary(&self) -> String {
        format!(
            "Queue: {} total ({} queued, {} running, {} done, {} failed) | Workers: {} online, {} busy",
            self.total_tasks,
            self.queued,
            self.running,
            self.completed,
            self.failed,
            self.workers_online,
            self.workers_busy
        )
    }
}

/// Progress tracker
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    total: usize,
    completed: usize,
    failed: usize,
    start_time: std::time::Instant,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self {
            total: 0,
            completed: 0,
            failed: 0,
            start_time: std::time::Instant::now(),
        }
    }
}

impl ProgressTracker {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: 0,
            failed: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn increment_completed(&mut self) {
        self.completed += 1;
    }

    pub fn increment_failed(&mut self) {
        self.failed += 1;
    }

    pub fn progress_percent(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        ((self.completed + self.failed) as f64 / self.total as f64) * 100.0
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    pub fn estimated_remaining_secs(&self) -> f64 {
        if self.completed == 0 {
            return 0.0;
        }
        let elapsed = self.elapsed_secs();
        let per_task = elapsed / self.completed as f64;
        let remaining = self.total - self.completed - self.failed;
        per_task * remaining as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_background_process() {
        let registry = ProcessRegistry::new();
        let id = registry.start("echo", vec!["hello".to_string()], None, None).await.unwrap();
        assert!(!id.is_empty());

        // Check status
        let process = registry.check(&id);
        assert!(process.is_some());
        assert_eq!(process.unwrap().command, "echo");
    }

    #[test]
    fn test_task_queue() {
        let mut queue = TaskQueue::new();

        // Add tasks
        queue.enqueue(DistributedTask::new("echo", vec!["1".to_string()]).with_priority(100));
        queue.enqueue(DistributedTask::new("echo", vec!["2".to_string()]).with_priority(200));
        queue.enqueue(DistributedTask::new("echo", vec!["3".to_string()]).with_priority(50));

        // Higher priority should come first
        let task = queue.dequeue("worker1").unwrap();
        assert_eq!(task.priority, 200);
    }

    #[test]
    fn test_progress_tracker() {
        let mut tracker = ProgressTracker::new(10);
        tracker.increment_completed();
        tracker.increment_completed();
        assert_eq!(tracker.progress_percent(), 20.0);
    }
}
