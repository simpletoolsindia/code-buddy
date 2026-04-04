//! Live model-list fetchers for each supported provider.
//!
//! Each function performs a single HTTP request to the provider's "list models"
//! endpoint and returns the model identifiers as a `Vec<String>`. On any error
//! (network failure, auth error, malformed JSON) the function returns an empty
//! list so the setup wizard can still offer a manual-entry fallback.

use serde_json::Value;
use tracing::debug;

/// Fetch available models from a running Ollama server.
///
/// Calls `GET http://{host}/api/tags` (default host `localhost:11434`).
pub async fn fetch_ollama_models(host: Option<&str>) -> Vec<String> {
    let base = host.unwrap_or("http://localhost:11434");
    let url = format!("{base}/api/tags");
    match do_get(&url, None).await {
        Ok(v) => v["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        Err(e) => {
            debug!("Ollama model list failed: {e}");
            vec![]
        }
    }
}

/// Fetch available models from a running LM Studio server.
///
/// Calls `GET http://{host}/v1/models` (default host `localhost:1234`).
pub async fn fetch_lm_studio_models(host: Option<&str>) -> Vec<String> {
    let base = host.unwrap_or("http://localhost:1234");
    let url = format!("{base}/v1/models");
    match do_get(&url, None).await {
        Ok(v) => parse_openai_model_list(&v),
        Err(e) => {
            debug!("LM Studio model list failed: {e}");
            vec![]
        }
    }
}

/// Fetch available models from `OpenRouter`.
///
/// Calls `GET https://openrouter.ai/api/v1/models` with Bearer auth.
/// Returns popular models only (filters to those with an `id` field).
pub async fn fetch_openrouter_models(api_key: &str) -> Vec<String> {
    let url = "https://openrouter.ai/api/v1/models";
    match do_get(url, Some(api_key)).await {
        Ok(v) => {
            let models = parse_openai_model_list(&v);
            if models.is_empty() {
                openrouter_fallback()
            } else {
                models
            }
        }
        Err(e) => {
            debug!("OpenRouter model list failed: {e}");
            openrouter_fallback()
        }
    }
}

/// Fetch available models from `OpenAI`.
///
/// Returns a curated shortlist of the most useful models plus the live list.
pub async fn fetch_openai_models(api_key: &str) -> Vec<String> {
    let url = "https://api.openai.com/v1/models";
    match do_get(url, Some(api_key)).await {
        Ok(v) => {
            let mut all = parse_openai_model_list(&v);
            // Filter to commonly-used chat models only.
            all.retain(|id| {
                id.starts_with("gpt-4")
                    || id.starts_with("gpt-3.5")
                    || id.starts_with("o1")
                    || id.starts_with("o3")
            });
            all.sort();
            if all.is_empty() {
                openai_fallback()
            } else {
                all
            }
        }
        Err(e) => {
            debug!("OpenAI model list failed: {e}");
            openai_fallback()
        }
    }
}

