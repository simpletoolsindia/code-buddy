//! Interactive REPL with slash commands support

use crate::api::ApiClient;
use crate::commands::output::OutputState;
use crate::state::AppState;
use anyhow::Result;
use std::io::{self, Write};

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/help", "Show available commands"),
    ("/quit", "Exit Code Buddy"),
    ("/exit", "Exit Code Buddy"),
    ("/clear", "Clear conversation history"),
    ("/status", "Show current configuration"),
    ("/model", "Change model"),
    ("/provider", "Change LLM provider"),
    ("/history", "Show conversation history"),
    ("/reset", "Reset conversation"),
    ("/models", "List available models"),
    ("/cost", "Show estimated costs"),
    ("/compact", "Compact context window"),
    ("/context", "Show context usage"),
    ("/system", "Show system prompt"),
    ("/set", "Set configuration option"),
    ("/update", "Check for or install updates"),
    ("/simplify", "Review code for quality issues"),
    ("/review", "Full code review"),
    ("/memory", "Manage project memory"),
    ("/diff", "Show changes made in session"),
    ("/rewind", "Rewind to a checkpoint"),
    ("/stats", "Show usage statistics"),
    ("/copy", "Copy last response to clipboard"),
    ("/btw", "Ask a side question"),
    ("/fast", "Toggle fast output mode"),
    ("/skills", "List available skills"),
    ("/agent", "Manage agents"),
    ("/changes", "Show files modified in session"),
];

pub async fn run(state: &mut AppState) -> Result<i32> {
    let mut output = OutputState::new();

    println!();
    println!("\x1b[1m\x1b[36m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m\x1b[36m│\x1b[0m  \x1b[1mCode Buddy\x1b[0m - Your AI Coding Companion");
    println!("\x1b[1m\x1b[36m╰─────────────────────────────────────────────────────────────\x1b[0m");

    // Show current config (simplified)
    show_status(state);
    println!();

    loop {
        print!("\x1b[32m❯\x1b[0m ");
        io::stdout().flush()?;

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                println!("\n\x1b[90mGoodbye!\x1b[0m");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("\n\x1b[31mError reading input:\x1b[0m {}", e);
                break;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // Handle slash commands
        if input.starts_with('/') {
            let result = handle_slash_command(input, state, &mut output).await?;
            if result == 1 {
                break;
            }
            continue;
        }

        // Handle regular prompt
        match handle_prompt(input, state, &mut output).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("\x1b[31mError:\x1b[0m {}", e);
            }
        }
    }

    Ok(0)
}

