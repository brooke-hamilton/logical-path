#![deny(missing_docs)]

//! Translate canonical (symlink-resolved) filesystem paths back to their
//! logical (symlink-preserving) equivalents.
//!
//! When a shell's current directory traverses a symlink (Unix) or an NTFS
//! junction, directory symlink, or subst drive (Windows), two different paths
//! refer to the same location: the **logical** path (preserving the
//! indirection) and the **canonical** path (with all indirections resolved).
//! This crate detects that mapping and provides bidirectional translation.
//!
//! # Quick Start
//!
//! ```no_run
//! use logical_path::LogicalPathContext;
//! use std::path::Path;
//!
//! let ctx = LogicalPathContext::detect();
//!
//! if ctx.has_mapping() {
//!     let canonical = Path::new("/mnt/wsl/workspace/project/src/main.rs");
//!     let logical = ctx.to_logical(canonical);
//!     println!("Logical: {}", logical.display());
//! }
//! ```
//!
//! # Platform Behavior
//!
//! - **Linux/macOS**: Reads `$PWD` and compares against `getcwd()` to detect
//!   symlink prefix mappings.
//! - **macOS**: System symlinks like `/var` → `/private/var` are handled
//!   automatically by the generic suffix-matching algorithm.
//! - **Windows**: Compares `current_dir()` (preserves junctions, subst drives)
//!   against `canonicalize()` (resolves to physical path) to detect NTFS
//!   junction, directory symlink, subst drive, and mapped drive mappings.
//!   The `\\?\` Extended Length Path prefix returned by `canonicalize()` is
//!   stripped before comparison.

#[cfg(not(windows))]
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// A context that holds zero or one active prefix mappings between
/// canonical (symlink-resolved) and logical (symlink-preserving) paths.
///
/// Created via [`LogicalPathContext::detect()`]. Immutable after construction.
///
/// # Thread Safety
///
/// `LogicalPathContext` is `Send + Sync` — it can be shared across threads.
///
/// # Platform Behavior
///
/// - **Linux/macOS**: Reads `$PWD` and compares against `getcwd()` to detect
///   symlink prefix mappings.
/// - **Windows**: Compares `current_dir()` (preserves junctions, subst drives,
///   mapped drives) against `canonicalize()` (resolves to physical path) to
///   detect NTFS junction, directory symlink, subst drive, and mapped drive
///   mappings. The `\\?\` prefix is stripped before comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalPathContext {
    mapping: Option<PrefixMapping>,
}

/// An internal type representing the divergence point between the logical
/// and canonical paths.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PrefixMapping {
    canonical_prefix: PathBuf,
    logical_prefix: PathBuf,
}

impl Default for LogicalPathContext {
    /// Returns a context with no active mapping.
    ///
    /// Equivalent to calling `detect()` in an environment with no symlinks
    /// in effect. All translations return their input unchanged.
    fn default() -> Self {
        LogicalPathContext { mapping: None }
    }
}

impl LogicalPathContext {
    /// Detect the active prefix mapping by comparing logical and canonical
    /// current working directory paths.
    ///
    /// - **Unix**: Compares `$PWD` (logical) against `getcwd()` (canonical).
    /// - **Windows**: Compares `current_dir()` (logical, preserves junctions/subst)
    ///   against `canonicalize(current_dir())` (canonical, physical path) with
    ///   `\\?\` prefix stripped.
    ///
    /// Returns a context with no active mapping when:
    /// - `$PWD` is unset (Unix)
    /// - The logical and canonical CWD are identical (no indirection in effect)
    /// - `$PWD` is stale (Unix: points to a non-existent directory)
    /// - The current directory cannot be determined
    ///
    /// # Panics
    ///
    /// This function never panics.
    #[must_use]
    pub fn detect() -> LogicalPathContext {
        #[cfg(windows)]
        {
            let cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    log::debug!("detect: current_dir() failed: {e}");
                    return LogicalPathContext { mapping: None };
                }
            };

            let canonical_cwd = match std::fs::canonicalize(&cwd) {
                Ok(c) => strip_extended_length_prefix(&c),
                Err(e) => {
                    log::debug!("detect: canonicalize({}) failed: {e}", cwd.display());
                    return LogicalPathContext { mapping: None };
                }
            };

            log::trace!(
                "detect (Windows): cwd={}, canonical_cwd={}",
                cwd.display(),
                canonical_cwd.display()
            );