/// Fetch available models from NVIDIA NIM endpoint.
///
/// Calls `GET https://integrate.api.nvidia.com/v1/models` with Bearer auth.
/// Returns the live model list filtered to commonly-used chat/instruct models.
/// Falls back to `nvidia_fallback()` on error.
pub async fn fetch_nvidia_models(api_key: &str) -> Vec<String> {
    let url = "https://integrate.api.nvidia.com/v1/models";
    match do_get(url, Some(api_key)).await {
        Ok(v) => {
            let all = parse_openai_model_list(&v);
            if all.is_empty() {
                nvidia_fallback()
            } else {
                // Filter to commonly-used chat/instruct models only.
                let mut filtered: Vec<String> = all
                    .into_iter()
                    .filter(|id| {
                        let id = id.as_str();
                        // Include known model families
                        id.starts_with("meta/")
                            || id.starts_with("mistralai/")
                            || id.starts_with("google/")
                            || id.starts_with("microsoft/")
                            || id.starts_with("deepseek-ai/")
                            || id.starts_with("qwen/")
                            || id.starts_with("nvidia/")
                            || id.starts_with("openai/")
                            || id.starts_with("databricks/")
                            || id.starts_with("moonshotai/")
                            || id.starts_with("abacusai/")
                            || id.starts_with("snowflake/")
                            || id.starts_with("upstage/")
                            || id.starts_with("bigcode/")
                            || id.starts_with("tiiuae/")
                            || id.starts_with("writer/")
                            || id.starts_with("ai21labs/")
                            || id.starts_with("baichuan-inc/")
                            // Exclude embeddings, vision-only, safety, reward models
                            && !id.contains("embed")
                            && !id.contains("safety")
                            && !id.contains("reward")
                            && !id.contains("reasoning")
                            && !id.contains("-reward")
                            && !id.contains("guardian")
                            && !id.contains("guard")
                            && !id.contains("parse")
                            && !id.contains("vision") // too large for chat
                            && !id.contains("gemma-2b")
                            && !id.contains("gemma-3-1b")
                            && !id.contains("gemma-3-4b")
                            && !id.contains("gemma-3n-")
                            && !id.contains("shieldgemma")
                            && !id.contains("granite-3.0-3b")
                            && !id.contains("granite-8b-code")
                            && !id.contains("granite-3.0-8b")
                            && !id.contains("granite-3.3-8b")
                            && !id.contains("nemotron-3-nano")
                            && !id.contains("nemotron-nano-12b")
                            && !id.contains("nemotron-nano-3-30b")
                            && !id.contains("nemotron-mini-4b")
                            && !id.contains("nemotron-4-mini-hindi")
                            && !id.contains("cosmos-")
                            && !id.contains("gliner-")
                            && !id.contains("baai/")
                            && !id.contains("starcoder2-7b")
                            && !id.contains("recurrentgemma")
                            && !id.contains("paligemma")
                            && !id.contains("kosmos-")
                            && !id.contains("neva-")
                            && !id.contains("nvclip")
                            && !id.contains("vila")
                            && !id.contains("riva-")
                            && !id.contains("streampetr")
                    })
                    .collect();
                filtered.sort();
                if filtered.is_empty() {
                    nvidia_fallback()
                } else {
                    filtered
                }
            }
        }
        Err(e) => {
            debug!("NVIDIA model list failed: {e}");
            nvidia_fallback()
        }
    }
}

/// Public re-export of the NVIDIA fallback list (for wizard non-T TY paths).
#[must_use]
pub fn nvidia_models() -> Vec<String> {
    nvidia_fallback()
}

// ── Internal helpers ──────────────────────────────────────────────────────────

async fn do_get(url: &str, bearer: Option<&str>) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let mut req = client.get(url).header("Accept", "application/json");
    if let Some(key) = bearer {
        req = req.bearer_auth(key);
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json::<Value>().await.map_err(|e| e.to_string())
}

