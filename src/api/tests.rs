//! Unit tests for API client

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_usage_default() {
        let usage = super::TokenUsage::default();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn test_completion_response_fields() {
        let usage = super::TokenUsage {
            input_tokens: 100,
            output_tokens: 200,
            total_tokens: 300,
        };
        let response = super::CompletionResponse {
            content: "Hello".to_string(),
            model: "test-model".to_string(),
            usage,
            stop_reason: Some("stop".to_string()),
        };
        assert_eq!(response.content, "Hello");
        assert_eq!(response.model, "test-model");
        assert_eq!(response.usage.total_tokens, 300);
    }

    #[test]
    fn test_llm_provider_variants() {
        // Test all provider variants exist
        let providers = vec![
            super::LlmProvider::Anthropic,
            super::LlmProvider::OpenAI,
            super::LlmProvider::OpenRouter,
            super::LlmProvider::Nvidia,
            super::LlmProvider::Ollama,
            super::LlmProvider::LmStudio,
            super::LlmProvider::Groq,
            super::LlmProvider::DeepSeek,
            super::LlmProvider::Mistral,
            super::LlmProvider::Perplexity,
            super::LlmProvider::Together,
            super::LlmProvider::Bedrock,
            super::LlmProvider::Azure,
            super::LlmProvider::Vertex,
            super::LlmProvider::Custom,
        ];

        for provider in providers {
            let debug_str = format!("{:?}", provider);
            assert!(!debug_str.is_empty());
        }
    }
}