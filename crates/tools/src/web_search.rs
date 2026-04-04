//! `web_search` tool — query the web using `DuckDuckGo`, Brave Search, or `SerpAPI`.
//!
//! # Backend priority (no API key needed for any)
//! 1. `BRAVE_SEARCH_API_KEY` / `brave_api_key` in config → Brave Search
//! 2. `SERPAPI_KEY` / `serpapi_key` in config → `SerpAPI`
//! 3. Neither set → `DuckDuckGo` (free, no key required) via HTML scraping
//!
//! # Security
//! The query string is URL-encoded before being included in the request URL.
//! No file-system access is performed.

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use serde_json::{Value, json};
use tracing::instrument;
use url::Url;
use websearch::providers::DuckDuckGoProvider;
use websearch::{SearchOptions, web_search};

use crate::Tool;

const MAX_RESULTS: u32 = 10;
const REQUEST_TIMEOUT_SECS: u64 = 15;

/// Backend to use for web search.
#[derive(Debug, Clone)]
pub enum SearchBackend {
    DuckDuckGo,
    Brave { api_key: String },
    SerpApi { api_key: String },
    None,
}

/// Tool that searches the web and returns a list of `{title, url, snippet}` objects.
pub struct WebSearchTool {
    backend: SearchBackend,
}

impl WebSearchTool {
    /// Create from explicit backend.
    #[must_use]
    pub fn new(backend: SearchBackend) -> Self {
        Self { backend }
    }

    /// Resolve backend from environment variables, then provided keys.
    /// Falls back to `DuckDuckGo` (free, no key needed) when nothing is configured.
    #[must_use]
    pub fn from_env(brave_key: Option<String>, serpapi_key: Option<String>) -> Self {
        let brave = brave_key
            .or_else(|| std::env::var("BRAVE_SEARCH_API_KEY").ok())
            .filter(|k| !k.is_empty());

        let serp = serpapi_key
            .or_else(|| std::env::var("SERPAPI_KEY").ok())
            .filter(|k| !k.is_empty());

        let backend = if let Some(key) = brave {
            SearchBackend::Brave { api_key: key }
        } else if let Some(key) = serp {
            SearchBackend::SerpApi { api_key: key }
        } else {
            // DuckDuckGo is always available — no API key needed
            SearchBackend::DuckDuckGo
        };

        Self { backend }
    }

    /// Returns `true` if any search backend is available (always true — `DuckDuckGo` is free).
    #[must_use]
    pub fn is_configured(&self) -> bool {
        !matches!(self.backend, SearchBackend::None)
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for current information. Returns a list of results with \
         title, URL, and a short snippet. Use this to look up recent events, \
         documentation, or any information that may have changed after your \
         training cutoff. Uses DuckDuckGo by default (free, no API key needed). \
         Optionally configure BRAVE_SEARCH_API_KEY or SERPAPI_KEY for higher quality results."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query string."
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (1-10, default 5).",
                    "minimum": 1,
                    "maximum": 10
                }
            },
            "required": ["query"]
        })
    }

    #[instrument(skip(self), fields(tool = "web_search"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let query = input["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "web_search".to_string(),
                reason: "missing required field 'query'".to_string(),
            })?
            .to_string();

        #[allow(clippy::cast_possible_truncation)]
        let max_results = input["max_results"]
            .as_u64()
            .map_or(5, |n| n.min(u64::from(MAX_RESULTS)) as u32);

        match &self.backend {
            SearchBackend::None => Ok(
                "web_search is not configured. \
                 Set BRAVE_SEARCH_API_KEY or SERPAPI_KEY to enable web search."
                    .to_string(),
            ),
            SearchBackend::DuckDuckGo => {
                duckduckgo_search(&query, max_results).await
            }
            SearchBackend::Brave { api_key } => {
                brave_search(&query, max_results, api_key).await
            }
            SearchBackend::SerpApi { api_key } => {
                serpapi_search(&query, max_results, api_key).await
            }
        }
    }
}

// ── Brave Search ──────────────────────────────────────────────────────────────

