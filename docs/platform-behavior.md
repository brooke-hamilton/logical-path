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

**Status**: Detection returns no mapping. All translations pass through unchanged.

### Why Windows Is Not Supported for Detection

Windows has no direct equivalent of `$PWD`:

- PowerShell maintains `$PWD` as a variable, but it is not passed to child processes as an environment variable in the same way as Unix shells.
- `cmd.exe` has no `$PWD` concept.
- `std::env::var_os("PWD")` typically returns `None` on Windows.

Without a reliable logical path source, the crate cannot detect any mapping.

### What Happens on Windows

- `LogicalPathContext::detect()` always returns a context with no mapping.
- `has_mapping()` always returns `false`.
- `to_logical()` and `to_canonical()` always return the input path unchanged.

The crate is safe to use on Windows — it simply acts as a no-op. Cross-platform tools can call `detect()` unconditionally without platform-specific conditionals.

### Known Limitations

- **NTFS junctions**: Not detected. Junctions are a form of symbolic link on Windows, but without a `$PWD` equivalent, the crate cannot determine the user's logical path.
- **`subst` drives**: Not detected. `subst` creates virtual drive letters mapped to directories, but the crate doesn't attempt to detect or translate these.
- **`\\?\` prefix**: `std::fs::canonicalize()` on Windows returns paths with the `\\?\` Extended Length Path prefix. Since no translation occurs on Windows, this prefix is not an issue.

### Future Work

Windows support for `subst` drives and NTFS junctions is a known limitation. Follow the project's issue tracker for updates.

## Cross-Platform Compatibility Summary

| Feature | Linux | macOS | Windows |
| ------- | ----- | ----- | ------- |
| Detection via `$PWD` | ✅ | ✅ | ❌ (no equivalent) |
| System symlinks handled | N/A | ✅ (`/private`) | N/A |
| User symlinks handled | ✅ | ✅ | ❌ |
| Case-sensitive matching | ✅ | ⚠️ (exact bytes) | N/A |
| Safe to call unconditionally | ✅ | ✅ | ✅ (no-op) |
| Compile and pass tests | ✅ | ✅ | ✅ |

## Conditional Compilation

The crate uses `#[cfg]` attributes to separate platform-specific code:

- `#[cfg(not(windows))]` — Detection logic, suffix-matching, and `$PWD` reading
- `#[cfg(windows)]` — Returns no mapping immediately
- `#[cfg(unix)]` — Integration tests that create real symlinks
- `#[cfg(target_os = "macos")]` — macOS-specific tests (e.g., `/var` → `/private/var`)
- `#[cfg(target_os = "linux")]` — Linux-specific tests

All three platforms are tested in CI.
