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

/// Curated model list for NVIDIA NIM (no public list endpoint without auth).
#[must_use]
pub fn nvidia_models() -> Vec<String> {
    vec![
        "meta/llama-3.3-70b-instruct".to_string(),
        "meta/llama-3.1-70b-instruct".to_string(),
        "meta/llama-3.1-8b-instruct".to_string(),
        "mistralai/mistral-large-2-instruct".to_string(),
        "mistralai/mixtral-8x22b-instruct-v0.1".to_string(),
        "google/gemma-2-27b-it".to_string(),
        "microsoft/phi-3-medium-128k-instruct".to_string(),
    ]
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
