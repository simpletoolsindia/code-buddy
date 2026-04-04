//! Path utilities

use std::path::{Path, PathBuf};

/// Normalize a path, resolving . and ..
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            _ => components.push(part),
        }
    }
    components.into_iter().collect()
}

/// Check if path is within allowed directories
pub fn is_path_allowed(path: &Path, allowed_dirs: &[PathBuf]) -> bool {
    let normalized = normalize_path(path);

    for dir in allowed_dirs {
        let normalized_dir = normalize_path(dir);
        if normalized.starts_with(&normalized_dir) {
            return true;
        }
    }

    false
}

/// Find project root by looking for markers
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let markers = vec![
        ".git",
        "package.json",
        "Cargo.toml",
        "go.mod",
        "pyproject.toml",
        "requirements.txt",
        "Pipfile",
        "pom.xml",
        "build.gradle",
        ".project-root",
    ];

    let mut current = Some(start.to_path_buf());

    while let Some(path) = current {
        for marker in &markers {
            if path.join(marker).exists() {
                return Some(path);
            }
        }
        current = path.parent().map(|p| p.to_path_buf());
    }

    None
}
