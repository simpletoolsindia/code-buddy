//! Print command - Run a prompt and print the response

use crate::commands::agent::Agent;
use crate::api::ApiClient;
use crate::cli::OutputFormat;
use crate::state::AppState;
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};

pub async fn run(
    prompt: Vec<String>,
    output_format: Option<OutputFormat>,
    state: &mut AppState,
) -> Result<i32> {
    let prompt_text = prompt.join(" ");
    let format = output_format.unwrap_or(OutputFormat::Text);

    if prompt_text.is_empty() {
        eprintln!("Error: No prompt provided");
        return Ok(1);
    }

    println!("Prompt: {}\n", prompt_text);

    // Create API client
    let api_client = ApiClient::new(state)?;

    match format {
        OutputFormat::Text => {
            run_text_mode(&api_client, &prompt_text, state).await
        }
        OutputFormat::Json => {
            run_json_mode(&api_client, &prompt_text, state).await
        }
        OutputFormat::StreamJson => {
            run_stream_mode(&api_client, &prompt_text, state).await
        }
    }
}

async fn run_text_mode(_api_client: &ApiClient, prompt: &str, state: &mut AppState) -> Result<i32> {
    // Use Agent for tool execution
    let mut agent = Agent::new(state)?;

    // Show spinner while working
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .unwrap()
    );
    spinner.set_message("Working...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let response = agent.run(prompt).await?;

    spinner.finish_with_message("Done!");

    println!("{}", response.content);
    println!("\n[Usage: {} tokens]", response.usage.total_tokens);

    // Update state with any changes
    *state = agent.state;

    Ok(0)
}

async fn run_json_mode(_api_client: &ApiClient, prompt: &str, state: &mut AppState) -> Result<i32> {
    // Use Agent for tool execution
    let mut agent = Agent::new(state)?;

    let response = agent.run(prompt).await?;

    let json = serde_json::json!({
        "content": response.content,
        "model": response.model,
        "usage": response.usage,
    });

    println!("{}", serde_json::to_string_pretty(&json)?);

    // Update state with any changes
    *state = agent.state;

    Ok(0)
}

async fn run_stream_mode(api_client: &ApiClient, prompt: &str, state: &mut AppState) -> Result<i32> {
    println!("=== Response (streaming) ===\n");

    let response = api_client.complete_streaming(prompt, &state.config, state).await?;

    println!("\n[Usage: {} tokens]", response.usage.total_tokens);

    // Update session
    state.add_message("user", prompt);
    state.add_message("assistant", &response.content);

    Ok(0)
}
