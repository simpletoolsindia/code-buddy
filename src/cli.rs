//! CLI Argument Parsing
//!
//! This module defines the command-line interface using clap.

use clap::{
    Command, Parser, Subcommand, ValueEnum,
};
use std::path::PathBuf;

/// Output format options
#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    Text,
    /// JSON output
    Json,
    /// Streaming JSON
    StreamJson,
}

/// Permission mode
#[derive(Debug, Clone, ValueEnum)]
pub enum PermissionMode {
    /// Default permission mode
    Default,
    /// Auto-accept edits
    AcceptEdits,
    /// Don't ask
    DontAsk,
    /// Plan mode
    Plan,
    /// Bypass all permissions
    BypassPermissions,
    /// Auto mode
    Auto,
}

/// Model aliases
#[derive(Debug, Clone, ValueEnum)]
pub enum ModelAlias {
    /// Opus model
    Opus,
    /// Sonnet model
    Sonnet,
    /// Haiku model
    Haiku,
    /// Best available model
    Best,
    /// Opus in plan mode
    OpusPlan,
}

impl ModelAlias {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModelAlias::Opus => "opus",
            ModelAlias::Sonnet => "sonnet",
            ModelAlias::Haiku => "haiku",
            ModelAlias::Best => "best",
            ModelAlias::OpusPlan => "opusplan",
        }
    }
}

/// Effort level
#[derive(Debug, Clone, ValueEnum)]
pub enum Effort {
    /// Low effort
    Low,
    /// Medium effort
    Medium,
    /// High effort
    High,
    /// Maximum effort
    Max,
}

/// CLI argument structure
#[derive(Debug, Parser)]
#[command(
    name = "code-buddy",
    about = "Code Buddy - AI coding assistant",
    long_about = None,
    version,
    author,
)]
pub struct Cli {
    /// Enable debug mode
    #[arg(short, long, global = true)]
    pub debug: bool,

    /// Enable verbose mode
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Print output and exit (non-interactive)
    #[arg(short, long, global = true)]
    pub print: bool,

    /// Output format
    #[arg(long, value_enum, global = true, hide = true)]
    pub output_format: Option<OutputFormat>,

    /// Model to use
    #[arg(long, global = true)]
    pub model: Option<String>,

    /// Continue the last conversation
    #[arg(short, long, global = true)]
    pub continue_session: bool,

    /// Resume a specific session
    #[arg(short = 'r', long, global = true, value_name = "SESSION_ID")]
    pub resume: Option<String>,

    /// Permission mode
    #[arg(long, value_enum, global = true)]
    pub permission_mode: Option<PermissionMode>,

    /// Additional directories to allow
    #[arg(long, value_name = "DIR", global = true)]
    pub add_dir: Vec<PathBuf>,

    /// MCP server configuration files
    #[arg(long, value_name = "CONFIG", global = true)]
    pub mcp_config: Vec<String>,

    /// Custom system prompt
    #[arg(long, value_name = "PROMPT", global = true)]
    pub system_prompt: Option<String>,

    /// Load settings from file
    #[arg(long, value_name = "FILE", global = true)]
    pub settings: Option<String>,

    /// Disable slash commands/skills
    #[arg(long, global = true)]
    pub disable_slash_commands: bool,

    /// Allow dangerous permissions
    #[arg(long, global = true)]
    pub allow_dangerously_skip_permissions: bool,

    /// Update to the latest version
    #[arg(long, global = true, hide = true)]
    pub self_update: bool,

    /// Agent to use
    #[arg(long, value_name = "AGENT", global = true)]
    pub agent: Option<String>,

    /// Custom agents JSON
    #[arg(long, value_name = "JSON", global = true)]
    pub agents: Option<String>,

    /// Effort level
    #[arg(long, value_enum, global = true)]
    pub effort: Option<Effort>,

    /// Maximum budget in USD
    #[arg(long, value_name = "AMOUNT", global = true)]
    pub max_budget_usd: Option<f64>,

    /// Session ID to use
    #[arg(long, value_name = "UUID", global = true)]
    pub session_id: Option<String>,

    /// Session name/display name
    #[arg(short, long, value_name = "NAME", global = true)]
    pub name: Option<String>,

    /// Bare/minimal mode
    #[arg(long, global = true)]
    pub bare: bool,

    /// IDE auto-connect
    #[arg(long, global = true)]
    pub ide: bool,

    /// Use MLX for local inference on Apple Silicon
    #[arg(long, global = true, hide = true)]
    pub mlx: bool,

    /// MLX model to use (e.g., mlx-community/llama-3.2-3b-instruct-4bit)
    #[arg(long, value_name = "MODEL_ID", global = true, hide = true)]
    pub mlx_model: Option<String>,

    /// Download an MLX model from HuggingFace
    #[arg(long, value_name = "MODEL_ID", global = true, hide = true)]
    pub mlx_download: Option<String>,

    /// List available MLX models
    #[arg(long, global = true, hide = true)]
    pub mlx_list_models: bool,

    /// Prompt to execute (use with -p/--print)
    #[arg(global = true)]
    pub prompt: Option<Vec<String>>,

    /// The command to execute
    #[command(subcommand)]
    pub command: Option<CommandEnum>,
}

/// Command variants
#[derive(Debug, Clone, Subcommand)]
pub enum CommandEnum {
    /// MCP server management
    #[command(subcommand)]
    Mcp(mcp::McpCommand),

    /// List configured agents
    Agents {
        /// List all agents
        #[arg(long)]
        list: bool,
    },

    /// Authentication management
    #[command(subcommand)]
    Auth(auth::AuthCommand),

    /// Interactive setup wizard
    Setup,

    /// Reset configuration
    Reset {
        /// Reset all settings
        #[arg(long)]
        all: bool,
    },

