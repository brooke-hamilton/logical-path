# logical-path

[![Crates.io](https://img.shields.io/crates/v/logical-path.svg)](https://crates.io/crates/logical-path)
[![Docs.rs](https://docs.rs/logical-path/badge.svg)](https://docs.rs/logical-path)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/brooke-hamilton/logical-path/actions/workflows/ci.yml/badge.svg)](https://github.com/brooke-hamilton/logical-path/actions/workflows/ci.yml)

**Translate canonical (symlink-resolved) filesystem paths back to their logical (symlink-preserving) equivalents.**

## The Problem

Rust CLI tools that display filesystem paths or emit `cd` directives silently resolve symlinks, moving users out of their logical directory tree. The root cause is that Rust's standard library provides no way to work with *logical* paths — only *physical* ones:

- **`std::env::current_dir()`** calls `getcwd(2)`, which returns the physical path.
- **`std::fs::canonicalize()`** resolves all symlinks by design.

The user's logical path lives in `$PWD`, a shell convention that `std` intentionally ignores. Any tool that calls these APIs and then shows a path to the user — or writes a `cd` directive for shell integration — will silently teleport the user from `/workspace/project` to `/mnt/wsl/workspace/project`.

## Why Not an Existing Crate?

| Crate | What it does | Gap |
| ----- | ------------ | --- |
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
use std::path::Path;

fn emit_cd_directive(target: &Path) {
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

> **Note:** The Validate step calls `std::fs::canonicalize()`, which requires the path to exist on disk. Translation of paths to non-existent files will always fall back to returning the input unchanged.

## Platform Notes

| | Linux | macOS | Windows |
| --- | ----- | ----- | ------- |
| Logical path source | `$PWD` | `$PWD` | No direct equivalent |
| System symlinks | User-created only | `/var`→`/private/var`, `/tmp`→`/private/tmp` | NTFS junctions, directory symlinks |
| Case sensitivity | Yes | No (APFS default) | No |
| `canonicalize()` quirks | None | `/private` prefixing | `\\?\` UNC prefix |

**macOS note:** System-level symlinks like `/var` → `/private/var` trigger this bug even without any user-created symlinks.

**Windows note:** `$PWD` has no direct OS-level equivalent. `subst` drive and junction detection is a known limitation; follow the tracking issue for updates.

## Use Cases

Any Rust CLI tool that:

- Writes `cd` directives for shell integration
- Displays filesystem paths to users
- Compares paths from different sources (e.g., `git worktree list` output vs the current directory)

Common environments: WSL with mounted VHDs, NFS/network mounts, macOS `/var`/`/tmp`, custom workspace symlinks.

## Documentation

For more in-depth documentation beyond this README, see the [`docs/`](docs/) directory:

- **[Architecture](docs/architecture.md)** — Data model, design invariants, module layout, and testability seams.
- **[How It Works](docs/how-it-works.md)** — Step-by-step walkthrough of the detection and translation algorithm with visual examples.
- **[API Reference](docs/api-reference.md)** — Detailed guide to every public method, with usage patterns and code examples.
- **[Platform Behavior](docs/platform-behavior.md)** — How the crate behaves on Linux, macOS, and Windows, including known quirks and limitations.
- **[Examples](docs/examples.md)** — Real-world integration patterns: shell directives, git worktrees, global context, and more.
- **[FAQ](docs/FAQ.md)** — Frequently asked questions about edge cases, design decisions, and platform support.

API documentation is also available on [docs.rs](https://docs.rs/logical-path).

## Contributing

Contributions are welcome! Please open an issue to discuss any significant changes before submitting a pull request. Bug reports, feature requests, and platform-specific test cases are especially appreciated.

## Minimum Supported Rust Version (MSRV)

The minimum supported Rust version is **1.85.0** (required by edition 2024). The MSRV is not changed without a minor-version bump.

## License

Licensed under the [MIT License](LICENSE).