async fn handle_slash_command(input: &str, state: &mut AppState, _output: &mut OutputState) -> Result<i32> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let cmd = parts.first().unwrap_or(&"").to_lowercase();

    match cmd.as_str() {
        "/help" | "/?" => {
            show_help();
        }
        "/more" => {
            show_more_help();
        }
        "/quit" | "/exit" | "/q" => {
            println!("\x1b[90mGoodbye!\x1b[0m");
            return Ok(1);
        }
        "/clear" | "/cls" => {
            state.clear_history();
            println!("\x1b[32m✓\x1b[0m Conversation history cleared\n");
        }
        "/status" => {
            show_status(state);
        }
        "/model" => {
            if let Some(model) = parts.get(1) {
                state.config.model = Some(model.to_string());
                state.save_config()?;
                println!("\x1b[32m✓\x1b[0m Model set to: {}\n", model);
            } else {
                let current = state.config.model.as_deref().unwrap_or("default");
                println!("Current model: {}\n", current);
                println!("Usage: /model <model-name>\n");
            }
        }
        "/provider" => {
            if let Some(provider) = parts.get(1) {
                state.config.llm_provider = provider.to_string();
                state.save_config()?;
                println!("\x1b[32m✓\x1b[0m Provider set to: {}\n", provider);
            } else {
                println!("Current provider: {}\n", state.config.llm_provider);
                println!("Usage: /provider <provider-name>\n");
            }
        }
        "/history" => {
            println!("\x1b[1m=== Conversation History ===\x1b[0m\n");
            for (i, msg) in state.conversation_history.iter().enumerate() {
                let role = if msg.role == "user" { "\x1b[34mYou\x1b[0m" } else { "\x1b[33mBuddy\x1b[0m" };
                let preview = if msg.content.len() > 50 {
                    format!("{}...", &msg.content[..50])
                } else {
                    msg.content.clone()
                };
                println!("[{}] {}: {}", i + 1, role, preview);
            }
            println!();
        }
        "/reset" => {
            state.clear_history();
            println!("\x1b[32m✓\x1b[0m Conversation reset\n");
        }
        "/models" => {
            println!("\x1b[1m=== Available Models ===\x1b[0m\n");
            println!("Current provider: {}\n", state.config.llm_provider);
            println!("Use 'code-buddy model <name>' to change model\n");
        }
        "/cost" => {
            println!("\x1b[1m=== Cost Estimation ===\x1b[0m\n");
            let input_tokens: u32 = state.conversation_history.iter()
                .filter(|m| m.role == "user")
                .map(|m| (m.content.len() / 4) as u32)
                .sum();
            let output_tokens: u32 = state.conversation_history.iter()
                .filter(|m| m.role == "assistant")
                .map(|m| (m.content.len() / 4) as u32)
                .sum();
            println!("Input tokens (estimated): {}", input_tokens);
            println!("Output tokens (estimated): {}", output_tokens);
            println!("Total tokens: {}", input_tokens + output_tokens);
            println!();
        }
        "/context" => {
            println!("\x1b[1m=== Context Usage ===\x1b[0m\n");
            let total = state.conversation_history.len();
            let tokens = state.estimate_context_tokens();
            let usage = state.context_usage_percent();
            let window = state.config.conversation_window.unwrap_or(200_000);
            println!("Messages in context: {}", total);
            println!("Estimated tokens: ~{}", tokens);
            println!("Context window: {} tokens", window);
            println!("Usage: {}%", usage);
            println!("Auto-compact: {}", if state.config.auto_compact { "ON" } else { "OFF" });
            println!("Compact threshold: {}%", state.config.compact_threshold);
            println!();
        }
        "/changes" | "/diff" => {
            show_session_changes(state);
        }
        "/compact" => {
            println!("=== Compacting Context ===\n");
            let tokens_before = state.estimate_context_tokens();
            let messages_before = state.conversation_history.len();
            let result = state.compact();
            let tokens_after = state.estimate_context_tokens();

            println!("✓ Context compacted successfully!");
            println!("  Messages: {} → {}", messages_before, result.compacted_messages);
            println!("  Tokens: ~{} → ~{} ({:.1}% reduction)",
                tokens_before, tokens_after,
                if tokens_before > 0 {
                    100.0 * (tokens_before - tokens_after) as f64 / tokens_before as f64
                } else { 0.0 });
            println!();
        }
        "/system" => {
            println!("=== System Configuration ===\n");
            println!("Provider: {}", state.config.llm_provider);
            println!("Model: {:?}", state.config.model.as_deref().unwrap_or("default"));
            println!("API Key: {}\n", if state.config.api_key.is_some() { "Configured" } else { "Not set" });
        }
        "/set" => {
            if parts.len() >= 3 {
                let key = parts[1];
                let value = parts[2];
                match key {
                    "provider" => {
                        state.config.llm_provider = value.to_string();
                        state.save_config()?;
                        println!("✓ Provider set to: {}\n", value);
                    }
                    "model" => {
                        state.config.model = Some(value.to_string());
                        state.save_config()?;
                        println!("✓ Model set to: {}\n", value);
                    }
                    _ => {
                        println!("Unknown setting: {}\n", key);
                    }
                }
            } else {
                println!("Usage: /set <key> <value>\n");
            }
        }
        "/update" => {
            println!();
            if let Ok(1) = crate::commands::update::check_and_update(true).await {
                println!("Run 'code-buddy update --yes' to update now, or restart to see this message again.");
            }
            println!();
        }
        "/simplify" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    /simplify - Code Review                    ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            let target = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| "*".to_string());
            println!("To run a code quality review, use natural language:");
            println!();
            println!("  \"simplify the code in {}\"", target);
            println!("  \"review {} for bugs and quality issues\"", target);
            println!("  \"find opportunities to improve code quality\"");
            println!();
            println!("The AI will analyze your code and suggest improvements for:");
            println!("  • Code reuse - identify duplicated patterns");
            println!("  • Quality issues - find potential bugs and anti-patterns");
            println!("  • Efficiency - suggest performance improvements");
            println!();
        }
        "/review" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                   /review - Full Code Review                  ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            let scope = parts.get(1).map(|s| s.to_string()).unwrap_or_else(|| "changes".to_string());
            println!("To run a full code review, use natural language:");
            println!();
            println!("  \"review the code\"");
            println!("  \"review all changes in this session\"");
            println!("  \"do a security review of the codebase\"");
            println!();
            println!("Review scope: {}", scope);
            println!();
            println!("The review covers:");
            println!("  • Correctness - Does the code work?");
            println!("  • Security - Any vulnerabilities?");
            println!("  • Performance - Any bottlenecks?");
            println!("  • Maintainability - Easy to understand?");
            println!("  • Testing - Adequate test coverage?");
            println!();
        }
        "/memory" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                  /memory - Project Memory                     ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("Persistent project memory commands:");
            println!();
            println!("  /memory list              - List all memory entries");
            println!("  /memory get <key>        - Get a specific entry");
            println!("  /memory set <key> <val>  - Set an entry");
            println!("  /memory delete <key>     - Delete an entry");
            println!("  /memory search <query>   - Search entries");
            println!("  /memory clear            - Clear all entries");
            println!();
            println!("Or use from terminal:");
            println!("  code-buddy memory list");
            println!("  code-buddy memory set project_name \"My Project\"");
            println!();
        }
        "/diff" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                      /diff - Changes                          ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("Changes made in this session:");
            println!();
            let changes = state.get_session_changes();
            if changes.is_empty() {
                println!("No file changes recorded in this session.");
                println!();
                println!("Run: git diff HEAD to see uncommitted changes");
            } else {
                for change in changes {
                    println!("  {}", change);
                }
            }
            println!();
            println!("Tip: Use 'git diff' or 'git status' to see actual changes");
            println!();
        }
        "/rewind" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    /rewind - Go Back                         ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            let checkpoints = state.get_checkpoints();
            if checkpoints.is_empty() {
                println!("No checkpoints available.");
                println!("Checkpoints are created automatically during long sessions.");
            } else {
                println!("Available checkpoints:");
                for (i, checkpoint) in checkpoints.iter().enumerate() {
                    println!("  {}: {}", i + 1, checkpoint);
                }
                println!();
                println!("Usage: /rewind <number> - Go back to a checkpoint");
            }
            println!();
        }
        "/stats" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    /stats - Usage Stats                      ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            let session_stats = state.get_session_stats();
            println!("Session Statistics:");
            println!();
            println!("  Messages:        {}", session_stats.messages);
            println!("  Responses:       {}", session_stats.responses);
            println!("  Tools used:     {}", session_stats.tools_used);
            println!("  Files modified: {}", session_stats.files_modified);
            println!("  Commands run:   {}", session_stats.commands_run);
            println!();
            println!("Token Usage:");
            println!();
            println!("  Input tokens:   ~{}", session_stats.input_tokens);
            println!("  Output tokens:  ~{}", session_stats.output_tokens);
            println!("  Total:          ~{}", session_stats.input_tokens + session_stats.output_tokens);
            println!();
            let model = state.config.model.as_deref().unwrap_or("default");
            println!("  Model:          {}", model);
            println!();
        }
        "/copy" => {
            if let Some(last_response) = state.get_last_response() {
                println!();
                println!("Last response copied to clipboard!");
                println!();
                println!("{}", last_response);
                println!();
                // Note: Actual clipboard copy would require platform-specific code
                // For now, just display the content
            } else {
                println!();
                println!("No response to copy yet.");
                println!();
            }
        }
        "/btw" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    /btw - Side Question                       ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("Ask a side question without affecting the main context:");
            println!();
            println!("Usage: /btw What is the weather?");
            println!();
            println!("The question will be asked and answered, but won't affect");
            println!("the main conversation flow or context.");
            println!();
            println!("Example: /btw How do I spell 'necessary'?");
            println!();
        }
        "/fast" => {
            state.fast_mode = !state.fast_mode;
            println!();
            println!("Fast mode: {}", if state.fast_mode { "ON" } else { "OFF" });
            println!();
            if state.fast_mode {
                println!("Fast mode enabled - responses will be shorter and faster.");
            } else {
                println!("Fast mode disabled - full responses enabled.");
            }
            println!();
        }
        "/skills" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    /skills - Available Skills                 ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("Built-in Skills:");
            println!();
            println!("  /simplify     - Review code for quality issues");
            println!("  /review       - Full code review with checklist");
            println!("  /tdd          - Test-driven development workflow");
            println!("  /debug        - Debug assistance");
            println!("  /batch        - Batch operations");
            println!();
            println!("Custom Skills:");
            println!();
            println!("  No custom skills installed.");
            println!("  Add skills to ~/.config/code-buddy/skills/");
            println!();
            println!("Install more skills: code-buddy plugin install <source>");
            println!();
        }
        "/agent" => {
            println!();
            println!("╔══════════════════════════════════════════════════════════════╗");
            println!("║                    /agent - Agent Management                 ║");
            println!("╚══════════════════════════════════════════════════════════════╝");
            println!();
            println!("Available agents:");
            println!();
            println!("  /agent list    - List available agents");
            println!("  /agent create  - Create a new agent");
            println!("  /agent switch  - Switch to a different agent");
            println!();
            println!("Built-in Agents:");
            println!();
            println!("  default        - Standard coding assistant");
            println!("  analyzer       - Code analysis specialist");
            println!("  debugger       - Debugging specialist");
            println!("  reviewer       - Code review specialist");
            println!();
            let agents = state.get_available_agents();
            if !agents.is_empty() {
                println!("Custom Agents:");
                for agent in agents {
                    println!("  {}         - {}", agent.0, agent.1);
                }
            }
            println!();
        }
        _ => {
            println!("Unknown command: {}\n", cmd);
            println!("Type /help for available commands\n");
        }
    }

    Ok(0)
}

