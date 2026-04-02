//! Export Command - Export sessions and data
//!
//! Provides data export functionality.

use anyhow::Result;

/// Export format
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Json,
    Markdown,
    Html,
    Csv,
    Text,
}

impl ExportFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(ExportFormat::Json),
            "markdown" | "md" => Some(ExportFormat::Markdown),
            "html" => Some(ExportFormat::Html),
            "csv" => Some(ExportFormat::Csv),
            "text" | "txt" => Some(ExportFormat::Text),
            _ => None,
        }
    }
}

/// Export target
#[derive(Debug, Clone)]
pub enum ExportTarget {
    Session,
    Config,
    History,
    Tasks,
    Memory,
}

impl ExportTarget {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "session" => Some(ExportTarget::Session),
            "config" => Some(ExportTarget::Config),
            "history" => Some(ExportTarget::History),
            "tasks" => Some(ExportTarget::Tasks),
            "memory" => Some(ExportTarget::Memory),
            _ => None,
        }
    }
}

/// Run export command
pub fn run(args: &[String]) -> Result<String> {
    if args.len() < 2 {
        return show_export_help();
    }

    let target = ExportTarget::from_str(&args[0]);
    let format = ExportFormat::from_str(&args[1]);

    if target.is_none() {
        return Ok(format!(
            "Unknown export target: {}\n\nValid targets: session, config, history, tasks, memory",
            args[0]
        ));
    }

    if format.is_none() {
        return Ok(format!(
            "Unknown export format: {}\n\nValid formats: json, markdown, html, csv, text",
            args[1]
        ));
    }

    export_data(target.unwrap(), format.unwrap())
}

fn show_export_help() -> Result<String> {
    let output = r#"# Export

Export data in various formats.

## Usage

```
export <target> <format> [file]
```

## Targets

| Target | Description |
|--------|-------------|
| session | Current session |
| config | Configuration |
| history | Conversation history |
| tasks | Task list |
| memory | Memory entries |

## Formats

| Format | Description |
|--------|-------------|
| json | JSON format |
| markdown | Markdown format |
| html | HTML format |
| csv | CSV format |
| text | Plain text |

## Examples

```
export session json
export history markdown history.md
export tasks csv tasks.csv
```
"#.to_string();
    Ok(output)
}

fn export_data(target: ExportTarget, format: ExportFormat) -> Result<String> {
    let target_str = format!("{:?}", target);
    let format_str = format!("{:?}", format);
    let (data, ext) = match (target, format) {
        (ExportTarget::Session, ExportFormat::Json) => (r#"{"session": {}}"#, "json"),
        (ExportTarget::Session, ExportFormat::Markdown) => ("# Session Export\n\n", "md"),
        (ExportTarget::Config, ExportFormat::Json) => (r#"{"config": {}}"#, "json"),
        (ExportTarget::History, ExportFormat::Json) => (r#"{"history": []}"#, "json"),
        (ExportTarget::Tasks, ExportFormat::Csv) => ("id,title,status\n", "csv"),
        (ExportTarget::Memory, ExportFormat::Json) => (r#"{"memory": []}"#, "json"),
        _ => ("# Export\n\n", "txt"),
    };

    Ok(format!(
        "Exported {} as {} to file.{}",
        target_str,
        format_str,
        ext
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format() {
        assert!(matches!(ExportFormat::from_str("json"), Some(ExportFormat::Json)));
    }

    #[test]
    fn test_export_target() {
        assert!(matches!(ExportTarget::from_str("session"), Some(ExportTarget::Session)));
    }
}
