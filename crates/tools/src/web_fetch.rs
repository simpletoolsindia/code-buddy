//! `web_fetch` tool — download and extract text content from a URL.
//!
//! # Backends
//! 1. **Firecrawl** (`FIRECRAWL_API_KEY` set) — returns clean markdown via the
//!    Firecrawl scrape API, best quality for complex pages.
//! 2. **Plain HTTP** (default) — fetches the raw HTML and strips it to body text
//!    using `scraper`. Simple and requires no API key.
//!
//! # Limits
//! Responses are capped at 512 KB to prevent context flooding. The tool also
//! rejects non-HTTP(S) schemes and validates that the URL is well-formed.

use async_trait::async_trait;
use code_buddy_errors::ToolError;
use scraper::{Html, Selector};
use serde_json::{Value, json};
use tracing::instrument;
use url::Url;

use crate::Tool;

const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 20;

/// Tool that fetches a URL and returns its text content.
pub struct WebFetchTool {
    firecrawl_api_key: Option<String>,
}

impl WebFetchTool {
    /// Create, resolving Firecrawl key from env or explicit value.
    #[must_use]
    pub fn new(firecrawl_api_key: Option<String>) -> Self {
        let key = firecrawl_api_key
            .or_else(|| std::env::var("FIRECRAWL_API_KEY").ok())
            .filter(|k| !k.is_empty());
        Self {
            firecrawl_api_key: key,
        }
    }

    /// Returns `true` if Firecrawl is configured.
    #[must_use]
    pub fn uses_firecrawl(&self) -> bool {
        self.firecrawl_api_key.is_some()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Fetch the text content of a web page. Use this to read documentation, \
         articles, or any publicly accessible URL. Returns plain text (HTML tags \
         stripped). An optional CSS selector filters to a specific part of the page. \
         Response is limited to 512 KB. Non-HTTP(S) URLs are rejected."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch (must be http or https)."
                },
                "selector": {
                    "type": "string",
                    "description": "Optional CSS selector to extract a specific element (e.g. 'article', 'main', '.content')."
                }
            },
            "required": ["url"]
        })
    }

    #[instrument(skip(self), fields(tool = "web_fetch"))]
    async fn execute(&self, input: Value) -> Result<String, ToolError> {
        let raw_url = input["url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs {
                tool: "web_fetch".to_string(),
                reason: "missing required field 'url'".to_string(),
            })?;

        let url = Url::parse(raw_url).map_err(|e| ToolError::InvalidArgs {
            tool: "web_fetch".to_string(),
            reason: format!("invalid URL: {e}"),
        })?;

        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(ToolError::InvalidArgs {
                tool: "web_fetch".to_string(),
                reason: format!(
                    "only http and https URLs are supported (got '{}')",
                    url.scheme()
                ),
            });
        }

        let selector = input["selector"].as_str().map(str::to_string);

        if let Some(ref key) = self.firecrawl_api_key {
            return firecrawl_fetch(raw_url, key).await;
        }

        plain_fetch(&url, selector.as_deref()).await
    }
}

// ── Plain HTTP + scraper ──────────────────────────────────────────────────────

async fn plain_fetch(url: &Url, selector: Option<&str>) -> Result<String, ToolError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("code-buddy/0.1 (web_fetch tool)")
        .build()
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_fetch".to_string(),
            reason: e.to_string(),
        })?;

    let resp = client
        .get(url.as_str())
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_fetch".to_string(),
            reason: format!("HTTP request failed: {e}"),
        })?;

    let status = resp.status();
    if !status.is_success() {
        return Ok(format!(
            "web_fetch: HTTP {status} for {url}"
        ));
    }

    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = resp.bytes().await.map_err(|e| ToolError::ExecutionFailed {
        tool: "web_fetch".to_string(),
        reason: format!("Failed to read response body: {e}"),
    })?;

    if bytes.len() > MAX_RESPONSE_BYTES {
        let truncated = &bytes[..MAX_RESPONSE_BYTES];
        let text = String::from_utf8_lossy(truncated).to_string();
        let parsed = extract_text(&text, selector, &content_type);
        return Ok(format!(
            "{parsed}\n\n[Response truncated at 512 KB]"
        ));
    }

    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(extract_text(&text, selector, &content_type))
}