async fn handle_prompt(prompt: &str, state: &mut AppState, output: &mut OutputState) -> Result<()> {
    println!();

    // Show thinking status
    output.show_thinking();

    let api_client = ApiClient::new(state)?;
    let response = match api_client.complete(prompt, &state.config, state).await {
        Ok(r) => r,
        Err(e) => {
            output.clear_status();
            show_error_help(&e, state);
            return Ok(());
        }
    };

    // Clear thinking status
    output.clear_status();

    // Show response in Claude Code style
    println!();
    println!("\x1b[1m\x1b[33m─────────────────────────────────────────────────────────────────────\x1b[0m");
    println!();

    // Format and display response
    let lines: Vec<&str> = response.content.lines().collect();
    for line in lines.iter().take(50) {
        println!("{}", line);
    }
    if lines.len() > 50 {
        println!("\x1b[90m... ({} more lines)\x1b[0m", lines.len() - 50);
    }

    // Show token usage
    output.show_token_usage(response.usage.input_tokens, response.usage.output_tokens);

    // Update conversation history
    state.add_message("user", prompt);
    state.add_message("assistant", &response.content);

    // Auto-compact if needed
    if let Some(result) = state.auto_compact_if_needed() {
        println!();
        println!("\x1b[36m⚡\x1b[0m Auto-compacted {} messages → {} messages",
            result.original_messages, result.compacted_messages);
        println!("\x1b[90m   ({}% context usage reached threshold of {}%)\x1b[0m",
            state.context_usage_percent(), state.config.compact_threshold);
    }

    println!();
    Ok(())
}

