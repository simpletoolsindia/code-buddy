//! Code Buddy Library
//!
//! This library exposes all Code Buddy modules for use by the binary
//! and for integration testing.

pub mod dirs;
pub mod cron;
pub mod memory;
pub mod sandbox;
pub mod skills_hub;
pub mod skin_engine;
pub mod profiles;
pub mod bg_process;
pub mod context_files;
pub mod prompt_cache;
pub mod acp_server;
pub mod container;
pub mod batch_runner;
pub mod mixture_of_agents;
pub mod image_gen;

// Re-export commonly used types
pub use cron::{CronJob, CronJobOutput, CronJobSummary};
pub use memory::{MemoryEntry, MemorySystem, MemorySearchResult};
pub use sandbox::{execute_code, execute_code_sync, quick_exec, ExecutionResult, Language, SandboxConfig};
pub use skills_hub::{SkillsHub, InstalledSkill, SkillMetadata};
pub use skin_engine::{SkinConfig, load_skin, list_skins, built_in_skins};
pub use profiles::{ProfileManager, Profile};
pub use context_files::{ContextLoader, ContextFile, ContextFileType};
pub use prompt_cache::{PromptCache, CacheStats, CacheBlockType};
pub use acp_server::{AcpServer, AcpMessage};
pub use container::{ContainerBackend, Backend, BackendResult, create_backend};
pub use batch_runner::{BatchRunner, BatchTask, BatchResult, BatchConfig, BatchStats};
pub use mixture_of_agents::{MixtureOfAgents, MoAAgent, MoAConfig, MoAResponse};
pub use image_gen::{ImageGenerator, ImageRequest, ImageResult, ImageProvider};