fn extract_text(body: &str, selector: Option<&str>, content_type: &str) -> String {
    if !content_type.contains("html") && !content_type.is_empty() {
        return body.to_string();
    }

    let document = Html::parse_document(body);

    if let Some(sel_str) = selector {
        if let Ok(sel) = Selector::parse(sel_str) {
            let parts: Vec<String> = document
                .select(&sel)
                .map(|el| el.text().collect::<Vec<_>>().join(" "))
                .collect();
            if !parts.is_empty() {
                return clean_whitespace(&parts.join("\n\n"));
            }
        }
    }

    let body_sel = Selector::parse("body").expect("'body' is a valid selector");
    let script_sel = Selector::parse("script, style, nav, header, footer")
        .expect("static selector is valid");

    let mut result_parts: Vec<String> = vec![];
    for body_el in document.select(&body_sel) {
        let mut text = body_el.text().collect::<Vec<_>>().join(" ");
        for script_el in document.select(&script_sel) {
            let script_text = script_el.text().collect::<Vec<_>>().join(" ");
            text = text.replace(&script_text, " ");
        }
        result_parts.push(text);
    }

    clean_whitespace(&result_parts.join("\n"))
}

fn clean_whitespace(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Firecrawl ─────────────────────────────────────────────────────────────────

async fn firecrawl_fetch(url: &str, api_key: &str) -> Result<String, ToolError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_fetch".to_string(),
            reason: e.to_string(),
        })?;

    let body = json!({
        "url": url,
        "formats": ["markdown"]
    });

    let resp = client
        .post("https://api.firecrawl.dev/v1/scrape")
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "web_fetch".to_string(),
            reason: format!("Firecrawl request failed: {e}"),
        })?;

    let data: Value = resp.json().await.map_err(|e| ToolError::ExecutionFailed {
        tool: "web_fetch".to_string(),
        reason: format!("Failed to parse Firecrawl response: {e}"),
    })?;

    if let Some(md) = data["data"]["markdown"].as_str() {
        let truncated = if md.len() > MAX_RESPONSE_BYTES {
            format!("{}\n\n[Truncated at 512 KB]", &md[..MAX_RESPONSE_BYTES])
        } else {
            md.to_string()
        };
        return Ok(truncated);
    }

    if let Some(err) = data["error"].as_str() {
        return Ok(format!("Firecrawl error: {err}"));
    }

    Ok("Firecrawl returned no content.".to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_https() {
        let tool = WebFetchTool::new(None);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(tool.execute(json!({"url": "ftp://example.com/file"})))
            .unwrap_err();
        assert!(
            matches!(err, ToolError::InvalidArgs { ref reason, .. } if reason.contains("http")),
            "expected scheme error, got {err:?}"
        );
    }

    #[test]
    fn rejects_invalid_url() {
        let tool = WebFetchTool::new(None);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(tool.execute(json!({"url": "not-a-url"})))
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs { .. }));
    }

    #[test]
    fn plain_html_extraction() {
        let html = "<html><body><h1>Hello</h1><p>World</p><script>ignore()</script></body></html>";
        let text = extract_text(html, None, "text/html");
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
    }

    #[test]
    fn selector_extraction() {
        let html = "<html><body><nav>nav</nav><article>Article content</article></body></html>";
        let text = extract_text(html, Some("article"), "text/html");
        assert!(text.contains("Article content"));
        assert!(!text.contains("nav"));
    }

    #[test]
    fn truncation_logic_at_512kb() {
        let big = "x".repeat(MAX_RESPONSE_BYTES + 100);
        let big_bytes = big.as_bytes();
        assert!(big_bytes.len() > MAX_RESPONSE_BYTES);
        let truncated = String::from_utf8_lossy(&big_bytes[..MAX_RESPONSE_BYTES]).to_string();
        let output = format!("{truncated}\n\n[Response truncated at 512 KB]");
        assert!(output.contains("[Response truncated at 512 KB]"));
        assert_eq!(truncated.len(), MAX_RESPONSE_BYTES);
    }

    #[test]
    fn firecrawl_not_configured_by_default() {
        std::env::remove_var("FIRECRAWL_API_KEY");
        let tool = WebFetchTool::new(None);
        assert!(!tool.uses_firecrawl());
    }
}
