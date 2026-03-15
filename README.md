# logical-path

[![Crates.io](https://img.shields.io/crates/v/logical-path.svg)](https://crates.io/crates/logical-path)
[![Docs.rs](https://docs.rs/logical-path/badge.svg)](https://docs.rs/logical-path)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/brooke-hamilton/logical-path/actions/workflows/ci.yml/badge.svg)](https://github.com/brooke-hamilton/logical-path/actions/workflows/ci.yml)

**Translate canonical (symlink-resolved) filesystem paths back to their logical (symlink-preserving) equivalents.**

---

## The Problem

Rust CLI tools that display filesystem paths or emit `cd` directives silently resolve symlinks, moving users out of their logical directory tree. The root cause is that Rust's standard library provides no way to work with *logical* paths — only *physical* ones:

- **`std::env::current_dir()`** calls `getcwd(2)`, which returns the physical path.
- **`std::fs::canonicalize()`** resolves all symlinks by design.

The user's logical path lives in `$PWD`, a shell convention that `std` intentionally ignores. Any tool that calls these APIs and then shows a path to the user — or writes a `cd` directive for shell integration — will silently teleport the user from `/workspace/project` to `/mnt/wsl/workspace/project`.

Two independent projects hit this bug and built nearly identical fixes: [worktrunk#968](https://github.com/max-sixty/worktrunk/issues/968) and [microsoft/Sysinternals-jcd#6](https://github.com/microsoft/Sysinternals-jcd/pull/6). The core algorithm each arrived at is ~60 lines; edge cases, cross-platform handling, and tests bring it to 300+. This crate packages that algorithm so you don't have to.

## Why Not an Existing Crate?

| Crate | What it does | Gap |
|---|---|---|
| [`dunce`](https://crates.io/crates/dunce) | Strips `\\?\` from Windows canonical paths | Doesn't preserve symlinks |
| [`path-absolutize`](https://crates.io/crates/path-absolutize) | Makes paths absolute without resolving symlinks | Lexical only; no `$PWD` awareness |
| [`path-dedot`](https://crates.io/crates/path-dedot) | Removes `.`/`..` lexically | Pure string manipulation |
| [`normalize-path`](https://crates.io/crates/normalize-path) | Normalizes separators and dots | Same as above |

These crates help you *avoid* canonicalization. The unserved problem is *undoing* canonicalization after it has already happened — because `git`, OS APIs, or other tools force it.

## Usage

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
logical-path = "0.1"
```

### Quick Start

```rust
use logical_path::LogicalPathContext;

// Detect any active symlink prefix mapping from $PWD vs getcwd().
// Returns None if no symlink is in effect or $PWD is unset.
let ctx = LogicalPathContext::detect();

// Translate a canonical path to its logical (symlink-preserving) equivalent.
// Falls back to the input path unchanged if no mapping applies.
let display_path = ctx.to_logical(&canonical_path);

// Translate a logical path to its canonical equivalent for filesystem operations.
let fs_path = ctx.to_canonical(&logical_path);
```

### Example: Shell Integration

```rust
use logical_path::LogicalPathContext;
use std::path::PathBuf;

fn emit_cd_directive(target: &PathBuf) {
    let ctx = LogicalPathContext::detect();
    // Without this, the user would be teleported to the canonical path.
    let logical = ctx.to_logical(target);
    println!("cd {}", logical.display());
}
```

## Algorithm

`LogicalPathContext::detect()` implements a five-step algorithm:

1. **Detect** — Compare `$PWD` (logical) against `getcwd()` (canonical).
2. **Map** — Suffix-match path components to find the divergence point; extract canonical and logical prefixes.
3. **Translate** — For any canonical path, strip the canonical prefix and prepend the logical prefix.
4. **Validate** — Round-trip check (`canonicalize(translated) == canonicalize(original)`) to catch prefix mappings broad enough to mistranslate unrelated paths.
5. **Fall back** — Return the canonical path unchanged if `$PWD` is unset, stale, or the mapping doesn't apply.

## Platform Notes

| | Linux | macOS | Windows |
|---|---|---|---|
| Logical path source | `$PWD` | `$PWD` | No direct equivalent |
| System symlinks | User-created only | `/var`→`/private/var`, `/tmp`→`/private/tmp` | NTFS junctions, directory symlinks |
| Case sensitivity | Yes | No (APFS default) | No |
| `canonicalize()` quirks | None | `/private` prefixing | `\\?\` UNC prefix |

**macOS note:** System-level symlinks like `/var` → `/private/var` trigger this bug even without any user-created symlinks.

**Windows note:** `$PWD` has no direct OS-level equivalent. `subst` drive and junction detection is a known limitation; follow the tracking issue for updates.

## Who This Affects

Any Rust CLI tool that:

- Writes `cd` directives for shell integration
- Displays filesystem paths to users
- Compares paths from different sources (e.g., `git worktree list` output vs the current directory)

Common environments: WSL with mounted VHDs, NFS/network mounts, macOS `/var`/`/tmp`, custom workspace symlinks.

## Minimum Supported Rust Version (MSRV)

The MSRV will be set once the initial implementation is in place and is determined by the crate's dependencies. The MSRV is not changed without a minor-version bump.

## License

Licensed under the [MIT License](LICENSE).
