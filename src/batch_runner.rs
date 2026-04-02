//! Batch Runner - Parallel trajectory generation
//!
//! Run multiple agent conversations in parallel for batch processing
//! and RL training data generation.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;

/// Batch task definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTask {
    pub id: String,
    pub prompt: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub max_iterations: Option<i32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Batch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub task_id: String,
    pub success: bool,
    pub response: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub tokens_used: Option<usize>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Tool call record for trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub args: HashMap<String, serde_json::Value>,
    pub result: String,
    pub timestamp: String,
}

/// Batch configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    pub max_parallel: usize,
    pub max_total: Option<usize>,
    pub save_trajectories: bool,
    pub trajectory_dir: PathBuf,
    pub max_iterations: i32,
    pub model: String,
    pub provider: Option<String>,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            max_total: None,
            save_trajectories: true,
            trajectory_dir: PathBuf::from("trajectories"),
            max_iterations: 50,
            model: "claude-opus-4.6".to_string(),
            provider: None,
        }
    }
}

/// Batch runner
pub struct BatchRunner {
    config: BatchConfig,
    results: Vec<BatchResult>,
}

impl BatchRunner {
    pub fn new(config: BatchConfig) -> Self {
        // Ensure trajectory directory exists
        if config.save_trajectories {
            let _ = std::fs::create_dir_all(&config.trajectory_dir);
        }
        Self {
            config,
            results: vec![],
        }
    }

    /// Run a batch of tasks
    pub async fn run(&mut self, tasks: Vec<BatchTask>) -> Vec<BatchResult> {
        let (tx, mut rx) = mpsc::channel(self.config.max_parallel);

        // Spawn workers
        let mut handles = vec![];
        for task in tasks {
            let tx = tx.clone();
            let config = self.config.clone();
            let handle = tokio::spawn(async move {
                let result = Self::run_task(task, config).await;
                let _ = tx.send(result).await;
            });
            handles.push(handle);
        }

        // Drop sender so receiver closes
        drop(tx);

        // Collect results
        let mut results = vec![];
        while let Some(result) = rx.recv().await {
            results.push(result);
        }

        // Wait for all tasks
        for handle in handles {
            let _ = handle.await;
        }

        self.results = results.clone();
        results
    }

    /// Run a single batch task
    async fn run_task(task: BatchTask, config: BatchConfig) -> BatchResult {
        let start = std::time::Instant::now();

        // In a real implementation, this would call the LLM API
        // For now, return a stub result
        let response = format!(
            "Batch task {} processed: {}",
            task.id,
            task.prompt.chars().take(50).collect::<String>()
        );

        let duration_ms = start.elapsed().as_millis() as u64;

        BatchResult {
            task_id: task.id,
            success: true,
            response: Some(response),
            tool_calls: vec![],
            tokens_used: Some(task.prompt.len() / 4),
            duration_ms,
            error: None,
        }
    }

    /// Save trajectory to file
    pub fn save_trajectory(&self, result: &BatchResult) -> Result<PathBuf> {
        if !self.config.save_trajectories {
            return Ok(PathBuf::new());
        }

        let filename = format!("trajectory_{}.json", result.task_id);
        let path = self.config.trajectory_dir.join(&filename);

        let trajectory = serde_json::json!({
            "task_id": result.task_id,
            "success": result.success,
            "response": result.response,
            "tool_calls": result.tool_calls,
            "tokens_used": result.tokens_used,
            "duration_ms": result.duration_ms,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        std::fs::write(&path, serde_json::to_string_pretty(&trajectory)?)?;
        Ok(path)
    }

    /// Get statistics
    pub fn stats(&self) -> BatchStats {
        let total = self.results.len();
        let success = self.results.iter().filter(|r| r.success).count();
        let total_duration: u64 = self.results.iter().map(|r| r.duration_ms).sum();
        let total_tokens: usize = self.results.iter().filter_map(|r| r.tokens_used).sum();

        BatchStats {
            total_tasks: total,
            successful: success,
            failed: total - success,
            success_rate: if total > 0 { success as f64 / total as f64 } else { 0.0 },
            avg_duration_ms: if total > 0 { total_duration / total as u64 } else { 0 },
            total_tokens,
        }
    }
}

/// Batch statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStats {
    pub total_tasks: usize,
    pub successful: usize,
    pub failed: usize,
    pub success_rate: f64,
    pub avg_duration_ms: u64,
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_runner() {
        let config = BatchConfig::default();
        let mut runner = BatchRunner::new(config);

        let tasks = vec![
            BatchTask {
                id: "1".to_string(),
                prompt: "Task 1".to_string(),
                model: None,
                provider: None,
                max_iterations: None,
                metadata: HashMap::new(),
            },
            BatchTask {
                id: "2".to_string(),
                prompt: "Task 2".to_string(),
                model: None,
                provider: None,
                max_iterations: None,
                metadata: HashMap::new(),
            },
        ];

        let results = runner.run(tasks).await;
        assert_eq!(results.len(), 2);

        let stats = runner.stats();
        assert_eq!(stats.total_tasks, 2);
    }
}
