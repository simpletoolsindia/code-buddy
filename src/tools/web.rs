//! Web Tools - WebFetchTool, WebSearchTool
//!
//! Provides web fetching and searching capabilities.

use anyhow::{anyhow, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strip HTML tags from text (simple html2text replacement)
fn strip_html(html: &str) -> String {
    Regex::new(r"<[^>]+>").unwrap().replace_all(html, "").to_string()
}

/// Web fetch request
#[derive(Debug, Clone)]
pub struct WebFetchRequest {
    pub url: String,
    pub prompt: Option<String>,
}

/// Web fetch result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchResult {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub status_code: u16,
    pub headers: HashMap<String, String>,
}

impl WebFetchResult {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            title: None,
            content: String::new(),
            status_code: 200,
            headers: HashMap::new(),
        }
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_status(mut self, code: u16) -> Self {
        self.status_code = code;
        self
    }
}

/// Web search request
#[derive(Debug, Clone)]
pub struct WebSearchRequest {
    pub query: String,
    pub limit: Option<usize>,
    pub safe_search: bool,
}

impl WebSearchRequest {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            limit: Some(10),
            safe_search: true,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn disable_safe_search(mut self) -> Self {
        self.safe_search = false;
        self
    }
}

/// Web search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: Option<String>,
}

impl WebSearchResult {
    pub fn new(title: &str, url: &str, snippet: &str) -> Self {
        Self {
            title: title.to_string(),
            url: url.to_string(),
            snippet: snippet.to_string(),
            source: None,
        }
    }
}

/// Web search response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResponse {
    pub query: String,
    pub results: Vec<WebSearchResult>,
    pub total: usize,
}

impl WebSearchResponse {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            results: Vec::new(),
            total: 0,
        }
    }

    pub fn add_result(&mut self, result: WebSearchResult) {
        self.total += 1;
        self.results.push(result);
    }
}

/// Fetch web content
pub async fn fetch_web(url: &str) -> Result<WebFetchResult> {
    let client = reqwest::Client::builder()
        .user_agent("Code-Buddy/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send().await?;

    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = response.text().await?;

    // Try to extract title
    let title = extract_title(&body);

    Ok(WebFetchResult::new(url)
        .with_content(&body)
        .with_title(&title)
        .with_status(status)
    )
}

/// Search the web
pub async fn search_web(query: &str, limit: usize) -> Result<WebSearchResponse> {
    let client = reqwest::Client::builder()
        .user_agent("Code-Buddy/1.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let url = format!(
        "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
        urlencoding::encode(query)
    );

    let response = client.get(&url).send().await?;
    let body = response.text().await?;

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| anyhow!("Failed to parse search response: {}", e))?;

    let mut search_response = WebSearchResponse::new(query);

    // Extract related topics (these are like search results)
    if let Some(related) = json.get("RelatedTopics").and_then(|v| v.as_array()) {
        for item in related.iter().take(limit) {
            if let (Some(text), Some(result_url)) = (
                item.get("Text").and_then(|v| v.as_str()),
                item.get("URL").and_then(|v| v.as_str()),
            ) {
                let result = WebSearchResult::new(
                    text.split(" - ").next().unwrap_or(text),
                    result_url,
                    text,
                );
                search_response.add_result(result);
            }
        }
    }

    // Also add abstract if available
    if let Some(abstract_text) = json.get("AbstractText").and_then(|v| v.as_str()) {
        if !abstract_text.is_empty() {
            if let Some(abstract_url) = json.get("AbstractURL").and_then(|v| v.as_str()) {
                let result = WebSearchResult::new(
                    json.get("Heading").and_then(|v| v.as_str()).unwrap_or(query),
                    abstract_url,
                    abstract_text,
                );
                search_response.add_result(result);
            }
        }
    }

    Ok(search_response)
}

/// Extract title from HTML
fn extract_title(html: &str) -> String {
    let lower = html.to_lowercase();
    if let Some(start) = lower.find("<title>") {
        if let Some(end) = lower[start..].find("</title>") {
            let title = &html[start + 7..start + end];
            return strip_html(title).trim().to_string();
        }
    }
    if let Some(start) = lower.find("property=\"og:title\"") {
        if let Some(content_start) = lower[start..].find("content=\"") {
            let content = &lower[start + content_start + 9..];
            if let Some(end) = content.find('"') {
                return strip_html(&content[..end]).trim().to_string();
            }
        }
    }
    String::new()
}

/// Format search results as markdown
pub fn format_search_results(response: &WebSearchResponse) -> String {
    let mut md = format!("# Web Search: {}\n\n", response.query);

    if response.results.is_empty() {
        md.push_str("No results found.\n");
    } else {
        md.push_str(&format!("Found {} results:\n\n", response.total));
        for (i, result) in response.results.iter().enumerate() {
            md.push_str(&format!(
                "{}. **{}**\n   {}\n   {}\n\n",
                i + 1,
                result.title,
                result.snippet,
                result.url
            ));
        }
    }

    md
}

/// Format fetch result as markdown
pub fn format_fetch_result(result: &WebFetchResult, prompt: Option<&str>) -> String {
    let mut md = format!("# Web Fetch: {}\n\n", result.url);

    if let Some(ref title) = result.title {
        md.push_str(&format!("**Title:** {}\n\n", title));
    }

    md.push_str(&format!("**Status:** {}\n\n", result.status_code));

    if let Some(ref p) = prompt {
        md.push_str(&format!("**Prompt:** {}\n\n", p));
    }

    // Truncate content if too long
    let content = if result.content.len() > 10000 {
        format!("{}...\n\n[Content truncated - {} chars total]",
            &result.content[..10000], result.content.len())
    } else {
        result.content.clone()
    };

    // Convert HTML to text
    let text = strip_html(&content);
    md.push_str(&format!("## Content\n\n{}\n", text));

    md
}

// ============================================================================
// Tool Implementations (Sync versions for compatibility)
// ============================================================================

use super::Tool;

/// Web search tool (sync version)
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> &str {
        "Search the web for information"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Usage: WebSearch <query>".to_string());
        }
        let query = &args[0];
        Ok(format!(
            "WebSearch requires async context. Use: search_web(\"{}\", 10).await",
            query
        ))
    }
}

/// Web fetch tool (sync version)
pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        "Fetch web page content"
    }

    fn execute(&self, args: &[String]) -> Result<String> {
        if args.is_empty() {
            return Ok("Usage: WebFetch <url>".to_string());
        }
        let url = &args[0];
        Ok(format!(
            "WebFetch requires async context. Use: fetch_web(\"{}\").await",
            url
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_fetch_result() {
        let result = WebFetchResult::new("https://example.com")
            .with_title("Example")
            .with_content("Hello World")
            .with_status(200);
        assert_eq!(result.title, Some("Example".to_string()));
        assert_eq!(result.status_code, 200);
    }

    #[test]
    fn test_web_search_request() {
        let req = WebSearchRequest::new("rust programming");
        assert_eq!(req.query, "rust programming");
        assert!(req.safe_search);
    }

    #[test]
    fn test_web_search_tool() {
        let tool = WebSearchTool::new();
        assert_eq!(tool.name(), "WebSearch");
    }

    #[test]
    fn test_web_fetch_tool() {
        let tool = WebFetchTool::new();
        assert_eq!(tool.name(), "WebFetch");
    }
}
