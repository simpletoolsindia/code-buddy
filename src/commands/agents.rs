//! Agents command - List configured agents

use crate::state::AppState;
use anyhow::Result;

pub async fn run(list: bool, state: &mut AppState) -> Result<i32> {
    if list {
        println!("=== Configured Agents ===\n");

        if state.config.agents.is_empty() {
            println!("No custom agents configured.");
            println!("Default agents: code-buddy (main), sonnet, haiku");
        } else {
            for (name, agent) in &state.config.agents {
                println!("Agent: {}", name);
                println!("  Model: {}", agent.model.as_deref().unwrap_or("default"));
                println!("  Description: {}", agent.description.as_deref().unwrap_or("N/A"));
                println!();
            }
        }

        println!("\nUsage: code-buddy --agent <agent-name>");
    } else {
        println!("Agents command requires --list flag");
        println!("Usage: code-buddy agents --list");
    }

    Ok(0)
}
