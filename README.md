<div align="center">

```
  ██████╗ ██████╗ ██████╗ ███████╗    ██████╗ ██╗   ██╗██████╗ ██████╗ ██╗   ██╗
 ██╔════╝██╔═══██╗██╔══██╗██╔════╝    ██╔══██╗██║   ██║██╔══██╗██╔══██╗╚██╗ ██╔╝
 ██║     ██║   ██║██║  ██║█████╗      ██████╔╝██║   ██║██║  ██║██║  ██║ ╚████╔╝
 ██║     ██║   ██║██║  ██║██╔══╝      ██╔══██╗██║   ██║██║  ██║██║  ██║  ╚██╔╝
 ╚██████╗╚██████╔╝██████╔╝███████╗    ██████╔╝╚██████╔╝██████╔╝██████╔╝   ██║
  ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝    ╚═════╝  ╚═════╝ ╚═════╝ ╚═════╝   ╚═╝
```

**AI coding assistant for your terminal — simple enough for anyone**

[![CI](https://github.com/simpletoolsindia/code-buddy/actions/workflows/ci.yml/badge.svg)](https://github.com/simpletoolsindia/code-buddy/actions/workflows/ci.yml)
[![Release](https://github.com/simpletoolsindia/code-buddy/actions/workflows/release.yml/badge.svg)](https://github.com/simpletoolsindia/code-buddy/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)

*Claude Code-style TUI · File & shell tools · Web search · NVIDIA · Ollama · LM Studio · OpenRouter · OpenAI*

</div>

---

## Quick Install

| Platform | Command |
|:---|:---|
| **Linux & macOS** | `curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh \| bash` |
| **Windows** | `irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 \| iex` |
| **Homebrew** | `brew install simpletoolsindia/tap/code-buddy` |
| **Cargo** | `cargo install --git https://github.com/simpletoolsindia/code-buddy --bin code-buddy --locked` |

> **First run**: just type `code-buddy`. The setup wizard launches automatically and guides you step by step.

---

## Features

| Feature | Description |
|:---|:---|
| **Simple setup** | Interactive wizard asks numbered questions — pick a provider, paste your key, choose a model |
| **Live model list** | Automatically fetches available models from your provider's API |
| **Remote access** | Connect to LM Studio or Ollama running on another computer (LAN or internet) |
| **Tool calling** | Read, write, and search files; run shell commands; search the web |
| **Terminal bell** | Rings when Code Buddy finishes thinking and is ready |
| **Streaming** | Watch the AI think and respond in real time |
| **Tab completion** | Press Tab to autocomplete slash commands |
| **Switch LLM mid-session** | Type `/provider` or `/model` to switch LLM without restarting |

---

## Supported Providers

| Provider | Type | Notes |
|:---|:---:|---|
| **NVIDIA NIM** | Cloud | Free credits, Llama/Mistral/Gemma models. Setup fetches models live from the API. |
| **OpenRouter** | Cloud | 200+ models including free tier |
| **OpenAI** | Cloud | GPT-4o, o1, o3 |
| **LM Studio** | Local/Remote | Run free models on your PC or a remote server |
| **Ollama** | Local/Remote | Free, no GPU needed, supports remote access |
| **Custom** | Either | Any OpenAI-compatible API endpoint |

---

## Usage

### Interactive chat (recommended)

```bash
code-buddy
```

The TUI shows your provider and model, lists available tools, and rings a bell when the AI is done thinking.

### One-shot question

```bash
code-buddy ask "How do I reverse a string in Python?"
```

### Commands

| Command | What it does |
|:---|:---|
| `/help` | Show all commands |
| `/quit` | Exit Code Buddy |
| `/clear` | Start a new conversation |
| `/status` | Check provider, model, and tools |
| `/tools` | See all available tools |
| `/provider` | Switch to a different AI provider (interactive) |
| `/model` | Switch to a different model (interactive) |

---

## Tools

Code Buddy has built-in tools it can use to help you. No extra setup needed.

| Tool | What it does |
|:---|:---|
| `read_file` | Read any file in your project |
| `write_file` | Create or overwrite a file |
| `edit_file` | Make targeted changes to a file |
| `glob_search` | Find files by pattern (e.g. `*.rs`) |
| `grep_search` | Search inside files for text |
| `run_shell` | Execute shell commands |
| `web_search` | Search the internet (free, no key needed) |
| `web_fetch` | Read a webpage as clean text |

---

## Setup Wizard

Run `code-buddy setup` anytime to reconfigure. The wizard:

1. **Choose provider** — numbered list, no arrow keys needed
2. **Enter API key** — or press Enter for local servers
3. **Pick a model** — popular models shown first
4. **Enable web search** — optional (uses DuckDuckGo by default, no key needed)

For **NVIDIA** (recommended for new users):
- Get a free key at [build.nvidia.com](https://build.nvidia.com/)
- Sign up → click your profile → Copy API Key
- Paste it when the wizard asks

For **LM Studio** (runs free on your PC):
- Download [LM Studio](https://lmstudio.ai/) and install it
- Load a model in LM Studio
- Press Enter when the wizard asks for the server URL

For **remote access** (connect to a server on your LAN):
- Enter the server's IP address when asked for the URL
- Example: `http://192.168.1.100:1234`

---

## Configuration

Config file: `~/.config/code-buddy/config.toml`

```bash
code-buddy config show              # View current settings
code-buddy config set model gpt-4o  # Change model
code-buddy config set provider nvidia
```

### Environment variables

Every setting can be overridden with an env var:

| Setting | Env Variable | Default |
|:---|:---|:---|
| Provider | `CODE_BUDDY_PROVIDER` | `lm-studio` |
| Model | `CODE_BUDDY_MODEL` | — |
| API Key | `CODE_BUDDY_API_KEY` | — |
| Endpoint | `CODE_BUDDY_ENDPOINT` | provider default |
| Streaming | `CODE_BUDDY_STREAMING` | `true` |
| Timeout | `CODE_BUDDY_TIMEOUT_SECONDS` | `120` |
| Brave API Key | `BRAVE_SEARCH_API_KEY` | — (optional, for Brave Search) |
| SerpAPI Key | `SERPAPI_KEY` | — (optional, for SerpAPI) |

Example with environment variables:

```bash
CODE_BUDDY_PROVIDER=nvidia \
CODE_BUDDY_MODEL=meta/llama-3.3-70b-instruct \
CODE_BUDDY_API_KEY=nvapi-... \
code-buddy ask "Hello"
```

### Web search

```bash
# DuckDuckGo (free, no API key needed — enabled by default)
# No configuration needed! Works out of the box.

# Brave Search (higher quality) — https://brave.com/search/api/
code-buddy config set brave_api_key YOUR_KEY

# SerpAPI (fallback) — https://serpapi.com/
code-buddy config set serpapi_key YOUR_KEY

# Firecrawl (better page extraction) — https://firecrawl.dev/
code-buddy config set firecrawl_api_key YOUR_KEY
```

---

## Build from Source

Requires **Rust 1.80+**.

```bash
git clone https://github.com/simpletoolsindia/code-buddy
cd code-buddy
cargo build --release
./target/release/code-buddy --version
```

---

## Project Layout

```
crates/
├── cli/        # Binary, commands, TUI
├── config/     # Config loading & env overrides
├── providers/  # Provider adapters + live model fetching
├── tools/      # Tool registry: file, shell, web
├── errors/     # Shared error types
├── runtime/    # Conversation loop & tool dispatch
└── transport/ # OpenAI-compatible HTTP + SSE streaming
```

---

## License

MIT © [simpletoolsindia](https://github.com/simpletoolsindia)
