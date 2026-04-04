//! MoA Tool - Mixture of Agents for ensemble reasoning
//!
//! Use when quality improves from multiple reasoning passes or perspectives.
//! Good for architecture review, debugging, security analysis, design tradeoffs.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Tool;

/// Mixture of Agents tool for ensemble reasoning
pub struct MoATool;

impl MoATool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MoATool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for MoATool {
    fn name(&self) -> &str {
        "MixtureOfAgents"
    }

    fn description(&self) -> &str {
        "Use multiple AI agents with different perspectives to analyze a problem, \
then synthesize their responses into a comprehensive answer. \
Best for: architecture review, debugging ambiguous failures, security reviews, \
design tradeoff analysis, root-cause exploration. \
Args: <prompt> [--agents <n>] [--model <model>]
Example: MixtureOfAgents('Review this architecture for scalability')
Example: MoA('Debug why the API returns 500 on POST /users')
Example: MoA('Security audit this authentication flow')"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("MixtureOfAgents tool usage:\n\
  MixtureOfAgents(<prompt>, [--agents <n>], [--model <model>])\n\
  Uses multiple agents to analyze the prompt from different angles\n\
  Returns synthesized response from all agents\n\
  Default: 3 agents, auto-select models based on complexity".to_string());
        }

        let prompt = args.first().map(|s| s.as_str()).unwrap_or("");

        // Parse optional flags
        let mut num_agents: usize = 3;
        let mut model_override: Option<String> = None;

        for i in 1..args.len() {
            match args[i].as_str() {
                "--agents" if i + 1 < args.len() => {
                    num_agents = args[i + 1].parse().unwrap_or(3);
                }
                "--model" if i + 1 < args.len() => {
                    model_override = Some(args[i + 1].clone());
                }
                _ => {}
            }
        }

        // Run MoA query using the existing module
        let config = code_buddy::mixture_of_agents::MoAConfig {
            agents: vec![
                code_buddy::mixture_of_agents::MoAAgent {
                    name: "Architect".to_string(),
                    model: model_override.clone().unwrap_or_else(|| "claude-sonnet-4-5".to_string()),
                    provider: None,
                    role: "software architect reviewing design patterns and scalability".to_string(),
                    prompt_template: None,
                },
                code_buddy::mixture_of_agents::MoAAgent {
                    name: "Security".to_string(),
                    model: model_override.clone().unwrap_or_else(|| "claude-sonnet-4-5".to_string()),
                    provider: None,
                    role: "security engineer looking for vulnerabilities and risks".to_string(),
                    prompt_template: None,
                },
                code_buddy::mixture_of_agents::MoAAgent {
                    name: "Pragmatist".to_string(),
                    model: model_override.unwrap_or_else(|| "claude-sonnet-4-5".to_string()),
                    provider: None,
                    role: "pragmatic engineer focused on implementation feasibility and tradeoffs".to_string(),
                    prompt_template: None,
                },
            ],
            aggregator_model: "claude-sonnet-4-5".to_string(),
            aggregator_provider: None,
            temperature: 0.7,
            max_tokens: 4096,
        };

        let moa = code_buddy::mixture_of_agents::MixtureOfAgents::new(config);

        // Run synchronously using block_on (since this is a sync Tool)
        let rt = tokio::runtime::Runtime::new()?;
        let response = rt.block_on(moa.query(prompt));

        match response {
            Ok(resp) => {
                let output = serde_json::to_string_pretty(&serde_json::json!({
                    "final_response": resp.final_response,
                    "agent_count": resp.agent_responses.len(),
                    "consensus_score": resp.consensus_score,
                    "tokens_used": resp.tokens_used,
                    "agent_responses": resp.agent_responses.into_iter().map(|r| {
                        serde_json::json!({
                            "agent": r.agent_name,
                            "response": r.response,
                            "confidence": r.confidence,
                            "reasoning": r.reasoning,
                        })
                    }).collect::<Vec<_>>(),
                }))?;
                Ok(output)
            }
            Err(e) => {
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                    "hint": "Ensure LLM API key is configured"
                }))?)
            }
        }
    }
}