async fn brave_search(query: &str, count: u32, api_key: &str) -> Result<String, ToolError> {
    let mut url = Url::parse("https://api.search.brave.com/res/v1/web/search")
        .expect("static URL is valid");
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("count", &count.to_string());

    let client = http_client()?;
    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .header("X-Subscription-Token", api_key)
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_search".to_string(),
            reason: format!("HTTP request failed: {e}"),
        })?;

    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Ok(
            "web_search: Brave API key is invalid or expired. \
             Please update BRAVE_SEARCH_API_KEY."
                .to_string(),
        );
    }

    let body: Value = resp.json().await.map_err(|e| ToolError::ExecutionFailed {
        tool: "web_search".to_string(),
        reason: format!("Failed to parse Brave response: {e}"),
    })?;

    let results = body["web"]["results"].as_array().map_or_else(
        || json!([]),
        |arr| {
            Value::Array(
                arr.iter()
                    .take(count as usize)
                    .map(|r| {
                        json!({
                            "title": r["title"].as_str().unwrap_or(""),
                            "url": r["url"].as_str().unwrap_or(""),
                            "snippet": r["description"].as_str().unwrap_or("")
                        })
                    })
                    .collect(),
            )
        },
    );

    serde_json::to_string_pretty(&results).map_err(|e| ToolError::ExecutionFailed {
        tool: "web_search".to_string(),
        reason: e.to_string(),
    })
}

// ── SerpAPI ───────────────────────────────────────────────────────────────────

async fn serpapi_search(query: &str, count: u32, api_key: &str) -> Result<String, ToolError> {
    let mut url =
        Url::parse("https://serpapi.com/search").expect("static URL is valid");
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("num", &count.to_string())
        .append_pair("api_key", api_key);

    let client = http_client()?;
    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_search".to_string(),
            reason: format!("HTTP request failed: {e}"),
        })?;

    let body: Value = resp.json().await.map_err(|e| ToolError::ExecutionFailed {
        tool: "web_search".to_string(),
        reason: format!("Failed to parse SerpAPI response: {e}"),
    })?;

    if let Some(err) = body["error"].as_str() {
        return Ok(format!("web_search error: {err}"));
    }

    let results = body["organic_results"].as_array().map_or_else(
        || json!([]),
        |arr| {
            Value::Array(
                arr.iter()
                    .take(count as usize)
                    .map(|r| {
                        json!({
                            "title": r["title"].as_str().unwrap_or(""),
                            "url": r["link"].as_str().unwrap_or(""),
                            "snippet": r["snippet"].as_str().unwrap_or("")
                        })
                    })
                    .collect(),
            )
        },
    );

    serde_json::to_string_pretty(&results).map_err(|e| ToolError::ExecutionFailed {
        tool: "web_search".to_string(),
        reason: e.to_string(),
    })
}

// ── DuckDuckGo (free, no API key) ─────────────────────────────────────────────

async fn duckduckgo_search(query: &str, max_results: u32) -> Result<String, ToolError> {
    let provider = DuckDuckGoProvider::new();
    let options = SearchOptions {
        query: query.to_string(),
        max_results: Some(max_results),
        debug: None,
        provider: Box::new(provider),
        ..Default::default()
    };

    let results = web_search(options).await.map_err(|e| ToolError::ExecutionFailed {
        tool: "web_search".to_string(),
        reason: format!("DuckDuckGo search failed: {e}"),
    })?;

    let formatted: Vec<Value> = results
        .into_iter()
        .map(|r| {
            json!({
                "title": r.title,
                "url": r.url,
                "snippet": r.snippet.unwrap_or_default()
            })
        })
        .collect();

    serde_json::to_string_pretty(&formatted).map_err(|e| ToolError::ExecutionFailed {
        tool: "web_search".to_string(),
        reason: e.to_string(),
    })
}

// ── Shared ────────────────────────────────────────────────────────────────────

fn http_client() -> Result<reqwest::Client, ToolError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_search".to_string(),
            reason: e.to_string(),
        })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_key_uses_duckduckgo() {
        let tool = WebSearchTool::from_env(None, None);
        assert!(tool.is_configured());
        assert!(matches!(tool.backend, SearchBackend::DuckDuckGo));
    }

    #[test]
    fn brave_key_sets_backend() {
        let tool = WebSearchTool::from_env(Some("bsk-fake".to_string()), None);
        assert!(tool.is_configured());
        assert!(matches!(tool.backend, SearchBackend::Brave { .. }));
    }

    #[test]
    fn serpapi_key_fallback() {
        let tool = WebSearchTool::from_env(None, Some("serp-fake".to_string()));
        assert!(tool.is_configured());
        assert!(matches!(tool.backend, SearchBackend::SerpApi { .. }));
    }

    #[test]
    fn brave_preferred_over_serpapi() {
        let tool = WebSearchTool::from_env(
            Some("brave-key".to_string()),
            Some("serp-key".to_string()),
        );
        assert!(matches!(tool.backend, SearchBackend::Brave { .. }));
    }

    #[tokio::test]
    async fn none_backend_execute_returns_message() {
        let tool = WebSearchTool::new(SearchBackend::None);
        let result = tool.execute(json!({"query": "test"})).await.unwrap();
        assert!(result.contains("not configured"));
    }
}
