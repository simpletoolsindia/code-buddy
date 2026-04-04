# Code Buddy

A local-first CLI coding assistant powered by LLMs. Streams responses, executes
file and shell tools, and works equally well with a local LM Studio server or
cloud providers like OpenRouter or NVIDIA AI Endpoints.

---

## Install

### One-line install (from repository)

```bash
bash install.sh
```

The script:
1. Detects Linux / macOS (including Apple Silicon)
2. Installs the Rust toolchain via `rustup` if it is not already present
3. Compiles a release binary with `cargo build --release`
4. Installs the binary to `~/.local/bin/code-buddy`
5. Prints PATH guidance if the directory is not in your shell's PATH

Re-running the script is safe — it upgrades an existing install.

### Options

```
bash install.sh --prefix /usr/local/bin   # custom install location
bash install.sh --check                   # verify existing install only
```

### Manual build

```bash
cargo build --release --bin code-buddy
cp target/release/code-buddy ~/.local/bin/
```

### Verify

```bash
code-buddy install --verify-only
```

---

## Quick start

```bash
# 1. Point at your LM Studio server (already running on localhost)
code-buddy config set provider lm-studio
code-buddy config set model mistral-7b-instruct

# 2. Start an interactive session
code-buddy run

# 3. Or ask a one-shot question
code-buddy ask "What does the function on line 42 do?"
```

---

## Configuration

Configuration is stored in `~/.config/code-buddy/config.toml`
(or `$XDG_CONFIG_HOME/code-buddy/config.toml` on Linux).

View all settings:

```bash
code-buddy config show
code-buddy config path        # print the config file path
```

Read or write individual fields:

```bash
code-buddy config get provider
code-buddy config set streaming true
```

### All fields

| Field            | Type    | Default                      | Description                                          |
|------------------|---------|------------------------------|------------------------------------------------------|
| `provider`       | string  | `lm-studio`                  | LLM backend. See *Providers* below.                  |
| `model`          | string  | *(not set)*                  | Model name sent to the API.                          |
| `endpoint`       | string  | *(provider default)*         | Override the API base URL.                           |
| `api_key`        | string  | *(not set)*                  | API key for authenticated providers.                 |
| `timeout_seconds`| integer | `120`                        | HTTP request timeout.                                |
| `max_retries`    | integer | `3`                          | Retries on transient errors (backoff: 200→400→800ms).|
| `streaming`      | bool    | `false`                      | Stream tokens to the terminal as they arrive.        |
| `debug`          | bool    | `false`                      | Print verbose request/response traces.               |
| `max_tokens`     | integer | `4096`                       | Maximum output tokens per request.                   |
| `temperature`    | float   | *(model default)*            | Sampling temperature (0.0–2.0).                      |
| `system_prompt`  | string  | *(not set)*                  | Prepended to every conversation.                     |

Environment variables override any file value using the pattern
`CODE_BUDDY_<FIELD_NAME_UPPER>` (e.g. `CODE_BUDDY_API_KEY`).

### Example `config.toml`

```toml
provider         = "openrouter"
model            = "meta-llama/llama-3.1-70b-instruct"
api_key          = "sk-or-..."       # or set CODE_BUDDY_API_KEY
streaming        = true
max_tokens       = 8192
temperature      = 0.2
system_prompt    = "You are a precise and concise Rust coding assistant."
```

---

## Providers

### LM Studio (default — local, no API key required)

```bash
code-buddy config set provider lm-studio
# Default endpoint: http://localhost:1234/v1
# Download and launch any GGUF model in LM Studio, then:
code-buddy config set model <model-name-shown-in-lm-studio>
```

### OpenRouter

```bash
code-buddy config set provider openrouter
code-buddy config set model meta-llama/llama-3.1-8b-instruct
export CODE_BUDDY_API_KEY="sk-or-v1-..."
```

Get a free API key at <https://openrouter.ai>.

### NVIDIA AI Endpoints