/// Show helpful error messages with suggestions
fn show_error_help(error: &anyhow::Error, state: &AppState) {
    let error_str = error.to_string();
    println!();

    // Check for common error types
    if error_str.contains("model") && error_str.contains("not found") {
        println!("\x1b[31m✗ Model not found\x1b[0m");
        println!();
        println!("The model '{}' doesn't exist or isn't available.", state.config.model.as_deref().unwrap_or("unknown"));
        println!();
        println!("\x1b[33mSuggestions:\x1b[0m");
        println!("  1. Check your API key is correct: \x1b[32mcode-buddy /status\x1b[0m");
        println!("  2. Try a different model: \x1b[32mcode-buddy /model <model-name>\x1b[0m");
        println!("  3. Run setup wizard: \x1b[32mcode-buddy --setup\x1b[0m");
        println!("  4. Check available models: \x1b[32mcode-buddy /models\x1b[0m");
    } else if error_str.contains("401") || error_str.contains("Unauthorized") || error_str.contains("api key") {
        println!("\x1b[31m✗ Authentication failed\x1b[0m");
        println!();
        println!("Your API key is missing or incorrect.");
        println!();
        println!("\x1b[33mTo fix this:\x1b[0m");
        println!("  1. Get an API key from your provider's website");
        println!("  2. Set it with: \x1b[32mcode-buddy --login YOUR_API_KEY\x1b[0m");
        println!("  3. Or set the environment variable:");
        println!("     \x1b[90mexport ANTHROPIC_API_KEY=sk-...  # for Claude\x1b[0m");
        println!("     \x1b[90mexport OPENAI_API_KEY=sk-...    # for GPT\x1b[0m");
    } else if error_str.contains("429") || error_str.contains("rate limit") {
        println!("\x1b[33m⚠ Rate limit exceeded\x1b[0m");
        println!();
        println!("You've made too many requests. Wait a moment and try again.");
        println!();
        println!("\x1b[33mTips:\x1b[0m");
        println!("  • Try a free model like: \x1b[32mgoogle/gemini-2.5-flash-preview-05-20:free\x1b[0m");
        println!("  • Use OpenRouter for free tier: \x1b[32mcode-buddy --provider openrouter\x1b[0m");
    } else if error_str.contains("connection") || error_str.contains("timeout") || error_str.contains("refused") {
        println!("\x1b[31m✗ Connection error\x1b[0m");
        println!();
        println!("Could not connect to the API server.");
        println!();
        println!("\x1b[33mSuggestions:\x1b[0m");
        println!("  1. Check your internet connection");
        println!("  2. Try using Ollama (free, works offline): \x1b[32mcode-buddy --setup\x1b[0m");
        println!("  3. Check if your API provider is down");
    } else {
        // Generic error
        println!("\x1b[31m✗ Error\x1b[0m: {}", error_str);
        println!();
        println!("\x1b[33mNeed help?\x1b[0m");
        println!("  • Run diagnostics: \x1b[32mcode-buddy --doctor\x1b[0m");
        println!("  • Reconfigure: \x1b[32mcode-buddy --setup\x1b[0m");
        println!("  • Check status: \x1b[32mcode-buddy /status\x1b[0m");
    }

    println!();
    println!("\x1b[90mRun \x1b[32m/code-buddy help\x1b[0m\x1b[90m for assistance\x1b[0m");
    println!();
}

