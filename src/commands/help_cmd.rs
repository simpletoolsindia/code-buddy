//! Help Command - Display help information
//!
//! Provides help for commands and features.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Help topic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpTopic {
    pub name: String,
    pub description: String,
    pub commands: Vec<HelpCommand>,
}

/// Help command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpCommand {
    pub name: String,
    pub usage: String,
    pub description: String,
}

/// Run help command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_general_help();
    }

    match args[0].as_str() {
        "commands" | "cmd" => show_commands_help(),
        "tools" => show_tools_help(),
        "skills" => show_skills_help(),
        "agents" => show_agents_help(),
        "hooks" => show_hooks_help(),
        "mcp" => show_mcp_help(),
        "config" => show_config_help(),
        "api" => show_api_help(),
        _ => show_general_help(),
    }
}

fn show_general_help() -> Result<String> {
    let help = r#"# Code Buddy Help

## Quick Start

```
code-buddy                    # Start interactive mode
code-buddy --help             # Show this help
code-buddy setup             # Initial setup
code-buddy doctor             # Check system health
```

## Commands

| Command | Description |
|---------|-------------|
| `/help` | Show this help |
| `/model` | Switch LLM model |
| `/context` | Show context info |
| `/cost` | Show cost estimation |
| `/compact` | Compact conversation |
| `/clear` | Clear conversation |
| `/stats` | Show session stats |
| `/diff` | Show changes |
| `/rewind` | Rewind conversation |
| `/plan` | Create implementation plan |
| `/tasks` | Manage tasks |

## Slash Commands

| Command | Description |
|---------|-------------|
| `/simplify` | Simplify code |
| `/review` | Code review |
| `/tdd` | Test-driven development |
| `/debug` | Debug assistant |
| `/batch` | Batch operations |
| `/security` | Security analysis |
| `/devops` | DevOps helper |
| `/docs` | Documentation helper |

## Tools

| Tool | Description |
|------|-------------|
| `Read` | Read files |
| `Write` | Write files |
| `Edit` | Edit files |
| `Glob` | Find files |
| `Grep` | Search content |
| `Bash` | Execute commands |
| `WebSearch` | Search web |
| `WebFetch` | Fetch web pages |

## Agents

| Agent | Description |
|-------|-------------|
| `default` | Standard assistant |
| `analyzer` | Code analysis |
| `debugger` | Debugging specialist |
| `reviewer` | Code review |
| `tester` | Testing specialist |
| `architect` | Architecture design |

## More Help

- `help commands` - List all commands
- `help tools` - List all tools
- `help skills` - List all skills
- `help agents` - List all agents
- `help hooks` - List all hooks
- `help mcp` - MCP server help
- `help config` - Configuration help
- `help api` - API documentation
"#;

    Ok(help.to_string())
}

