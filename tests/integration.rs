use logical_path::LogicalPathContext;

/// Serializes tests that mutate process-global state (current directory and `$PWD`).
///
/// All environment-mutating tests in this file acquire this lock before touching
/// `$PWD` or the current directory, so they never run concurrently with each other.
/// Tests that only call `detect()` without first acquiring this lock must not
/// mutate environment variables.
#[cfg(unix)]
static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// RAII guard that restores the process working directory and `$PWD` on drop.
///
/// Holds `ENV_MUTEX` for its entire lifetime, serializing all environment mutations
/// within this test binary. `set_var`/`remove_var` are `unsafe` in Rust 2024 because
/// concurrent environment modification is undefined behaviour; holding `ENV_MUTEX`
/// ensures that no two tests in this binary modify the environment at the same time.
#[cfg(unix)]
struct EnvGuard {
    saved_dir: std::path::PathBuf,
    saved_pwd: Option<std::ffi::OsString>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(unix)]
impl EnvGuard {
    /// Acquires `ENV_MUTEX` and snapshots the current environment state.
    ///
    /// If the mutex is poisoned (a previous test panicked while holding it), the
    /// guard is recovered and the snapshot is taken from whatever state the
    /// environment is in, allowing subsequent tests to run independently.
    fn acquire() -> Self {
        let lock = ENV_MUTEX.lock().unwrap_or_else(|e| {
            eprintln!("EnvGuard: recovering poisoned ENV_MUTEX after a previous test panic");
            e.into_inner()
        });
        EnvGuard {
            saved_dir: std::env::current_dir().expect("current_dir"),
            saved_pwd: std::env::var_os("PWD"),
            _lock: lock,
        }
    }

    fn set(&self, canonical_dir: &std::path::Path, logical_pwd: &std::path::Path) {
        std::env::set_current_dir(canonical_dir).expect("set_current_dir");
        // SAFETY: ENV_MUTEX is held for the duration of this test, serializing all
        // environment mutations within this test binary. No other test-file code
        // modifies $PWD or CWD without holding the same lock.
        unsafe { std::env::set_var("PWD", logical_pwd) };
    }
}

#[cfg(unix)]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.saved_dir);
        // SAFETY: Same invariant as in `set` — ENV_MUTEX is still held here.
        match &self.saved_pwd {
            Some(p) => unsafe { std::env::set_var("PWD", p) },
            None => unsafe { std::env::remove_var("PWD") },
        }
    }
}

// T012: detect() inside a real symlink directory returns context with has_mapping() == true
#[cfg(unix)]
#[test]
fn detect_inside_real_symlink_has_mapping() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("project");
    let link_base = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();
    symlink(base.join("real"), &link_base).unwrap();

    guard.set(&real_dir, &link_base.join("project"));
    let ctx = LogicalPathContext::detect();

    assert!(ctx.has_mapping());
}

// T012a: detect() with nested symlinks (symlink through another symlink)
// detects only the outermost divergence mapping
#[cfg(unix)]
#[test]
fn detect_nested_symlinks_detects_outermost_divergence() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("project");
    let link1 = base.join("link1");
    let link2 = base.join("link2");

    std::fs::create_dir_all(&real_dir).unwrap();
    // link1 -> real, link2 -> link1 (nested)
    symlink(base.join("real"), &link1).unwrap();
    symlink(&link1, &link2).unwrap();

    // canonical CWD resolves both symlinks: real/project
    // $PWD follows the outermost symlink: link2/project
    guard.set(&real_dir, &link2.join("project"));
    let ctx = LogicalPathContext::detect();

    assert!(ctx.has_mapping());
}

// T020: to_logical() with real symlink environment
#[cfg(unix)]
#[test]
fn to_logical_with_real_symlink() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("src");
    let link_base = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();
    symlink(base.join("real"), &link_base).unwrap();

    guard.set(&real_dir, &link_base.join("src"));
    let ctx = LogicalPathContext::detect();

    let canonical_path = base.join("real").join("src");
    let result = ctx.to_logical(&canonical_path);
    assert_eq!(result, link_base.join("src"));
}

// T027: to_canonical() with real symlink environment
#[cfg(unix)]
#[test]
fn to_canonical_with_real_symlink() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("src");
    let link_base = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();
    symlink(base.join("real"), &link_base).unwrap();

    guard.set(&real_dir, &link_base.join("src"));
    let ctx = LogicalPathContext::detect();

    let logical_path = link_base.join("src");
    let result = ctx.to_canonical(&logical_path);
    assert_eq!(result, base.join("real").join("src"));
}

// T034b: Idempotence — to_logical on already-logical path with real detect()
#[cfg(unix)]
#[test]
fn to_logical_idempotent_with_real_symlink() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("src");
    let link_base = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();
    symlink(base.join("real"), &link_base).unwrap();

    guard.set(&real_dir, &link_base.join("src"));
    let ctx = LogicalPathContext::detect();

    // First translation: canonical → logical
    let canonical = base.join("real").join("src");
    let logical = ctx.to_logical(&canonical);
    assert_eq!(logical, link_base.join("src"));

    // Second translation: already-logical → should return unchanged
    let again = ctx.to_logical(&logical);
    assert_eq!(again, logical);
}

// T035b: File path translation with real symlink environment
#[cfg(unix)]
#[test]
fn to_logical_translates_file_path_with_real_symlink() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("src");
    let link_base = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();
    std::fs::write(real_dir.join("main.rs"), b"fn main() {}").unwrap();
    symlink(base.join("real"), &link_base).unwrap();

    guard.set(&real_dir, &link_base.join("src"));
    let ctx = LogicalPathContext::detect();

    // Translate a file path (not just a directory)
    let canonical_file = base.join("real").join("src").join("main.rs");
    let result = ctx.to_logical(&canonical_file);
    assert_eq!(result, link_base.join("src").join("main.rs"));
}

// ===== US5: Cross-platform tests =====

// T036: Platform-gated test for Linux
#[cfg(target_os = "linux")]
#[test]
fn detect_with_real_symlink_on_linux() {
    use std::os::unix::fs::symlink;

    let guard = EnvGuard::acquire();

    let dir = tempfile::tempdir().unwrap();
    let base = std::fs::canonicalize(dir.path()).unwrap();
    let real_dir = base.join("real").join("project");
    let link_base = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();
    symlink(base.join("real"), &link_base).unwrap();

    guard.set(&real_dir, &link_base.join("project"));
    let ctx = LogicalPathContext::detect();

    assert!(ctx.has_mapping());

    // Verify translation works
    let canonical = base.join("real").join("project");
    let logical = ctx.to_logical(&canonical);
    assert_eq!(logical, link_base.join("project"));
}

// T038: Platform-gated test for Windows
#[cfg(windows)]
#[test]
fn detect_returns_no_mapping_on_windows() {
    let ctx = LogicalPathContext::detect();
    assert!(!ctx.has_mapping());

    // All translations return input unchanged
    let path = std::path::Path::new("C:\\Users\\user\\project\\src\\main.rs");
    assert_eq!(ctx.to_logical(path), path.to_path_buf());
    assert_eq!(ctx.to_canonical(path), path.to_path_buf());
}
