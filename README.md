# Code Buddy

**Code Buddy** is a powerful AI coding assistant CLI with support for 14+ LLM providers. It brings Claude Code-like capabilities to any machine learning model.

## Performance

Code Buddy is **5-19x faster** than original Claude Code (Node.js):

| Metric | Code Buddy (Rust) | Claude Code (Node.js) | Speedup |
|--------|-------------------|----------------------|---------|
| Simple Math (2+2) | **0.36s** | 6.9s | ⭐ **19x faster** |
| Code Generation | **1.5s** | 7.0s | ⭐ **4.6x faster** |
| Explanation | **0.73s** | 8.5s | ⭐ **11.6x faster** |

### Why Code Buddy is Faster:

- **Rust**: Native binary, no runtime overhead
- **Single Binary**: ~5MB vs ~12MB (Node.js bundle)
- **Instant Startup**: No Node.js initialization
- **Low Memory**: ~20MB vs ~100MB

## Features

- **Multi-Provider Support**: Works with Anthropic, OpenAI, OpenRouter, NVIDIA NIM, Ollama, LM Studio, Groq, DeepSeek, Mistral, Perplexity, Together, AWS Bedrock, Azure, and Google Vertex AI
- **Local Models**: Run entirely on your own hardware with Ollama or LM Studio
- **Free Models**: Access free models via OpenRouter
- **MCP Server Support**: Connect to Model Context Protocol servers
- **Streaming Responses**: Real-time token-by-token output
- **Conversation History**: Maintains context across interactions
- **Extensible**: Built with Rust for performance and safety

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/simpletoolsindia/code-buddy.git
cd code-buddy

# Build
cargo build --release

# Install
cargo install --path . --force

# Or copy binary directly
cp target/release/code-buddy ~/.local/bin/
```

### Prerequisites

- **Rust** (for building from source): Install via [rustup](https://rustup.rs/)
- **API Key**: Depending on your provider (see Configuration below)

## Quick Start

### 1. Configure Your LLM Provider

```bash
# For NVIDIA NIM (FREE, fast inference) - RECOMMENDED
code-buddy config set llm_provider nvidia
code-buddy config set api_key YOUR_NVIDIA_API_KEY

# For Ollama (local models - no API key needed)
code-buddy config set llm_provider ollama

# For OpenRouter (includes free models)
code-buddy config set llm_provider openrouter
code-buddy config set api_key your-openrouter-key

# For Anthropic
code-buddy config set api_key your-anthropic-key

# For OpenAI
code-buddy config set llm_provider openai
code-buddy config set api_key your-openai-key
```

### 2. Run Your First Prompt

```bash
# Interactive mode (when full implementation is ready)
code-buddy

# Non-interactive mode (print response and exit)
code-buddy -p "Hello, write a hello world in Python"

# With streaming output
code-buddy -p "Explain this code" --output-format stream-json

# With specific model
code-buddy -p "Write a Rust web server" --model opus
```

## Configuration

### Configuration File

Config is stored at:
- **Linux**: `~/.config/code-buddy/config.json`
- **macOS**: `~/Library/Application Support/code-buddy/config.json`
- **Windows**: `%APPDATA%/code-buddy/config.json`

### Environment Variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `OPENROUTER_API_KEY` | OpenRouter API key |
| `ANTHROPIC_BASE_URL` | Custom API endpoint |
| `OLLAMA_HOST` | Ollama host (default: localhost:11434) |

### CLI Configuration

```bash
# List current config
code-buddy config list

# Get specific value
code-buddy config get llm_provider

# Set values
code-buddy config set llm_provider ollama
code-buddy config set model llama3.2

# Edit config file directly
code-buddy config edit
```

## LLM Providers

### Ollama (Recommended for Local Development)

Run models locally on your machine.

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a model
ollama pull llama3.2
ollama pull qwen3-coder:30b

# Use with code-buddy
code-buddy config set llm_provider ollama
code-buddy -p "Write a REST API in Go"
```

### OpenRouter (Free Models Available)

Access 100+ models including Claude, GPT, and Gemini.

```bash
# Get API key from https://openrouter.ai/keys
export OPENROUTER_API_KEY=sk-or-...

# Use with code-buddy
code-buddy config set llm_provider openrouter
code-buddy config set api_key $OPENROUTER_API_KEY

# Use free models
code-buddy config set model google/gemini-2.5-flash-preview-05-20:free
code-buddy -p "Optimize this SQL query"
```

### Anthropic (Claude)

```bash
# Get API key from https://console.anthropic.com/
code-buddy config set api_key $ANTHROPIC_API_KEY
```

