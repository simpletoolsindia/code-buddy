//! Multi-Agent System
//!
//! Provides multi-agent orchestration with parallel execution and communication.
//! Similar to Claude Code's Agent Teams feature.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Agent definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub model: Option<String>,
    pub enabled: bool,
}

impl Agent {
    pub fn new(id: &str, name: &str, role: &str, description: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            role: role.to_string(),
            description: description.to_string(),
            system_prompt: format!("You are {}, a {}. {}", name, role, description),
            tools: vec![],
            model: None,
            enabled: true,
        }
    }

    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.tools = tools.into_iter().map(String::from).collect();
        self
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }
}

/// Built-in agents
pub fn default_agents() -> Vec<Agent> {
    vec![
        Agent::new("default", "Code Buddy", "general coding assistant", "Helps with coding tasks")
            .with_tools(vec!["Read", "Write", "Edit", "Bash", "Grep", "Glob", "WebSearch", "WebFetch"]),
        Agent::new("analyzer", "Code Analyzer", "code analysis specialist", "Analyzes code for patterns and issues")
            .with_tools(vec!["Read", "Grep", "Glob", "WebFetch"]),
        Agent::new("debugger", "Debugger", "debugging specialist", "Helps debug issues and find bugs")
            .with_tools(vec!["Read", "Bash", "Grep", "Glob"]),
        Agent::new("reviewer", "Code Reviewer", "code review specialist", "Performs thorough code reviews")
            .with_tools(vec!["Read", "Bash", "Grep", "Glob"]),
        Agent::new("tester", "Test Engineer", "testing specialist", "Writes and runs tests")
            .with_tools(vec!["Read", "Write", "Bash", "Glob"]),
        Agent::new("architect", "Architect", "software architect", "Designs system architecture")
            .with_tools(vec!["Read", "Glob", "WebFetch"]),
        Agent::new("explore", "Explorer", "codebase exploration agent", "Explores and maps codebase structure")
            .with_tools(vec!["Read", "Glob", "Grep"]),
        Agent::new("plan", "Planner", "planning agent", "Creates detailed implementation plans")
            .with_tools(vec!["Read", "Write", "Glob", "WebFetch"]),
        Agent::new("verification", "Verifier", "verification agent", "Verifies implementations against specs")
            .with_tools(vec!["Read", "Write", "Bash", "Glob"]),
        Agent::new("guide", "Claude Code Guide", "guide agent", "Helps users learn Claude Code features")
            .with_tools(vec!["Read"]),
        Agent::new("claude-code-guide", "Claude Code Guide", "guide agent", "Helps users learn Claude Code features")
            .with_tools(vec!["Read"]),
        Agent::new("general-purpose", "General Purpose Agent", "general purpose agent", "Handles diverse tasks")
            .with_tools(vec!["Read", "Write", "Edit", "Bash", "Grep", "Glob", "WebSearch", "WebFetch"]),
        Agent::new("statusline-setup", "Statusline Setup", "statusline configuration agent", "Configures statusline display")
            .with_tools(vec!["Read", "Write"]),
    ]
}

/// Agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Agent task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTask {
    pub id: String,
    pub description: String,
    pub assigned_to: Option<String>,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Agent team
pub struct AgentTeam {
    pub name: String,
    pub agents: Vec<Agent>,
    pub tasks: Vec<AgentTask>,
    pub messages: Vec<AgentMessage>,
}

impl AgentTeam {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            agents: default_agents(),
            tasks: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Add a custom agent
    pub fn add_agent(&mut self, agent: Agent) {
        self.agents.push(agent);
    }

    /// Get agent by ID
    pub fn get_agent(&self, id: &str) -> Option<&Agent> {
        self.agents.iter().find(|a| a.id == id)
    }

    /// Create a task
    pub fn create_task(&mut self, description: &str, dependencies: Vec<String>) -> &AgentTask {
        let task = AgentTask {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            assigned_to: None,
            status: TaskStatus::Pending,
            result: None,
            dependencies,
        };
        self.tasks.push(task);
        self.tasks.last().unwrap()
    }

    /// Assign task to agent
    pub fn assign_task(&mut self, task_id: &str, agent_id: &str) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            if self.agents.iter().any(|a| a.id == agent_id) {
                task.assigned_to = Some(agent_id.to_string());
                task.status = TaskStatus::InProgress;
                return true;
            }
        }
        false
    }

    /// Send message between agents
    pub fn send_message(&mut self, from: &str, to: &str, content: &str) {
        let msg = AgentMessage {
            from: from.to_string(),
            to: to.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
        };
        self.messages.push(msg);
    }

    /// Get pending tasks
    pub fn get_pending_tasks(&self) -> Vec<&AgentTask> {
        self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).collect()
    }

    /// Get agent's tasks
    pub fn get_agent_tasks(&self, agent_id: &str) -> Vec<&AgentTask> {
        self.tasks.iter().filter(|t| t.assigned_to.as_deref() == Some(agent_id)).collect()
    }

    /// Complete a task
    pub fn complete_task(&mut self, task_id: &str, result: &str) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Completed;
            task.result = Some(result.to_string());
            return true;
        }
        false
    }

    /// Check if all tasks are complete
    pub fn all_tasks_complete(&self) -> bool {
        self.tasks.iter().all(|t| t.status == TaskStatus::Completed)
    }

    /// Get team status
    pub fn status(&self) -> TeamStatus {
        let total = self.tasks.len();
        let completed = self.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let in_progress = self.tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count();
        let pending = self.tasks.iter().filter(|t| t.status == TaskStatus::Pending).count();

        TeamStatus {
            team_name: self.name.clone(),
            total_tasks: total,
            completed_tasks: completed,
            in_progress_tasks: in_progress,
            pending_tasks: pending,
            active_agents: self.agents.iter().filter(|a| a.enabled).count(),
        }
    }
}