fn parse_openai_model_list(v: &Value) -> Vec<String> {
    v["data"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|m| m["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Public re-export of the `OpenRouter` fallback list (for wizard non-TTY paths).
#[must_use]
pub fn openrouter_fallback_pub() -> Vec<String> {
    openrouter_fallback()
}

/// Public re-export of the `OpenAI` fallback list.
#[must_use]
pub fn openai_fallback_pub() -> Vec<String> {
    openai_fallback()
}

fn openrouter_fallback() -> Vec<String> {
    vec![
        "anthropic/claude-3.5-sonnet".to_string(),
        "anthropic/claude-3-haiku".to_string(),
        "openai/gpt-4o".to_string(),
        "openai/gpt-4o-mini".to_string(),
        "google/gemini-2.0-flash-001".to_string(),
        "meta-llama/llama-3.3-70b-instruct".to_string(),
        "mistralai/mistral-large-2411".to_string(),
        "deepseek/deepseek-r1".to_string(),
    ]
}

fn openai_fallback() -> Vec<String> {
    vec![
        "gpt-4o".to_string(),
        "gpt-4o-mini".to_string(),
        "gpt-4-turbo".to_string(),
        "o1".to_string(),
        "o1-mini".to_string(),
        "o3-mini".to_string(),
    ]
}

fn nvidia_fallback() -> Vec<String> {
    vec![
        // Meta Llama
        "meta/llama-3.3-70b-instruct".to_string(),
        "meta/llama-3.1-405b-instruct".to_string(),
        "meta/llama-3.1-70b-instruct".to_string(),
        "meta/llama-3.1-8b-instruct".to_string(),
        "meta/llama-3.2-1b-instruct".to_string(),
        "meta/llama-3.2-3b-instruct".to_string(),
        "meta/llama-4-maverick-17b-128e-instruct".to_string(),
        "meta/llama-4-scout-17b-16e-instruct".to_string(),
        "meta/llama3-70b-instruct".to_string(),
        "meta/llama3-8b-instruct".to_string(),
        "meta/codellama-70b".to_string(),
        // Mistral
        "mistralai/mistral-large-3-675b-instruct-2512".to_string(),
        "mistralai/mistral-large-2-instruct".to_string(),
        "mistralai/mistral-large".to_string(),
        "mistralai/mistral-medium-3-instruct".to_string(),
        "mistralai/mistral-small-4-119b-2603".to_string(),
        "mistralai/mistral-small-24b-instruct".to_string(),
        "mistralai/mistral-small-3.1-24b-instruct-2503".to_string(),
        "mistralai/mistral-nemotron".to_string(),
        "mistralai/devstral-2-123b-instruct-2512".to_string(),
        "mistralai/mathstral-7b-v0.1".to_string(),
        "mistralai/codestral-22b-instruct-v0.1".to_string(),
        "mistralai/mixtral-8x22b-instruct-v0.1".to_string(),
        "mistralai/mistral-7b-instruct-v0.3".to_string(),
        // Google Gemma
        "google/gemma-4-31b-it".to_string(),
        "google/gemma-3-27b-it".to_string(),
        "google/gemma-2-27b-it".to_string(),
        "google/gemma-3-12b-it".to_string(),
        "google/gemma-2-9b-it".to_string(),
        "google/codegemma-7b".to_string(),
        "google/codegemma-1.1-7b".to_string(),
        // Microsoft Phi
        "microsoft/phi-4-mini-instruct".to_string(),
        "microsoft/phi-4-multimodal-instruct".to_string(),
        "microsoft/phi-4-mini-flash-reasoning".to_string(),
        "microsoft/phi-3-medium-128k-instruct".to_string(),
        "microsoft/phi-3-mini-128k-instruct".to_string(),
        // DeepSeek
        "deepseek-ai/deepseek-v3.2".to_string(),
        "deepseek-ai/deepseek-v3.1".to_string(),
        "deepseek-ai/deepseek-r1-distill-qwen-32b".to_string(),
        "deepseek-ai/deepseek-r1-distill-qwen-14b".to_string(),
        "deepseek-ai/deepseek-coder-6.7b-instruct".to_string(),
        // Qwen
        "qwen/qwen3.5-397b-a17b".to_string(),
        "qwen/qwen3.5-122b-a10b".to_string(),
        "qwen/qwen3-next-80b-a3b-instruct".to_string(),
        "qwen/qwen3-next-80b-a3b-thinking".to_string(),
        "qwen/qwen3-coder-480b-a35b-instruct".to_string(),
        "qwen/qwen2.5-coder-32b-instruct".to_string(),
        "qwen/qwen2.5-coder-7b-instruct".to_string(),
        "qwen/qwq-32b".to_string(),
        // Others
        "databricks/dbrx-instruct".to_string(),
        "moonshotai/kimi-k2-instruct".to_string(),
        "moonshotai/kimi-k2.5".to_string(),
        "abacusai/dracarys-llama-3.1-70b-instruct".to_string(),
        "snowflake/arctic-embed-s".to_string(),
        "upstage/solar-10.7b-instruct".to_string(),
        "writer/palmyra-creative-122b".to_string(),
        "writer/palmyra-fin-70b-32k".to_string(),
        "ai21labs/jamba-1.5-large-instruct".to_string(),
        "baichuan-inc/baichuan2-13b-chat".to_string(),
        "tiiuae/falcon3-7b-instruct".to_string(),
    ]
}
