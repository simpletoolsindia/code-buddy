//! `ask` subcommand — send a single prompt and print the response.

use crate::args::AskArgs;
use code_buddy_config::AppConfig;
use tracing::debug;

pub async fn run(config: &AppConfig, args: AskArgs) -> i32 {
    let prompt = args.prompt.join(" ");
    if prompt.is_empty() {
        eprintln!("Error: no prompt provided. Usage: code-buddy ask <your question>");
        return 1;
    }

    debug!("ask command: provider={}, model={:?}", config.provider, config.model);

    // Phase 2 will wire in the actual provider call.
    // For now, display what would be sent.
    println!("[code-buddy] Provider: {}", config.provider);
    println!("[code-buddy] Endpoint: {}", config.resolved_endpoint());
    if let Some(ref model) = config.model {
        println!("[code-buddy] Model: {model}");
    }
    println!("[code-buddy] Prompt: {prompt}");
    println!();
    println!("Provider integration not yet implemented. This is the Phase 1 foundation.");
    println!("Phase 2 will wire in the LLM provider adapters.");

    0
}
