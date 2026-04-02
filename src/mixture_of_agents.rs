//! Mixture of Agents - Ensemble reasoning with multiple agents
//!
//! Combines responses from multiple agents for improved reasoning.
//! Based on the MoA architecture from Hermes Agent.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent in the ensemble
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoAAgent {
    pub name: String,
    pub model: String,
    pub provider: Option<String>,
    pub role: String,
    pub prompt_template: Option<String>,
}

/// MoA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoAConfig {
    pub agents: Vec<MoAAgent>,
    pub aggregator_model: String,
    pub aggregator_provider: Option<String>,
    pub temperature: f32,
    pub max_tokens: usize,
}

/// Aggregated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoAResponse {
    pub final_response: String,
    pub agent_responses: Vec<AgentResponse>,
    pub consensus_score: f32,
    pub tokens_used: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub agent_name: String,
    pub response: String,
    pub confidence: f32,
    pub reasoning: Option<String>,
}

/// Mixture of Agents
pub struct MixtureOfAgents {
    config: MoAConfig,
}

impl MixtureOfAgents {
    pub fn new(config: MoAConfig) -> Self {
        Self { config }
    }

    /// Run ensemble query
    pub async fn query(&self, prompt: &str) -> Result<MoAResponse> {
        // Collect responses from all agents
        let mut agent_responses = vec![];

        for agent in &self.config.agents {
            let response = self.call_agent(agent, prompt).await?;
            agent_responses.push(response);
        }

        // Aggregate responses
        let aggregated = self.aggregate(agent_responses.clone(), prompt).await?;

        Ok(aggregated)
    }

    /// Call a single agent
    async fn call_agent(&self, agent: &MoAAgent, prompt: &str) -> Result<AgentResponse> {
        // Build agent-specific prompt
        let agent_prompt = if let Some(template) = &agent.prompt_template {
            template.replace("{prompt}", prompt)
        } else {
            format!(
                "You are {}. {}:\n\n{}",
                agent.name, agent.role, prompt
            )
        };

        // In a real implementation, this would call the LLM API
        // For now, return a stub response
        let response = format!(
            "[{}] Response to: {}",
            agent.name,
            prompt.chars().take(50).collect::<String>()
        );

        Ok(AgentResponse {
            agent_name: agent.name.clone(),
            response,
            confidence: 0.8,
            reasoning: None,
        })
    }

    /// Aggregate responses using the aggregator model
    async fn aggregate(&self, agent_responses: Vec<AgentResponse>, original_prompt: &str) -> Result<MoAResponse> {
        // Build aggregation prompt
        let mut aggregation_prompt = format!("Original question: {}\n\n", original_prompt);
        aggregation_prompt.push_str("Responses from different agents:\n\n");

        for (i, resp) in agent_responses.iter().enumerate() {
            aggregation_prompt.push_str(&format!(
                "[Agent {} - {}]:\n{}\n\n",
                i + 1, resp.agent_name, resp.response
            ));
        }

        aggregation_prompt.push_str(
            "\nSynthesize the best aspects of each response into a comprehensive answer."
        );

        // In a real implementation, this would call the aggregator model
        // For now, use the longest/most confident response
        let best_response = agent_responses
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap())
            .cloned()
            .unwrap();

        let consensus_score = self.calculate_consensus(&agent_responses);

        let total_tokens = agent_responses.iter().map(|r| r.response.len() / 4).sum();

        Ok(MoAResponse {
            final_response: best_response.response,
            agent_responses,
            consensus_score,
            tokens_used: total_tokens,
        })
    }

    /// Calculate consensus score between agents
    fn calculate_consensus(&self, responses: &[AgentResponse]) -> f32 {
        if responses.len() < 2 {
            return 1.0;
        }

        // Simple consensus: check if responses have similar length and structure
        let total: f32 = responses.iter().map(|r| r.confidence).sum();
        let avg = total / responses.len() as f32;

        // Check variance
        let variance: f32 = responses
            .iter()
            .map(|r| (r.confidence - avg).powi(2))
            .sum::<f32>() / responses.len() as f32;

        // Low variance = high consensus
        let consensus = 1.0 - variance.sqrt();
        consensus.max(0.0).min(1.0)
    }

    /// Get default MoA configuration
    pub fn default_config() -> Self {
        let agents = vec![
            MoAAgent {
                name: "Researcher".to_string(),
                model: "claude-opus-4.6".to_string(),
                provider: Some("anthropic".to_string()),
                role: "You research thoroughly and cite sources.".to_string(),
                prompt_template: None,
            },
            MoAAgent {
                name: "Critic".to_string(),
                model: "claude-opus-4.6".to_string(),
                provider: Some("anthropic".to_string()),
                role: "You identify flaws and weaknesses in arguments.".to_string(),
                prompt_template: None,
            },
            MoAAgent {
                name: "Builder".to_string(),
                model: "claude-opus-4.6".to_string(),
                provider: Some("anthropic".to_string()),
                role: "You focus on practical implementation and solutions.".to_string(),
                prompt_template: None,
            },
            MoAAgent {
                name: "Explainer".to_string(),
                model: "claude-opus-4.6".to_string(),
                provider: Some("anthropic".to_string()),
                role: "You explain complex concepts clearly and simply.".to_string(),
                prompt_template: None,
            },
        ];

        Self {
            config: MoAConfig {
                agents,
                aggregator_model: "claude-opus-4.6".to_string(),
                aggregator_provider: Some("anthropic".to_string()),
                temperature: 0.7,
                max_tokens: 4096,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_moa_query() {
        let moa = MixtureOfAgents::default_config();
        let result = moa.query("What is 2+2?").await.unwrap();
        assert!(!result.final_response.is_empty());
        assert!(result.agent_responses.len() >= 2);
    }

    #[test]
    fn test_consensus_calculation() {
        let moa = MixtureOfAgents::default_config();
        let responses = vec![
            AgentResponse {
                agent_name: "A".to_string(),
                response: "Test".to_string(),
                confidence: 0.8,
                reasoning: None,
            },
            AgentResponse {
                agent_name: "B".to_string(),
                response: "Test".to_string(),
                confidence: 0.8,
                reasoning: None,
            },
        ];
        let score = moa.calculate_consensus(&responses);
        assert!(score > 0.5);
    }
}
