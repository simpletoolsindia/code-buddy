//! CLI Argument Parsing
//!
//! This module defines the command-line interface using clap.

use clap::{
    builder::PossibleValue,
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

    /// Update CLI
    Update,

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
}

// MCP subcommands
pub mod mcp {
    use clap::{Parser, Subcommand};

    #[derive(Debug, Clone, Subcommand)]
    pub enum McpCommand {
        /// Add an MCP server
        Add {
            /// Server name
            name: String,
            /// Command or URL
            command_or_url: String,
            /// Additional arguments
            #[arg(trailing_var_arg = true)]
            args: Vec<String>,
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
    use clap::{Parser, Subcommand};

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
    use clap::{Parser, Subcommand};

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
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        let cli = Cli::parse_from(["code-buddy", "--print", "Hello, world!"]);
        assert!(cli.print);
    }

    #[test]
    fn test_mcp_command() {
        let cli = Cli::parse_from(["code-buddy", "mcp", "list"]);
        match cli.command {
            Some(CommandEnum::Mcp(mcp::McpCommand::List)) => {}
            _ => panic!("Expected MCP list command"),
        }
    }
}
