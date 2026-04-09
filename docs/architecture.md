# Architecture

This document describes the internal architecture of the `logical-path` crate: its data model, key types, invariants, and design decisions.

## Overview

`logical-path` is a single-crate Rust library with no runtime dependencies. It translates filesystem paths between two representations:

- **Canonical paths** — symlink-resolved, as returned by `std::fs::canonicalize()` or `std::env::current_dir()`.
- **Logical paths** — symlink-preserving, as stored in the shell's `$PWD` environment variable.

The crate detects the mapping between these two path representations at a single divergence point and provides bidirectional translation.

## Data Model

```text
LogicalPathContext
  └── mapping: Option<PrefixMapping>
                  ├── canonical_prefix: PathBuf
                  └── logical_prefix: PathBuf
```

### `LogicalPathContext` (public)

The sole public type. Encapsulates zero or one active prefix mappings. Immutable after construction.

| Field | Type | Description |
| ----- | ---- | ----------- |
| `mapping` | `Option<PrefixMapping>` | The detected prefix pair, or `None` |

**Traits**: `Debug`, `Clone`, `PartialEq`, `Eq`, `Default`, `Send`, `Sync`

**Construction**: Always via `LogicalPathContext::detect()`. There is no public constructor that exposes `PrefixMapping` internals. The `Default` implementation returns a context with no active mapping.

### `PrefixMapping` (internal)

Not exposed in the public API. Holds the two prefixes that diverge between the logical and canonical paths.

| Field | Type | Description |
| ----- | ---- | ----------- |
| `canonical_prefix` | `PathBuf` | The canonical (resolved) prefix, e.g., `/mnt/wsl/workspace` |
| `logical_prefix` | `PathBuf` | The logical (symlink) prefix, e.g., `/workspace` |

**Invariants**:

- When detection succeeds from a shell-provided absolute `$PWD`, both prefixes are non-empty absolute paths.
- The canonical prefix differs from the logical prefix.
- When detection is based on an absolute `$PWD`, the suffixes (path components after the divergence point) match between `$PWD` and `getcwd()`.

### `TranslationDirection` (internal)

An enum that selects whether `translate()` maps canonical → logical or logical → canonical. Avoids duplicating the translation logic.

## Public API Surface

| Method | Signature | Description |
| ------ | --------- | ----------- |
| `detect()` | `fn detect() -> LogicalPathContext` | Reads `$PWD` and `getcwd()`, computes mapping |
| `has_mapping()` | `fn has_mapping(&self) -> bool` | Returns `true` if an active mapping was detected |
| `to_logical()` | `fn to_logical(&self, path: &Path) -> PathBuf` | Canonical → logical translation |
| `to_canonical()` | `fn to_canonical(&self, path: &Path) -> PathBuf` | Logical → canonical translation |

All four methods are annotated with `#[must_use]`.

## Design Invariants

1. **Immutability** — Once constructed, a `LogicalPathContext` never changes. Callers who need to refresh the mapping call `detect()` again.
2. **Fallback guarantee** — Every call to `to_logical()` or `to_canonical()` returns a non-empty `PathBuf`. If translation cannot be performed, the input path is returned as-is.
3. **Round-trip correctness** — For any successfully translated path `p'`, `canonicalize(p') == canonicalize(p)` where `p` is the original input.
4. **Existence requirement** — Round-trip validation calls `std::fs::canonicalize()`, which requires paths to exist on disk. Paths to non-existent files always fall back.
5. **No-panic guarantee** — No public method panics under any input, including non-UTF-8 paths, missing environment variables, or stale symlinks.

## Error Handling Strategy

The crate uses no `Result` types in its public API. All errors are handled internally by falling back to the input path:

- `$PWD` unset → no mapping
- `$PWD` stale (non-existent) → no mapping
- `canonicalize()` fails → fallback to input
- Round-trip validation fails → fallback to input
- Windows platform → no mapping

This design makes the library safe to adopt unconditionally. Callers who need to detect whether translation occurred can compare the input and output.

## Module Layout

The crate is a single `lib.rs` file. All types and functions live in the crate root:

```text
src/
  lib.rs          # All public and internal types, detection, translation, tests
tests/
  integration.rs  # Integration tests with real symlinks on the filesystem
```

Unit tests are colocated in `lib.rs` under `#[cfg(test)] mod tests`. Integration tests that mutate process-global state (`$PWD`, CWD) live in `tests/integration.rs` and use a mutex-based `EnvGuard` to serialize environment mutations.

## Testability Seam

The public `detect()` reads from process-global state (`$PWD` and CWD). To enable unit testing without mutating global state, detection logic is extracted into a `pub(crate)` helper:

```rust
pub(crate) fn detect_from(pwd: Option<&OsStr>, canonical_cwd: &Path) -> LogicalPathContext
```

This allows unit tests to pass arbitrary `$PWD` and CWD values directly, while integration tests exercise the real `detect()` with actual symlinks on the filesystem.

## Dependencies

- **Runtime**: None. The crate depends only on the Rust standard library.
- **Dev**: [`tempfile`](https://crates.io/crates/tempfile) for creating temporary directories and symlinks in integration tests.

## Platform Compilation

Platform-specific code is gated with `#[cfg]` attributes:

- `#[cfg(not(windows))]` — Detection and suffix-matching logic (Unix-only)
- `#[cfg(windows)]` — Returns `None` mapping (no `$PWD` equivalent)
- `#[cfg(target_os = "macos")]` — macOS-specific tests
- `#[cfg(target_os = "linux")]` — Linux-specific tests
