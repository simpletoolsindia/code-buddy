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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Parse CLI arguments from a string slice (simulates shell argv).
    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("code-buddy").chain(args.iter().copied()))
    }

    // ── Subcommand dispatch ───────────────────────────────────────────────────

    #[test]
    fn no_subcommand_is_valid() {
        let cli = parse(&[]).expect("bare invocation should parse");
        assert!(cli.subcommand.is_none());
    }

    #[test]
    fn ask_subcommand_parses() {
        let cli = parse(&["ask", "What", "is", "Rust?"]).expect("ask should parse");
        match cli.subcommand {
            Some(Subcommand::Ask(args)) => {
                assert_eq!(args.prompt, vec!["What", "is", "Rust?"]);
            }
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn ask_single_quoted_arg_is_one_element() {
        let cli = parse(&["ask", "What is Rust?"]).expect("ask should parse");
        match cli.subcommand {
            Some(Subcommand::Ask(args)) => {
                assert_eq!(args.prompt, vec!["What is Rust?"]);
            }
            other => panic!("expected Ask, got {other:?}"),
        }
    }

    #[test]
    fn ask_with_no_tools_flag() {
        let cli = parse(&["ask", "--no-tools", "hello"]).expect("should parse");
        if let Some(Subcommand::Ask(args)) = cli.subcommand {
            assert!(args.no_tools);
        } else {
            panic!("expected Ask subcommand");
        }
    }

    #[test]
    fn run_subcommand_parses() {
        let cli = parse(&["run"]).expect("run should parse");
        assert!(matches!(cli.subcommand, Some(Subcommand::Run(_))));
    }

    #[test]
    fn run_with_no_tools_flag() {
        let cli = parse(&["run", "--no-tools"]).expect("should parse");
        if let Some(Subcommand::Run(args)) = cli.subcommand {
            assert!(args.no_tools);
        } else {
            panic!("expected Run subcommand");
        }
    }

    #[test]
    fn config_show_subcommand_parses() {
        let cli = parse(&["config", "show"]).expect("config show should parse");
        if let Some(Subcommand::Config(args)) = cli.subcommand {
            assert!(matches!(args.action, ConfigAction::Show));
        } else {
            panic!("expected Config subcommand");
        }
    }

    #[test]
    fn config_get_subcommand_parses() {
        let cli = parse(&["config", "get", "provider"]).expect("config get should parse");
        if let Some(Subcommand::Config(args)) = cli.subcommand {
            if let ConfigAction::Get { field } = args.action {
                assert_eq!(field, "provider");
            } else {
                panic!("expected ConfigAction::Get");
            }
        } else {
            panic!("expected Config subcommand");
        }
    }

    #[test]
    fn config_set_subcommand_parses() {
        let cli =
            parse(&["config", "set", "provider", "openai"]).expect("config set should parse");
        if let Some(Subcommand::Config(args)) = cli.subcommand {
            if let ConfigAction::Set { field, value } = args.action {
                assert_eq!(field, "provider");
                assert_eq!(value, "openai");
            } else {
                panic!("expected ConfigAction::Set");
            }
        } else {
            panic!("expected Config subcommand");
        }
    }

    #[test]
    fn config_path_subcommand_parses() {
        let cli = parse(&["config", "path"]).expect("config path should parse");
        assert!(matches!(
            cli.subcommand,
            Some(Subcommand::Config(ConfigArgs {
                action: ConfigAction::Path
            }))
        ));
    }

    #[test]
    fn install_subcommand_parses() {
        let cli = parse(&["install"]).expect("install should parse");
        assert!(matches!(cli.subcommand, Some(Subcommand::Install(_))));
    }

    #[test]
    fn install_verify_only_flag() {
        let cli = parse(&["install", "--verify-only"]).expect("should parse");
        if let Some(Subcommand::Install(args)) = cli.subcommand {
            assert!(args.verify_only);
        } else {
            panic!("expected Install subcommand");
        }
    }

    // ── Global flags ─────────────────────────────────────────────────────────

    #[test]
    fn provider_flag_short() {
        let cli = parse(&["-p", "openrouter", "ask", "hi"]).expect("should parse");
        assert_eq!(cli.provider.as_deref(), Some("openrouter"));
    }

    #[test]
    fn provider_flag_long() {
        let cli = parse(&["--provider", "nvidia", "ask", "hi"]).expect("should parse");
        assert_eq!(cli.provider.as_deref(), Some("nvidia"));
    }

    #[test]
    fn model_flag_short() {
        let cli = parse(&["-m", "mistral-7b", "run"]).expect("should parse");
        assert_eq!(cli.model.as_deref(), Some("mistral-7b"));
    }

    #[test]
    fn model_flag_long() {
        let cli = parse(&["--model", "llama3", "run"]).expect("should parse");
        assert_eq!(cli.model.as_deref(), Some("llama3"));
    }

    #[test]
    fn debug_flag() {
        let cli = parse(&["--debug", "run"]).expect("should parse");
        assert!(cli.debug);
    }

    #[test]
    fn no_color_flag() {
        let cli = parse(&["--no-color", "run"]).expect("should parse");
        assert!(cli.no_color);
    }

    #[test]
    fn output_text_flag() {
        let cli = parse(&["--output", "text", "run"]).expect("should parse");
        assert!(matches!(cli.output, Some(OutputFormat::Text)));
    }

    #[test]
    fn output_json_flag() {
        let cli = parse(&["--output", "json", "ask", "hi"]).expect("should parse");
        assert!(matches!(cli.output, Some(OutputFormat::Json)));
    }

    #[test]
    fn flags_before_subcommand() {
        let cli = parse(&["--debug", "--provider", "openrouter", "run"])
            .expect("flags before subcommand should parse");
        assert!(cli.debug);
        assert_eq!(cli.provider.as_deref(), Some("openrouter"));
        assert!(matches!(cli.subcommand, Some(Subcommand::Run(_))));
    }

    #[test]
    fn unknown_subcommand_fails() {
        assert!(parse(&["foobar"]).is_err(), "unknown subcommand should fail");
    }

    #[test]
    fn invalid_output_value_fails() {
        assert!(
            parse(&["--output", "csv", "run"]).is_err(),
            "invalid output format should fail"
        );
    }
}
