//! CLI argument definitions using clap.

use clap::{Args, Parser, Subcommand as ClapSubcommand, ValueEnum};

/// Output format selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable plain text.
    Text,
    /// Structured JSON (useful for scripting).
    Json,
}

/// Top-level CLI arguments.
#[derive(Debug, Parser)]
#[command(
    name = "code-buddy",
    about = "AI coding assistant for local and open-source LLMs",
    long_about = "Code Buddy is a CLI tool for interacting with LLMs locally (LM Studio, \
                  Ollama) or via cloud providers (OpenRouter, NVIDIA, OpenAI).\n\n\
                  Run without arguments to start an interactive session.",
    version,
    author
)]
pub struct Cli {
    /// LLM provider override (overrides config).
    /// Valid: lm-studio, openrouter, nvidia, openai, custom
    #[arg(short = 'p', long, global = true, env = "CODE_BUDDY_PROVIDER")]
    pub provider: Option<String>,

    /// Model name override (overrides config).
    #[arg(short = 'm', long, global = true, env = "CODE_BUDDY_MODEL")]
    pub model: Option<String>,

    /// Enable debug logging.
    #[arg(long, global = true, env = "CODE_BUDDY_DEBUG")]
    pub debug: bool,

    /// Disable ANSI color output.
    #[arg(long, global = true, env = "NO_COLOR")]
    pub no_color: bool,

    /// Output format.
    #[arg(long, value_enum, global = true)]
    pub output: Option<OutputFormat>,

    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(Debug, ClapSubcommand)]
pub enum Subcommand {
    /// Send a single prompt and exit.
    Ask(AskArgs),

    /// Start an interactive session (default).
    Run(RunArgs),

    /// Read, write, or display configuration.
    Config(ConfigArgs),

    /// Post-install setup and verification.
    Install(InstallArgs),
}

/// Arguments for the `ask` subcommand.
#[derive(Debug, Args, Default)]
pub struct AskArgs {
    /// The prompt to send to the model.
    pub prompt: Vec<String>,

    /// Disable tool calling for this request.
    #[arg(long)]
    pub no_tools: bool,

    /// Stream the response token by token.
    #[arg(long)]
    pub stream: bool,
}

/// Arguments for the `run` subcommand.
#[derive(Debug, Args, Default)]
pub struct RunArgs {
    /// Disable tool calling in this session.
    #[arg(long)]
    pub no_tools: bool,
}

/// Arguments for the `config` subcommand.
#[derive(Debug, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Debug, ClapSubcommand)]
pub enum ConfigAction {
    /// Display the current configuration.
    Show,

    /// Get a single configuration value.
    Get {
        /// Config field name.
        field: String,
    },

    /// Set a configuration value.
    Set {
        /// Config field name.
        field: String,
        /// New value.
        value: String,
    },

    /// Print the path to the config file.
    Path,
}

/// Arguments for the `install` subcommand.
#[derive(Debug, Args, Default)]
pub struct InstallArgs {
    /// Only verify existing installation without making changes.
    #[arg(long)]
    pub verify_only: bool,
}
