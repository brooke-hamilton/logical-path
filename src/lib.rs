#![deny(missing_docs)]

//! Translate canonical (symlink-resolved) filesystem paths back to their
//! logical (symlink-preserving) equivalents.
//!
//! When a shell's current directory traverses a symlink, two different paths
//! refer to the same location: the **logical** path (from `$PWD`, preserving
//! the symlink) and the **canonical** path (from `getcwd()`, with symlinks
//! resolved). This crate detects that mapping and provides bidirectional
//! translation.
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
//! - **Windows**: `detect()` returns no active mapping; all translations return
//!   the input unchanged.

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
/// - **Windows**: Always reports no active mapping (`$PWD` has no OS-level
///   equivalent). All translations fall back to returning the input unchanged.
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

impl LogicalPathContext {
    /// Detect the active symlink prefix mapping by comparing `$PWD` (logical)
    /// against `getcwd()` (canonical).
    ///
    /// Returns a context with no active mapping when:
    /// - `$PWD` is unset
    /// - `$PWD` equals the canonical CWD (no symlink in effect)
    /// - `$PWD` is stale (points to a non-existent directory)
    /// - The current directory cannot be determined
    /// - Running on Windows
    ///
    /// # Panics
    ///
    /// This function never panics.
    #[must_use]
    pub fn detect() -> LogicalPathContext {
        #[cfg(windows)]
        {
            LogicalPathContext { mapping: None }
        }

        #[cfg(not(windows))]
        {
            let pwd = std::env::var_os("PWD");
            let canonical_cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(_) => return LogicalPathContext { mapping: None },
            };
            Self::detect_from(pwd.as_deref(), &canonical_cwd)
        }
    }

    /// Internal helper for testability: takes `$PWD` and canonical CWD as
    /// parameters instead of reading from global process state.
    #[cfg(not(windows))]
    pub(crate) fn detect_from(pwd: Option<&OsStr>, canonical_cwd: &Path) -> LogicalPathContext {
        let pwd = match pwd {
            Some(p) if !p.is_empty() => Path::new(p),
            _ => return LogicalPathContext { mapping: None },
        };

        // If pwd and canonical CWD are identical, no mapping needed
        if pwd == canonical_cwd {
            return LogicalPathContext { mapping: None };
        }

        // Validate that pwd resolves to canonical_cwd. This rejects stale $PWD
        // values (non-existent directories) and divergent $PWD assignments.
        match std::fs::canonicalize(pwd) {
            Ok(canonical_pwd) if canonical_pwd == canonical_cwd => {}
            _ => return LogicalPathContext { mapping: None },
        }

        match find_divergence_point(canonical_cwd, pwd) {
            Some((canonical_prefix, logical_prefix)) => LogicalPathContext {
                mapping: Some(PrefixMapping {
                    canonical_prefix,
                    logical_prefix,
                }),
            },
            None => LogicalPathContext { mapping: None },
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
            None => return fallback,
        };

        // Relative paths → return input unchanged
        if path.is_relative() {
            return fallback;
        }

        let (from_prefix, to_prefix) = match direction {
            TranslationDirection::ToLogical => (&mapping.canonical_prefix, &mapping.logical_prefix),
            TranslationDirection::ToCanonical => {
                (&mapping.logical_prefix, &mapping.canonical_prefix)
            }
        };

        // Path must start with the source prefix
        let suffix = match path.strip_prefix(from_prefix) {
            Ok(s) => s,
            Err(_) => return fallback,
        };

        let translated = to_prefix.join(suffix);

        // Round-trip validation: canonicalize both and compare
        let original_canonical = match std::fs::canonicalize(path) {
            Ok(c) => c,
            Err(_) => return fallback,
        };
        let translated_canonical = match std::fs::canonicalize(&translated) {
            Ok(c) => c,
            Err(_) => return fallback,
        };

        if original_canonical == translated_canonical {
            translated
        } else {
            fallback
        }
    }
}

enum TranslationDirection {
    ToLogical,
    ToCanonical,
}

/// Find the divergence point between a canonical path and a logical path
/// by comparing path components from the end (longest common suffix).
///
/// Returns `Some((canonical_prefix, logical_prefix))` if the paths share a
/// common suffix but differ in their prefixes. Returns `None` if the paths
/// are identical or share no common suffix components.
#[cfg(not(windows))]
fn find_divergence_point(canonical: &Path, logical: &Path) -> Option<(PathBuf, PathBuf)> {
    let canonical_components: Vec<_> = canonical.components().collect();
    let logical_components: Vec<_> = logical.components().collect();

    // Find the longest common suffix
    let mut common_suffix_len = 0;
    let mut c_iter = canonical_components.iter().rev();
    let mut l_iter = logical_components.iter().rev();

    loop {
        match (c_iter.next(), l_iter.next()) {
            (Some(c), Some(l)) if c == l => common_suffix_len += 1,
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

    // T009: find_divergence_point tests
    #[cfg(not(windows))]
    #[test]
    fn divergence_identical_paths_returns_none() {
        let result = find_divergence_point(
            Path::new("/home/user/project"),
            Path::new("/home/user/project"),
        );
        assert_eq!(result, None);
    }

    #[cfg(not(windows))]
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

    #[cfg(not(windows))]
    #[test]
    fn divergence_no_common_components_returns_none() {
        let result = find_divergence_point(Path::new("/a/b/c"), Path::new("/x/y/z"));
        assert_eq!(result, None);
    }

    #[cfg(not(windows))]
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

    #[cfg(not(windows))]
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

    #[cfg(not(windows))]
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

    #[cfg(not(windows))]
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

    #[cfg(not(windows))]
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
    fn ctx_with_mapping(canonical: &str, logical: &str) -> LogicalPathContext {
        LogicalPathContext {
            mapping: Some(PrefixMapping {
                canonical_prefix: PathBuf::from(canonical),
                logical_prefix: PathBuf::from(logical),
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

        let ctx = ctx_with_mapping(
            canonical_base.to_str().unwrap(),
            logical_base.to_str().unwrap(),
        );

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

        let ctx = ctx_with_mapping(
            canonical_base.to_str().unwrap(),
            logical_base.to_str().unwrap(),
        );

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

        let ctx = ctx_with_mapping(real_base.to_str().unwrap(), bogus_logical.to_str().unwrap());

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

        let ctx = ctx_with_mapping(
            bogus_canonical.to_str().unwrap(),
            link_base.to_str().unwrap(),
        );

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
}
