//! Web tools - Search and fetch web content

use anyhow::Result;
use std::time::Duration;

/// Web search tool
pub struct WebSearch;

impl WebSearch {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Tool for WebSearch {
    fn name(&self) -> &str {
        "WebSearch"
    }

    fn description(&self) -> &str {
        "Search the web (requires async context)"
    }

    fn execute(&self, _args: &[String]) -> Result<String> {
        Ok("WebSearch requires async context. Use the CLI directly.".to_string())
    }
}

/// Web fetch tool
pub struct WebFetch;

impl WebFetch {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WebFetch {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Tool for WebFetch {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        "Fetch web page content (requires async context)"
    }

    fn execute(&self, _args: &[String]) -> Result<String> {
        Ok("WebFetch requires async context. Use the CLI directly.".to_string())
    }
}
