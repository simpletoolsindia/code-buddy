//! Multi-provider model management

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub name: String,
    pub display_name: String,
    pub base_url: String,
    pub api_key_env: String,
    pub default_model: String,
    pub free_models: Vec<String>,
    pub supports_streaming: bool,
}

#[derive(Debug, Clone)]
pub struct MultiProvider {
    pub providers: Vec<Provider>,
}

impl Default for MultiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiProvider {
    pub fn new() -> Self {
        Self {
            providers: vec![
                Provider {
                    name: "anthropic".to_string(),
                    display_name: "Anthropic Claude".to_string(),
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key_env: "ANTHROPIC_API_KEY".to_string(),
                    default_model: "claude-sonnet-4-5".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "openai".to_string(),
                    display_name: "OpenAI".to_string(),
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key_env: "OPENAI_API_KEY".to_string(),
                    default_model: "gpt-4o".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "openrouter".to_string(),
                    display_name: "OpenRouter".to_string(),
                    base_url: "https://openrouter.ai/api/v1".to_string(),
                    api_key_env: "OPENROUTER_API_KEY".to_string(),
                    default_model: "anthropic/claude-3.5-haiku:free".to_string(),
                    free_models: vec![
                        "anthropic/claude-3.5-haiku:free".to_string(),
                        "google/gemma-3-4b-it:free".to_string(),
                        "meta-llama/llama-3.3-70b-instruct:free".to_string(),
                    ],
                    supports_streaming: true,
                },
                Provider {
                    name: "nvidia".to_string(),
                    display_name: "NVIDIA NIM".to_string(),
                    base_url: "https://integrate.api.nvidia.com/v1".to_string(),
                    api_key_env: "NVIDIA_API_KEY".to_string(),
                    default_model: "meta/llama-3.1-8b-instruct".to_string(),
                    free_models: vec![
                        "meta/llama-3.1-8b-instruct".to_string(),
                        "mistralai/mistral-7b-instruct-v0.3".to_string(),
                        "google/gemma-2-9b-it".to_string(),
                    ],
                    supports_streaming: true,
                },
                Provider {
                    name: "ollama".to_string(),
                    display_name: "Ollama".to_string(),
                    base_url: "http://localhost:11434/v1".to_string(),
                    api_key_env: String::new(),
                    default_model: "llama3.2".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "lmstudio".to_string(),
                    display_name: "LM Studio".to_string(),
                    base_url: "http://localhost:1234/v1".to_string(),
                    api_key_env: String::new(),
                    default_model: "local-model".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "groq".to_string(),
                    display_name: "Groq".to_string(),
                    base_url: "https://api.groq.com/openai/v1".to_string(),
                    api_key_env: "GROQ_API_KEY".to_string(),
                    default_model: "llama-3.1-8b-instant".to_string(),
                    free_models: vec!["llama-3.1-8b-instant".to_string()],
                    supports_streaming: true,
                },
                Provider {
                    name: "deepseek".to_string(),
                    display_name: "DeepSeek".to_string(),
                    base_url: "https://api.deepseek.com/v1".to_string(),
                    api_key_env: "DEEPSEEK_API_KEY".to_string(),
                    default_model: "deepseek-chat".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "mistral".to_string(),
                    display_name: "Mistral AI".to_string(),
                    base_url: "https://api.mistral.ai/v1".to_string(),
                    api_key_env: "MISTRAL_API_KEY".to_string(),
                    default_model: "mistral-small-latest".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "perplexity".to_string(),
                    display_name: "Perplexity".to_string(),
                    base_url: "https://api.perplexity.ai".to_string(),
                    api_key_env: "PERPLEXITY_API_KEY".to_string(),
                    default_model: "llama-3.1-sonar-small-128k-online".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "together".to_string(),
                    display_name: "Together AI".to_string(),
                    base_url: "https://api.together.xyz/v1".to_string(),
                    api_key_env: "TOGETHER_API_KEY".to_string(),
                    default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "bedrock".to_string(),
                    display_name: "AWS Bedrock".to_string(),
                    base_url: "https://bedrock.us-east-1.amazonaws.com".to_string(),
                    api_key_env: "AWS_ACCESS_KEY_ID".to_string(),
                    default_model: "anthropic.claude-3-sonnet".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "azure".to_string(),
                    display_name: "Azure OpenAI".to_string(),
                    base_url: std::env::var("AZURE_OPENAI_ENDPOINT")
                        .unwrap_or_else(|_| "https://<resource>.openai.azure.com".to_string()),
                    api_key_env: "AZURE_OPENAI_KEY".to_string(),
                    default_model: "gpt-4o".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
                Provider {
                    name: "vertex".to_string(),
                    display_name: "Google Vertex AI".to_string(),
                    base_url: "https://{location}-aiplatform.googleapis.com/v1".to_string(),
                    api_key_env: "GOOGLE_API_KEY".to_string(),
                    default_model: "claude-3-5-sonnet".to_string(),
                    free_models: vec![],
                    supports_streaming: true,
                },
            ],
        }
    }

    pub fn get(&self, name: &str) -> Option<&Provider> {
        self.providers.iter().find(|p| p.name == name)
    }

    pub fn list(&self) -> Vec<&Provider> {
        self.providers.iter().collect()
    }

    pub fn list_free_providers(&self) -> Vec<&Provider> {
        self.providers.iter()
            .filter(|p| !p.free_models.is_empty())
            .collect()
    }
}