            Self::detect_from_cwd(&cwd, &canonical_cwd)
        }

        #[cfg(not(windows))]
        {
            let pwd = std::env::var_os("PWD");
            let canonical_cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(e) => {
                    log::debug!("detect: current_dir() failed: {e}");
                    return LogicalPathContext { mapping: None };
                }
            };
            log::trace!(
                "detect (Unix): PWD={:?}, canonical_cwd={}",
                pwd,
                canonical_cwd.display()
            );
            Self::detect_from(pwd.as_deref(), &canonical_cwd)
        }
    }

    /// Internal helper for testability: takes `$PWD` and canonical CWD as
    /// parameters instead of reading from global process state.
    #[cfg(not(windows))]
    pub(crate) fn detect_from(pwd: Option<&OsStr>, canonical_cwd: &Path) -> LogicalPathContext {
        let pwd = match pwd {
            Some(p) if !p.is_empty() => Path::new(p),
            _ => {
                log::trace!("detect_from: PWD is unset or empty, no mapping");
                return LogicalPathContext { mapping: None };
            }
        };

        // If pwd and canonical CWD are identical, no mapping needed
        if pwd == canonical_cwd {
            log::trace!("detect_from: PWD == canonical CWD, no mapping");
            return LogicalPathContext { mapping: None };
        }

        // Validate that pwd resolves to canonical_cwd. This rejects stale $PWD
        // values (non-existent directories) and divergent $PWD assignments.
        match std::fs::canonicalize(pwd) {
            Ok(canonical_pwd) if canonical_pwd == canonical_cwd => {}
            _ => {
                log::trace!("detect_from: PWD validation failed (stale or divergent), no mapping");
                return LogicalPathContext { mapping: None };
            }
        }

        match find_divergence_point(canonical_cwd, pwd) {
            Some((canonical_prefix, logical_prefix)) => {
                log::debug!(
                    "detect_from: mapping detected: {} → {}",
                    canonical_prefix.display(),
                    logical_prefix.display()
                );
                LogicalPathContext {
                    mapping: Some(PrefixMapping {
                        canonical_prefix,
                        logical_prefix,
                    }),
                }
            }
            None => {
                log::trace!("detect_from: no divergence found");
                LogicalPathContext { mapping: None }
            }
        }
    }

    /// Internal helper for Windows testability: takes the CWD and its
    /// canonicalized form as parameters instead of reading from global
    /// process state.
    #[cfg(windows)]
    pub(crate) fn detect_from_cwd(cwd: &Path, canonical_cwd: &Path) -> LogicalPathContext {
        if cwd == canonical_cwd {
            log::trace!("detect_from_cwd: cwd == canonical_cwd, no mapping");
            return LogicalPathContext { mapping: None };
        }

        match find_divergence_point(canonical_cwd, cwd) {
            Some((canonical_prefix, logical_prefix)) => {
                log::debug!(
                    "detect_from_cwd: mapping detected: {} → {}",
                    canonical_prefix.display(),
                    logical_prefix.display()
                );
                LogicalPathContext {
                    mapping: Some(PrefixMapping {
                        canonical_prefix,
                        logical_prefix,
                    }),
                }
            }
            None => {
                log::trace!("detect_from_cwd: no divergence found");
                LogicalPathContext { mapping: None }
            }
        }
    }

    /// Returns `true` if an active prefix mapping was detected.
    ///
    /// When this returns `false`, [`to_logical()`](Self::to_logical) and
    /// [`to_canonical()`](Self::to_canonical) will always return their input
    /// unchanged.
    #[must_use]
    pub fn has_mapping(&self) -> bool {
        self.mapping.is_some()
    }

    /// Translate a canonical (symlink-resolved) path to its logical
    /// (symlink-preserving) equivalent.
    ///
    /// If the context has an active mapping and the path starts with the
    /// canonical prefix, the canonical prefix is replaced with the logical
    /// prefix. The translation is validated via round-trip canonicalization
    /// before being returned.
    ///
    /// Returns the input path unchanged when:
    /// - No active mapping exists
    /// - The path does not start with the canonical prefix
    /// - The path is relative (not absolute)
    /// - Round-trip validation fails
    /// - Canonicalization of the translated path fails (e.g., path doesn't exist on disk)
    ///
    /// # Panics
    ///
    /// This function never panics, even with non-UTF-8 path components.
    #[must_use]
    pub fn to_logical(&self, path: &Path) -> PathBuf {
        self.translate(path, TranslationDirection::ToLogical)
    }

    /// Translate a logical (symlink-preserving) path to its canonical
    /// (symlink-resolved) equivalent.
    ///
    /// If the context has an active mapping and the path starts with the
    /// logical prefix, the logical prefix is replaced with the canonical
    /// prefix. The translation is validated via round-trip canonicalization
    /// before being returned.
    ///
    /// Returns the input path unchanged when:
    /// - No active mapping exists
    /// - The path does not start with the logical prefix
    /// - The path is relative (not absolute)
    /// - Round-trip validation fails
    /// - Canonicalization of the translated path fails (e.g., path doesn't exist on disk)
    ///
    /// # Panics
    ///
    /// This function never panics, even with non-UTF-8 path components.
    #[must_use]
    pub fn to_canonical(&self, path: &Path) -> PathBuf {
        self.translate(path, TranslationDirection::ToCanonical)
    }

    fn translate(&self, path: &Path, direction: TranslationDirection) -> PathBuf {
        let fallback = path.to_path_buf();

        // No mapping → return input unchanged
        let mapping = match &self.mapping {
            Some(m) => m,
            None => {
                log::trace!("translate: no mapping, returning input unchanged");
                return fallback;
            }
        };

        // Relative paths → return input unchanged
        if path.is_relative() {
            log::trace!("translate: relative path, returning input unchanged");
            return fallback;
        }

        let (from_prefix, to_prefix) = match direction {
            TranslationDirection::ToLogical => (&mapping.canonical_prefix, &mapping.logical_prefix),
            TranslationDirection::ToCanonical => {
                (&mapping.logical_prefix, &mapping.canonical_prefix)
            }
        };

        // On Windows, strip the \\?\ prefix only for prefix matching.
        // Keep the original `path` and `fallback` unchanged so callers get
        // back the exact input on no-op paths, and so any later operations
        // in this function can still use the original path.
        #[cfg(windows)]
        let path_for_match_buf = strip_extended_length_prefix(path);
        #[cfg(windows)]
        let path_for_match = path_for_match_buf.as_path();
        #[cfg(not(windows))]
        let path_for_match = path;

        // Path must start with the source prefix
        let suffix = match path_for_match.strip_prefix(from_prefix) {
            Ok(s) => s,
            Err(_) => {
                log::trace!(
                    "translate: path does not start with source prefix ({}), returning unchanged",
                    from_prefix.display()
                );
                return fallback;
            }
        };

        let translated = to_prefix.join(suffix);

        // Round-trip validation: canonicalize both and compare
        let original_canonical = match std::fs::canonicalize(path) {
            Ok(c) => c,
            Err(e) => {
                log::trace!(
                    "translate: canonicalize({}) failed: {e}, returning unchanged",
                    path.display()
                );
                return fallback;
            }
        };
        let translated_canonical = match std::fs::canonicalize(&translated) {
            Ok(c) => c,
            Err(e) => {
                log::trace!(
                    "translate: canonicalize({}) failed: {e}, returning unchanged",
                    translated.display()
                );
                return fallback;
            }
        };

        // On Windows, strip \\?\ prefix from canonicalized paths before comparison
        #[cfg(windows)]
        let original_canonical = strip_extended_length_prefix(&original_canonical);
        #[cfg(windows)]
        let translated_canonical = strip_extended_length_prefix(&translated_canonical);

        if original_canonical == translated_canonical {
            translated
        } else {
            log::trace!(
                "translate: round-trip validation failed ({} != {}), returning unchanged",
                original_canonical.display(),
                translated_canonical.display()
            );
            fallback
        }
    }
}

