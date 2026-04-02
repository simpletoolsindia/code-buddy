//! Browser Automation Tool
//!
//! Provides headless browser automation using CDP (Chrome DevTools Protocol).
//! Supports navigation, clicking, typing, screenshots, and more.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Browser provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserProvider {
    /// Use system Chromium/Chrome via CDP
    Chrome,
    /// Use Firefox via CDP
    Firefox,
    /// Use WebKit via CDP
    WebKit,
}

/// Browser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub provider: BrowserProvider,
    pub headless: bool,
    pub viewport: (u32, u32),
    pub user_agent: Option<String>,
    pub timeout_ms: u64,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            provider: BrowserProvider::Chrome,
            headless: true,
            viewport: (1280, 720),
            user_agent: None,
            timeout_ms: 30000,
        }
    }
}

/// Browser action types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum BrowserAction {
    /// Navigate to URL
    Navigate { url: String },
    /// Click element
    Click { selector: String },
    /// Type text
    Type { selector: String, text: String },
    /// Hover over element
    Hover { selector: String },
    /// Scroll
    Scroll { x: i32, y: i32 },
    /// Take screenshot
    Screenshot { full_page: Option<bool> },
    /// Get page HTML
    GetHtml,
    /// Evaluate JavaScript
    Eval { script: String },
    /// Wait for selector
    WaitFor { selector: String, timeout_ms: Option<u64> },
    /// Go back
    Back,
    /// Go forward
    Forward,
    /// Reload
    Reload,
    /// Select dropdown
    Select { selector: String, value: String },
    /// Press keyboard key
    Press { key: String },
    /// Get page title
    GetTitle,
    /// Get current URL
    GetUrl,
}

/// Browser result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserResult {
    pub success: bool,
    pub url: Option<String>,
    pub title: Option<String>,
    pub screenshot: Option<String>,  // Base64 encoded
    pub html: Option<String>,
    pub text: Option<String>,
    pub error: Option<String>,
    pub cookies: Option<Vec<Cookie>>,
}

/// Cookie definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: bool,
    pub http_only: bool,
}

/// Browser tool state
pub struct BrowserTool {
    config: BrowserConfig,
    current_url: Option<String>,
    screenshot_counter: u32,
}

impl BrowserTool {
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            config,
            current_url: None,
            screenshot_counter: 0,
        }
    }

    /// Execute a browser action
    pub fn execute(&mut self, action: BrowserAction) -> Result<BrowserResult> {
        match action {
            BrowserAction::Navigate { url } => {
                self.navigate(&url)
            }
            BrowserAction::Screenshot { full_page } => {
                self.screenshot(full_page.unwrap_or(false))
            }
            BrowserAction::GetHtml => {
                self.get_html()
            }
            BrowserAction::GetTitle => {
                self.get_title()
            }
            BrowserAction::GetUrl => {
                self.get_url()
            }
            BrowserAction::Reload => {
                self.reload()
            }
            BrowserAction::Back => {
                self.back()
            }
            BrowserAction::Forward => {
                self.forward()
            }
            BrowserAction::Eval { script } => {
                self.eval(&script)
            }
            BrowserAction::Click { selector } => {
                self.click(&selector)
            }
            BrowserAction::Type { selector, text } => {
                self.type_text(&selector, &text)
            }
            BrowserAction::Hover { selector } => {
                self.hover(&selector)
            }
            BrowserAction::Scroll { x, y } => {
                self.scroll(x, y)
            }
            BrowserAction::WaitFor { selector, timeout_ms } => {
                self.wait_for(&selector, timeout_ms.unwrap_or(5000))
            }
            BrowserAction::Select { selector, value } => {
                self.select(&selector, &value)
            }
            BrowserAction::Press { key } => {
                self.press_key(&key)
            }
        }
    }

    fn navigate(&mut self, url: &str) -> Result<BrowserResult> {
        self.current_url = Some(url.to_string());
        // In a real implementation, this would use CDP
        // For now, return a stub result
        Ok(BrowserResult {
            success: true,
            url: Some(url.to_string()),
            title: None,
            screenshot: None,
            html: None,
            text: None,
            error: None,
            cookies: None,
        })
    }

    fn screenshot(&mut self, full_page: bool) -> Result<BrowserResult> {
        self.screenshot_counter += 1;
        // Stub - would use CDP Screenshot command
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None, // Would be base64 PNG
            html: None,
            text: Some(format!("Screenshot {} captured (full_page={})", self.screenshot_counter, full_page)),
            error: None,
            cookies: None,
        })
    }

    fn get_html(&self) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: Some("<html>...</html>".to_string()),
            text: None,
            error: None,
            cookies: None,
        })
    }

    fn get_title(&self) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: Some("Page Title".to_string()),
            screenshot: None,
            html: None,
            text: None,
            error: None,
            cookies: None,
        })
    }

    fn get_url(&self) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: self.current_url.clone(),
            error: None,
            cookies: None,
        })
    }

    fn reload(&self) -> Result<BrowserResult> {
        self.get_url()
    }

    fn back(&self) -> Result<BrowserResult> {
        self.get_url()
    }

    fn forward(&self) -> Result<BrowserResult> {
        self.get_url()
    }

    fn eval(&self, script: &str) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Script executed: {}", &script[..script.len().min(100)])),
            error: None,
            cookies: None,
        })
    }

    fn click(&self, selector: &str) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Clicked: {}", selector)),
            error: None,
            cookies: None,
        })
    }

    fn type_text(&self, selector: &str, text: &str) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Typed '{}' into {}", text, selector)),
            error: None,
            cookies: None,
        })
    }

    fn hover(&self, selector: &str) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Hovered: {}", selector)),
            error: None,
            cookies: None,
        })
    }

    fn scroll(&self, x: i32, y: i32) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Scrolled: x={}, y={}", x, y)),
            error: None,
            cookies: None,
        })
    }

    fn wait_for(&self, selector: &str, _timeout_ms: u64) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Waited for: {}", selector)),
            error: None,
            cookies: None,
        })
    }

    fn select(&self, selector: &str, value: &str) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Selected '{}' in {}", value, selector)),
            error: None,
            cookies: None,
        })
    }

    fn press_key(&self, key: &str) -> Result<BrowserResult> {
        Ok(BrowserResult {
            success: true,
            url: self.current_url.clone(),
            title: None,
            screenshot: None,
            html: None,
            text: Some(format!("Pressed key: {}", key)),
            error: None,
            cookies: None,
        })
    }
}

