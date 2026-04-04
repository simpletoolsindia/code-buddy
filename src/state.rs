//! Application state management

use crate::config::{CompactionResult, Config};
use crate::plugins::PluginRegistry;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Default context window size (200k tokens for Claude)
const DEFAULT_CONTEXT_WINDOW: usize = 200_000;

// Estimated tokens per character (rough approximation)
const TOKENS_PER_CHAR: usize = 4;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Config,
    pub session_id: Option<String>,
    pub conversation_history: Vec<ConversationMessage>,
    pub hooks: Vec<Hook>,
    pub tools: Vec<Tool>,
    pub context: ContextData,
    pub plugin_registry: Option<PluginRegistry>,
    pub fast_mode: bool,
    pub last_response: Option<String>,
    pub checkpoints: Vec<Checkpoint>,
    pub session_changes: Vec<String>,
    pub current_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hook {
    pub event: String,
    pub command: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message_count: usize,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ContextData {
    pub current_directory: Option<std::path::PathBuf>,
    pub allowed_directories: Vec<std::path::PathBuf>,
    pub project_root: Option<std::path::PathBuf>,
    pub session_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub messages: usize,
    pub responses: usize,
    pub tools_used: usize,
    pub files_modified: usize,
    pub commands_run: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let plugin_registry = PluginRegistry::load_plugins(&config).ok();
        Self {
            config,
            session_id: Some(uuid::Uuid::new_v4().to_string()),
            conversation_history: Vec::new(),
            hooks: Vec::new(),
            tools: Self::default_tools(),
            context: ContextData::default(),
            plugin_registry,
            fast_mode: false,
            last_response: None,
            checkpoints: Vec::new(),
            session_changes: Vec::new(),
            current_agent: "default".to_string(),
        }
    }

    fn default_tools() -> Vec<Tool> {
        vec![
            Tool {
                name: "Read".to_string(),
                description: "Read files from the filesystem".to_string(),
                enabled: true,
            },
            Tool {
                name: "Write".to_string(),
                description: "Write files to the filesystem".to_string(),
                enabled: true,
            },
            Tool {
                name: "Edit".to_string(),
                description: "Edit files with line-based changes".to_string(),
                enabled: true,
            },
            Tool {
                name: "Glob".to_string(),
                description: "Find files by glob pattern".to_string(),
                enabled: true,
            },
            Tool {
                name: "Grep".to_string(),
                description: "Search file contents".to_string(),
                enabled: true,
            },
            Tool {
                name: "Bash".to_string(),
                description: "Execute shell commands".to_string(),
                enabled: true,
            },
            Tool {
                name: "WebSearch".to_string(),
                description: "Search the web".to_string(),
                enabled: true,
            },
            Tool {
                name: "WebFetch".to_string(),
                description: "Fetch web page content".to_string(),
                enabled: true,
            },
        ]
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.conversation_history.push(ConversationMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
        });
    }

    pub fn save_config(&mut self) -> Result<()> {
        self.config.save()
    }

    pub fn load_config(&mut self) -> Result<()> {
        self.config = Config::load()?;
        Ok(())
    }

    pub fn set_session_id(&mut self, id: String) {
        self.session_id = Some(id);
    }

    pub fn clear_history(&mut self) {
        self.conversation_history.clear();
    }

    pub fn add_hook(&mut self, event: String, command: String) {
        self.hooks.push(Hook {
            event,
            command,
            enabled: true,
        });
    }

    pub fn remove_hook(&mut self, event: &str) {
        self.hooks.retain(|h| h.event != event);
    }

    /// Get estimated context size in tokens
    pub fn estimate_context_tokens(&self) -> usize {
        let total_chars: usize = self.conversation_history
            .iter()
            .map(|m| m.content.len())
            .sum();
        total_chars / TOKENS_PER_CHAR
    }

    /// Get context window usage percentage
    pub fn context_usage_percent(&self) -> u8 {
        let context_window = self.config.conversation_window.unwrap_or(DEFAULT_CONTEXT_WINDOW);
        let usage = (self.estimate_context_tokens() * 100) / context_window;
        usage as u8
    }

    /// Check if compaction is needed based on config thresholds
    pub fn needs_compaction(&self) -> bool {
        if !self.config.auto_compact {
            return false;
        }
        self.context_usage_percent() >= self.config.compact_threshold
    }

    /// Manually compact conversation history
    /// Returns a summary of what was compacted
    pub fn compact(&mut self) -> CompactionResult {
        let original_count = self.conversation_history.len();
        let keep_messages = self.config.compact_messages;

        if self.conversation_history.len() <= keep_messages {
            return CompactionResult {
                original_messages: original_count,
                compacted_messages: self.conversation_history.len(),
                summary: "No compaction needed - conversation already within limits".to_string(),
                timestamp: chrono::Utc::now(),
            };
        }

        // Get messages to keep (most recent ones)
        let keep = self.conversation_history.len() - keep_messages;
        let recent: Vec<_> = self.conversation_history.iter()
            .skip(keep)
            .map(|m| m.content.chars().take(100).collect::<String>() + "...")
            .collect();

        // Create summary of older messages
        let summary = if keep > 0 {
            format!(
                "[Previous {} messages summarized: {}]",
                keep,
                recent.join(" | ")
            )
        } else {
            "No previous messages to summarize".to_string()
        };

        // Keep only recent messages and add summary as a system message
        let remaining: Vec<_> = self.conversation_history.iter()
            .skip(keep)
            .cloned()
            .collect();
        self.conversation_history = remaining;

        // Add summary as a system message at the beginning
        self.conversation_history.insert(0, ConversationMessage {
            role: "system".to_string(),
            content: summary.clone(),
            timestamp: chrono::Utc::now(),
        });

        CompactionResult {
            original_messages: original_count,
            compacted_messages: self.conversation_history.len(),
            summary,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Auto-compact if needed, returns Some(result) if compaction was performed
    pub fn auto_compact_if_needed(&mut self) -> Option<CompactionResult> {
        if self.needs_compaction() {
            Some(self.compact())
        } else {
            None
        }
    }

    /// Get session changes (files modified)
    pub fn get_session_changes(&self) -> Vec<String> {
        self.session_changes.clone()
    }

    /// Add a session change
    pub fn add_session_change(&mut self, change: String) {
        self.session_changes.push(change);
    }

    /// Get checkpoints
    pub fn get_checkpoints(&self) -> Vec<String> {
        self.checkpoints.iter().map(|c| c.description.clone()).collect()
    }

    /// Create a checkpoint
    pub fn create_checkpoint(&mut self, description: &str) {
        let checkpoint = Checkpoint {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            message_count: self.conversation_history.len(),
            description: description.to_string(),
        };
        self.checkpoints.push(checkpoint);
    }

    /// Rewind to a checkpoint
    pub fn rewind_to_checkpoint(&mut self, index: usize) -> bool {
        if index < self.checkpoints.len() {
            let checkpoint = &self.checkpoints[index];
            // Keep only messages up to checkpoint
            self.conversation_history.truncate(checkpoint.message_count);
            true
        } else {
            false
        }
    }

    /// Get session statistics
    pub fn get_session_stats(&self) -> SessionStats {
        let messages = self.conversation_history.iter().filter(|m| m.role == "user").count();
        let responses = self.conversation_history.iter().filter(|m| m.role == "assistant").count();
        let input_tokens: usize = self.conversation_history.iter()
            .filter(|m| m.role == "user")
            .map(|m| m.content.len() / 4)
            .sum();
        let output_tokens: usize = self.conversation_history.iter()
            .filter(|m| m.role == "assistant")
            .map(|m| m.content.len() / 4)
            .sum();

        SessionStats {
            messages,
            responses,
            tools_used: 0,
            files_modified: self.session_changes.len(),
            commands_run: 0,
            input_tokens,
            output_tokens,
        }
    }

    /// Get last response
    pub fn get_last_response(&self) -> Option<String> {
        self.last_response.clone()
    }

    /// Set last response
    pub fn set_last_response(&mut self, response: String) {
        self.last_response = Some(response);
    }

    /// Get available agents
    pub fn get_available_agents(&self) -> Vec<(String, String)> {
        vec![
            ("default".to_string(), "Standard coding assistant".to_string()),
            ("analyzer".to_string(), "Code analysis specialist".to_string()),
            ("debugger".to_string(), "Debugging specialist".to_string()),
            ("reviewer".to_string(), "Code review specialist".to_string()),
        ]
    }
}
