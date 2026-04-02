//! Application state management

use crate::config::Config;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Config,
    pub session_id: Option<String>,
    pub conversation_history: Vec<ConversationMessage>,
    pub hooks: Vec<Hook>,
    pub tools: Vec<Tool>,
    pub context: ContextData,
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

#[derive(Debug, Clone, Default)]
pub struct ContextData {
    pub current_directory: Option<std::path::PathBuf>,
    pub allowed_directories: Vec<std::path::PathBuf>,
    pub project_root: Option<std::path::PathBuf>,
    pub session_metadata: HashMap<String, String>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            session_id: Some(uuid::Uuid::new_v4().to_string()),
            conversation_history: Vec::new(),
            hooks: Vec::new(),
            tools: Self::default_tools(),
            context: ContextData::default(),
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
}
