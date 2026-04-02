//! Background Process Management
//!
//! Run commands in background with status notifications.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

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

        let mut child = cmd.spawn()?;

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

        self.processes.lock().unwrap().insert(id.clone(), process);
        self.children.lock().unwrap().insert(id.clone(), child);

        Ok(id)
    }

    /// Check process status
    pub fn check(&self, id: &str) -> Option<BackgroundProcess> {
        let mut processes = self.processes.lock().unwrap();

        if let Some(process) = processes.get_mut(id) {
            // Try to poll child
            if let Some(child_id) = process.pid {
                let mut children = self.children.lock().unwrap();
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
        let mut children = self.children.lock().unwrap();
        if let Some(mut child) = children.remove(id) {
            child.start_kill()?;
        }

        let mut processes = self.processes.lock().unwrap();
        if let Some(process) = processes.get_mut(id) {
            process.status = ProcessStatus::Cancelled;
            process.completed_at = Some(chrono::Local::now().to_rfc3339());
        }

        Ok(())
    }

    /// Get output of a process
    pub async fn get_output(&self, id: &str) -> Result<(String, String)> {
        let mut children = self.children.lock().unwrap();
        if let Some(child) = children.get_mut(id) {
            let out = if let Some(mut out) = child.stdout.take() {
                let mut buf = vec![];
                let _ = tokio::io::AsyncReadExt::read_to_end(&mut out, &mut buf).await;
                String::from_utf8_lossy(&buf).to_string()
            } else {
                String::new()
            };

            let err = if let Some(mut err) = child.stderr.take() {
                let mut buf = vec![];
                let _ = tokio::io::AsyncReadExt::read_to_end(&mut err, &mut buf).await;
                String::from_utf8_lossy(&buf).to_string()
            } else {
                String::new()
            };

            return Ok((out, err));
        }
        Ok((String::new(), String::new()))
    }

    /// List all processes
    pub fn list(&self) -> Vec<BackgroundProcess> {
        let processes = self.processes.lock().unwrap();
        processes.values().cloned().collect()
    }

    /// Cleanup completed processes
    pub fn cleanup(&self) {
        let mut processes = self.processes.lock().unwrap();
        let mut children = self.children.lock().unwrap();

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
}