    /// Interactive REPL mode
    Interactive,

    /// Check health
    Doctor,

    /// Install native build
    Install {
        /// Target version (stable, latest, or specific version)
        target: Option<String>,
    },

    /// Update CLI to latest version
    Update {
        /// Actually perform the update (default: just check)
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Configuration management
    #[command(subcommand)]
    Config(config::ConfigCommand),

    /// Model selection
    Model {
        /// Model to use
        model: Option<String>,
    },

    /// Login with API key
    Login {
        /// API key
        api_key: Option<String>,
    },

    /// Logout
    Logout,

    /// Show status
    Status,

    /// Show version
    Version,

    /// Plugin management
    #[command(subcommand)]
    Plugin(plugin::PluginCommand),
}

// MCP subcommands
pub mod mcp {
    use clap::Subcommand;

    #[derive(Debug, Clone, Subcommand)]
    pub enum McpCommand {
        /// Add an MCP server
        Add {
            /// Server name
            #[arg(value_name = "NAME")]
            name: String,
            /// Command and arguments as single string
            #[arg(value_name = "COMMAND")]
            command: String,
        },
        /// Add MCP server from JSON
        AddJson {
            /// Server name
            name: String,
            /// JSON configuration
            json: String,
        },
        /// Import from Claude Desktop
        AddFromClaudeDesktop,
        /// List MCP servers
        List,
        /// Get MCP server details
        Get {
            /// Server name
            name: String,
        },
        /// Remove an MCP server
        Remove {
            /// Server name
            name: String,
        },
        /// Start MCP server
        Serve,
        /// Reset project choices
        ResetProjectChoices,
    }
}

// Auth subcommands
pub mod auth {
    use clap::Subcommand;

    #[derive(Debug, Clone, Subcommand)]
    pub enum AuthCommand {
        /// Login to Anthropic
        Login {
            /// API key
            #[arg(short)]
            api_key: Option<String>,
        },
        /// Logout
        Logout,
        /// Show auth status
        Status,
    }
}

// Config subcommands
pub mod config {
    use clap::Subcommand;

    #[derive(Debug, Clone, Subcommand)]
    pub enum ConfigCommand {
        /// List configuration
        List,
        /// Get a configuration value
        Get {
            /// Key
            key: String,
        },
        /// Set a configuration value
        Set {
            /// Key
            key: String,
            /// Value
            value: String,
        },
        /// Edit configuration file
        Edit,
    }
}

// Plugin subcommands
pub mod plugin {
    use clap::Subcommand;

    #[derive(Debug, Clone, Subcommand)]
    pub enum PluginCommand {
        /// List installed plugins
        List {
            /// Output as JSON
            #[arg(long)]
            json: bool,
            /// Show all plugins including disabled
            #[arg(long)]
            all: bool,
        },
        /// Add a plugin from git URL or local path
        Add {
            /// Plugin source (git URL or local path)
            source: String,
            /// Installation scope (user or project)
            #[arg(long)]
            scope: Option<PluginScopeArg>,
        },
        /// Remove an installed plugin
        Remove {
            /// Plugin name
            name: String,
        },
        /// Enable a plugin
        Enable {
            /// Plugin name
            name: String,
        },
        /// Disable a plugin
        Disable {
            /// Plugin name
            name: String,
        },
        /// Update plugin(s)
        Update {
            /// Plugin name (omit to update all)
            name: Option<String>,
        },
        /// Search for available plugins
        Search {
            /// Search query
            query: String,
        },
        /// List all available skills
        Skills,
        /// Marketplace management
        Marketplace {
            #[command(subcommand)]
            subcmd: Option<MarketplaceSubcommand>,
        },
        /// Validate a plugin at a path
        Validate {
            /// Plugin path to validate
            path: String,
        },
        /// Reload all plugins
        Reload,
    }

    #[derive(Debug, Clone, Copy, clap::ValueEnum)]
    pub enum PluginScopeArg {
        User,
        Project,
    }

    #[derive(Debug, Clone, Subcommand)]
    pub enum MarketplaceSubcommand {
        /// List configured marketplaces
        List,
        /// Add a new marketplace
        Add {
            /// Marketplace source (git repo or URL)
            source: String,
        },
        /// Remove a marketplace
        Remove {
            /// Marketplace name
            name: String,
        },
        /// Update marketplace listings
        Update {
            /// Marketplace name (omit for all)
            name: Option<String>,
        },
    }
}

impl Cli {
    /// Print help message
    pub fn print_help() {
        let mut cmd = Command::new("code-buddy")
            .about("Code Buddy - AI coding assistant")
            .long_about(
                "Code Buddy starts an interactive REPL by default.\n\
                 Use -p/--print for non-interactive output.\n\
                 \n\
                 Quick Start:\n\
                 - code-buddy setup               Interactive setup wizard\n\
                 - code-buddy -p \"prompt\"        Run a prompt\n\
                 - code-buddy                     Start interactive REPL\n\
                 \n\
                 Configuration:\n\
                 - code-buddy config list         Show all config\n\
                 - code-buddy config set <key> <val>  Set config\n\
                 - code-buddy reset               Reset configuration\n\
                 - code-buddy reset --all         Full factory reset\n\
                 \n\
                 In REPL mode, use /commands:\n\
                 - /help  /quit  /clear  /status  /model  /history\n\
                 \n\
                 Examples:\n\
                 - code-buddy -p \"Write hello world in Python\"\n\
                 - code-buddy --model llama3.2 -p \"Explain this code\"\n\
                 - code-buddy setup\n\
                 \n\
                 See 'code-buddy help <command>' for detailed command help.",
            );

        cmd.print_help().unwrap();
        println!();
    }
}

#[cfg(test)]
mod tests;
