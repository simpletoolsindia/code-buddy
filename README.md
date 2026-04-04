# Code Buddy v3.0.0

<div align="center">

![Code Buddy](https://img.shields.io/badge/Code%20Buddy-v3.0.0-gold?style=for-the-badge)
![Rust](https://img.shields.io/badge/Rust-1.75+-orange?style=for-the-badge)
![License](https://img.shields.io/badge/License-MIT-green?style=for-the-badge)
![Performance](https://img.shields.io/badge/Performance-5--19x%20faster-blue?style=for-the-badge)

**High-performance AI coding assistant CLI** with 21++ LLM providers, built-in tools, and Claude Code-like capabilities.

[Features](#features) • [Quick Start](#quick-start) • [Installation](#installation) • [Tools](#tools) • [Documentation](#documentation)

</div>

---

## Why Code Buddy?

Code Buddy is **5-19x faster** than Node.js Claude Code, written in Rust for native performance.

| Benchmark | Code Buddy (Rust) | Claude Code (Node.js) | Speedup |
|-----------|-------------------|----------------------|---------|
| Simple Math | **0.36s** | 6.9s | `⭐ 19x faster` |
| Code Generation | **1.5s** | 7.0s | `⭐ 4.6x faster` |
| Explanation | **0.73s** | 8.5s | `⭐ 11.6x faster` |

**Built with Rust** for:
- **Native binary** (~5MB) — no runtime overhead
- **Instant startup** — no Node.js initialization  
- **Low memory** — ~20MB vs ~100MB
- **Concurrent execution** — built-in async support

---

## Features

### Multi-Provider LLM Support (20+ Providers)

```
┌─────────────────────────────────────────────────────────────────┐
│                     CODE BUDDY                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ☁️  CLOUD PROVIDERS          │  🏠  LOCAL PROVIDERS          │
│   ─────────────────────       │  ──────────────────────       │
│   • Anthropic (Claude)         │  • Ollama                     │
│   • OpenAI (GPT-4/4o)         │  • LM Studio                  │
│   • OpenRouter (100+ models)  │  • MLX (Apple Silicon)        │
│   • NVIDIA NIM (FREE tier)    │                               │
│   • Groq                      │  🔧  OTHER PROVIDERS           │
│   • DeepSeek                  │  ──────────────────────       │
│   • Mistral                   │  • AWS Bedrock                 │
│   • Perplexity                │  • Azure OpenAI               │
│   • Together AI               │  • Google Vertex AI            │
│   • HuggingFace Inference     │  • Fireworks AI               │
│   • Cerebras (Ultra-fast)     │  • SambaNova                  │
│   • Cohere, AI21, Replicate   │                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Built-in Tools (23 Tools)

| Category | Tools |
|----------|-------|
| **File** | `Read`, `Write`, `Edit`, `Glob`, `Grep`, `Mkdir`, `Rm`, `Cp`, `Mv` |
| **Execution** | `Bash`, `Sandbox`, `Container`, `Batch` |
| **Web** | `WebSearch`, `WebFetch` |
| **AI** | `MixtureOfAgents`, `ImageGenerate` |
| **System** | `Cron`, `Skin`, `Profile`, `AcpServer` |
| **Interactive** | `AskUserQuestion`, `NotebookEdit` |
| **MCP** | `McpServers`, `ListMcpResources`, `ReadMcpResource` |
| **Tasks** | `TaskCreate`, `TaskComplete` |

### Advanced Capabilities

| Feature | Description |
|---------|-------------|
| **Skills Hub** | Browse & install skills from agentskills.io marketplace |
| **Prompt Caching** | 75% cost savings, 5x speed boost (Anthropic) |
| **Memory System** | SQLite + FTS5 full-text search across conversations |
| **Cron Jobs** | Schedule recurring tasks with intervals or cron expressions |
| **Code Sandbox** | Secure code execution (Python, Rust, Go, JS, etc.) |
| **Container Support** | Docker, SSH, Modal, Daytona backends |
| **Batch Runner** | Parallel task execution with concurrency control |
| **Mixture of Agents** | Ensemble reasoning with multiple AI perspectives |
| **Image Generation** | AI images via OpenAI DALL-E, Stability AI |
| **Skin Engine** | Theme customization (4 built-in skins) |
| **Multi-Instance Profiles** | Isolated environments with `CODE_BUDDY_HOME` |
| **ACP Server** | IDE integration for VS Code, JetBrains, Zed |
| **Context Files** | AGENTS.md & CLAUDE.md project guidelines |
| **Vision Support** | Screenshot capture & vision model detection |
| **Computer Use** | Mouse/keyboard control for automation |
| **Advanced Collaboration** | Shared context, voting, parallel execution |
| **Distributed Execution** | Background queue, progress tracking, worker management |

### Multi-Agent System

```
┌────────────────────────────────────────────────────┐
│                 AGENT ARCHITECTURE                │
├────────────────────────────────────────────────────┤
│                                                    │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│   │ Analyzer │  │ Debugger │  │Reviewer  │      │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘      │
│        │             │             │             │
│        └─────────────┼─────────────┘             │
│                      ▼                           │
│              ┌─────────────┐                     │
│              │   Ensemble  │                     │
│              │   (MoA)    │                     │
│              └──────┬──────┘                     │
│                     │                            │
│        ┌────────────┼────────────┐              │
│        ▼            ▼            ▼              │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐        │
│   │Architect│  │Security │  │Pragmatist│        │
│   └─────────┘  └─────────┘  └─────────┘        │
│                                                    │
└────────────────────────────────────────────────────┘
```

---

## Quick Start

### One-Command Installation

```bash
# MLX (Apple Silicon Mac - FREE, local inference)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s mlx

# NVIDIA NIM (FREE tier - RECOMMENDED)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s nvidia YOUR_NVIDIA_API_KEY

# OpenRouter (free models available)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s openrouter YOUR_API_KEY

# Ollama (local models - no API key needed)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash -s ollama
```

### Run Your First Prompt

```bash
# Interactive REPL mode
code-buddy

# One-liner mode
code-buddy -p "Write a hello world in Python"

# With specific model
code-buddy -p "Explain this code" --model opus

# Streaming output
code-buddy -p "Write a Rust web server" --output-format stream-json
```

---

## Installation

### Automated Installers

```bash
# Linux/macOS (one-liner)
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install-simple.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex
```

### Via Cargo

```bash
cargo install --git https://github.com/simpletoolsindia/code-buddy.git
```

### From Source

```bash
git clone https://github.com/simpletoolsindia/code-buddy.git
cd code-buddy
cargo install --path . --force
```

---

## Tools

### File Operations

```bash
Read("/path/to/file.rs")
Write("/path/file.txt", "content")
Edit("/path/file.txt", "old text", "new text")
Glob("**/*.rs")
Grep("TODO", "src/")
```

### Code Execution

```bash
# Bash - run shell commands
Bash("ls -la && cargo build")

# Sandbox - safe code execution
Sandbox("python", "print('Hello from sandbox!')")
Sandbox("rust", "fn main() { println!(\"Hello\"); }")

# Container - Docker/SSH execution
Container("docker", "cargo build", "--image rust:1.75")
Container("ssh", "ls", "--host user@server")
```

### Scheduling & Automation

```bash
# Cron - schedule recurring tasks
Cron("create", "30m", "Check disk space")
Cron("create", "0 9 * * *", "Morning report")
Cron("list")
Cron("delete", "job-id")

# Batch - parallel execution
Batch("lint module1.rs", "lint module2.rs", "lint module3.rs", "--concurrency 4")
```

### AI-Powered Tools

```bash
# Mixture of Agents - ensemble reasoning
MixtureOfAgents("Review this architecture for scalability")
MoA("Debug why API returns 500 on POST /users", "--agents 3")

# Image Generation
ImageGenerate("A futuristic city at sunset", "--width 1024 --height 512")
ImageGenerate("Architecture diagram", "--provider openai")
```

### System Management

```bash
# Profiles - isolated environments
Profile("list")
Profile("create", "work-project")
Profile("switch", "work-project")

# Skin - theme customization
Skin("list")
Skin("apply", "dracula")
Skin("create", "my-theme", "Custom dark theme")

# ACP Server - IDE integration
AcpServer("start", "--port 8080")
AcpServer("status")
```

---

## Documentation

### Configuration

```bash
# Interactive setup
code-buddy setup

# CLI configuration
code-buddy config set llm_provider nvidia
code-buddy config set api_key YOUR_KEY
code-buddy config set model llama-3.1-nemotron-70b-instruct
```

### LLM Providers

| Provider | Command | API Key Required |
|----------|---------|------------------|
| **NVIDIA NIM** | `--provider nvidia` | Yes (FREE tier) |
| **OpenRouter** | `--provider openrouter` | Yes (free models) |
| **Ollama** | `--provider ollama` | No |
| **MLX** | `--provider mlx` | No (local) |
| **Anthropic** | `--provider anthropic` | Yes |
| **OpenAI** | `--provider openai` | Yes |

### Environment Variables

```bash
# API Keys
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
OPENROUTER_API_KEY=sk-or-...
NVIDIA_API_KEY=nvapi-...

# Configuration
LLM_PROVIDER=nvidia          # Default provider
CODE_BUDDY_HOME=~/.code-buddy  # Data directory
AUTO_COMPACT=true            # Enable auto-compact
```

### MCP Server Support

```bash
# Add MCP server
code-buddy mcp add filesystem npx -y @modelcontextprotocol/server-filesystem

# Import from Claude Desktop
code-buddy mcp add-from-claude-desktop

# List servers
code-buddy mcp list
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        CODE BUDDY CLI                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐        │
│  │  CLI Layer  │   │ Command Layer│   │  API Layer   │        │
│  │   (clap)    │──▶│   (REPL)     │──▶│  (reqwest)   │        │
│  └──────────────┘   └──────────────┘   └──────────────┘        │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                     TOOLS LAYER                            │  │
│  │  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐            │  │
│  │  │ File   │ │Execute │ │  Web   │ │   AI   │            │  │
│  │  │ Ops   │ │(Sandbox│ │(Search │ │(MoA/   │            │  │
│  │  │       │ │/Batch) │ │ /Fetch)│ │ Image) │            │  │
│  │  └────────┘ └────────┘ └────────┘ └────────┘            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐        │
│  │    Cron      │   │   Memory     │   │    Skills    │        │
│  │  Scheduler   │   │   (SQLite)   │   │    Hub       │        │
│  └──────────────┘   └──────────────┘   └──────────────┘        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Development

```bash
# Build
cargo build

# Run
cargo run -- -p "Hello, world!"

# Tests
cargo test

# Format
cargo fmt

# Lint
cargo clippy
```

---

## Roadmap

### Completed (v3.0.0)

- [x] Multi-provider support (21++ providers)
- [x] Apple Silicon MLX support
- [x] Skills Hub marketplace
- [x] Prompt Caching (75% cost savings)
- [x] SQLite Memory with FTS5
- [x] Cron Job Scheduling
- [x] Code Sandbox
- [x] Container Support (Docker/SSH/Modal)
- [x] Batch Runner
- [x] Mixture of Agents
- [x] Image Generation
- [x] Skin Engine
- [x] Multi-Instance Profiles
- [x] ACP Server (IDE Integration)
- [x] Context Files (AGENTS.md/CLAUDE.md)
- [x] Vision & Computer Use
- [x] 50+ CLI Commands
- [x] 23 Built-in Tools
- [x] Enhanced streaming with JSON/tool parsing
- [x] Additional providers (HuggingFace, Fireworks, Cerebras, SambaNova)

### Coming Soon

- [ ] More integrations and enhancements
- [ ] Advanced agent collaboration
- [ ] Distributed execution

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

---

## Links

| Resource | URL |
|----------|-----|
| Repository | https://github.com/simpletoolsindia/code-buddy |
| Issues | https://github.com/simpletoolsindia/code-buddy/issues |
| MLX Models | https://huggingface.co/mlx-community |
| Skills Hub | https://agentskills.io |

---

<div align="center">

**Built with Rust** for speed and reliability.

*Code Buddy — Your AI Coding Companion*

</div>
