# Platform Behavior

This document covers how `logical-path` behaves on each supported platform, including known quirks and limitations.

## Linux

**Status**: Fully supported.

### How Detection Works

- **Logical path source**: `$PWD` environment variable, maintained by the shell.
- **Canonical path source**: `std::env::current_dir()` → `getcwd(2)`.
- **`canonicalize()` behavior**: Returns a clean absolute path with all symlinks resolved. No platform-specific prefixing.

### Common Scenarios

| Scenario | `$PWD` | `getcwd()` | Result |
| -------- | ------ | ---------- | ------ |
| WSL with mounted VHD | `/workspace/project` | `/mnt/wsl/workspace/project` | Mapping: `/mnt/wsl` ↔ `/` |
| NFS/network mount symlink | `/data/project` | `/nfs/server1/data/project` | Mapping: `/nfs/server1` ↔ `/` |
| Custom workspace symlink | `/home/user/work` | `/opt/projects/work` | Mapping: `/opt/projects` ↔ `/home/user` |
| No symlinks | `/home/user/project` | `/home/user/project` | No mapping |

### Case Sensitivity

Linux filesystems are case-sensitive by default. Path component matching is exact byte comparison. This is correct behavior — `/Workspace` and `/workspace` are genuinely different paths.

## macOS

**Status**: Fully supported.

### Detection on macOS

- **Logical path source**: `$PWD` environment variable, maintained by the shell.
- **Canonical path source**: `std::env::current_dir()` → `getcwd(2)`.
- **`canonicalize()` behavior**: Resolves all symlinks, including system-level ones. Adds `/private` prefix for system symlinks.

### System Symlinks

macOS has built-in symlinks that affect most development tools:

| Symlink | Target | Effect |
| ------- | ------ | ------ |
| `/var` | `/private/var` | `canonicalize("/var/...")` → `/private/var/...` |
| `/tmp` | `/private/tmp` | `canonicalize("/tmp/...")` → `/private/tmp/...` |
| `/etc` | `/private/etc` | `canonicalize("/etc/...")` → `/private/etc/...` |

These symlinks trigger the canonical/logical path divergence even without any user-created symlinks. For example, any Rust tool that calls `canonicalize()` on a path under `/var` will silently switch to the `/private/var` equivalent.

The generic suffix-matching algorithm handles these without any macOS-specific code:

```text
$PWD     = /var/folders/xyz/T/test
getcwd() = /private/var/folders/xyz/T/test

Suffix match: var / folders / xyz / T / test
Canonical prefix: /private
Logical prefix: /
```

### Case Sensitivity on macOS

APFS (the default macOS filesystem) is case-insensitive but case-preserving. The `logical-path` crate performs exact byte comparison on path components. This means:

- `/Users/brooke/project` and `/users/brooke/project` are treated as different paths by the crate.
- In practice, this rarely causes issues because `$PWD` and `getcwd()` use consistent casing, and the suffix-matching algorithm compares these two sources against each other.
- Callers on case-insensitive filesystems should normalize casing before comparing translated paths if case-insensitive comparison is needed.

## Windows

**Status**: Fully supported. Detects NTFS junctions, directory symlinks, `subst` drives, and mapped network drives.

### Detection on Windows

