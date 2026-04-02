//! Files Command - File listing and information
//!
//! Provides file listing and information.

use anyhow::Result;
use std::path::PathBuf;

/// Run files command
pub fn run(args: &[String]) -> Result<String> {
    if args.is_empty() {
        return list_files(".");
    }

    match args[0].as_str() {
        "list" | "ls" => {
            let path = args.get(1).map(|s| s.as_str()).unwrap_or(".");
            list_files(path)
        }
        "info" | "stat" => {
            if args.len() < 2 {
                return Ok("Usage: files info <path>".to_string());
            }
            file_info(&args[1])
        }
        "size" => {
            if args.len() < 2 {
                return Ok("Usage: files size <path>".to_string());
            }
            file_size(&args[1])
        }
        "find" => {
            if args.len() < 2 {
                return Ok("Usage: files find <pattern>".to_string());
            }
            find_files(&args[1])
        }
        _ => {
            list_files(&args[0])
        }
    }
}

fn list_files(path: &str) -> Result<String> {
    let mut output = format!("# Files in {}\n\n", path);

    let entries = std::fs::read_dir(path)?;
    for entry in entries.take(50) {
        if let Ok(entry) = entry {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                output.push_str(&format!("{}/\n", name));
            } else {
                output.push_str(&format!("{}\n", name));
            }
        }
    }

    Ok(output)
}

fn file_info(path: &str) -> Result<String> {
    let metadata = std::fs::metadata(path)?;
    let mut output = format!("# File Info: {}\n\n", path);
    output.push_str(&format!("Size: {} bytes\n", metadata.len()));
    output.push_str(&format!("Is file: {}\n", metadata.is_file()));
    output.push_str(&format!("Is dir: {}\n", metadata.is_dir()));

    if let Ok(modified) = metadata.modified() {
        output.push_str(&format!("Modified: {:?}\n", modified));
    }

    Ok(output)
}

fn file_size(path: &str) -> Result<String> {
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();
    let size_str = if size < 1024 {
        format!("{} B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    };

    Ok(format!("{}: {}\n", path, size_str))
}

fn find_files(pattern: &str) -> Result<String> {
    let mut output = format!("# Find: {}\n\n", pattern);
    output.push_str("Files matching pattern:\n\n");

    // Simple pattern matching
    for entry in walkdir::WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.contains(pattern) {
                output.push_str(&format!("{}\n", path.display()));
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert!(true);
    }
}
