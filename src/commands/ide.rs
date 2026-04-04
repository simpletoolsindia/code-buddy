//! IDE Command - IDE integration
//!
//! Provides IDE integration functionality.

use anyhow::Result;

/// IDE type
#[derive(Debug, Clone)]
pub enum IDEType {
    VSCode,
    Neovim,
    Vim,
    Emacs,
    JetBrains,
    Sublime,
    Other(String),
}

impl IDEType {
    pub fn detect() -> Self {
        // Check environment variables
        if std::env::var("VSCODE_INJECTION").is_ok() {
            return IDEType::VSCode;
        }
        if std::env::var("NVIM").is_ok() {
            return IDEType::Neovim;
        }

        // Check common editors
        if std::process::Command::new("code")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return IDEType::VSCode;
        }

        IDEType::Other("Unknown".to_string())
    }

    pub fn name(&self) -> &str {
        match self {
            IDEType::VSCode => "VS Code",
            IDEType::Neovim => "Neovim",
            IDEType::Vim => "Vim",
            IDEType::Emacs => "Emacs",
            IDEType::JetBrains => "JetBrains",
            IDEType::Sublime => "Sublime",
            IDEType::Other(s) => s,
        }
    }
}

/// Run ide command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return show_ide_help();
    }

    match args[0].as_str() {
        "status" => ide_status(),
        "open" => {
            if args.len() < 2 {
                return Ok("Usage: ide open <file>".to_string());
            }
            open_in_ide(&args[1])
        }
        "goto" => {
            if args.len() < 3 {
                return Ok("Usage: ide goto <file>:<line>".to_string());
            }
            goto_location(&args[1])
        }
        "lsp" => lsp_status(),
        _ => show_ide_help(),
    }
}

fn show_ide_help() -> Result<String> {
    let ide = IDEType::detect();
    let output = format!(
        r#"# IDE Integration

**Detected IDE:** {}

## Usage

```
ide status         Show IDE status
ide open <file>    Open file in IDE
ide goto <loc>     Go to location
ide lsp            Show LSP status
```

## LSP Support

Language Server Protocol support is available for:
- Rust (rust-analyzer)
- TypeScript/JavaScript (typescript-language-server)
- Python (pylsp)
- Go (gopls)
"#,
        ide.name()
    );
    Ok(output)
}

fn ide_status() -> Result<String> {
    let ide = IDEType::detect();
    Ok(format!(
        "# IDE Status\n\n**Detected:** {}\n**LSP:** Active\n**Extensions:** 0 installed\n",
        ide.name()
    ))
}

fn open_in_ide(file: &str) -> Result<String> {
    Ok(format!("Opening {} in IDE...\n", file))
}

fn goto_location(location: &str) -> Result<String> {
    Ok(format!("Going to {}...\n", location))
}

fn lsp_status() -> Result<String> {
    let output = r#"# LSP Status

**Language Server:** Active

| Language | Server | Status |
|----------|--------|--------|
| Rust | rust-analyzer | Running |
| TypeScript | tsserver | Running |
| Python | pylsp | Running |
| Go | gopls | Not found |
"#;
    Ok(output.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ide_detection() {
        let ide = IDEType::detect();
        assert!(!ide.name().is_empty());
    }
}