- **Logical path source**: `std::env::current_dir()` → `GetCurrentDirectoryW`. Preserves junctions, subst drives, directory symlinks, and mapped network drives because the OS maintains the process CWD as-is.
- **Canonical path source**: `std::fs::canonicalize()` → `GetFinalPathNameByHandleW`. Resolves all indirections to the physical path and prepends the `\\?\` Extended Length Path prefix.
- **`\\?\` stripping**: The library strips the `\\?\` prefix from `canonicalize()` output before any comparison or prefix matching. Two forms are handled:
  - `\\?\C:\...` → `C:\...` (local paths)
  - `\\?\UNC\server\share\...` → `\\server\share\...` (UNC paths)

The detection compares `current_dir()` (logical) against `canonicalize(current_dir())` (canonical, with `\\?\` stripped). If the two differ, the suffix-matching divergence algorithm extracts the prefix mapping — the same algorithm used on Unix, but with case-insensitive component comparison.

### No `$PWD` Staleness Check

On Unix, `$PWD` is a user-controlled environment variable that can become stale (e.g., the target directory is deleted or `$PWD` is manually reassigned). The crate validates `$PWD` by canonicalizing it and comparing against `getcwd()`.

On Windows, `current_dir()` is maintained by the OS, not by a shell variable. It is always current by definition, so no staleness check is needed or applied.

### Windows Scenarios

| Scenario | `current_dir()` | `canonicalize()` (stripped) | Result |
| -------- | --------------- | --------------------------- | ------ |
| NTFS junction `C:\workspace` → `D:\projects\workspace` | `C:\workspace\project` | `D:\projects\workspace\project` | Mapping: `D:\projects\workspace` ↔ `C:\workspace` |
| Directory symlink `C:\link` → `D:\target` | `C:\link\src` | `D:\target\src` | Mapping: `D:\target` ↔ `C:\link` |
| `subst S: C:\long\path` | `S:\project` | `C:\long\path\project` | Mapping: `C:\long\path` ↔ `S:\` |
| `net use Z: \\server\share` | `Z:\folder` | `\\server\share\folder` | Mapping: `\\server\share` ↔ `Z:\` |
| No indirections | `C:\Users\dev\project` | `C:\Users\dev\project` | No mapping |

### Case Sensitivity on Windows are case-insensitive but case-preserving. The library uses ordinal case-insensitive comparison (`OsStr::eq_ignore_ascii_case()`) for path component matching during suffix analysis. This ensures that `C:\Workspace` and `C:\workspace` are treated as the same component during divergence detection

The returned path values preserve the original casing from their source — the library never modifies casing in translated paths.

### `\\?\` Extended Length Path Prefix

`std::fs::canonicalize()` on Windows always returns paths with the `\\?\` prefix (e.g., `\\?\C:\Users\dev\project`). The library handles this transparently:

- `\\?\` prefixes are stripped from canonicalized paths during detection.
- `\\?\` prefixes are stripped during round-trip validation in `to_logical()` and `to_canonical()`.
- Callers may pass `\\?\`-prefixed paths to `to_logical()` or `to_canonical()` — the library strips the prefix before prefix matching.

### Fallback Behavior

The library returns the input path unchanged in all conditions where translation cannot be confidently performed:

- No junctions, subst drives, or other indirections in the current directory path
- A junction or subst drive was removed after `detect()` was called (round-trip validation catches this)
- The path is relative
- The path does not start with the mapped prefix
- The path does not exist on disk (required for round-trip validation)

### Trace Diagnostics

When a `log`-compatible logger is active, the library emits trace-level diagnostic messages:

- `detect()` logs the `current_dir()` and `canonicalize()` values being compared
- `detect_from_cwd()` logs whether a mapping was detected and the prefix pair
- `translate()` logs fallback reasons when round-trip validation fails or the path is outside the mapped prefix

## Cross-Platform Compatibility Summary

| Feature | Linux | macOS | Windows |
| ------- | ----- | ----- | ------- |
| Detection | ✅ via `$PWD` vs `getcwd()` | ✅ via `$PWD` vs `getcwd()` | ✅ via `current_dir()` vs `canonicalize()` |
| System symlinks handled | N/A | ✅ (`/private`) | ✅ (junctions, dir symlinks) |
| User symlinks handled | ✅ | ✅ | ✅ (junctions, dir symlinks, subst, mapped drives) |
| Case-sensitive matching | ✅ | ⚠️ (exact bytes) | ✅ (case-insensitive) |
| `\\?\` prefix stripping | N/A | N/A | ✅ (automatic) |
| Safe to call unconditionally | ✅ | ✅ | ✅ |
| Compile and pass tests | ✅ | ✅ | ✅ |

## Conditional Compilation

The crate uses `#[cfg]` attributes to separate platform-specific code:

- `#[cfg(not(windows))]` — Unix detection logic (`$PWD` reading, staleness validation, `detect_from()` helper)
- `#[cfg(windows)]` — Windows detection logic (`current_dir()` vs `canonicalize()`, `\\?\` stripping, `detect_from_cwd()` helper)
- `#[cfg(unix)]` — Integration tests that create real symlinks and unit tests with Unix-style paths
- `#[cfg(target_os = "macos")]` — macOS-specific tests (e.g., `/var` → `/private/var`)
- `#[cfg(target_os = "linux")]` — Linux-specific tests

The suffix-matching divergence algorithm (`find_divergence_point`) and the translation logic (`translate`) are cross-platform. Component comparison uses an internal `components_equal()` helper that dispatches to case-sensitive comparison on Unix and case-insensitive comparison on Windows.

All three platforms are tested in CI.
