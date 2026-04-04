<div align="center">

```
  ██████╗ ██████╗ ██████╗ ███████╗    ██████╗ ██╗   ██╗██████╗ ██████╗ ██╗   ██╗
 ██╔════╝██╔═══██╗██╔══██╗██╔════╝    ██╔══██╗██║   ██║██╔══██╗██╔══██╗╚██╗ ██╔╝
 ██║     ██║   ██║██║  ██║█████╗      ██████╔╝██║   ██║██║  ██║██║  ██║ ╚████╔╝ 
 ██║     ██║   ██║██║  ██║██╔══╝      ██╔══██╗██║   ██║██║  ██║██║  ██║  ╚██╔╝  
 ╚██████╗╚██████╔╝██████╔╝███████╗    ██████╔╝╚██████╔╝██████╔╝██████╔╝   ██║   
  ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝    ╚═════╝  ╚═════╝ ╚═════╝ ╚═════╝   ╚═╝   
```

**AI coding assistant for your terminal**

[![CI](https://github.com/simpletoolsindia/code-buddy/actions/workflows/ci.yml/badge.svg)](https://github.com/simpletoolsindia/code-buddy/actions/workflows/ci.yml)
[![Release](https://github.com/simpletoolsindia/code-buddy/actions/workflows/release.yml/badge.svg)](https://github.com/simpletoolsindia/code-buddy/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.80%2B-orange?logo=rust)](https://www.rust-lang.org/)

*Claude Code-style TUI · File & shell tools · Web search · Ollama · LM Studio · OpenRouter · OpenAI*

</div>

---

## Quick Install

<table>
<tr>
<td><b>Linux & macOS</b></td>
<td>

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | sh
```

</td>
</tr>
<tr>
<td><b>Windows</b></td>
<td>

```powershell
irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex
```

</td>
</tr>
<tr>
<td><b>Homebrew</b></td>
<td>

```bash
brew install simpletoolsindia/tap/code-buddy
```

</td>
</tr>
<tr>
<td><b>Cargo</b></td>
<td>

```bash
cargo install --git https://github.com/simpletoolsindia/code-buddy --bin code-buddy --locked
```

</td>
</tr>
</table>

> **Tip:** After install, just run `code-buddy` — the setup wizard launches automatically on first run.

---

## Features

| | |
|---|---|
| **Interactive TUI** | Coloured banner, live streaming spinner, slash commands |
| **Setup wizard** | One-time guided config — pick provider → model → API keys |
| **Live model list** | Auto-fetches available models from Ollama, LM Studio, OpenRouter, OpenAI |
| **Built-in tools** | Read/write files, run shell commands, search the web, fetch pages |
| **Web search** | Brave Search (or SerpAPI fallback) + Firecrawl page extraction |
| **Multi-provider** | Ollama, LM Studio, OpenRouter, OpenAI, NVIDIA, or any OpenAI-compatible endpoint |

---

## Supported Providers

| Provider | Type | Notes |
|:---|:---:|---|
| **Ollama** | Local | `http://localhost:11434` · models auto-listed |
| **LM Studio** | Local | `http://localhost:1234` · models auto-listed |
| **OpenRouter** | Cloud | Hundreds of open & commercial models |
| **OpenAI** | Cloud | GPT-4o, o3, and more |
| **NVIDIA** | Cloud | NVIDIA AI Endpoints |
| **Custom** | Either | Any OpenAI-compatible endpoint |

---

## Usage

### Start an interactive session

```bash
code-buddy
```

#### Slash commands

| Command | What it does |
|:---|:---|
| `/tools` | List all active tools |
| `/status` | Show provider, model, and web tool status |
| `/exit` | End the session |

### One-shot question

```bash
code-buddy ask "How do I reverse a linked list?" --file src/main.rs
```

### Manage config

```bash
code-buddy setup                              # re-run the setup wizard
code-buddy config show                        # view all settings
code-buddy config set brave_api_key YOUR_KEY  # update a single field
code-buddy config path                        # show config file location
```

---

## Tools

| Tool | Description |
|:---|:---|
| `read_file` | Read any file |
| `write_file` | Create or overwrite a file |
| `list_dir` | List directory contents |
| `run_shell` | Execute a shell command |
| `web_search` | Search the web via Brave or SerpAPI |
| `web_fetch` | Fetch a webpage and render it as clean text |

### Enable web search

```bash
# Brave Search (preferred) — https://brave.com/search/api/
code-buddy config set brave_api_key YOUR_KEY

# SerpAPI (fallback) — https://serpapi.com/
code-buddy config set serpapi_key YOUR_KEY

# Firecrawl (richer extraction, optional) — https://firecrawl.dev/
code-buddy config set firecrawl_api_key YOUR_KEY
```

---

## Configuration

Config file: `~/.config/code-buddy/config.toml`

Every field can be overridden with an environment variable.

| Field | Environment Variable | Default |
|:---|:---|:---|
| `provider` | `CODE_BUDDY_PROVIDER` | `lm_studio` |
| `model` | `CODE_BUDDY_MODEL` | `mistral` |
| `api_key` | `CODE_BUDDY_API_KEY` | — |
| `endpoint` | `CODE_BUDDY_ENDPOINT` | provider default |
| `system_prompt` | `CODE_BUDDY_SYSTEM_PROMPT` | built-in |
| `temperature` | `CODE_BUDDY_TEMPERATURE` | `0.2` |
| `max_tokens` | `CODE_BUDDY_MAX_TOKENS` | `4096` |
| `timeout_seconds` | `CODE_BUDDY_TIMEOUT_SECONDS` | `60` |
| `brave_api_key` | `CODE_BUDDY_BRAVE_API_KEY` | — |
| `serpapi_key` | `CODE_BUDDY_SERPAPI_KEY` | — |
| `firecrawl_api_key` | `CODE_BUDDY_FIRECRAWL_API_KEY` | — |
| `streaming` | `CODE_BUDDY_STREAMING` | `true` |

---

## Homebrew

The tap at [`simpletoolsindia/homebrew-tap`](https://github.com/simpletoolsindia/homebrew-tap) is automatically updated on every release via GitHub Actions.

```bash
# One-liner (no separate tap step needed)
brew install simpletoolsindia/tap/code-buddy

# Or add the tap first, then use short names for updates
brew tap simpletoolsindia/tap
brew install code-buddy
brew upgrade code-buddy
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

Or build via the installer:

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | sh -s -- --source
```

---

## Releasing

Push a version tag to trigger the full release pipeline — binaries for Linux x64/arm64, macOS x64/arm64, and Windows x64 are built, a GitHub Release is published, and the Homebrew formula is updated automatically.

```bash
make publish TAG=v0.2.0
```

---

## Project Layout

```
crates/
├── cli/        # Binary, commands, TUI
├── config/     # Config loading & env overrides
├── providers/  # Provider adapters + SSE streaming
├── tools/      # Tool registry: file, shell, web
├── errors/     # Shared error types
└── runtime/    # Conversation loop & tool dispatch
```

```bash
make test   # run all 241 tests
make lint   # clippy + rustfmt check
```

---

## License

MIT © [simpletoolsindia](https://github.com/simpletoolsindia)
