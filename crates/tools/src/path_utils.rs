//! Shared path-safety utilities used by all file and search tools.

use std::path::{Component, Path, PathBuf};

use code_buddy_errors::ToolError;

/// Normalize a path by resolving `.` and `..` components **without** touching the
/// filesystem. This allows the function to work on paths that do not yet exist.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}

/// Resolve `path` relative to `cwd` and enforce that the result is within `cwd`.
///
/// - Relative paths are joined onto `cwd`.
/// - Absolute paths are accepted only if they start with the canonical `cwd`.
/// - `..` components that escape `cwd` are rejected as traversal.
///
/// Returns the fully-resolved (normalized) absolute path on success.
///
/// # Errors
/// - [`ToolError::ExecutionFailed`] if `cwd` cannot be canonicalized.
/// - [`ToolError::PathTraversal`] if the resolved path falls outside `cwd`.
pub(crate) fn resolve_within_cwd(
    tool: &str,
    cwd: &Path,
    path: &str,
) -> Result<PathBuf, ToolError> {
    let canon_cwd = cwd.canonicalize().map_err(|e| ToolError::ExecutionFailed {
        tool: tool.to_string(),
        reason: format!("cannot resolve cwd: {e}"),
    })?;

    let joined = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        canon_cwd.join(path)
    };

    let normalized = normalize_path(&joined);

    if !normalized.starts_with(&canon_cwd) {
        return Err(ToolError::PathTraversal {
            tool: tool.to_string(),
            path: path.to_string(),
        });
    }

    Ok(normalized)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn relative_path_resolves_within_cwd() {
        let dir = tmp();
        let result = resolve_within_cwd("test", dir.path(), "sub/file.rs").unwrap();
        assert!(result.starts_with(dir.path()));
        assert!(result.ends_with("sub/file.rs"));
    }

    #[test]
    fn traversal_via_dotdot_is_rejected() {
        let dir = tmp();
        let err = resolve_within_cwd("test", dir.path(), "../../etc/passwd").unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn absolute_path_within_cwd_is_allowed() {
        let dir = tmp();
        let inner = dir.path().join("allowed.txt");
        let result = resolve_within_cwd("test", dir.path(), inner.to_str().unwrap()).unwrap();
        assert_eq!(result, inner);
    }

    #[test]
    fn absolute_path_outside_cwd_is_rejected() {
        let dir = tmp();
        let err = resolve_within_cwd("test", dir.path(), "/etc/passwd").unwrap_err();
        assert!(matches!(err, ToolError::PathTraversal { .. }));
    }

    #[test]
    fn dot_components_are_normalized() {
        let dir = tmp();
        let result = resolve_within_cwd("test", dir.path(), "a/./b").unwrap();
        assert!(result.ends_with("a/b"));
    }
}
