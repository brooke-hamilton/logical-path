use logical_path::LogicalPathContext;

// T012: detect() inside a real symlink directory returns context with has_mapping() == true
#[cfg(unix)]
#[test]
fn detect_inside_real_symlink_has_mapping() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let real_base = dir.path().join("real");
    let link_base = dir.path().join("link");
    let subdir = "project";

    std::fs::create_dir_all(real_base.join(subdir)).unwrap();
    symlink(&real_base, &link_base).unwrap();

    // Simulate: canonical CWD = real_base/project, $PWD = link_base/project
    let ctx = LogicalPathContext::detect_from(
        Some(link_base.join(subdir).as_os_str()),
        &real_base.join(subdir),
    );

    assert!(ctx.has_mapping());
}

// T012a: detect() with nested symlinks (symlink through another symlink)
// detects only the outermost divergence mapping
#[cfg(unix)]
#[test]
fn detect_nested_symlinks_detects_outermost_divergence() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let real_base = dir.path().join("real");
    let link1 = dir.path().join("link1");
    let link2 = dir.path().join("link2");
    let subdir = "project";

    std::fs::create_dir_all(real_base.join(subdir)).unwrap();
    // link1 -> real
    symlink(&real_base, &link1).unwrap();
    // link2 -> link1 (nested symlink)
    symlink(&link1, &link2).unwrap();

    // canonical CWD resolves both symlinks: real/project
    // $PWD follows the outermost symlink: link2/project
    let ctx = LogicalPathContext::detect_from(
        Some(link2.join(subdir).as_os_str()),
        &real_base.join(subdir),
    );

    assert!(ctx.has_mapping());
}

// T020: to_logical() with real symlink environment
#[cfg(unix)]
#[test]
fn to_logical_with_real_symlink() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let real_base = dir.path().join("real");
    let link_base = dir.path().join("link");

    std::fs::create_dir_all(real_base.join("src")).unwrap();
    symlink(&real_base, &link_base).unwrap();

    let ctx = LogicalPathContext::detect_from(
        Some(link_base.join("src").as_os_str()),
        &real_base.join("src"),
    );

    let canonical_path = real_base.join("src");
    let result = ctx.to_logical(&canonical_path);
    assert_eq!(result, link_base.join("src"));
}

// T027: to_canonical() with real symlink environment
#[cfg(unix)]
#[test]
fn to_canonical_with_real_symlink() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let real_base = dir.path().join("real");
    let link_base = dir.path().join("link");

    std::fs::create_dir_all(real_base.join("src")).unwrap();
    symlink(&real_base, &link_base).unwrap();

    let ctx = LogicalPathContext::detect_from(
        Some(link_base.join("src").as_os_str()),
        &real_base.join("src"),
    );

    let logical_path = link_base.join("src");
    let result = ctx.to_canonical(&logical_path);
    assert_eq!(result, real_base.join("src"));
}

// ===== US5: Cross-platform tests =====

// T036: Platform-gated test for Linux
#[cfg(target_os = "linux")]
#[test]
fn detect_with_real_symlink_on_linux() {
    use std::os::unix::fs::symlink;

    let dir = tempfile::tempdir().unwrap();
    let real_base = dir.path().join("real");
    let link_base = dir.path().join("link");

    std::fs::create_dir_all(real_base.join("project")).unwrap();
    symlink(&real_base, &link_base).unwrap();

    let ctx = LogicalPathContext::detect_from(
        Some(link_base.join("project").as_os_str()),
        &real_base.join("project"),
    );

    assert!(ctx.has_mapping());

    // Verify translation works
    let canonical = real_base.join("project");
    let logical = ctx.to_logical(&canonical);
    assert_eq!(logical, link_base.join("project"));
}

// T037: Platform-gated test for macOS — handles /private prefix
#[cfg(target_os = "macos")]
#[test]
fn detect_handles_macos_private_prefix() {
    // On macOS, /var is a symlink to /private/var
    // detect_from with $PWD=/var/... and canonical=/private/var/... should detect mapping
    let ctx = LogicalPathContext::detect_from(
        Some(std::ffi::OsStr::new("/var/folders")),
        std::path::Path::new("/private/var/folders"),
    );

    assert!(ctx.has_mapping());
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