```bash
code-buddy config set provider nvidia
code-buddy config set model meta/llama-3.1-8b-instruct
export CODE_BUDDY_API_KEY="nvapi-..."
```

Sign up at <https://build.nvidia.com>.

### OpenAI

```bash
code-buddy config set provider openai
code-buddy config set model gpt-4o-mini
export CODE_BUDDY_API_KEY="sk-..."
```

### Custom endpoint

For any OpenAI-compatible API (Ollama, vLLM, LocalAI, etc.):

```bash
code-buddy config set provider custom
code-buddy config set endpoint http://localhost:11434/v1
code-buddy config set model llama3.2
```

---

## Usage

### `ask` — single-shot prompt

```bash
code-buddy ask "Explain this error: cannot borrow as mutable"
code-buddy ask --stream "Refactor this function for me"
code-buddy ask --no-tools "What is 2 + 2?"   # skip tool calling
```

### `run` — interactive REPL

```bash
code-buddy run
code-buddy run --no-tools   # disable tool calling for this session
```

#### Slash commands

| Command     | Description                          |
|-------------|--------------------------------------|
| `/help`     | Show available commands              |
| `/quit`     | Exit Code Buddy                      |
| `/exit`     | Exit Code Buddy                      |
| `/clear`    | Clear conversation history           |
| `/status`   | Show current configuration           |
| `/model`    | Show active model                    |
| `/provider` | Show active provider and endpoint    |
| `/context`  | Show number of messages in history   |

### Global flags

```
-p, --provider <NAME>    Override provider for this invocation
-m, --model <NAME>       Override model for this invocation
    --debug              Verbose request/response traces
    --no-color           Disable ANSI colours
    --output <FORMAT>    text (default) or json
```

---

## Tool calling

Code Buddy gives the model access to six built-in tools. All file operations
are confined to the current working directory — the model cannot read, write,
or execute anything outside it.

| Tool          | What it does                                             |
|---------------|----------------------------------------------------------|
| `read_file`   | Read file contents (with optional line range)            |
| `write_file`  | Write or create a file                                   |
| `edit_file`   | Apply a targeted search-and-replace edit to a file       |
| `bash`        | Run a shell command (confined to CWD, 30 s timeout)      |
| `glob_search` | Find files by glob pattern                               |
| `grep_search` | Search file contents with a regular expression           |

Disable tools for a single call:

```bash
code-buddy ask --no-tools "What time is it?"
```

---

## Context management

Code Buddy automatically compacts conversation history when it approaches the
token budget (`max_tokens × 6` by default). The oldest user+assistant pair is
dropped first, keeping recent context intact and preventing runaway token usage.

---

## Security notes

- **Path confinement**: every file and directory operation is validated against
  the canonical CWD. Symlink traversal, `../` sequences, and absolute paths
  outside CWD are all rejected.
- **Shell safety**: `bash` commands are validated against an allowlist of
  allowed absolute paths; `../` tokens and absolute patterns that escape CWD
  are rejected before execution.
- **No `unsafe` code**: the entire workspace compiles with `unsafe_code = "forbid"`.

---

## Development

```bash
# Run all tests
cargo test --workspace

# Run only unit tests (fast, no network)
cargo test --workspace --lib

# Build release binary
cargo build --release

# Enable debug traces
code-buddy --debug ask "hello"
```

### Project structure

```
crates/
  cli/        — CLI entry point, argument parsing, subcommand dispatch
  config/     — AppConfig: load/save/validate TOML config
  errors/     — Shared error types (ConfigError, TransportError, ToolError, …)
  providers/  — Provider registry, LM Studio / OpenRouter / NVIDIA / OpenAI backends
  runtime/    — ConversationRuntime: tool-call loop, history, context compaction
  tools/      — ToolRegistry, 6 built-in tools, path confinement, JSON schema validation
  transport/  — Provider trait, MessageRequest/Response, streaming types
  telemetry/  — tracing subscriber initialisation
  utils/      — Shared utility helpers
```

---

## Licence

MIT
