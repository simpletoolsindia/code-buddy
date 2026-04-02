# Code Buddy

**Code Buddy** is a powerful AI coding assistant CLI with support for 15+ LLM providers. It brings Claude Code-like capabilities to any machine with native performance.

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

### Core Features
- **Multi-Provider Support**: Anthropic, OpenAI, OpenRouter, NVIDIA NIM, Ollama, LM Studio, Groq, DeepSeek, Mistral, Perplexity, Together, AWS Bedrock, Azure, Google Vertex AI, **MLX (Apple Silicon)**
- **Local Models**: Run entirely on your own hardware with Ollama, LM Studio, or MLX
- **Apple Silicon MLX**: Native LLM inference on M1/M2/M3/M4 Macs using Apple's MLX framework
- **Free Models**: Access free models via OpenRouter and NVIDIA NIM
- **MCP Server Support**: Connect to Model Context Protocol servers
- **Streaming Responses**: Real-time token-by-token output
- **Conversation History**: Maintains context across interactions
- **Auto-Compact**: Automatic summarization of long conversations
- **Plugin System**: Extensible architecture for custom functionality

### Developer Tools
- **Tool Execution**: Built-in tools for file operations, bash commands, web search, and more
- **Bash Execution**: Run shell commands with safety checks
- **File Operations**: Read, write, edit, mkdir, rm, cp, mv
- **Web Search**: Search the web for current information
- **Web Fetch**: Retrieve and analyze web page content
- **Pattern Matching**: Glob and grep for code exploration

### Commands & Interface
- **Interactive REPL**: Chat-style interface with slash commands
- **Print Mode**: One-liner mode for quick queries
- **JSON Output**: Machine-readable output format
- **Streaming**: Real-time token streaming

## Installation

### One-Command Install (Linux/macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash
```

Or with configuration:

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s nvidia YOUR_NVIDIA_API_KEY
```

### Windows

```powershell
# Run in PowerShell (Admin)
irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex
```

Or with configuration:

```powershell
irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex -ProviderName nvidia -ApiKey YOUR_KEY
```

### Via Cargo (Cross-Platform)

```bash
cargo install --git https://github.com/simpletoolsindia/code-buddy.git
```

### From Source

```bash
git clone https://github.com/simpletoolsindia/code-buddy.git
cd code-buddy
cargo install --path . --force
```

### Prerequisites

