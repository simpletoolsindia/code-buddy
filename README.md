# Code Buddy ✻

> AI coding assistant for your terminal — Claude Code-style TUI, file & shell tools,
> web search, and support for Ollama, LM Studio, OpenRouter, OpenAI, NVIDIA, and more.

---

## Install in One Command

### Linux & macOS

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex
```

### Homebrew

```bash
brew install simpletoolsindia/tap/code-buddy
```

Or add the tap once and install without the prefix:

```bash
brew tap simpletoolsindia/tap
brew install code-buddy
```

### Cargo

```bash
cargo install --git https://github.com/simpletoolsindia/code-buddy --bin code-buddy --locked
```

> After install, just run `code-buddy` — the setup wizard launches automatically on first run.

---

## What it does

- **Interactive session** — streams responses from any LLM, right in your terminal
- **Built-in tools** — read/write files, run shell commands, search the web, fetch pages
- **Claude Code-style TUI** — coloured banner, live spinner, slash commands (`/tools`, `/status`)
- **Setup wizard** — one-time guided config: pick provider → model → API keys
- **Live model list** — auto-fetches available models from Ollama, LM Studio, OpenRouter, OpenAI
- **Web search** — Brave Search (or SerpAPI fallback) + page fetch with HTML-to-text extraction

---

## Supported Providers

| Provider | Local? | Notes |
|---|---|---|
| **Ollama** | Yes | `http://localhost:11434` — models auto-listed |
| **LM Studio** | Yes | `http://localhost:1234` — models auto-listed |
| **OpenRouter** | Cloud | Hundreds of open & commercial models |
| **OpenAI** | Cloud | GPT-4o, o3, etc. |
| **NVIDIA** | Cloud | NVIDIA AI Endpoints |
| **Custom** | Either | Any OpenAI-compatible endpoint |

---

## Usage

### Start a session

```bash
code-buddy          # interactive session in the current directory
```

**Slash commands:**

| Command | Description |
|---|---|
| `/tools` | List active tools |
| `/status` | Provider, model, web tool availability |
| `/exit` | End session |

### One-shot question

```bash
code-buddy ask "How do I reverse a linked list?" --file src/main.rs
```

### Config

```bash
code-buddy setup                              # re-run the setup wizard
code-buddy config show                        # view all settings
code-buddy config set brave_api_key YOUR_KEY  # update a field
code-buddy config path                        # location of config file
```

---

## Tools

| Tool | Description |
|---|---|
| `read_file` | Read a file |
| `write_file` | Create or overwrite a file |
| `list_dir` | List directory contents |
| `run_shell` | Execute a shell command |
| `web_search` | Search the web (Brave or SerpAPI) |
| `web_fetch` | Fetch and render a webpage as text |

### Enable web tools

```bash
# Brave Search (preferred) — https://brave.com/search/api/
code-buddy config set brave_api_key YOUR_KEY

# SerpAPI fallback — https://serpapi.com/
code-buddy config set serpapi_key YOUR_KEY

# Firecrawl (richer extraction, optional) — https://firecrawl.dev/
code-buddy config set firecrawl_api_key YOUR_KEY
```

---

## Configuration

Config file: `~/.config/code-buddy/config.toml`
Every field can be overridden by an environment variable.

| Field | Env var | Default |
|---|---|---|
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

## Build from Source

Requires Rust 1.80+.

```bash
git clone https://github.com/simpletoolsindia/code-buddy
cd code-buddy
cargo build --release
./target/release/code-buddy --version
```

Or let the installer build it:

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | sh -s -- --source
```

---

## Release

Tag a version to trigger a GitHub Actions release (builds for Linux x64/arm64, macOS x64/arm64, Windows x64):

```bash
make publish TAG=v0.2.0
```

---

## Project Layout

```
crates/
  cli/        # Binary, commands, TUI
  config/     # Config loading & env overrides
  providers/  # Provider adapters + SSE streaming
  tools/      # Tool registry: file, shell, web
  errors/     # Shared error types
  agent/      # Conversation runtime & tool dispatch
```

```bash
make test    # run all 241 tests
make lint    # clippy + rustfmt
```

---

## License

MIT © [simpletoolsindia](https://github.com/simpletoolsindia)
