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
/// Resolution strategy:
/// 1. Canonicalize `cwd` (resolves symlinks in the project root itself).
/// 2. Lexically normalize the target path (resolves `.` and `..` without I/O).
/// 3. **Lexical confinement check** — reject any path that escapes `cwd` after
///    normalization. This catches `..` traversal before any I/O occurs.
/// 4. **Symlink confinement check** — if the normalized path already exists on
///    disk, canonicalize it (resolves all symlinks) and verify the result is
///    still within `cwd`. This prevents a symlink inside `cwd` from pointing
///    outside and bypassing step 3.
/// 5. For non-existent paths (e.g. files about to be created), the lexical check
///    is sufficient — no symlinks can be followed for paths that do not yet exist.
///
/// Returns the fully-resolved path on success (canonicalized if it exists,
/// lexically normalized otherwise).
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

    // Step 3: lexical check — catches `..` traversal for all paths.
    let normalized = normalize_path(&joined);
    if !normalized.starts_with(&canon_cwd) {
        return Err(ToolError::PathTraversal {
            tool: tool.to_string(),
            path: path.to_string(),
        });
    }

    // Step 4: symlink check — only relevant for existing paths.
    if normalized.exists() {
        let canonical = normalized
            .canonicalize()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: tool.to_string(),
                reason: format!("cannot canonicalize path '{path}': {e}"),
            })?;
        if !canonical.starts_with(&canon_cwd) {
            return Err(ToolError::PathTraversal {
                tool: tool.to_string(),
                path: path.to_string(),
            });
        }
        return Ok(canonical);
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
        // The result may be canonicalized (symlinks resolved) — just check filename.
        assert!(
            result.ends_with("allowed.txt"),
            "expected path ending in allowed.txt, got {result:?}"
        );
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

    /// A symlink inside `cwd` that points outside must be rejected.
    ///
    /// This regression test ensures that `resolve_within_cwd` does NOT permit a
    /// symlink-based escape after the lexical check passes.
    #[cfg(unix)]
    #[test]
    fn symlink_pointing_outside_cwd_is_rejected() {
        let dir = tmp();
        let symlink_path = dir.path().join("escape");
        std::os::unix::fs::symlink("/etc", &symlink_path).unwrap();

        let err =
            resolve_within_cwd("test", dir.path(), "escape").unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for symlink outside cwd, got {err:?}"
        );
    }

    /// A symlink inside `cwd` pointing to another location inside `cwd` is allowed.
    #[cfg(unix)]
    #[test]
    fn symlink_within_cwd_is_allowed() {
        let dir = tmp();
        let target = dir.path().join("real.txt");
        std::fs::write(&target, "content").unwrap();
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let result = resolve_within_cwd("test", dir.path(), "link.txt").unwrap();
        // Should resolve to the real.txt target (canonicalized).
        assert!(
            result.ends_with("real.txt"),
            "expected canonical target, got {result:?}"
        );
    }
}
