# Code Buddy ✻

A local-first AI coding assistant for your terminal — Claude Code-style TUI,
file & shell tools, web search, and first-class support for Ollama, LM Studio,
OpenRouter, OpenAI, NVIDIA, and any OpenAI-compatible endpoint.

---

## Quick Install

### Linux & macOS (one command)

```bash
curl -fsSL https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/simpletoolsindia/code-buddy/main/install.ps1 | iex
```

### Homebrew (macOS / Linux)

```bash
brew tap simpletoolsindia/tap
brew install code-buddy
```

### Cargo

```bash
cargo install --git https://github.com/simpletoolsindia/code-buddy --bin code-buddy --locked
```

---

## First-Run Setup Wizard

The first time you run `code-buddy`, a guided wizard walks you through:

1. **Provider** — Ollama · LM Studio · OpenRouter · OpenAI · NVIDIA · Custom
2. **API key** — prompted only for cloud providers
3. **Model** — auto-populated from your provider's live model list
4. **Brave Search key** (optional) — enables the `web_search` tool
5. **Firecrawl key** (optional) — enables full-page web fetch

Run the wizard again any time:

```bash
code-buddy setup
```

---

## Supported Providers

| Provider | Requires key? | Notes |
|---|---|---|
| **Ollama** | No | `http://localhost:11434` — models auto-listed |
| **LM Studio** | No | `http://localhost:1234` — models auto-listed |
| **OpenRouter** | Yes | Hundreds of open & commercial models |
| **OpenAI** | Yes | GPT-4o, o3, etc. |
| **NVIDIA** | Yes | NVIDIA AI Endpoints (Llama, Mistral, …) |
| **Custom** | Optional | Any OpenAI-compat endpoint |

---

## Usage

### Interactive session (default)

```bash
code-buddy          # start a session in the current directory
code-buddy run      # explicit
```

**Slash commands inside a session:**

| Command | Description |
|---|---|
| `/tools` | List active tools |
| `/status` | Show provider, model, and tool availability |
| `/exit` or `/quit` | End the session |

### One-shot question

```bash
code-buddy ask "Explain this function" --file src/main.rs
```

### Config management

```bash
code-buddy config show                        # print current config
code-buddy config get provider                # single field
code-buddy config set brave_api_key sk-...    # update a field
code-buddy config path                        # show config file location
```

---

## Tools

Code Buddy ships a built-in tool registry. All tools are available by default
unless disabled with `--no-tools`.

| Tool | Description |
|---|---|
| `read_file` | Read a file from the workspace |
| `write_file` | Create or overwrite a file |
| `list_dir` | List directory contents |
| `run_shell` | Execute a shell command |
| `web_search` | Search the web (Brave or SerpAPI) |
| `web_fetch` | Fetch and render a webpage as text |

### Enabling web tools

```bash
# Brave Search (preferred)
code-buddy config set brave_api_key YOUR_KEY
# or
export CODE_BUDDY_BRAVE_API_KEY=YOUR_KEY

# SerpAPI (fallback)
code-buddy config set serpapi_key YOUR_KEY

# Firecrawl (richer page extraction, optional)
code-buddy config set firecrawl_api_key YOUR_KEY
```

---

## Configuration

Config lives at `~/.config/code-buddy/config.toml` (XDG-compliant).
Every field can be overridden with an environment variable.

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

## Building From Source

Requires Rust 1.80+.

```bash
git clone https://github.com/simpletoolsindia/code-buddy
cd code-buddy
cargo build --release
./target/release/code-buddy --version
```

Or let the installer build it for you:

```bash
curl -fsSL .../install.sh | sh -s -- --source
```

---

## Contributing

Pull requests are welcome. The codebase is a Cargo workspace:

```
crates/
  cli/        # Binary entry-point, commands, TUI
  config/     # Config loading, env overrides, validation
  providers/  # Provider adapters + SSE streaming
  tools/      # Tool registry, file/shell/web tools
  errors/     # Shared error types
  agent/      # Conversation runtime & tool dispatch
```

Run the test suite:

```bash
make test      # or: cargo test --all
make lint      # clippy + rustfmt check
```

---

## License

MIT © simpletoolsindia