enum TranslationDirection {
    ToLogical,
    ToCanonical,
}

/// Strip the `\\?\` Extended Length Path prefix from Windows paths.
///
/// - `\\?\C:\...` → `C:\...`
/// - `\\?\UNC\server\share\...` → `\\server\share\...`
/// - All other paths → returned unchanged
#[cfg(windows)]
fn strip_extended_length_prefix(path: &Path) -> PathBuf {
    let s = match path.to_str() {
        Some(s) => s,
        None => return path.to_path_buf(),
    };

    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }

    if let Some(rest) = s.strip_prefix(r"\\?\") {
        // Only strip if followed by a drive letter and colon
        let mut chars = rest.chars();
        if let Some(drive) = chars.next() {
            if drive.is_ascii_alphabetic() {
                if let Some(':') = chars.next() {
                    return PathBuf::from(rest);
                }
            }
        }
    }

    path.to_path_buf()
}

/// Compare two path components for equality.
///
/// - **Unix**: Case-sensitive comparison (`==`)
/// - **Windows**: Ordinal case-insensitive comparison (`eq_ignore_ascii_case`)
fn components_equal(a: &std::path::Component<'_>, b: &std::path::Component<'_>) -> bool {
    #[cfg(windows)]
    {
        a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

/// Find the divergence point between a canonical path and a logical path
/// by comparing path components from the end (longest common suffix).
///
/// Returns `Some((canonical_prefix, logical_prefix))` if the paths share a
/// common suffix but differ in their prefixes. Returns `None` if the paths
/// are identical or share no common suffix components.
fn find_divergence_point(canonical: &Path, logical: &Path) -> Option<(PathBuf, PathBuf)> {
    let canonical_components: Vec<_> = canonical.components().collect();
    let logical_components: Vec<_> = logical.components().collect();

    // Find the longest common suffix
    let mut common_suffix_len = 0;
    let mut c_iter = canonical_components.iter().rev();
    let mut l_iter = logical_components.iter().rev();

    loop {
        match (c_iter.next(), l_iter.next()) {
            (Some(c), Some(l)) if components_equal(c, l) => common_suffix_len += 1,
            _ => break,
        }
    }

    if common_suffix_len == 0 {
        return None;
    }

    let canonical_prefix_len = canonical_components.len() - common_suffix_len;
    let logical_prefix_len = logical_components.len() - common_suffix_len;

    // If both prefixes are empty, paths are identical
    if canonical_prefix_len == 0 && logical_prefix_len == 0 {
        return None;
    }

    let canonical_prefix: PathBuf = canonical_components[..canonical_prefix_len]
        .iter()
        .collect();
    let logical_prefix: PathBuf = logical_components[..logical_prefix_len].iter().collect();

    // Both prefixes must be non-empty absolute paths, and they must differ
    if canonical_prefix == logical_prefix {
        return None;
    }

    Some((canonical_prefix, logical_prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    // T007: has_mapping() tests
    #[test]
    fn has_mapping_returns_false_when_none() {
        let ctx = LogicalPathContext { mapping: None };
        assert!(!ctx.has_mapping());
    }

    #[test]
    fn has_mapping_returns_true_when_some() {
        let ctx = LogicalPathContext {
            mapping: Some(PrefixMapping {
                canonical_prefix: PathBuf::from("/mnt/wsl/workspace"),
                logical_prefix: PathBuf::from("/workspace"),
            }),
        };
        assert!(ctx.has_mapping());
    }

    // T007a: Send + Sync compile-time assertion
    #[test]
    fn logical_path_context_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LogicalPathContext>();
    }

    // T007b: Default returns no-mapping context
    #[test]
    fn default_returns_no_mapping() {
        let ctx = LogicalPathContext::default();
        assert!(!ctx.has_mapping());
        assert_eq!(ctx, LogicalPathContext { mapping: None });
    }

    // T009: find_divergence_point tests
    #[cfg(unix)]
    #[test]
    fn divergence_identical_paths_returns_none() {
        let result = find_divergence_point(
            Path::new("/home/user/project"),
            Path::new("/home/user/project"),
        );
        assert_eq!(result, None);
    }

    #[cfg(unix)]
    #[test]
    fn divergence_common_suffix_different_prefixes() {
        let result = find_divergence_point(
            Path::new("/mnt/wsl/workspace/project/src"),
            Path::new("/workspace/project/src"),
        );
        // "workspace", "project", "src" are the common suffix;
        // prefixes are what remain: /mnt/wsl vs /
        assert_eq!(
            result,
            Some((PathBuf::from("/mnt/wsl"), PathBuf::from("/")))
        );
    }

    #[cfg(unix)]
    #[test]
    fn divergence_no_common_components_returns_none() {
        let result = find_divergence_point(Path::new("/a/b/c"), Path::new("/x/y/z"));
        assert_eq!(result, None);
    }

    #[cfg(unix)]
    #[test]
    fn divergence_trailing_slashes() {
        // Path::components() normalizes trailing slashes
        let result = find_divergence_point(
            Path::new("/real/base/project/"),
            Path::new("/link/project/"),
        );
        assert_eq!(
            result,
            Some((PathBuf::from("/real/base"), PathBuf::from("/link")))
        );
    }

    #[cfg(unix)]
    #[test]
    fn divergence_dot_components() {
        // Path::components() normalizes `.` (CurDir)
        let result = find_divergence_point(
            Path::new("/real/./base/project"),
            Path::new("/link/./project"),
        );
        assert_eq!(
            result,
            Some((PathBuf::from("/real/base"), PathBuf::from("/link")))
        );
    }

    #[cfg(unix)]
    #[test]
    fn divergence_dotdot_components() {
        // `..` is preserved as a component by Path::components() — it doesn't
        // resolve against the filesystem. Paths with `..` will appear as
        // distinct components.
        let result = find_divergence_point(
            Path::new("/real/base/../base/project"),
            Path::new("/link/project"),
        );
        // components: [/, real, base, .., base, project] vs [/, link, project]
        // common suffix from end: project matches, then base != link → stop
        // canonical prefix: [/, real, base, .., base] = /real/base/../base
        // logical prefix: [/, link] = /link
        assert_eq!(
            result,
            Some((PathBuf::from("/real/base/../base"), PathBuf::from("/link")))
        );
    }

    #[cfg(unix)]
    #[test]
    fn divergence_redundant_separators() {
        // Path::components() normalizes redundant separators
        let result = find_divergence_point(
            Path::new("/real///base//project"),
            Path::new("/link//project"),
        );
        assert_eq!(
            result,
            Some((PathBuf::from("/real/base"), PathBuf::from("/link")))
        );
    }

    #[cfg(unix)]
    #[test]
    fn divergence_macos_private_prefix() {
        let result = find_divergence_point(
            Path::new("/private/var/folders/tmp"),
            Path::new("/var/folders/tmp"),
        );
        assert_eq!(
            result,
            Some((PathBuf::from("/private"), PathBuf::from("/")))
        );
    }

    // T010: detect_from() with pwd matching canonical CWD → no mapping
    #[cfg(not(windows))]
    #[test]
    fn detect_from_pwd_matches_canonical_returns_no_mapping() {
        use std::ffi::OsStr;
        let cwd = Path::new("/home/user/project");
        let ctx = LogicalPathContext::detect_from(Some(OsStr::new("/home/user/project")), cwd);
        assert!(!ctx.has_mapping());
    }

    // T011: detect_from() with pwd as None → no mapping
    #[cfg(not(windows))]
    #[test]
    fn detect_from_pwd_none_returns_no_mapping() {
        let cwd = Path::new("/home/user/project");
        let ctx = LogicalPathContext::detect_from(None, cwd);
        assert!(!ctx.has_mapping());
    }

    // T013: detect_from() with stale pwd (non-existent path) → no mapping
    #[cfg(not(windows))]
    #[test]
    fn detect_from_stale_pwd_returns_no_mapping() {
        use std::ffi::OsStr;
        let cwd = Path::new("/home/user/project");
        let ctx = LogicalPathContext::detect_from(
            Some(OsStr::new("/nonexistent/stale/path/project")),
            cwd,
        );
        // canonicalize("/nonexistent/stale/path/project") fails → no mapping
        assert!(!ctx.has_mapping());
    }

    // T033: detect_from() with corrupted/partially-resolved pwd → no mapping
    #[cfg(not(windows))]
    #[test]
    fn detect_from_corrupted_pwd_returns_no_mapping() {
        use std::ffi::OsStr;
        let cwd = Path::new("/home/user/project");
        let ctx = LogicalPathContext::detect_from(Some(OsStr::new("")), cwd);
        assert!(!ctx.has_mapping());
    }

    // T037: detect_from() with macOS /var → /private/var system symlink pattern
    #[cfg(target_os = "macos")]
    #[test]
    fn detect_from_macos_private_prefix_has_mapping() {
        // On macOS, /var is a symlink to /private/var.
        // Validate that a $PWD path under /var is detected as a mapping.
        use std::ffi::OsStr;
        let logical_path = Path::new("/var/folders");
        let Ok(canonical_cwd) = std::fs::canonicalize(logical_path) else {
            return; // Skip if /var/folders doesn't exist
        };
        if canonical_cwd == logical_path {
            return; // Skip if /var is not a symlink on this system
        }
        let ctx = LogicalPathContext::detect_from(Some(OsStr::new("/var/folders")), &canonical_cwd);
        assert!(ctx.has_mapping());
    }

    // Helper to build a context with a known mapping for unit tests
    fn ctx_with_mapping(
        canonical: impl AsRef<Path>,
        logical: impl AsRef<Path>,
    ) -> LogicalPathContext {
        LogicalPathContext {
            mapping: Some(PrefixMapping {
                canonical_prefix: canonical.as_ref().to_path_buf(),
                logical_prefix: logical.as_ref().to_path_buf(),
            }),
        }
    }

    fn ctx_no_mapping() -> LogicalPathContext {
        LogicalPathContext { mapping: None }
    }

    // ===== US2: to_logical() tests =====

    // T017: to_logical() with active mapping and path under canonical prefix
    #[cfg(unix)]
    #[test]
    fn to_logical_translates_path_under_canonical_prefix() {
        // Use real filesystem paths so canonicalize() works for round-trip
        let dir = tempfile::tempdir().unwrap();
        let canonical_base = dir.path().join("real");
        let logical_base = dir.path().join("link");

        std::fs::create_dir_all(canonical_base.join("src")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&canonical_base, &logical_base).unwrap();

        let ctx = ctx_with_mapping(&canonical_base, &logical_base);

        let input = canonical_base.join("src");
        let result = ctx.to_logical(&input);
        assert_eq!(result, logical_base.join("src"));
    }

    // T018: to_logical() with active mapping and path NOT under canonical prefix
    #[test]
    fn to_logical_returns_input_when_not_under_prefix() {
        let ctx = ctx_with_mapping("/mnt/wsl/workspace", "/workspace");
        let input = Path::new("/some/other/path");
        let result = ctx.to_logical(input);
        assert_eq!(result, input.to_path_buf());
    }

    // T019: to_logical() with no active mapping returns input unchanged
    #[test]
    fn to_logical_returns_input_when_no_mapping() {
        let ctx = ctx_no_mapping();
        let input = Path::new("/home/user/project/src/main.rs");
        let result = ctx.to_logical(input);
        assert_eq!(result, input.to_path_buf());
    }

    // T019a: to_logical() with a relative path returns input unchanged
    #[test]
    fn to_logical_returns_input_for_relative_path() {
        let ctx = ctx_with_mapping("/mnt/wsl/workspace", "/workspace");
        let input = Path::new("src/main.rs");
        let result = ctx.to_logical(input);
        assert_eq!(result, input.to_path_buf());
    }

    // ===== US3: to_canonical() tests =====

    // T024: to_canonical() with active mapping and path under logical prefix
    #[cfg(unix)]
    #[test]
    fn to_canonical_translates_path_under_logical_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let canonical_base = dir.path().join("real");
        let logical_base = dir.path().join("link");

        std::fs::create_dir_all(canonical_base.join("src")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&canonical_base, &logical_base).unwrap();

        let ctx = ctx_with_mapping(&canonical_base, &logical_base);

        let input = logical_base.join("src");
        let result = ctx.to_canonical(&input);
        assert_eq!(result, canonical_base.join("src"));
    }

    // T025: to_canonical() with active mapping and path NOT under logical prefix
    #[test]
    fn to_canonical_returns_input_when_not_under_prefix() {
        let ctx = ctx_with_mapping("/mnt/wsl/workspace", "/workspace");
        let input = Path::new("/some/other/path");
        let result = ctx.to_canonical(input);
        assert_eq!(result, input.to_path_buf());
    }

    // T026: to_canonical() with no active mapping returns input unchanged
    #[test]
    fn to_canonical_returns_input_when_no_mapping() {
        let ctx = ctx_no_mapping();
        let input = Path::new("/home/user/project/src/main.rs");
        let result = ctx.to_canonical(input);
        assert_eq!(result, input.to_path_buf());
    }

    // T026a: to_canonical() with a relative path returns input unchanged
    #[test]
    fn to_canonical_returns_input_for_relative_path() {
        let ctx = ctx_with_mapping("/mnt/wsl/workspace", "/workspace");
        let input = Path::new("../foo/bar.rs");
        let result = ctx.to_canonical(input);
        assert_eq!(result, input.to_path_buf());
    }

    // ===== US4: Fallback guarantee tests =====

    // T031: to_logical() and to_canonical() return input when round-trip would fail
    #[cfg(unix)]
    #[test]
    fn to_logical_falls_back_when_roundtrip_fails() {
        // Create a mapping that is syntactically valid but the translated path
        // doesn't exist, so canonicalize() fails → fallback
        let dir = tempfile::tempdir().unwrap();
        let real_base = dir.path().join("real");
        let bogus_logical = dir.path().join("bogus_link");
        std::fs::create_dir_all(real_base.join("src")).unwrap();
        // No symlink created, so canonicalize of bogus_link/src will fail

        let ctx = ctx_with_mapping(&real_base, &bogus_logical);

        let input = real_base.join("src");
        let result = ctx.to_logical(&input);
        // Translated path doesn't exist → fallback to input
        assert_eq!(result, input);
    }

    #[cfg(unix)]
    #[test]
    fn to_canonical_falls_back_when_roundtrip_fails() {
        let dir = tempfile::tempdir().unwrap();
        let bogus_canonical = dir.path().join("bogus_real");
        let link_base = dir.path().join("link");
        std::fs::create_dir_all(link_base.join("src")).unwrap();
        // No real directory behind bogus_canonical

        let ctx = ctx_with_mapping(&bogus_canonical, &link_base);

        let input = link_base.join("src");
        let result = ctx.to_canonical(&input);
        assert_eq!(result, input);
    }

    // T032: non-UTF-8 paths don't panic
    #[cfg(unix)]
    #[test]
    fn non_utf8_paths_dont_panic() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let non_utf8 = OsStr::from_bytes(&[0xff, 0xfe]);
        let ctx = ctx_with_mapping("/mnt/wsl/workspace", "/workspace");

        // to_logical with non-utf8 path — should not panic
        let input = Path::new(non_utf8);
        let result = ctx.to_logical(input);
        assert_eq!(result, input.to_path_buf());

        // to_canonical with non-utf8 path — should not panic
        let result = ctx.to_canonical(input);
        assert_eq!(result, input.to_path_buf());

        // detect_from with non-utf8 pwd — should not panic
        let ctx2 = LogicalPathContext::detect_from(Some(non_utf8), Path::new("/home/user"));
        let _ = ctx2;
    }

    // T034: Idempotence — to_logical on an already-logical path returns it unchanged
    #[cfg(unix)]
    #[test]
    fn to_logical_idempotent_on_logical_path() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let canonical_base = dir.path().join("real");
        let logical_base = dir.path().join("link");

        std::fs::create_dir_all(canonical_base.join("src")).unwrap();
        symlink(&canonical_base, &logical_base).unwrap();

        let ctx = ctx_with_mapping(&canonical_base, &logical_base);

        // Applying to_logical to an already-logical path should return it unchanged
        // because the logical prefix doesn't start with the canonical prefix.
        let logical_path = logical_base.join("src");
        let result = ctx.to_logical(&logical_path);
        assert_eq!(result, logical_path);
    }

    // T034a: Idempotence — to_canonical on an already-canonical path returns it unchanged
    #[cfg(unix)]
    #[test]
    fn to_canonical_idempotent_on_canonical_path() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let canonical_base = dir.path().join("real");
        let logical_base = dir.path().join("link");

        std::fs::create_dir_all(canonical_base.join("src")).unwrap();
        symlink(&canonical_base, &logical_base).unwrap();

        let ctx = ctx_with_mapping(&canonical_base, &logical_base);

        // Applying to_canonical to an already-canonical path should return it unchanged
        // because the canonical prefix doesn't start with the logical prefix.
        let canonical_path = canonical_base.join("src");
        let result = ctx.to_canonical(&canonical_path);
        assert_eq!(result, canonical_path);
    }

    // T035: detect_from with divergent $PWD (valid dir that canonicalizes elsewhere)
    #[cfg(not(windows))]
    #[test]
    fn detect_from_divergent_pwd_returns_no_mapping() {
        // Create two real directories — $PWD points to dir_a but CWD is dir_b
        let dir = tempfile::tempdir().unwrap();
        let dir_a = dir.path().join("a");
        let dir_b = dir.path().join("b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();

        let canonical_a = std::fs::canonicalize(&dir_a).unwrap();
        let canonical_b = std::fs::canonicalize(&dir_b).unwrap();

        // $PWD is valid but canonicalizes to dir_a, not dir_b → no mapping
        let ctx = LogicalPathContext::detect_from(Some(canonical_a.as_os_str()), &canonical_b);
        assert!(!ctx.has_mapping());
    }

    // T035a: Translation works on file paths, not just directories
    #[cfg(unix)]
    #[test]
    fn to_logical_translates_file_paths() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let canonical_base = dir.path().join("real");
        let logical_base = dir.path().join("link");

        std::fs::create_dir_all(canonical_base.join("src")).unwrap();
        // Create a file so canonicalize() succeeds during round-trip validation
        std::fs::write(canonical_base.join("src").join("main.rs"), b"fn main() {}").unwrap();
        symlink(&canonical_base, &logical_base).unwrap();

        let ctx = ctx_with_mapping(&canonical_base, &logical_base);

        let canonical_file = canonical_base.join("src").join("main.rs");
        let result = ctx.to_logical(&canonical_file);
        assert_eq!(result, logical_base.join("src").join("main.rs"));

        // And back
        let logical_file = logical_base.join("src").join("main.rs");
        let back = ctx.to_canonical(&logical_file);
        assert_eq!(back, canonical_base.join("src").join("main.rs"));
    }

    // T030a: Parameterised round-trip test covering ≥10 distinct path structures
    #[cfg(unix)]
    #[test]
    fn roundtrip_parameterized_test() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        // Canonicalize the temp dir base so that detect_from's pwd validation
        // succeeds on systems where the temp directory is under a symlink
        // (e.g., macOS where /tmp → /private/tmp).
        let base = std::fs::canonicalize(dir.path()).unwrap();
        let real_base = base.join("real");
        let link_base = base.join("link");

        // Create a directory tree for testing
        let subdirs = [
            "src",
            "src/main",
            "src/lib",
            "tests",
            "tests/unit",
            "docs",
            "docs/api",
            "build",
            "build/debug",
            "config",
        ];

        for subdir in &subdirs {
            std::fs::create_dir_all(real_base.join(subdir)).unwrap();
        }
        symlink(&real_base, &link_base).unwrap();

        let ctx = LogicalPathContext::detect_from(
            Some(link_base.join("src").as_os_str()),
            &real_base.join("src"),
        );

        // Test canonical → logical → canonical round-trip for each subdir
        for subdir in &subdirs {
            let canonical = real_base.join(subdir);
            let logical = ctx.to_logical(&canonical);
            let expected_logical = link_base.join(subdir);
            assert_eq!(
                logical, expected_logical,
                "to_logical failed for {}",
                subdir
            );

            let back_to_canonical = ctx.to_canonical(&logical);
            assert_eq!(
                back_to_canonical, canonical,
                "to_canonical round-trip failed for {}",
                subdir
            );
        }
    }

    // ===== Windows-specific unit tests =====

    // T003: strip_extended_length_prefix tests
    #[cfg(windows)]
    #[test]
    fn strip_prefix_drive_letter() {
        let result = strip_extended_length_prefix(Path::new(r"\\?\C:\Users\dev"));
        assert_eq!(result, PathBuf::from(r"C:\Users\dev"));
    }

    #[cfg(windows)]
    #[test]
    fn strip_prefix_unc() {
        let result = strip_extended_length_prefix(Path::new(r"\\?\UNC\server\share\folder"));
        assert_eq!(result, PathBuf::from(r"\\server\share\folder"));
    }

    #[cfg(windows)]
    #[test]
    fn strip_prefix_no_prefix_unchanged() {
        let result = strip_extended_length_prefix(Path::new(r"C:\Users\dev"));
        assert_eq!(result, PathBuf::from(r"C:\Users\dev"));
    }

    #[cfg(windows)]
    #[test]
    fn strip_prefix_empty_unchanged() {
        let result = strip_extended_length_prefix(Path::new(""));
        assert_eq!(result, PathBuf::from(""));
    }

    // T005: case-insensitive find_divergence_point on Windows
    #[cfg(windows)]
    #[test]
    fn divergence_case_insensitive_matching_components() {
        // Same components differing only in case should match → no divergence
        let result = find_divergence_point(
            Path::new(r"C:\Users\Dev\Project"),
            Path::new(r"C:\users\dev\project"),
        );
        assert_eq!(result, None);
    }

    #[cfg(windows)]
    #[test]
    fn divergence_windows_junction_like_paths() {
        // Junction-like: D:\Projects\Workspace\src vs C:\workspace\src
        // Common suffix (case-insensitive): workspace\src
        let result = find_divergence_point(
            Path::new(r"D:\Projects\Workspace\src"),
            Path::new(r"C:\workspace\src"),
        );
        assert_eq!(
            result,
            Some((PathBuf::from(r"D:\Projects"), PathBuf::from(r"C:\")))
        );
    }

    #[cfg(windows)]
    #[test]
    fn divergence_windows_identical_paths() {
        let result = find_divergence_point(
            Path::new(r"C:\Users\dev\project"),
            Path::new(r"C:\Users\dev\project"),
        );
        assert_eq!(result, None);
    }

    // T007: detect_from_cwd tests
    #[cfg(windows)]
    #[test]
    fn detect_from_cwd_equal_paths_no_mapping() {
        let ctx = LogicalPathContext::detect_from_cwd(
            Path::new(r"C:\Users\dev\project"),
            Path::new(r"C:\Users\dev\project"),
        );
        assert!(!ctx.has_mapping());
    }

    #[cfg(windows)]
    #[test]
    fn detect_from_cwd_different_paths_with_common_suffix() {
        let ctx = LogicalPathContext::detect_from_cwd(
            Path::new(r"S:\workspace\src"),
            Path::new(r"D:\projects\workspace\src"),
        );
        assert!(ctx.has_mapping());
    }

    #[cfg(windows)]
    #[test]
    fn detect_from_cwd_different_paths_no_common_suffix() {
        let ctx = LogicalPathContext::detect_from_cwd(
            Path::new(r"X:\completely\different"),
            Path::new(r"Y:\totally\unrelated"),
        );
        assert!(!ctx.has_mapping());
    }

    // T010a: to_logical with \\?\-prefixed canonical path on Windows
    #[cfg(windows)]
    #[test]
    fn to_logical_strips_extended_prefix_from_input() {
        // Use real filesystem paths so canonicalize() round-trip succeeds
        let dir = tempfile::tempdir().unwrap();
        let canonical_base = std::fs::canonicalize(dir.path()).unwrap();
        let real_dir = canonical_base.join("real");
        let link_dir = canonical_base.join("link");

        std::fs::create_dir_all(real_dir.join("src")).unwrap();

        // Create an NTFS junction: link_dir -> real_dir
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link_dir)
            .arg(&real_dir)
            .output()
            .expect("mklink /J");
        assert!(status.status.success(), "mklink /J failed");

        let ctx = ctx_with_mapping(&real_dir, &link_dir);

        // The caller provides a \\?\-prefixed canonical path (FR-008)
        let prefixed_input = PathBuf::from(format!(
            r"\\?\{}",
            real_dir.join("src").to_str().unwrap()
        ));
        let result = ctx.to_logical(&prefixed_input);
        assert_eq!(result, link_dir.join("src"));

        // Clean up junction
        let _ = std::process::Command::new("cmd")
            .args(["/C", "rd"])
            .arg(&link_dir)
            .output();
    }

    // T021: detect_from_cwd fallback with identical paths
    #[cfg(windows)]
    #[test]
    fn detect_from_cwd_identical_returns_fallback() {
        let path = Path::new(r"C:\Users\dev\project");
        let ctx = LogicalPathContext::detect_from_cwd(path, path);
        assert!(!ctx.has_mapping());

        // to_logical and to_canonical return input unchanged
        let input = Path::new(r"C:\Users\dev\project\src\main.rs");
        assert_eq!(ctx.to_logical(input), input.to_path_buf());
        assert_eq!(ctx.to_canonical(input), input.to_path_buf());
    }

    // T022: relative paths on Windows return unchanged
    #[cfg(windows)]
    #[test]
    fn windows_relative_path_returns_unchanged() {
        let ctx = ctx_with_mapping(r"D:\projects\workspace", r"C:\workspace");

        let input = Path::new(r"src\main.rs");
        assert_eq!(ctx.to_logical(input), input.to_path_buf());
        assert_eq!(ctx.to_canonical(input), input.to_path_buf());
    }
}