fn show_help() {
    println!();
    println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m│\x1b[0m  \x1b[1mEssential Commands\x1b[0m");
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    println!();
    println!("  \x1b[32m/help\x1b[0m        Show all commands (you're looking at it!)");
    println!("  \x1b[32m/quit\x1b[0m        Exit Code Buddy");
    println!("  \x1b[32m/clear\x1b[0m       Clear conversation history");
    println!("  \x1b[32m/status\x1b[0m      Check your setup");
    println!();
    println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m│\x1b[0m  \x1b[1mConfiguration\x1b[0m");
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    println!();
    println!("  \x1b[32m/setup\x1b[0m       Reconfigure Code Buddy");
    println!("  \x1b[32m/login\x1b[0m        Set your API key");
    println!("  \x1b[32m/model\x1b[0m <name> Change the AI model");
    println!("  \x1b[32m/doctor\x1b[0m       Check for problems");
    println!();
    println!("\x1b[90mTip: Type your question naturally and press Enter!\x1b[0m");
    println!("\x1b[90mType /more for advanced commands.\x1b[0m");
    println!();
}

fn show_more_help() {
    println!();
    println!("\x1b[1m╭─────────────────────────────────────────────────────────────\x1b[0m");
    println!("\x1b[1m│\x1b[0m  \x1b[1mAdvanced Commands\x1b[0m");
    println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    println!();
    println!("  \x1b[33m/history\x1b[0m      View conversation history");
    println!("  \x1b[33m/reset\x1b[0m         Start fresh conversation");
    println!("  \x1b[33m/context\x1b[0m       Show context usage");
    println!("  \x1b[33m/compact\x1b[0m       Reduce context size");
    println!("  \x1b[33m/stats\x1b[0m         Show usage statistics");
    println!();
    println!("  \x1b[33m/copy\x1b[0m          Copy last response");
    println!("  \x1b[33m/btw\x1b[0m <question> Ask a quick side question");
    println!("  \x1b[33m/fast\x1b[0m          Toggle fast/short responses");
    println!();
    println!("\x1b[90mRun /help to see essential commands again.\x1b[0m");
    println!();
}

fn show_status(state: &AppState) {
    println!("\x1b[1m╭─ Current Setup ─────────────────────────────────────────────\x1b[0m");

    let provider_display: String = if state.config.base_url.is_some() && !state.config.base_url.as_ref().unwrap().is_empty() {
        let base = state.config.base_url.as_ref().unwrap();
        if base.contains("ollama") {
            "Ollama (local)".to_string()
        } else if base.contains("nvidia") {
            "NVIDIA NIM".to_string()
        } else if base.contains("openrouter") {
            "OpenRouter".to_string()
        } else if base.contains("anthropic") {
            "Anthropic".to_string()
        } else if base.contains("openai") {
            "OpenAI".to_string()
        } else {
            format!("Custom")
        }
    } else {
        match state.config.llm_provider.as_str() {
            "ollama" => "Ollama (local, free!)".to_string(),
            "openrouter" => "OpenRouter".to_string(),
            "anthropic" => "Anthropic (Claude)".to_string(),
            "openai" => "OpenAI (GPT)".to_string(),
            "nvidia" => "NVIDIA NIM".to_string(),
            "groq" => "Groq".to_string(),
            "deepseek" => "DeepSeek".to_string(),
            "mlx" => "MLX (local)".to_string(),
            other => other.to_string(),
        }
    };
    println!("│  Provider: \x1b[36m{}\x1b[0m", provider_display);

    let model = state.config.model.as_deref().unwrap_or("default");
    println!("│  Model:    \x1b[36m{}\x1b[0m", model);

    if state.config.api_key.is_some() {
        println!("│  API Key:  \x1b[32m✓ Configured\x1b[0m");
    } else if state.config.llm_provider == "ollama" || state.config.llm_provider == "mlx" {
        println!("│  API Key:  \x1b[32m✓ Not needed (local)\x1b[0m");
    } else {
        println!("│  API Key:  \x1b[31m⚠ Not set\x1b[0m - run /login to set it");
    }

    if state.conversation_history.is_empty() {
        println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    } else {
        println!("│  Messages: {}", state.conversation_history.len());
        println!("\x1b[1m╰─────────────────────────────────────────────────────────────\x1b[0m");
    }
}

fn show_session_changes(state: &AppState) {
    println!("\x1b[1m=== Files Modified in Session ===\x1b[0m\n");

    let changes = state.get_session_changes();
    if changes.is_empty() {
        println!("\x1b[90mNo file changes recorded in this session.\x1b[0m");
        println!("\nRun \x1b[33mgit status\x1b[0m to see actual changes");
    } else {
        for change in changes {
            // Parse change format: "type:path" or just path
            if change.starts_with("create:") {
                println!("  \x1b[32m+ {}\x1b[0m", &change[7..]);
            } else if change.starts_with("edit:") || change.starts_with("modify:") {
                let path = if change.contains(':') {
                    change.split(':').nth(1).unwrap_or(&change)
                } else {
                    &change
                };
                println!("  \x1b[33mM {}\x1b[0m", path);
            } else if change.starts_with("delete:") || change.starts_with("remove:") {
                println!("  \x1b[31mD {}\x1b[0m", &change[7..]);
            } else {
                // Default to modified
                println!("  \x1b[33mM {}\x1b[0m", change);
            }
        }
    }
    println!();
}
