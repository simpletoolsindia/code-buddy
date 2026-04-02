//! API Client for LLM providers
//!
//! Supports multiple providers:
//! - Anthropic (Claude)
//! - OpenAI (GPT models)
//! - OpenRouter (100+ models)
//! - NVIDIA NIM (open-source models)
//! - Ollama (local models)
//! - LM Studio (local models)
//! - Groq (fast inference)
//! - DeepSeek (affordable)
//! - And more...

use crate::config::Config;
use crate::state::{AppState, ConversationMessage};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: Client,
    provider: LlmProvider,
}

#[derive(Debug, Clone)]
pub enum LlmProvider {
    Anthropic,
    OpenAI,
    OpenRouter,
    Nvidia,
    Ollama,
    LmStudio,
    Groq,
    DeepSeek,
    Mistral,
    Perplexity,
    Together,
    Bedrock,
    Azure,
    Vertex,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(default)]
    stream: bool,
}

// Anthropic-specific request/response
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(rename = "input_tokens")]
    input_tokens: u32,
    #[serde(rename = "output_tokens")]
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl ApiClient {
    pub fn new(state: &AppState) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        let provider = match state.config.llm_provider.to_lowercase().as_str() {
            "anthropic" => LlmProvider::Anthropic,
            "openai" => LlmProvider::OpenAI,
            "openrouter" => LlmProvider::OpenRouter,
            "nvidia" | "nvidia-nim" => LlmProvider::Nvidia,
            "ollama" => LlmProvider::Ollama,
            "lmstudio" | "lm-studio" => LlmProvider::LmStudio,
            "groq" => LlmProvider::Groq,
            "deepseek" => LlmProvider::DeepSeek,
            "mistral" => LlmProvider::Mistral,
            "perplexity" => LlmProvider::Perplexity,
            "together" => LlmProvider::Together,
            "bedrock" | "aws" => LlmProvider::Bedrock,
            "azure" => LlmProvider::Azure,
            "vertex" | "google" => LlmProvider::Vertex,
            _ => LlmProvider::Custom,
        };

        Ok(Self { client, provider })
    }

    fn get_base_url(&self, config: &Config) -> String {
        if let Some(url) = &config.base_url {
            if !url.is_empty() {
                return url.clone();
            }
        }

        match self.provider {
            LlmProvider::Anthropic => "https://api.anthropic.com".to_string(),
            LlmProvider::OpenAI => "https://api.openai.com/v1".to_string(),
            LlmProvider::OpenRouter => "https://openrouter.ai/api/v1".to_string(),
            LlmProvider::Nvidia => "https://integrate.api.nvidia.com/v1".to_string(),
            LlmProvider::Ollama => "http://localhost:11434/v1".to_string(),
            LlmProvider::LmStudio => "http://localhost:1234/v1".to_string(),
            LlmProvider::Groq => "https://api.groq.com/openai/v1".to_string(),
            LlmProvider::DeepSeek => "https://api.deepseek.com/v1".to_string(),
            LlmProvider::Mistral => "https://api.mistral.ai/v1".to_string(),
            LlmProvider::Perplexity => "https://api.perplexity.ai".to_string(),
            LlmProvider::Together => "https://api.together.xyz/v1".to_string(),
            LlmProvider::Bedrock => "https://bedrock.us-east-1.amazonaws.com".to_string(),
            LlmProvider::Azure => std::env::var("AZURE_OPENAI_ENDPOINT")
                .unwrap_or_else(|_| "https://<resource>.openai.azure.com".to_string()),
            LlmProvider::Vertex => "https://{location}-aiplatform.googleapis.com/v1".to_string(),
            LlmProvider::Custom => "https://api.anthropic.com".to_string(),
        }
    }

    fn get_model(&self, config: &Config) -> String {
        let model = if let Some(model) = &config.model {
            if !model.is_empty() {
                return model.clone();
            }
        };

        match self.provider {
            LlmProvider::Anthropic => "claude-sonnet-4-5".to_string(),
            LlmProvider::OpenAI => "gpt-4o".to_string(),
            LlmProvider::OpenRouter => "anthropic/claude-3.5-haiku:free".to_string(),
            LlmProvider::Nvidia => "meta/llama-3.1-8b-instruct".to_string(),
            LlmProvider::Ollama => "llama3.2".to_string(),
            LlmProvider::LmStudio => "local-model".to_string(),
            LlmProvider::Groq => "llama-3.1-8b-instant".to_string(),
            LlmProvider::DeepSeek => "deepseek-chat".to_string(),
            LlmProvider::Mistral => "mistral-small-latest".to_string(),
            LlmProvider::Perplexity => "llama-3.1-sonar-small-128k-online".to_string(),
            LlmProvider::Together => "meta-llama/Llama-3.3-70B-Instruct-Turbo".to_string(),
            LlmProvider::Bedrock => "anthropic.claude-3-sonnet".to_string(),
            LlmProvider::Azure => "gpt-4o".to_string(),
            LlmProvider::Vertex => "claude-3-5-sonnet".to_string(),
            LlmProvider::Custom => "claude-sonnet-4-5".to_string(),
        }
    }

    fn get_headers(&self, config: &Config) -> HashMap<String, String> {
        let mut headers = HashMap::new();

        match self.provider {
            LlmProvider::Anthropic => {
                headers.insert("x-api-key".to_string(), config.api_key.clone().unwrap_or_default());
                headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
                headers.insert("content-type".to_string(), "application/json".to_string());
            }
            LlmProvider::OpenRouter => {
                if let Some(key) = &config.api_key {
                    headers.insert("authorization".to_string(), format!("Bearer {}", key));
                }
                headers.insert("HTTP-Referer".to_string(), "https://claude-code.local".to_string());
                headers.insert("X-Title".to_string(), "Claude Code".to_string());
            }
            LlmProvider::Nvidia => {
                headers.insert("authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
                headers.insert("NVIDIA-Cusation-Id".to_string(), "nexus-2".to_string());
            }
            LlmProvider::Ollama | LlmProvider::LmStudio => {
                // No auth required for local
            }
            _ => {
                if let Some(key) = &config.api_key {
                    headers.insert("authorization".to_string(), format!("Bearer {}", key));
                }
            }
        }

        headers
    }

    pub async fn complete(&self, prompt: &str, config: &Config, state: &AppState) -> Result<CompletionResponse> {
        let base_url = self.get_base_url(config);
        let model = self.get_model(config);

        // Use Anthropic API only for default Anthropic endpoint
        if matches!(self.provider, LlmProvider::Anthropic)
            && !config.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false)
        {
            return self.complete_anthropic(prompt, &base_url, &model, config).await;
        }

        // Use OpenAI-compatible API for other providers or custom endpoints
        self.complete_openai(prompt, &base_url, &model, config, state).await
    }

    pub async fn complete_streaming(&self, prompt: &str, config: &Config, state: &AppState) -> Result<CompletionResponse> {
        let base_url = self.get_base_url(config);
        let model = self.get_model(config);

        if matches!(self.provider, LlmProvider::Anthropic)
            && !config.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false)
        {
            // Anthropic streaming is different - use text mode fallback
            return self.complete_anthropic(prompt, &base_url, &model, config).await;
        }

        self.complete_openai_streaming(prompt, &base_url, &model, config, state).await
    }

    async fn complete_anthropic(&self, prompt: &str, base_url: &str, model: &str, config: &Config) -> Result<CompletionResponse> {
        let request = AnthropicRequest {
            model: model.to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 4096,
            stream: None,
        };

        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let mut req_builder = self.client
            .post(&url)
            .header("x-api-key", config.api_key.clone().unwrap_or_default())
            .header("anthropic-version", "2023-06-01")
            .json(&request);

        let response = req_builder.send().await.context("Request failed")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, error_text);
        }

        let anthropic_resp: AnthropicResponse = response.json().await.context("Failed to parse response")?;

        let content = anthropic_resp.content.first()
            .and_then(|c| c.text.clone())
            .unwrap_or_default();

        let usage = anthropic_resp.usage.map(|u| TokenUsage {
            input_tokens: u.input_tokens,
            output_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        }).unwrap_or_default();

        Ok(CompletionResponse {
            content,
            model: anthropic_resp.model.unwrap_or_else(|| model.to_string()),
            usage,
            stop_reason: None,
        })
    }

    async fn complete_openai(&self, prompt: &str, base_url: &str, model: &str, config: &Config, state: &AppState) -> Result<CompletionResponse> {
        let headers = self.get_headers(config);
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        // Build messages from conversation history + current prompt
        let mut messages: Vec<ChatMessage> = state.conversation_history.iter()
            .map(|msg| ChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        // Add current user message
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = OpenAIRequest {
            model: model.to_string(),
            messages,
            temperature: None,
            max_tokens: Some(4096),
            stream: false,
        };

        let mut req_builder = self.client.post(&url).json(&request);

        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder.send().await.context("Request failed")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, error_text);
        }

        let openai_resp: OpenAIResponse = response.json().await.context("Failed to parse response")?;

        let content = openai_resp.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = openai_resp.usage.map(|u| TokenUsage {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }).unwrap_or_default();

        Ok(CompletionResponse {
            content,
            model: openai_resp.model.unwrap_or_else(|| model.to_string()),
            usage,
            stop_reason: openai_resp.choices.first().and_then(|c| c.finish_reason.clone()),
        })
    }

    async fn complete_openai_streaming(&self, prompt: &str, base_url: &str, model: &str, config: &Config, state: &AppState) -> Result<CompletionResponse> {
        let headers = self.get_headers(config);
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        // Build messages from conversation history + current prompt
        let mut messages: Vec<ChatMessage> = state.conversation_history.iter()
            .map(|msg| ChatMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        let request = OpenAIRequest {
            model: model.to_string(),
            messages,
            temperature: None,
            max_tokens: Some(4096),
            stream: true,
        };

        let mut req_builder = self.client.post(&url).json(&request);

        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }

        let response = req_builder.send().await.context("Request failed")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("API error ({}): {}", status, error_text);
        }

        // Handle SSE streaming response
        let mut stream = response.bytes_stream();
        let mut full_content = String::new();
        let mut total_tokens = 0u32;

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
            };

            // Parse SSE lines: data: {"choices":[{"delta":{"content":"..."}}]}
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        continue;
                    }
                    // Try to extract delta content
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json.pointer("/choices/0/delta/content")
                            .and_then(|v| v.as_str())
                        {
                            print!("{}", content);
                            std::io::Write::flush(&mut std::io::stdout()).ok();
                            full_content.push_str(content);
                        }
                    }
                }
            }
        }
        println!(); // Newline after streaming

        Ok(CompletionResponse {
            content: full_content,
            model: model.to_string(),
            usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                total_tokens,
            },
            stop_reason: Some("stop".to_string()),
        })
    }

    pub async fn list_models(&self, config: &Config) -> Result<Vec<String>> {
        let base_url = self.get_base_url(config);

        let url = match self.provider {
            LlmProvider::OpenRouter => format!("{}/models", base_url.trim_end_matches('/')),
            _ => return Ok(vec!["default".to_string()]),
        };

        let mut req_builder = self.client.get(&url);

        if let Some(key) = &config.api_key {
            req_builder = req_builder.header("authorization", format!("Bearer {}", key));
        }

        let response = req_builder.send().await.context("Failed to list models")?;

        if !response.status().is_success() {
            return Ok(vec!["default".to_string()]);
        }

        #[derive(Deserialize)]
        struct ModelListResponse {
            data: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            id: String,
        }

        if let Ok(list) = response.json::<ModelListResponse>().await {
            Ok(list.data.into_iter().map(|m| m.id).collect())
        } else {
            Ok(vec!["default".to_string()])
        }
    }
}