- **Rust** (optional, auto-installed by installer): Install via [rustup](https://rustup.rs/)
- **API Key**: Depending on your provider (see Configuration below)
- **Python** (optional, for MLX): Required for Apple Silicon MLX inference
- **Ollama** (optional, for local models): Install from [ollama.com](https://ollama.com)

## Quick Start

### One-Command Setup (Recommended)

Install + Configure + Run:

```bash
# With NVIDIA NIM (FREE)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s nvidia YOUR_NVIDIA_API_KEY

# Then run:
code-buddy -p "Hello, world!"
```

### Interactive Setup

```bash
# Install (if not done)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash

# Run setup wizard (includes MLX setup on Apple Silicon)
code-buddy setup

# Or run directly
code-buddy -p "Write a hello world in Python"
```

### Provider Configuration

```bash
# NVIDIA NIM (FREE, fast inference) - RECOMMENDED
code-buddy config set llm_provider nvidia
code-buddy config set api_key YOUR_NVIDIA_API_KEY

# Ollama (local models - no API key needed)
code-buddy config set llm_provider ollama

# MLX (Apple Silicon - FREE, local inference)
code-buddy config set llm_provider mlx
code-buddy --mlx  # Interactive MLX setup

# OpenRouter (includes free models)
code-buddy config set llm_provider openrouter
code-buddy config set api_key your-openrouter-key

# Anthropic
code-buddy config set api_key your-anthropic-key

# OpenAI
code-buddy config set llm_provider openai
code-buddy config set api_key your-openai-key
```

### 2. Run Your First Prompt

```bash
# Interactive mode (REPL)
code-buddy

# Non-interactive mode (print response and exit)
code-buddy -p "Hello, write a hello world in Python"

# With streaming output
code-buddy -p "Explain this code" --output-format stream-json

# With specific model
code-buddy -p "Write a Rust web server" --model opus
```

## MLX Apple Silicon Support

Code Buddy supports native LLM inference on Apple Silicon Macs using Apple's MLX framework.

### Setup

```bash
# Interactive MLX setup
code-buddy --mlx

# List available models
code-buddy --mlx-list-models

# Download a specific model
code-buddy --mlx-download mlx-community/llama-3.2-3b-instruct-4bit

# Set MLX as provider
code-buddy --mlx-model mlx-community/llama-3.2-3b-instruct-4bit
```

### Popular MLX Models

| Model | Size | Description |
|-------|------|-------------|
| Llama 3.2 1B | ~700MB | Lightweight, fast |
| Llama 3.2 3B | ~2GB | Balanced performance |
| Qwen 2.5 1.5B | ~1GB | Efficient |
| Gemma 2B | ~1.8GB | Google's model |
| Llama 3.1 8B | ~5GB | Full-featured |
| Mistral 7B | ~4GB | High quality |

### Manual Installation

```bash
# Install mlx-lm Python package
pip install mlx-lm

# Verify installation
python3 -c "import mlx_lm; print('MLX ready!')"

# Use with code-buddy
code-buddy --provider mlx --model mlx-community/llama-3.2-3b-instruct-4bit -p "Hello"
```

## Configuration

### Interactive Setup

The easiest way to get started is using the interactive setup wizard:

```bash
code-buddy setup
```

This will guide you through:
1. Selecting your LLM provider (Cloud or Local)
2. Choosing a model (including MLX models on Apple Silicon)
3. Entering your API key (if needed)

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
| `NVIDIA_API_KEY` | NVIDIA NIM API key |
| `GROQ_API_KEY` | Groq API key |
| `DEEPSEEK_API_KEY` | DeepSeek API key |
| `TOGETHER_API_KEY` | Together AI API key |
| `ANTHROPIC_BASE_URL` | Custom API endpoint |
| `OLLAMA_HOST` | Ollama host (default: localhost:11434) |
| `LLM_PROVIDER` | Default provider (anthropic, openai, ollama, mlx, etc.) |

### Auto-Compact Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `AUTO_COMPACT` | true | Enable auto-compact |
| `COMPACT_THRESHOLD` | 85 | % of context window to trigger compact |
| `COMPACT_MESSAGES` | 20 | Messages to keep after compact |

### CLI Configuration

```bash
# List current config
code-buddy config list

# Get specific value
code-buddy config get llm_provider

# Set values
code-buddy config set llm_provider ollama
code-buddy config set model llama3.2
code-buddy config set api_key YOUR_KEY

# Reset configuration
code-buddy reset                # Show reset options
code-buddy reset --all         # Full factory reset

# Edit config file directly
code-buddy config edit
```

## LLM Providers

### MLX (Apple Silicon - Recommended for Mac Users)

Native LLM inference using Apple's MLX framework. No API key needed, runs locally.

```bash
# Setup MLX
code-buddy --mlx

# Use with code-buddy
code-buddy --provider mlx --model mlx-community/llama-3.2-3b-instruct-4bit -p "Hello"
```

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
| LM Studio | `lmstudio` |
| **MLX** | `mlx` |

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

# Reset project choices
code-buddy mcp reset-project-choices
```

## REPL Commands

In interactive REPL mode (`code-buddy`), use slash commands:

| Command | Description |
|---------|-------------|
| `/help` | Show available commands |
| `/quit`, `/exit` | Exit Code Buddy |
| `/clear` | Clear conversation history |
| `/status` | Show current configuration |
| `/model <name>` | Change model |
| `/provider <name>` | Change LLM provider |
| `/history` | Show conversation history |
| `/reset` | Reset conversation |
| `/models` | List available models |
| `/cost` | Show estimated costs |
| `/compact` | Compact context window |
| `/context` | Show context usage |
| `/system` | Show system configuration |
| `/set <key> <value>` | Set configuration option |
| `/update` | Check for or install updates |

## Commands

```bash
# Setup & Configuration
code-buddy setup                         # Interactive setup wizard
code-buddy reset                          # Reset configuration
code-buddy reset --all                   # Full factory reset

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
code-buddy mcp list                     # List MCP servers
code-buddy mcp add <name> <command>     # Add MCP server
code-buddy mcp remove <name>            # Remove MCP server

# MLX (Apple Silicon)
code-buddy --mlx                        # Interactive MLX setup
code-buddy --mlx-list-models           # List available MLX models
code-buddy --mlx-download <model-id>    # Download MLX model
code-buddy --mlx-model <model-id>       # Set MLX model

# Agents
code-buddy agents --list                # List configured agents

# System
code-buddy status                        # Show status
code-buddy doctor                        # Health checks (includes MLX check)
code-buddy version                       # Show version

# Installation
code-buddy install [target]             # Install native build
code-buddy update                        # Check for updates
code-buddy --self-update                # Self-update
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
  --provider <provider>       LLM provider (anthropic, openai, ollama, mlx, etc.)
  --agent <name>             Agent to use
  --verbose                   Enable verbose output
  -d, --debug [filter]       Enable debug mode
  --mlx                       MLX setup (Apple Silicon)
  --mlx-model <model-id>     Set MLX model
  --mlx-download <model-id>  Download MLX model
  --mlx-list-models           List MLX models
  -h, --help                  Show help
  -v, --version               Show version
```

## Built-in Tools

Code Buddy includes powerful built-in tools:

| Tool | Description | Example |
|------|-------------|---------|
| `bash` | Execute shell commands | `bash "ls -la"` |
| `read` | Read file contents | `read "/path/to/file"` |
| `write` | Write content to file | `write "/path" "content"` |
| `edit` | Edit file with changes | `edit "/path" "old" "new"` |
| `mkdir` | Create directory | `mkdir "/path/to/dir"` |
| `rm` | Remove file/directory | `rm "/path"` |
| `cp` | Copy file | `cp "/src" "/dest"` |
| `mv` | Move/rename file | `mv "/src" "/dest"` |
| `grep` | Search file contents | `grep "/path" "pattern"` |
| `glob` | Find files by pattern | `glob "/path" "*.rs"` |
| `websearch` | Search the web | `websearch "query"` |
| `webfetch` | Fetch web page | `webfetch "https://..."` |

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
|    - print, auth, config, mcp, model, status, setup     |
|    - REPL with slash commands                             |
+-------------------------------------------------------------+
|  API Layer (api/)                                           |
|    - Multi-provider client                                  |
|    - Format translation                                     |
|    - Streaming support                                     |
+-------------------------------------------------------------+
|  MLX Layer (mlx/)                                          |
|    - Apple Silicon detection                               |
|    - Model download from HuggingFace                      |
|    - Local inference via mlx-lm                           |
+-------------------------------------------------------------+
|  Provider Adapters                                         |
|  +---------+ +---------+ +----------+ +---------+ +------+ |
|  |Anthropic| | OpenAI  | | OpenRouter| | Ollama  | | MLX  | ... |
|  +---------+ +---------+ +----------+ +---------+ +------+ |
+-------------------------------------------------------------+
|  Tools Layer (tools/)                                      |
|    - Bash execution                                       |
|    - File operations                                       |
|    - Web search/fetch                                     |
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

# Run tests with coverage
cargo tarpaulin --out Html

# Format code
cargo fmt

# Lint
cargo clippy
```

## Testing

Code Buddy has comprehensive unit tests:

```bash
# Run all tests
cargo test

# Run tests single-threaded (avoids env var pollution)
cargo test -- --test-threads=1

# Run with coverage
cargo tarpaulin
```

## Roadmap

- [x] Multi-provider support (15+ providers)
- [x] Apple Silicon MLX support
- [x] Print mode with streaming
- [x] Conversation history
- [x] Auto-compact feature
- [x] MCP server integration
- [x] Configuration management
- [x] Interactive REPL mode
- [x] Full tool execution (Bash, Read, Edit, Glob, Grep, Web)
- [x] Plugin system
- [x] Comprehensive unit tests (114+ tests)
- [ ] Voice mode
- [ ] IDE integration
- [ ] Team collaboration features

## License

MIT License - see LICENSE file for details.

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## Links

- **Repository**: https://github.com/simpletoolsindia/code-buddy
- **Issues**: https://github.com/simpletoolsindia/code-buddy/issues
- **MLX Models**: https://huggingface.co/mlx-community
