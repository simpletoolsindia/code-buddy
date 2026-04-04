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
        let len = self.tasks.len();
        self.tasks.push(task);
        // Return reference to the task we just added (never panics since we just pushed)
        &self.tasks[len]
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

/// Advanced agent collaboration features
/// Shared context between agents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentContext {
    /// Shared key-value store for agents
    pub shared_data: HashMap<String, serde_json::Value>,
    /// Task results that agents have produced
    pub task_results: HashMap<String, AgentTaskResult>,
    /// Files or resources agents have created
    pub created_resources: Vec<String>,
    /// Decisions made by the team
    pub decisions: Vec<TeamDecision>,
    /// Notes from different agents
    pub notes: Vec<AgentNote>,
}

impl AgentContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set shared data
    pub fn set(&mut self, key: &str, value: serde_json::Value) {
        self.shared_data.insert(key.to_string(), value);
    }

    /// Get shared data
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.shared_data.get(key)
    }

    /// Add a task result
    pub fn add_result(&mut self, task_id: &str, agent_id: &str, result: String) {
        self.task_results.insert(task_id.to_string(), AgentTaskResult {
            task_id: task_id.to_string(),
            agent_id: agent_id.to_string(),
            result,
            timestamp: chrono::Utc::now(),
        });
    }

    /// Add a team decision
    pub fn add_decision(&mut self, description: String, votes: HashMap<String, bool>, decided_by: &str) {
        self.decisions.push(TeamDecision {
            description,
            votes,
            decided_by: decided_by.to_string(),
            timestamp: chrono::Utc::now(),
        });
    }

    /// Add an agent note
    pub fn add_note(&mut self, agent_id: &str, note: &str) {
        self.notes.push(AgentNote {
            agent_id: agent_id.to_string(),
            note: note.to_string(),
            timestamp: chrono::Utc::now(),
        });
    }
}

/// Agent task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTaskResult {
    pub task_id: String,
    pub agent_id: String,
    pub result: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Team decision with voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDecision {
    pub description: String,
    pub votes: HashMap<String, bool>, // agent_id -> vote (true = yes, false = no)
    pub decided_by: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TeamDecision {
    /// Check if decision is approved
    pub fn is_approved(&self) -> bool {
        let yes_votes = self.votes.values().filter(|v| **v).count();
        let total_votes = self.votes.len();
        total_votes > 0 && yes_votes > total_votes / 2
    }

    /// Get approval percentage
    pub fn approval_percent(&self) -> f64 {
        if self.votes.is_empty() {
            return 0.0;
        }
        let yes_votes = self.votes.values().filter(|v| **v).count();
        (yes_votes as f64 / self.votes.len() as f64) * 100.0
    }
}

/// Agent note
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentNote {
    pub agent_id: String,
    pub note: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[derive(Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}


/// Enhanced task with priority
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedTask {
    pub id: String,
    pub description: String,
    pub assigned_to: Option<String>,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub dependencies: Vec<String>,
    pub priority: TaskPriority,
    pub estimated_duration_secs: Option<u64>,
    pub tags: Vec<String>,
}

impl EnhancedTask {
    pub fn new(description: &str, priority: TaskPriority) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            assigned_to: None,
            status: TaskStatus::Pending,
            result: None,
            dependencies: Vec::new(),
            priority,
            estimated_duration_secs: None,
            tags: Vec::new(),
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<&str>) -> Self {
        self.dependencies = deps.into_iter().map(String::from).collect();
        self
    }

    pub fn with_duration(mut self, secs: u64) -> Self {
        self.estimated_duration_secs = Some(secs);
        self
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }
}

/// Agent team with advanced collaboration
pub struct AdvancedTeam {
    pub base: AgentTeam,
    pub context: AgentContext,
    pub max_parallel: usize,
}

impl AdvancedTeam {
    pub fn new(name: &str, max_parallel: usize) -> Self {
        Self {
            base: AgentTeam::new(name),
            context: AgentContext::new(),
            max_parallel,
        }
    }

    /// Add shared data
    pub fn share_data(&mut self, key: &str, value: serde_json::Value) {
        self.context.set(key, value);
    }

    /// Get shared data
    pub fn get_shared_data(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    /// Run parallel tasks
    pub async fn run_parallel(&mut self, tasks: Vec<EnhancedTask>) -> Vec<AgentTaskResult> {
        use tokio::task;

        let mut handles = Vec::new();

        for task in tasks.into_iter().take(self.max_parallel) {
            let agent_id = find_best_agent(&self.base.agents, &task.description);
            let task_id = task.id.clone();

            // Spawn a task for parallel execution
            let handle = task::spawn(async move {
                // Simulate work
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                AgentTaskResult {
                    task_id,
                    agent_id,
                    result: "Task completed".to_string(),
                    timestamp: chrono::Utc::now(),
                }
            });

            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }

        results
    }

    /// Create a team decision with voting
    pub fn vote(&mut self, description: &str, decided_by: &str) -> TeamDecision {
        let mut votes = HashMap::new();

        // Each enabled agent votes
        for agent in &self.base.agents {
            if agent.enabled {
                // Simple majority voting - in real impl, agents would actually reason
                votes.insert(agent.id.clone(), true);
            }
        }

        let decision = TeamDecision {
            description: description.to_string(),
            votes,
            decided_by: decided_by.to_string(),
            timestamp: chrono::Utc::now(),
        };

        self.context.add_decision(description.to_string(), decision.votes.clone(), decided_by);
        decision
    }

    /// Create a collaborative note
    pub fn add_note(&mut self, agent_id: &str, note: &str) {
        self.context.add_note(agent_id, note);
    }

    /// Get context summary
    pub fn context_summary(&self) -> String {
        format!(
            "Shared data: {} keys, Results: {}, Decisions: {}, Notes: {}",
            self.context.shared_data.len(),
            self.context.task_results.len(),
            self.context.decisions.len(),
            self.context.notes.len()
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
