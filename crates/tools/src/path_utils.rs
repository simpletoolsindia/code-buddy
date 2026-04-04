//! Shared path utilities: normalization and CWD confinement enforcement.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use code_buddy_errors::ToolError;

/// Lexically normalize a path: collapse `.` and `..` without any I/O.
///
/// This is intentionally pure (no syscalls). Real symlink resolution is done
/// separately in [`resolve_within_cwd`] via `canonicalize`.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// Resolve `path` relative to `cwd` and enforce that the result is within `cwd`.
///
/// # Resolution strategy
///
/// 1. **Canonicalize `cwd`** — resolves any symlinks in the project root itself.
/// 2. **Lexically normalize** the target path (no I/O, resolves `.` / `..`).
/// 3. **Lexical confinement check** — reject if the normalized path escapes `cwd`.
///    This catches `..` traversal before any disk access.
/// 4. **Ancestor canonicalization** — walk up from the normalized path to find the
///    longest prefix that currently exists on disk.  Canonicalize that prefix
///    (resolving all symlinks, including in intermediate directories) and re-check
///    that it is still within `cwd`.
///
///    This step is critical for **new files inside symlinked directories**:
///    `cwd/link/newfile.txt` passes the lexical check but `cwd/link` may be a
///    symlink pointing outside `cwd`.  Canonicalizing the *parent* `cwd/link`
///    reveals the escape even though the target file does not yet exist.
///
/// 5. **Reconstruct the final path** from the canonicalized prefix + any
///    non-existent suffix components, ensuring the returned path is fully
///    anchored to the real CWD.
///
/// # Errors
/// - [`ToolError::ExecutionFailed`] if `cwd` cannot be canonicalized.
/// - [`ToolError::PathTraversal`] if the resolved path escapes `cwd` at any step.
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

    // Step 4 & 5: find the nearest existing ancestor, canonicalize it to
    // resolve all symlinks (including in parent directories), then re-verify
    // it is still within `canon_cwd`.
    //
    // This closes the "symlinked parent + non-existent target" escape:
    //   cwd/link (→ /outside) + new.txt  →  /outside/new.txt
    // The file doesn't exist but the *parent* does, and its canonical form
    // reveals the escape.
    let (canon_prefix, suffix) =
        existing_ancestor_canonical(&normalized).map_err(|e| ToolError::ExecutionFailed {
            tool: tool.to_string(),
            reason: format!("cannot canonicalize ancestor of '{path}': {e}"),
        })?;

    if !canon_prefix.starts_with(&canon_cwd) {
        return Err(ToolError::PathTraversal {
            tool: tool.to_string(),
            path: path.to_string(),
        });
    }

    // Rebuild: canon_prefix + non-existent suffix components.
    let result = suffix
        .iter()
        .fold(canon_prefix, |acc, component| acc.join(component));
    Ok(result)
}

/// Walk up `path` to find the deepest prefix that currently exists on disk.
///
/// Returns `(canonicalized_prefix, remaining_suffix_components)` where
/// `remaining_suffix_components` are the path components (in order) that were
/// stripped off because they do not yet exist on disk.
///
/// # Example
/// If `/project/src` exists but `/project/src/new/file.rs` does not:
/// ```text
/// existing_ancestor_canonical("/project/src/new/file.rs")
///   -> (canonical("/project/src"), ["new", "file.rs"])
/// ```
fn existing_ancestor_canonical(
    path: &Path,
) -> std::io::Result<(PathBuf, Vec<OsString>)> {
    let mut suffix: Vec<OsString> = Vec::new();
    let mut current: &Path = path;

    loop {
        if current.exists() {
            let canon = current.canonicalize()?;
            // Suffix was accumulated in reverse order (deepest first); reverse it.
            suffix.reverse();
            return Ok((canon, suffix));
        }

        match current.parent() {
            Some(parent) if parent != current => {
                // Record the component we're stripping off.
                if let Some(name) = current.file_name() {
                    suffix.push(name.to_os_string());
                }
                current = parent;
            }
            _ => {
                // Reached filesystem root without finding an existing component.
                // Return the original path unchanged (the lexical check already
                // verified it starts with cwd, so this is safe).
                suffix.reverse();
                return Ok((path.to_path_buf(), suffix));
            }
        }
    }
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
        assert!(result.starts_with(dir.path().canonicalize().unwrap()));
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

    /// A symlink inside `cwd` that points outside must be rejected —
    /// even when the TARGET file does not yet exist.
    ///
    /// This is the bug the reviewer caught: `cwd/link/new.txt` where
    /// `cwd/link -> /outside` used to pass because `new.txt` doesn't exist.
    #[cfg(unix)]
    #[test]
    fn symlink_dir_escape_on_new_file_is_rejected() {
        let dir = tmp();
        let outside = tmp(); // simulates /outside
        // Create cwd/link -> outside_dir
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();

        // Target file does NOT exist yet — previously bypassed the check.
        let err = resolve_within_cwd("test", dir.path(), "link/newfile.txt").unwrap_err();
        assert!(
            matches!(err, ToolError::PathTraversal { .. }),
            "expected PathTraversal for symlinked parent + non-existent file, got {err:?}"
        );
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

    /// A new file inside a valid (non-symlink) subdirectory should be allowed,
    /// even when the file itself doesn't exist yet.
    #[test]
    fn new_file_in_existing_dir_is_allowed() {
        let dir = tmp();
        let subdir = dir.path().join("src");
        std::fs::create_dir(&subdir).unwrap();

        let result = resolve_within_cwd("test", dir.path(), "src/new_file.rs").unwrap();
        assert!(
            result.ends_with("src/new_file.rs"),
            "expected valid path for new file in existing dir, got {result:?}"
        );
    }

    /// A new file in a completely non-existent subdirectory is allowed as long
    /// as the path stays within cwd (write_file may create the directory).
    #[test]
    fn new_file_in_nonexistent_dir_is_allowed() {
        let dir = tmp();
        let result =
            resolve_within_cwd("test", dir.path(), "does/not/exist/file.rs").unwrap();
        assert!(
            result.ends_with("does/not/exist/file.rs"),
            "expected valid path for new nested file, got {result:?}"
        );
    }
}