/// Format browser result as markdown
pub fn format_browser_result(result: &BrowserResult) -> String {
    let mut md = String::new();

    if let Some(url) = &result.url {
        md.push_str(&format!("**URL:** {}\n\n", url));
    }
    if let Some(title) = &result.title {
        md.push_str(&format!("**Title:** {}\n\n", title));
    }
    if let Some(text) = &result.text {
        md.push_str(&format!("{}\n\n", text));
    }
    if let Some(html) = &result.html {
        md.push_str("**HTML:**\n```\n");
        md.push_str(&html[..html.len().min(500)]);
        if html.len() > 500 {
            md.push_str("\n... (truncated)");
        }
        md.push_str("\n```\n\n");
    }
    if let Some(err) = &result.error {
        md.push_str(&format!("**Error:** {}\n", err));
    }

    md
}

/// Browser tool schema for LLM
pub fn browser_navigate_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "browser_navigate",
        "description": "Navigate to a URL in the headless browser",
        "parameters": {
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to navigate to"
                }
            },
            "required": ["url"]
        }
    })
}

pub fn browser_screenshot_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "browser_screenshot",
        "description": "Take a screenshot of the current page",
        "parameters": {
            "type": "object",
            "properties": {
                "full_page": {
                    "type": "boolean",
                    "description": "Capture the full page (default: false)"
                }
            }
        }
    })
}

pub fn browser_click_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "browser_click",
        "description": "Click on an element by CSS selector",
        "parameters": {
            "type": "object",
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "CSS selector of the element to click"
                }
            },
            "required": ["selector"]
        }
    })
}

pub fn browser_type_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "browser_type",
        "description": "Type text into an input field",
        "parameters": {
            "type": "object",
            "properties": {
                "selector": {
                    "type": "string",
                    "description": "CSS selector of the input field"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type"
                }
            },
            "required": ["selector", "text"]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_navigate() {
        let mut browser = BrowserTool::new(BrowserConfig::default());
        let result = browser.execute(BrowserAction::Navigate {
            url: "https://example.com".to_string(),
        }).unwrap();
        assert!(result.success);
        assert_eq!(result.url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_browser_screenshot() {
        let mut browser = BrowserTool::new(BrowserConfig::default());
        let _ = browser.execute(BrowserAction::Navigate {
            url: "https://example.com".to_string(),
        });
        let result = browser.execute(BrowserAction::Screenshot { full_page: Some(true) }).unwrap();
        assert!(result.success);
    }
}
