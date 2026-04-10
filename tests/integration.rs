use logical_path::LogicalPathContext;

/// Serializes tests that mutate process-global state (current directory and `$PWD`).
///
/// All environment-mutating tests in this file acquire this lock before touching
/// `$PWD` or the current directory, so they never run concurrently with each other.
/// Tests that only call `detect()` without first acquiring this lock must not
/// mutate environment variables.
#[cfg(unix)]
static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Serializes Windows tests that mutate process-global state (CWD, junctions, subst drives).
///
/// Windows tests acquire this lock via `WinEnvGuard::new()` before touching CWD or
/// creating/removing OS-level path indirections, so they never run concurrently.
#[cfg(windows)]
static WIN_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
// This test is removed/updated per T032 — Windows detect() now returns mappings
// when indirections exist. The old no-op test is replaced by the comprehensive
// Windows integration tests below.

// ===== Windows Integration Test Infrastructure (T012) =====

#[cfg(windows)]
mod windows_helpers {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Strip the `\\?\` extended-length path prefix that `canonicalize()` adds
    /// on Windows, so test paths match the regular DOS-style paths that
    /// `detect()` stores in its mapping.
    pub fn strip_extended_prefix(p: PathBuf) -> PathBuf {
        let s = match p.to_str() {
            Some(s) => s,
            None => return p,
        };
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            PathBuf::from(rest)
        } else {
            p
        }
    }

    /// Create an NTFS junction from `link` to `target` via `cmd /c mklink /J`.
    pub fn create_junction(link: &Path, target: &Path) -> std::io::Result<()> {
        let output = Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "mklink /J failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))
        }
    }

    /// Create a directory symlink from `link` to `target` via `cmd /c mklink /D`.
    pub fn create_dir_symlink(link: &Path, target: &Path) -> std::io::Result<()> {
        let output = Command::new("cmd")
            .args(["/c", "mklink", "/D"])
            .arg(link)
            .arg(target)
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "mklink /D failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))
        }
    }

    /// Remove an NTFS junction via `cmd /c rd`.
    pub fn remove_junction(link: &Path) -> std::io::Result<()> {
        let output = Command::new("cmd").args(["/c", "rd"]).arg(link).output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("rd failed: {}", String::from_utf8_lossy(&output.stderr)),
            ))
        }
    }

    /// Create a subst drive mapping from `letter` to `target`.
    pub fn create_subst(letter: char, target: &Path) -> std::io::Result<()> {
        let output = Command::new("subst")
            .arg(format!("{letter}:"))
            .arg(target)
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("subst failed: {}", String::from_utf8_lossy(&output.stderr)),
            ))
        }
    }

    /// Remove a subst drive mapping.
    pub fn remove_subst(letter: char) -> std::io::Result<()> {
        let output = Command::new("subst")
            .arg(format!("{letter}:"))
            .arg("/D")
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!(
                    "subst /D failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))
        }
    }

    /// RAII guard that saves/restores CWD and cleans up junctions/subst drives on drop.
    ///
    /// Holds `WIN_ENV_MUTEX` for its entire lifetime, serializing all Windows environment
    /// mutations within this test binary so that CWD changes and OS-level path indirections
    /// (junctions, subst drives) do not interfere across concurrently running tests.
    pub struct WinEnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        saved_dir: PathBuf,
        junctions: Vec<PathBuf>,
        subst_drives: Vec<char>,
    }

    impl WinEnvGuard {
        pub fn new() -> Self {
            let lock = super::WIN_ENV_MUTEX.lock().unwrap_or_else(|e| {
                eprintln!(
                    "WinEnvGuard: recovering poisoned WIN_ENV_MUTEX after a previous test panic"
                );
                e.into_inner()
            });
            WinEnvGuard {
                _lock: lock,
                saved_dir: std::env::current_dir().expect("current_dir"),
                junctions: Vec::new(),
                subst_drives: Vec::new(),
            }
        }

        pub fn track_junction(&mut self, link: PathBuf) {
            self.junctions.push(link);
        }

        pub fn track_subst(&mut self, letter: char) {
            self.subst_drives.push(letter);
        }

        pub fn untrack_junction(&mut self, link: &PathBuf) {
            self.junctions.retain(|j| j != link);
        }

        pub fn untrack_subst(&mut self, letter: char) {
            self.subst_drives.retain(|&d| d != letter);
        }

        pub fn set_cwd(&self, dir: &Path) {
            std::env::set_current_dir(dir).expect("set_current_dir");
        }
    }

    impl Drop for WinEnvGuard {
        fn drop(&mut self) {
            // Restore CWD first so junctions/subst can be removed
            let _ = std::env::set_current_dir(&self.saved_dir);
            for junction in &self.junctions {
                let _ = remove_junction(junction);
            }
            for &letter in &self.subst_drives {
                let _ = remove_subst(letter);
            }
        }
    }

    /// Find an unused Windows drive letter by checking which root paths (`X:\`) do not exist.
    ///
    /// Scans from `Z:` down to `D:` (reverse order) to avoid commonly used drive letters
    /// like `C:` (system), `A:`/`B:` (floppy legacy), and physical drives that often start
    /// from `D:`. Returns the first letter whose root path is not currently present, or
    /// `None` if all candidate letters are already occupied.
    pub fn find_unused_drive_letter() -> Option<char> {
        ('D'..='Z')
            .rev()
            .find(|&c| !std::path::Path::new(&format!("{c}:\\")).exists())
    }
}

// ===== Phase 3: US1 — NTFS Junction and Directory Symlink Detection =====

// T013: Junction detection
#[cfg(windows)]
#[test]
fn detect_junction_has_mapping() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();
    assert!(ctx.has_mapping());
}

// T014: Junction to_logical translation
#[cfg(windows)]
#[test]
fn detect_junction_to_logical() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("link");
    let subdir = real_dir.join("src");

    std::fs::create_dir_all(&subdir).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();
    let canonical = real_dir.join("src");
    let result = ctx.to_logical(&canonical);
    assert_eq!(result, link_dir.join("src"));
}

// T015: No junction → no mapping (end-to-end)
#[cfg(windows)]
#[test]
fn detect_no_junction_no_mapping() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());

    let guard = WinEnvGuard::new();
    guard.set_cwd(&base);

    let ctx = LogicalPathContext::detect();
    assert!(!ctx.has_mapping());
}

// T016: Junction removed after detect → fallback
#[cfg(windows)]
#[test]
fn detect_junction_removed_fallback() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("link");
    let subdir = real_dir.join("src");

    std::fs::create_dir_all(&subdir).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();

    // Move CWD away so junction can be removed
    guard.set_cwd(&base);
    remove_junction(&link_dir).expect("remove junction");
    guard.untrack_junction(&link_dir);

    // to_logical should fall back since round-trip validation fails
    let canonical = real_dir.join("src");
    let result = ctx.to_logical(&canonical);
    assert_eq!(result, canonical);
}

// T016a: Directory symlink detection
#[cfg(windows)]
#[test]
fn detect_dir_symlink_has_mapping() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("symlink");

    std::fs::create_dir_all(&real_dir).unwrap();

    let mut guard = WinEnvGuard::new();
    if create_dir_symlink(&link_dir, &real_dir).is_err() {
        // Skip if directory symlinks require elevation
        return;
    }
    guard.track_junction(link_dir.clone()); // rd works for dir symlinks too
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();
    assert!(ctx.has_mapping());

    let canonical = real_dir.join("file.txt");
    std::fs::write(&canonical, b"test").unwrap();
    let result = ctx.to_logical(&canonical);
    assert_eq!(result, link_dir.join("file.txt"));
}

// T016b: Junction round-trip (SC-005)
#[cfg(windows)]
#[test]
fn junction_roundtrip() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("link");
    let subdir = real_dir.join("src");

    std::fs::create_dir_all(&subdir).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();
    let canonical = real_dir.join("src");
    let logical = ctx.to_logical(&canonical);
    let back = ctx.to_canonical(&logical);
    assert_eq!(back, canonical);
}

// ===== Phase 4: US2 — Subst Drive Detection =====

// T017: Subst drive detection
#[cfg(windows)]
#[test]
fn detect_subst_has_mapping() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());

    let letter =
        find_unused_drive_letter().expect("no unused drive letter available for subst test");
    let mut guard = WinEnvGuard::new();
    create_subst(letter, &base).expect("subst");
    guard.track_subst(letter);
    guard.set_cwd(std::path::Path::new(&format!("{letter}:\\")));

    let ctx = LogicalPathContext::detect();
    assert!(ctx.has_mapping());
}

// T018: Subst to_logical translation
#[cfg(windows)]
#[test]
fn detect_subst_to_logical() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let subdir = base.join("src");
    std::fs::create_dir_all(&subdir).unwrap();

    let letter =
        find_unused_drive_letter().expect("no unused drive letter available for subst test");
    let mut guard = WinEnvGuard::new();
    create_subst(letter, &base).expect("subst");
    guard.track_subst(letter);
    guard.set_cwd(std::path::Path::new(&format!("{letter}:\\")));

    let ctx = LogicalPathContext::detect();
    let canonical = subdir;
    let result = ctx.to_logical(&canonical);
    assert_eq!(result, std::path::PathBuf::from(format!(r"{letter}:\src")));
}

// T019: Subst to_logical with path outside mapping
#[cfg(windows)]
#[test]
fn subst_to_logical_outside_mapping() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());

    let letter =
        find_unused_drive_letter().expect("no unused drive letter available for subst test");
    let mut guard = WinEnvGuard::new();
    create_subst(letter, &base).expect("subst");
    guard.track_subst(letter);
    guard.set_cwd(std::path::Path::new(&format!("{letter}:\\")));

    let ctx = LogicalPathContext::detect();
    let outside = std::path::Path::new(r"C:\Windows\System32");
    assert_eq!(ctx.to_logical(outside), outside.to_path_buf());
}

// T020: Subst removed after detect → fallback
#[cfg(windows)]
#[test]
fn subst_removed_fallback() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let subdir = base.join("src");
    std::fs::create_dir_all(&subdir).unwrap();

    let letter =
        find_unused_drive_letter().expect("no unused drive letter available for subst test");
    let mut guard = WinEnvGuard::new();
    create_subst(letter, &base).expect("subst");
    guard.track_subst(letter);
    guard.set_cwd(std::path::Path::new(&format!("{letter}:\\")));

    let ctx = LogicalPathContext::detect();

    // Move CWD away and remove subst
    guard.set_cwd(&base);
    let _ = remove_subst(letter);
    guard.untrack_subst(letter);

    let result = ctx.to_logical(&subdir);
    assert_eq!(result, subdir);
}

// T020a: Subst round-trip (SC-005)
#[cfg(windows)]
#[test]
fn subst_roundtrip() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let subdir = base.join("src");
    std::fs::create_dir_all(&subdir).unwrap();

    let letter =
        find_unused_drive_letter().expect("no unused drive letter available for subst test");
    let mut guard = WinEnvGuard::new();
    create_subst(letter, &base).expect("subst");
    guard.track_subst(letter);
    guard.set_cwd(std::path::Path::new(&format!("{letter}:\\")));

    let ctx = LogicalPathContext::detect();
    let canonical = subdir;
    let logical = ctx.to_logical(&canonical);
    let back = ctx.to_canonical(&logical);
    assert_eq!(back, canonical);
}

// ===== Phase 5: US5 — Graceful Fallback on Windows =====

// T023: Junction retargeted → stale mapping → fallback
#[cfg(windows)]
#[test]
fn junction_retarget_fallback() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir1 = base.join("real1");
    let real_dir2 = base.join("real2");
    let link_dir = base.join("link");
    let subdir = real_dir1.join("src");

    std::fs::create_dir_all(&subdir).unwrap();
    std::fs::create_dir_all(real_dir2.join("src")).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir1).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();

    // Retarget junction to a different directory
    guard.set_cwd(&base);
    remove_junction(&link_dir).expect("remove junction");
    create_junction(&link_dir, &real_dir2).expect("mklink /J new target");

    // to_logical should fall back — round-trip validation catches stale mapping
    let canonical = real_dir1.join("src");
    let result = ctx.to_logical(&canonical);
    assert_eq!(result, canonical);
}

// ===== Phase 7: US4 — Translate Logical-to-Canonical on Windows =====

// T026: to_canonical with active junction mapping
#[cfg(windows)]
#[test]
fn junction_to_canonical() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("link");
    let subdir = real_dir.join("src");

    std::fs::create_dir_all(&subdir).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();
    let logical = link_dir.join("src");
    let result = ctx.to_canonical(&logical);
    assert_eq!(result, real_dir.join("src"));
}

// T027: to_canonical with path outside junction prefix
#[cfg(windows)]
#[test]
fn junction_to_canonical_outside_prefix() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());
    let real_dir = base.join("real");
    let link_dir = base.join("link");

    std::fs::create_dir_all(&real_dir).unwrap();

    let mut guard = WinEnvGuard::new();
    create_junction(&link_dir, &real_dir).expect("mklink /J");
    guard.track_junction(link_dir.clone());
    guard.set_cwd(&link_dir);

    let ctx = LogicalPathContext::detect();
    let outside = std::path::Path::new(r"C:\Windows\System32");
    assert_eq!(ctx.to_canonical(outside), outside.to_path_buf());
}

// T028: to_canonical with no mapping
#[cfg(windows)]
#[test]
fn no_mapping_to_canonical_unchanged() {
    use windows_helpers::*;

    let dir = tempfile::tempdir().unwrap();
    let base = strip_extended_prefix(std::fs::canonicalize(dir.path()).unwrap());

    let guard = WinEnvGuard::new();
    guard.set_cwd(&base);

    let ctx = LogicalPathContext::detect();
    let input = std::path::Path::new(r"C:\Users\dev\project\src\main.rs");
    assert_eq!(ctx.to_canonical(input), input.to_path_buf());
}