### OpenAI

```bash
code-buddy config set llm_provider openai
code-buddy config set api_key $OPENAI_API_KEY
```

### NVIDIA NIM (FREE Tier Available)

Access NVIDIA's hosted models with a free tier. Get your API key at [build.nvidia.com](https://build.nvidia.com/).

```bash
# Get free API key from https://build.nvidia.com/
code-buddy config set llm_provider nvidia
code-buddy config set api_key $NVIDIA_API_KEY

# Free tier includes: llama-3.1-nemotron-70b-instruct, llama-3.1-8b-instruct, etc.
code-buddy -p "Write a Python web scraper"
```

### Other Providers

| Provider | LLM Provider Value |
|----------|-------------------|
| Groq | `groq` |
| DeepSeek | `deepseek` |
| Mistral | `mistral` |
| Perplexity | `perplexity` |
| Together | `together` |
| AWS Bedrock | `bedrock` |
| Azure OpenAI | `azure` |
| Google Vertex | `vertex` |

## MCP Server Support

Code Buddy supports Model Context Protocol servers.

```bash
# Add an MCP server
code-buddy mcp add my-server npx -y @modelcontextprotocol/server-filesystem

# List configured servers
code-buddy mcp list

# Import from Claude Desktop
code-buddy mcp add-from-claude-desktop

# Remove a server
code-buddy mcp remove my-server
```

## Commands

```bash
# Authentication
code-buddy auth login [--api-key KEY]   # Login with API key
code-buddy auth logout                   # Logout and clear credentials
code-buddy auth status                   # Show auth status

# Configuration
code-buddy config list                   # List all config
code-buddy config get <key>             # Get config value
code-buddy config set <key> <value>     # Set config value
code-buddy config edit                   # Edit config in $EDITOR

# Model
code-buddy model                         # Show current model
code-buddy model <model-name>           # Set default model

# MCP Servers
code-buddy mcp list                      # List MCP servers
code-buddy mcp add <name> <command>     # Add MCP server
code-buddy mcp remove <name>            # Remove MCP server

# Agents
code-buddy agents --list                # List configured agents

# System
code-buddy status                        # Show status
code-buddy doctor                        # Health checks
code-buddy version                       # Show version

# Installation
code-buddy install [target]             # Install native build
code-buddy update                        # Check for updates
```

## Output Formats

```bash
# Plain text (default)
code-buddy -p "Hello"

# JSON output
code-buddy -p "Hello" --output-format json

# Streaming JSON (verbose)
code-buddy -p "Hello" --output-format stream-json --verbose
```

## Options

```bash
code-buddy [options] [command] [prompt]

Options:
  -p, --print                 Print response and exit
  --output-format <format>    Output format: text, json, stream-json
  --model <model>            Model to use
  --agent <name>             Agent to use
  --verbose                   Enable verbose output
  -d, --debug [filter]       Enable debug mode
  -h, --help                  Show help
  -v, --version               Show version

Print Mode:
  code-buddy -p "Your prompt here"

Interactive Mode:
  code-buddy
```

## Architecture

```
+-------------------------------------------------------------+
|                      Code Buddy CLI                         |
+-------------------------------------------------------------+
|  CLI Layer (clap)                                           |
|    - Argument parsing                                       |
|    - Command routing                                       |
+-------------------------------------------------------------+
|  Command Layer (commands/)                                  |
|    - print, auth, config, mcp, model, status, etc.        |
+-------------------------------------------------------------+
|  API Layer (api/)                                           |
|    - Multi-provider client                                  |
|    - Format translation                                     |
|    - Streaming support                                     |
+-------------------------------------------------------------+
|  Provider Adapters                                         |
|  +---------+ +---------+ +----------+ +---------+         |
|  |Anthropic| | OpenAI  | | OpenRouter| | Ollama  | ...    |
|  +---------+ +---------+ +----------+ +---------+         |
+-------------------------------------------------------------+
```

## Development

```bash
# Build
cargo build

# Run
cargo run -- -p "Hello"

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Roadmap

- [x] Multi-provider support (14+ providers)
- [x] Print mode with streaming
- [x] Conversation history
- [x] MCP server integration
- [x] Configuration management
- [ ] Interactive REPL mode
- [ ] Full tool execution (Bash, Read, Edit, etc.)
- [ ] Plugin system
- [ ] Voice mode

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## Links

- **Repository**: https://github.com/simpletoolsindia/code-buddy
- **Issues**: https://github.com/simpletoolsindia/code-buddy/issues
