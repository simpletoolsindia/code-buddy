//! `ask` subcommand — send a single prompt and print the response.
//!
//! With `--no-tools`: streams directly from the provider (fast, no tool loop).
//! Without `--no-tools`: runs through `ConversationRuntime` for tool-call support.

use std::io::{self, Write};

use crate::args::AskArgs;
use code_buddy_config::AppConfig;
use code_buddy_providers::ProviderRegistry;
use code_buddy_runtime::{ConversationRuntime, RuntimeConfig, TextSink};
use code_buddy_tools::ToolRegistry;
use code_buddy_transport::{MessageRequest, StreamEvent};
use tracing::debug;

pub async fn run(config: &AppConfig, args: AskArgs) -> i32 {
    let prompt = args.prompt.join(" ");
    if prompt.is_empty() {
        eprintln!("Error: no prompt provided. Usage: code-buddy ask <your question>");
        return 1;
    }

    let provider = match ProviderRegistry::from_config(config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    };

    debug!(
        "ask: provider={} model={:?} streaming={} no_tools={}",
        config.provider, config.model, config.streaming, args.no_tools
    );

    // ── No-tools path: stream directly ───────────────────────────────────────
    if args.no_tools {
        return ask_direct(config, provider, &prompt, args.stream || config.streaming).await;
    }

    // ── Tool-calling path via ConversationRuntime ─────────────────────────────
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut tools = ToolRegistry::new();
    tools.register_builtin(cwd);
    tools.register_web_tools(
        config.brave_api_key.clone(),
        config.serpapi_key.clone(),
        config.firecrawl_api_key.clone(),
    );

    let rt_config = RuntimeConfig {
        model: config.model.clone().unwrap_or_else(|| "local-model".to_string()),
        max_tokens: config.max_tokens.unwrap_or(4096),
        temperature: config.temperature,
        system_prompt: config.system_prompt.clone(),
        streaming: args.stream || config.streaming,
        debug: config.debug,
        ..Default::default()
    };

    let mut runtime = ConversationRuntime::new(provider, tools, rt_config);

    let sink = TextSink::new(Box::new(|text: &str| {
        print!("{text}");
        io::stdout().flush().unwrap_or(());
    }));

    match runtime.run_turn(&prompt, sink).await {
        Ok(summary) => {
            if !summary.response_text.ends_with('\n') {
                println!();
            }
            if config.debug {
                eprintln!(
                    "[tokens: in={} out={} | tool_calls={} iterations={}]",
                    summary.input_tokens,
                    summary.output_tokens,
                    summary.tool_calls_made,
                    summary.iterations,
                );
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

/// Direct streaming path used with `--no-tools`.
async fn ask_direct(
    config: &AppConfig,
    provider: Box<dyn code_buddy_transport::Provider>,
    prompt: &str,
    streaming: bool,
) -> i32 {
    let model = config
        .model
        .clone()
        .unwrap_or_else(|| "local-model".to_string());

    let mut request = MessageRequest::simple(model, prompt);
    if let Some(temp) = config.temperature {
        request.temperature = Some(temp);
    }
    if let Some(mt) = config.max_tokens {
        request.max_tokens = mt;
    }
    if let Some(ref sys) = config.system_prompt {
        request.system = Some(sys.clone());
    }

    if streaming {
        request.stream = true;
        match provider.stream(&request).await {
            Err(e) => {
                eprintln!("Stream error: {e}");
                return 1;
            }
            Ok(mut source) => loop {
                match source.next_event().await {
                    Ok(None) => break,
                    Ok(Some(StreamEvent::TextDelta(text))) => {
                        print!("{text}");
                        io::stdout().flush().unwrap_or(());
                    }
                    Ok(Some(StreamEvent::MessageStop)) => {
                        println!();
                        break;
                    }
                    Ok(Some(StreamEvent::Usage(u))) => {
                        debug!("usage: in={} out={}", u.input_tokens, u.output_tokens);
                    }
                    Ok(Some(StreamEvent::ToolUseDelta { name, input_json, .. })) => {
                        debug!("tool call: {name}({input_json})");
                    }
                    Err(e) => {
                        eprintln!("\nStream error: {e}");
                        return 1;
                    }
                }
            },
        }
    } else {
        match provider.send(&request).await {
            Err(e) => {
                eprintln!("Error: {e}");
                return 1;
            }
            Ok(response) => {
                print!("{}", response.text_content());
                if !response.text_content().ends_with('\n') {
                    println!();
                }
                if config.debug {
                    eprintln!(
                        "[tokens: in={} out={}]",
                        response.usage.input_tokens, response.usage.output_tokens
                    );
                }
            }
        }
    }

    0
}
