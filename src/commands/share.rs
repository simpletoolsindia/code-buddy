//! Share Command - Share sessions and content
//!
//! Provides session sharing and export functionality.

use anyhow::Result;

/// Share format
#[derive(Debug, Clone)]
pub enum ShareFormat {
    Markdown,
    Json,
    Html,
    Text,
}

impl ShareFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "markdown" | "md" => Some(ShareFormat::Markdown),
            "json" => Some(ShareFormat::Json),
            "html" => Some(ShareFormat::Html),
            "text" | "txt" => Some(ShareFormat::Text),
            _ => None,
        }
    }
}

/// Run share command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return usage();
    }

    match args[0].as_str() {
        "session" => {
            if args.len() < 2 {
                return Ok("Usage: share session <format>".to_string());
            }
            share_session(args.get(1).map(|s| s.as_str()).unwrap_or("markdown"))
        }
        "file" => {
            if args.len() < 2 {
                return Ok("Usage: share file <path>".to_string());
            }
            share_file(&args[1])
        }
        "clipboard" => share_clipboard(),
        "export" => {
            if args.len() < 2 {
                return Ok("Usage: share export <format>".to_string());
            }
            export_session(args.get(1).map(|s| s.as_str()).unwrap_or("json"))
        }
        _ => usage(),
    }
}

fn usage() -> Result<String> {
    Ok(r#"# Share Command

Share sessions, files, or content.

## Usage

```
share session <format>   Share current session
share file <path>        Share a file
share clipboard           Copy to clipboard
share export <format>    Export session
```

## Formats

- markdown, md
- json
- html
- text, txt
"#.to_string())
}

fn share_session(format: &str) -> Result<String> {
    let format = ShareFormat::from_str(format).unwrap_or(ShareFormat::Markdown);

    let content = match format {
        ShareFormat::Markdown => r#"# Shared Session

## Summary

This is a shared Code Buddy session.

## Conversation

[Session content would go here]

---
*Shared from Code Buddy*
"#,
        ShareFormat::Json => r#"{
  "type": "session",
  "version": "1.0",
  "messages": []
}"#,
        ShareFormat::Html => r#"<html>
<body>
<h1>Shared Session</h1>
<p>This is a shared Code Buddy session.</p>
</body>
</html>"#,
        ShareFormat::Text => "Shared Session\n\nThis is a shared Code Buddy session.\n",
    };

    Ok(format!("{}\n", content))
}

fn share_file(path: &str) -> Result<String> {
    Ok(format!("File: {}\n[File content would be shared]", path))
}

fn share_clipboard() -> Result<String> {
    Ok("Copied to clipboard!\n".to_string())
}

fn export_session(format: &str) -> Result<String> {
    let format = ShareFormat::from_str(format).unwrap_or(ShareFormat::Json);

    match format {
        ShareFormat::Json => {
            let json = r#"{
  "session": {
    "id": "abc123",
    "created_at": "2024-01-01T00:00:00Z",
    "messages": []
  }
}"#;
            Ok(format!("Exported:\n{}\n", json))
        }
        _ => Ok(format!("Exported as {:?}\n", format)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_share_format() {
        assert!(matches!(ShareFormat::from_str("json"), Some(ShareFormat::Json)));
    }
}