/// Team status report
#[derive(Debug, Clone, Serialize)]
pub struct TeamStatus {
    pub team_name: String,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub in_progress_tasks: usize,
    pub pending_tasks: usize,
    pub active_agents: usize,
}

impl TeamStatus {
    pub fn progress_percent(&self) -> f64 {
        if self.total_tasks == 0 {
            100.0
        } else {
            (self.completed_tasks as f64 / self.total_tasks as f64) * 100.0
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "{}: {} tasks ({} done, {} in progress, {} pending) by {} agents",
            self.team_name,
            self.total_tasks,
            self.completed_tasks,
            self.in_progress_tasks,
            self.pending_tasks,
            self.active_agents
        )
    }
}

/// Run multi-agent workflow
pub async fn run_team_workflow(team: &mut AgentTeam) -> Result<HashMap<String, String>> {
    let mut results = HashMap::new();

    // Get pending task IDs
    let pending_ids: Vec<String> = team.get_pending_tasks()
        .iter()
        .map(|t| t.id.clone())
        .collect();

    for task_id in pending_ids {
        // Get task description (we need to re-borrow)
        let task_desc = {
            let task = team.tasks.iter().find(|t| t.id == task_id);
            match task {
                Some(t) => t.description.clone(),
                None => continue,
            }
        };

        // Check if dependencies are met
        let deps_met = {
            let task = team.tasks.iter().find(|t| t.id == task_id);
            match task {
                Some(t) => t.dependencies.is_empty()
                    || t.dependencies.iter().all(|dep| {
                        team.tasks
                            .iter()
                            .find(|t| &t.id == dep)
                            .map(|t| t.status == TaskStatus::Completed)
                            .unwrap_or(false)
                    }),
                None => false,
            }
        };

        if deps_met {
            // Find best agent for the task
            let agent_id = find_best_agent(&team.agents, &task_desc);
            team.assign_task(&task_id, &agent_id);

            // Simulate task execution
            let result = format!("Task '{}' executed by {}", task_desc, agent_id);
            team.complete_task(&task_id, &result);
            results.insert(task_id.clone(), result);
        }
    }

    Ok(results)
}

/// Find best agent for a task
fn find_best_agent(agents: &[Agent], task_description: &str) -> String {
    // Simple heuristic: match keywords
    let task_lower = task_description.to_lowercase();

    for agent in agents {
        if !agent.enabled {
            continue;
        }

        // Check role keywords
        let role_match = agent.role.to_lowercase();
        if task_lower.contains("debug") && role_match.contains("debug") {
            return agent.id.clone();
        }
        if task_lower.contains("test") && role_match.contains("test") {
            return agent.id.clone();
        }
        if task_lower.contains("review") && role_match.contains("review") {
            return agent.id.clone();
        }
        if task_lower.contains("architect") && role_match.contains("architect") {
            return agent.id.clone();
        }
        if task_lower.contains("analyze") && role_match.contains("analysis") {
            return agent.id.clone();
        }
    }

    // Default to first enabled agent
    agents.iter().find(|a| a.enabled).map(|a| a.id.clone()).unwrap_or_else(|| "default".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = Agent::new("test", "Test Agent", "tester", "A test agent");
        assert_eq!(agent.id, "test");
        assert_eq!(agent.name, "Test Agent");
        assert!(agent.enabled);
    }

    #[test]
    fn test_team_creation() {
        let team = AgentTeam::new("Test Team");
        assert_eq!(team.name, "Test Team");
        assert!(!team.agents.is_empty());
    }

    #[test]
    fn test_task_management() {
        let mut team = AgentTeam::new("Test Team");
        let task_id = {
            let task = team.create_task("Test task", vec![]);
            assert_eq!(task.status, TaskStatus::Pending);
            task.id.clone()
        };

        assert!(team.assign_task(&task_id, "default"));

        assert!(team.complete_task(&task_id, "Done"));
        let updated = team.tasks.iter().find(|t| t.id == task_id).unwrap();
        assert_eq!(updated.status, TaskStatus::Completed);
    }

    #[test]
    fn test_team_status() {
        let mut team = AgentTeam::new("Test Team");
        let task_id = {
            let task = team.create_task("Test", vec![]);
            task.id.clone()
        };
        team.complete_task(&task_id, "Done");

        let status = team.status();
        assert_eq!(status.total_tasks, 1);
        assert_eq!(status.completed_tasks, 1);
        assert_eq!(status.progress_percent(), 100.0);
    }
}
