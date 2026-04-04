//! `ask` subcommand — send a single prompt and print the response.

use crate::args::AskArgs;
use code_buddy_config::AppConfig;
use code_buddy_providers::ProviderRegistry;
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
        "ask: provider={} model={:?} streaming={}",
        config.provider, config.model, config.streaming
    );

    let model = config
        .model
        .clone()
        .unwrap_or_else(|| "local-model".to_string());

    let mut request = MessageRequest::simple(model, &prompt);
    if let Some(temp) = config.temperature {
        request.temperature = Some(temp);
    }
    if let Some(mt) = config.max_tokens {
        request.max_tokens = mt;
    }
    if let Some(ref sys) = config.system_prompt {
        request.system = Some(sys.clone());
    }

    if config.streaming {
        request.stream = true;
        match provider.stream(&request).await {
            Err(e) => {
                eprintln!("Stream error: {e}");
                return 1;
            }
            Ok(mut source) => {
                loop {
                    match source.next_event().await {
                        Ok(None) => break,
                        Ok(Some(StreamEvent::TextDelta(text))) => {
                            print!("{text}");
                        }
                        Ok(Some(StreamEvent::MessageStop)) => {
                            println!();
                            break;
                        }
                        Ok(Some(StreamEvent::Usage(u))) => {
                            debug!(
                                "usage: in={} out={}",
                                u.input_tokens, u.output_tokens
                            );
                        }
                        Ok(Some(StreamEvent::ToolUseDelta { name, input_json, .. })) => {
                            debug!("tool call: {name}({input_json})");
                        }
                        Err(e) => {
                            eprintln!("\nStream error: {e}");
                            return 1;
                        }
                    }
                }
            }
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