fn show_commands_help() -> Result<String> {
    Ok(r#"# Commands

## Session Commands

| Command | Description |
|---------|-------------|
| `help` | Show help |
| `exit` | Exit Code Buddy |
| `clear` | Clear conversation |
| `compact` | Compact conversation |
| `stats` | Show statistics |

## Development Commands

| Command | Description |
|---------|-------------|
| `diff` | Show git diff |
| `commit` | Git commit |
| `plan` | Create plan |
| `tasks` | Manage tasks |

## Information Commands

| Command | Description |
|---------|-------------|
| `context` | Show context |
| `cost` | Show cost |
| `model` | Show/change model |
| `status` | Show status |
"#.to_string())
}

fn show_tools_help() -> Result<String> {
    Ok(r#"# Tools

## File Tools

| Tool | Description |
|------|-------------|
| `Read` | Read files from filesystem |
| `Write` | Write files to filesystem |
| `Edit` | Edit files with changes |
| `Glob` | Find files by pattern |
| `Grep` | Search file contents |

## Execution Tools

| Tool | Description |
|------|-------------|
| `Bash` | Execute shell commands |
| `WebSearch` | Search the web |
| `WebFetch` | Fetch web pages |

## Interactive Tools

| Tool | Description |
|------|-------------|
| `AskUserQuestion` | Ask user questions |
| `NotebookEdit` | Edit Jupyter notebooks |

## MCP Tools

| Tool | Description |
|------|-------------|
| `ListMcpResources` | List MCP resources |
| `ReadMcpResource` | Read MCP resource |
| `McpServers` | Manage MCP servers |
"#.to_string())
}

fn show_skills_help() -> Result<String> {
    Ok(r#"# Skills

## Built-in Skills

| Skill | Description |
|-------|-------------|
| `simplify` | Review code quality |
| `review` | Full code review |
| `tdd` | Test-driven development |
| `debug` | Debugging assistant |
| `batch` | Batch operations |
| `security` | Security analysis |
| `devops` | DevOps helper |
| `database` | Database helper |
| `docs` | Documentation helper |

## Usage

```
/simplify           # Review code
/review             # Full review
/tdd                # Test-driven dev
/debug              # Debug assistant
```
"#.to_string())
}

fn show_agents_help() -> Result<String> {
    Ok(r#"# Agents

## Built-in Agents

| Agent | Description |
|-------|-------------|
| `default` | Standard coding assistant |
| `analyzer` | Code analysis specialist |
| `debugger` | Debugging specialist |
| `reviewer` | Code review specialist |
| `tester` | Testing specialist |
| `architect` | Software architect |

## Usage

```
/agent analyzer     # Switch to analyzer agent
/agent debugger     # Switch to debugger
```

## Team Mode

Create multi-agent teams:

```
/agent team create my-team
/agent team add analyzer
/agent team add tester
```
"#.to_string())
}

fn show_hooks_help() -> Result<String> {
    Ok(r#"# Hooks

## Available Hooks

| Hook | Description |
|------|-------------|
| `before_write` | Before file write |
| `after_write` | After file write |
| `before_submit` | Before message submit |
| `after_submit` | After message submit |
| `on_error` | On error occurrence |
| `on_compact` | On conversation compact |
| `on_tool_use` | On tool execution |

## Usage

```bash
code-buddy hooks list
code-buddy hooks add before_write "echo 'Writing file'"
code-buddy hooks remove before_write
```
"#.to_string())
}

fn show_mcp_help() -> Result<String> {
    Ok(r#"# MCP (Model Context Protocol)

## MCP Servers

Connect to MCP servers for extended capabilities.

## Usage

```bash
code-buddy mcp list          # List connected servers
code-buddy mcp add <url>    # Add a server
code-buddy mcp remove <name> # Remove a server
```

## Resources

- `ListMcpResources` - List available resources
- `ReadMcpResource` - Read a specific resource
"#.to_string())
}

fn show_config_help() -> Result<String> {
    Ok(r#"# Configuration

## Config File

Location: `~/.config/code-buddy/config.json`

## Settings

| Setting | Description | Default |
|---------|-------------|---------|
| `model` | LLM model | claude-opus |
| `temperature` | Response randomness | 0.7 |
| `maxTokens` | Max response tokens | 8192 |
| `autoCompact` | Auto compact | true |

## Commands

```bash
code-buddy config list     # List all config
code-buddy config get <key> # Get a value
code-buddy config set <key> <value> # Set a value
code-buddy config edit     # Edit config file
```
"#.to_string())
}

fn show_api_help() -> Result<String> {
    Ok(r#"# API Documentation

## LLM Providers

Code Buddy supports multiple LLM providers:

| Provider | Description |
|----------|-------------|
| `anthropic` | Anthropic Claude |
| `openai` | OpenAI GPT |
| `openrouter` | OpenRouter |
| `ollama` | Ollama local |
| `mlx` | Apple MLX |
| `nvidia` | NVIDIA NIM |

## Environment Variables

- `ANTHROPIC_API_KEY` - Anthropic API key
- `OPENAI_API_KEY` - OpenAI API key
- `OPENROUTER_API_KEY` - OpenRouter API key
"#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help() {
        let help = show_general_help().unwrap();
        assert!(help.contains("Code Buddy Help"));
    }
}
