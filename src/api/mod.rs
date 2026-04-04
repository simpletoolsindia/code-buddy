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
use crate::state::AppState;
use crate::streaming::{StreamingConfig, StreamingParser, StreamingEvent};
use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, instrument};

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
    Mlx, // Apple Silicon MLX
    Cohere,      // Cohere AI
    AI21,        // AI21 Jurassic
    AlephAlpha,  // Aleph Alpha
    Cloudflare,  // Cloudflare Workers AI
    Replicate,   // Replicate
    Anyscale,    // Anyscale Endpoints
    HuggingFace, // HuggingFace Inference API
    Fireworks,   // Fireworks AI
    Cerebras,    // Cerebras
    SambaNova,   // SambaNova
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
        let builder = Client::builder()
            .timeout(Duration::from_secs(120));

        // Apply insecure_ssl config if enabled (for testing with self-signed certs)
        let client = if state.config.insecure_ssl {
            builder
                .danger_accept_invalid_certs(true)
                .build()
                .context("Failed to create HTTP client")?
        } else {
            builder
                .build()
                .context("Failed to create HTTP client")?
        };

        // Validate API key is set for providers that require it
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
            "mlx" | "mlx-community" => LlmProvider::Mlx,
            "huggingface" | "hf" => LlmProvider::HuggingFace,
            "fireworks" | "fw" => LlmProvider::Fireworks,
            "cerebras" | "cb" => LlmProvider::Cerebras,
            "sambanova" | "sn" => LlmProvider::SambaNova,
            _ => LlmProvider::Custom,
        };

        // Validate API key for providers that require it
        let requires_api_key = !matches!(provider, LlmProvider::Ollama | LlmProvider::LmStudio | LlmProvider::Mlx);
        if requires_api_key && state.config.api_key.is_none() {
            anyhow::bail!(
                "API key required for provider '{}' but not configured. \
                 Set it via API_KEY environment variable or --api-key flag.",
                state.config.llm_provider
            );
        }

        Ok(Self { client, provider })
    }

    fn get_base_url(&self, config: &Config) -> Result<String> {
        if let Some(url) = &config.base_url {
            if !url.is_empty() {
                return Ok(url.clone());
            }
        }

        let url = match self.provider {
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
            LlmProvider::Vertex => {
                let location = std::env::var("VERTEX_LOCATION")
                    .unwrap_or_else(|_| "us-central1".to_string());
                // Validate location matches GCP region format to prevent SSRF
                if !location.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                    anyhow::bail!("Invalid VERTEX_LOCATION: must contain only alphanumeric, hyphen, or underscore characters");
                }
                if location.len() > 30 {
                    anyhow::bail!("Invalid VERTEX_LOCATION: must be 30 characters or less");
                }
                format!("https://{}-aiplatform.googleapis.com/v1", location)
            }
            LlmProvider::Mlx => "local".to_string(), // MLX runs locally
            LlmProvider::Cohere => "https://api.cohere.ai/v2".to_string(),
            LlmProvider::AI21 => "https://api.ai21.com/studio/v1".to_string(),
            LlmProvider::AlephAlpha => "https://api.aleph-alpha.com/v1".to_string(),
            LlmProvider::Cloudflare => "https://api.cloudflare.com/client/v4/ai".to_string(),
            LlmProvider::Replicate => "https://api.replicate.com/v1".to_string(),
            LlmProvider::Anyscale => "https://api.endpoints.anyscale.com/v1".to_string(),
            LlmProvider::HuggingFace => "https://api-inference.huggingface.co/proxy/v1".to_string(),
            LlmProvider::Fireworks => "https://api.fireworks.ai/inference/v1".to_string(),
            LlmProvider::Cerebras => "https://api.cerebras.ai/v1".to_string(),
            LlmProvider::SambaNova => "https://api.sambanova.ai/v1".to_string(),
            LlmProvider::Custom => "https://api.anthropic.com".to_string(),
        };
        Ok(url)
    }

    fn get_model(&self, config: &Config) -> String {
        if let Some(model) = &config.model {
            if !model.is_empty() {
                return model.clone();
            }
        };

        match self.provider {
            LlmProvider::Anthropic => "claude-sonnet-4-6".to_string(),
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
            LlmProvider::Mlx => "mlx-community/llama-3.2-3b-instruct-4bit".to_string(),
            LlmProvider::Cohere => "command-r-plus".to_string(),
            LlmProvider::AI21 => "jamba-1.5-large".to_string(),
            LlmProvider::AlephAlpha => "luminous-base".to_string(),
            LlmProvider::Cloudflare => "@cf/meta/llama-3.1-8b-instruct-awq".to_string(),
            LlmProvider::Replicate => "meta/llama-3-70b-instruct".to_string(),
            LlmProvider::Anyscale => "meta-llama/Llama-3-8b-chat-hf".to_string(),
            LlmProvider::HuggingFace => "mistralai/Mistral-7B-Instruct-v0.3".to_string(),
            LlmProvider::Fireworks => "accounts/fireworks/models/llama-v3p1-70b-instruct".to_string(),
            LlmProvider::Cerebras => "cerebras/llama-3.3-70b".to_string(),
            LlmProvider::SambaNova => "Meta-Llama-3.1-70B-Instruct".to_string(),
            LlmProvider::Custom => "claude-sonnet-4-6".to_string(),
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
                headers.insert("NVIDIA-Call-Id".to_string(), "nexus-2".to_string());
            }
            LlmProvider::Ollama | LlmProvider::LmStudio => {
                // No auth required for local
            }
            LlmProvider::Cohere => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            LlmProvider::AI21 => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
            }
            LlmProvider::AlephAlpha => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
            }
            LlmProvider::HuggingFace => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            LlmProvider::Fireworks => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            LlmProvider::Cerebras => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            LlmProvider::SambaNova => {
                headers.insert("Authorization".to_string(), format!("Bearer {}", config.api_key.clone().unwrap_or_default()));
                headers.insert("Content-Type".to_string(), "application/json".to_string());
            }
            _ => {
                if let Some(key) = &config.api_key {
                    headers.insert("authorization".to_string(), format!("Bearer {}", key));
                }
            }
        }

        headers
    }

    #[instrument(skip(self, config, state), fields(provider = ?self.provider, model))]
    pub async fn complete(&self, prompt: &str, config: &Config, state: &AppState) -> Result<CompletionResponse> {
        let base_url = self.get_base_url(config)?;
        let model = self.get_model(config);

        debug!("Starting completion request to {} with model {}", base_url, model);

        // Handle MLX separately (local inference on Apple Silicon)
        if matches!(self.provider, LlmProvider::Mlx) {
            return self.complete_mlx(prompt, &model, state).await;
        }

        // Use Anthropic API only for default Anthropic endpoint
        if matches!(self.provider, LlmProvider::Anthropic)
            && !config.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false)
        {
            return self.complete_anthropic(prompt, &base_url, &model, config).await;
        }

        // Use OpenAI-compatible API for other providers or custom endpoints
        self.complete_openai(prompt, &base_url, &model, config, state).await
    }

    #[instrument(skip(self, config, state), fields(provider = ?self.provider, model))]
    pub async fn complete_streaming(&self, prompt: &str, config: &Config, state: &AppState) -> Result<CompletionResponse> {
        let base_url = self.get_base_url(config)?;
        let model = self.get_model(config);

        // Handle MLX separately
        if matches!(self.provider, LlmProvider::Mlx) {
            return self.complete_mlx_streaming(prompt, &model, state).await;
        }

        if matches!(self.provider, LlmProvider::Anthropic)
            && !config.base_url.as_ref().map(|u| !u.is_empty()).unwrap_or(false)
        {
            // Anthropic streaming is different - use text mode fallback
            return self.complete_anthropic(prompt, &base_url, &model, config).await;
        }

        self.complete_openai_streaming(prompt, &base_url, &model, config, state).await
    }

    /// Complete using MLX (Apple Silicon local inference)
    async fn complete_mlx(&self, prompt: &str, model: &str, _state: &AppState) -> Result<CompletionResponse> {
        use crate::mlx::MlxConfig;

        let mlx_config = MlxConfig::new();

        if !MlxConfig::is_apple_silicon() {
            anyhow::bail!("MLX is only available on Apple Silicon Macs");
        }

        if !mlx_config.check_mlx_lm_installed() {
            anyhow::bail!("mlx-lm not installed. Run: pip install mlx-lm");
        }

        // Build full prompt with conversation history
        let full_prompt = self.build_mlx_prompt(prompt, _state);

        debug!("Running MLX inference with model: {}", model);

        let content = mlx_config.generate(&full_prompt, model).await?;
        let input_len = full_prompt.len();
        let output_len = content.len();

        // Estimate token usage (rough approximation)
        let total_tokens = (output_len / 4) as u32;

        Ok(CompletionResponse {
            content,
            model: model.to_string(),
            usage: TokenUsage {
                input_tokens: (input_len / 4) as u32,
                output_tokens: total_tokens,
                total_tokens: ((input_len + output_len) / 4) as u32,
            },
            stop_reason: Some("stop".to_string()),
        })
    }

    /// Complete using MLX with streaming
    async fn complete_mlx_streaming(&self, prompt: &str, model: &str, _state: &AppState) -> Result<CompletionResponse> {
        use crate::mlx::MlxConfig;
        use std::sync::{Arc, Mutex};

        let mlx_config = MlxConfig::new();

        if !MlxConfig::is_apple_silicon() {
            anyhow::bail!("MLX is only available on Apple Silicon Macs");
        }

        if !mlx_config.check_mlx_lm_installed() {
            anyhow::bail!("mlx-lm not installed. Run: pip install mlx-lm");
        }

        let full_prompt = self.build_mlx_prompt(prompt, _state);
        let input_len = full_prompt.len();

        debug!("Running MLX streaming inference with model: {}", model);

        let full_content = Arc::new(Mutex::new(String::new()));

        let full_content_clone = full_content.clone();
        mlx_config.generate_streaming(&full_prompt, model, move |chunk| {
            print!("{}", chunk);
            std::io::Write::flush(&mut std::io::stdout()).ok();
            if let Ok(mut content) = full_content_clone.lock() {
                content.push_str(chunk);
            }
        }).await?;

        println!(); // Newline after streaming

        // Extract the content from the Arc<Mutex<String>>
        let content = full_content.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
        let output_len = content.len();
        let total_tokens = (output_len / 4) as u32;

        Ok(CompletionResponse {
            content,
            model: model.to_string(),
            usage: TokenUsage {
                input_tokens: (input_len / 4) as u32,
                output_tokens: total_tokens,
                total_tokens: ((input_len + output_len) / 4) as u32,
            },
            stop_reason: Some("stop".to_string()),
        })
    }

    /// Build prompt for MLX with conversation history
    fn build_mlx_prompt(&self, prompt: &str, state: &AppState) -> String {
        let mut messages: Vec<String> = Vec::new();

        // Add system prompt if present
        if let Some(system) = &state.config.system_prompt {
            messages.push(format!("<|system|>\n{}</s>\n", system));
        }

        // Add conversation history
        for msg in &state.conversation_history {
            match msg.role.as_str() {
                "user" => messages.push(format!("<|user|>\n{}</s>\n", msg.content)),
                "assistant" => messages.push(format!("<|assistant|>\n{}</s>\n", msg.content)),
                _ => messages.push(format!("<|{}|>\n{}</s>\n", msg.role, msg.content)),
            }
        }

        // Add current prompt
        messages.push(format!("<|user|>\n{}</s>\n<|assistant|>\n", prompt));

        messages.join("")
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

        let req_builder = self.client
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

        // Use enhanced streaming parser
        let mut streaming_config = StreamingConfig::default();
        streaming_config.detect_json = true;
        streaming_config.extract_tools = true;
        let mut parser = StreamingParser::new(streaming_config);

        // Handle SSE streaming response
        let mut stream = response.bytes_stream();
        let mut input_tokens: u32 = 0;
        let mut output_tokens: u32 = 0;
        let mut tool_calls = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
            };

            // Parse SSE lines using enhanced streaming parser
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                let events = parser.process_sse_line(line);
                for event in &events {
                    match event {
                        StreamingEvent::Text { content } => {
                            print!("{}", content);
                            std::io::Write::flush(&mut std::io::stdout()).ok();
                        }
                        StreamingEvent::ToolCall { name, arguments } => {
                            tool_calls.push((name.clone(), arguments.clone()));
                        }
                        StreamingEvent::Usage { prompt_tokens, completion_tokens, .. } => {
                            input_tokens = *prompt_tokens;
                            output_tokens = *completion_tokens;
                        }
                        StreamingEvent::Done { .. } => {
                            debug!("Streaming complete");
                        }
                        StreamingEvent::Json { .. } => {
                            debug!("Received JSON event");
                        }
                        _ => {}
                    }
                }
            }
        }
        println!(); // Newline after streaming

        let full_content = parser.get_content().to_string();

        // Estimate tokens from content if provider didn't give us counts
        let (input_tokens, output_tokens) = if input_tokens == 0 && output_tokens == 0 {
            // Rough estimate: ~4 chars per token for both input and output
            let estimated_input = (prompt.len() / 4) as u32;
            let estimated_output = (full_content.len() / 4) as u32;
            (estimated_input, estimated_output)
        } else {
            (input_tokens, output_tokens)
        };

        Ok(CompletionResponse {
            content: full_content,
            model: model.to_string(),
            usage: TokenUsage {
                input_tokens,
                output_tokens,
                total_tokens: input_tokens.saturating_add(output_tokens),
            },
            stop_reason: Some("stop".to_string()),
        })
    }

    pub async fn list_models(&self, config: &Config) -> Result<Vec<String>> {
        let base_url = self.get_base_url(config)?;

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
