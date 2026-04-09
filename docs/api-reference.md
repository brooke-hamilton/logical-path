# API Reference

This guide covers the public API of the `logical-path` crate in detail, with usage patterns and examples for each method.

> **See also**: The generated [API documentation on docs.rs](https://docs.rs/logical-path) for the full rustdoc reference.

## `LogicalPathContext`

The primary (and only) public type. Holds zero or one detected prefix mappings between canonical and logical paths. Immutable after construction.

### Trait Implementations

| Trait | Behavior |
| ----- | -------- |
| `Debug` | Prints the internal mapping state for diagnostics |
| `Clone` | Cheap clone (two `PathBuf` fields at most) |
| `PartialEq`, `Eq` | Structural equality comparison |
| `Default` | Returns a context with no active mapping |
| `Send`, `Sync` | Safe to share across threads (auto-derived) |

### `LogicalPathContext::detect()`

```rust
#[must_use]
pub fn detect() -> LogicalPathContext
```

Detect the active symlink prefix mapping by comparing `$PWD` (logical) against `getcwd()` (canonical).

**Returns** a `LogicalPathContext` value. This function never fails — if detection cannot determine a valid mapping, the returned context simply has no active mapping.

**Returns no mapping when**:

- `$PWD` is unset or empty
- `$PWD` equals the canonical CWD (no symlink in effect)
- `$PWD` is stale (points to a non-existent directory)
- `$PWD` doesn't resolve to the same canonical CWD
- The current directory cannot be determined
- Running on Windows

**Usage pattern**: Call once at program startup and reuse the context for the lifetime of the process. If the environment changes (e.g., the user `cd`s elsewhere), call `detect()` again.

```rust
use logical_path::LogicalPathContext;

let ctx = LogicalPathContext::detect();
// Reuse `ctx` for all path translations in this session.
```

**Thread safety**: The returned context is `Send + Sync` and can be shared across threads via `Arc<LogicalPathContext>` or stored in a `lazy_static`/`OnceLock`.

### `LogicalPathContext::has_mapping()`

```rust
#[must_use]
pub fn has_mapping(&self) -> bool
```

Returns `true` if an active prefix mapping was detected.

When this returns `false`, `to_logical()` and `to_canonical()` will always return their input unchanged. Useful for short-circuiting or diagnostics.

```rust
use logical_path::LogicalPathContext;

let ctx = LogicalPathContext::detect();

if ctx.has_mapping() {
    println!("Symlink prefix mapping is active");
} else {
    println!("No symlink mapping detected — paths will pass through unchanged");
}
```

### `LogicalPathContext::to_logical()`

```rust
#[must_use]
pub fn to_logical(&self, path: &Path) -> PathBuf
```

Translate a canonical (symlink-resolved) path to its logical (symlink-preserving) equivalent.

**Parameters**:

- `path` — A canonical path, typically from `std::fs::canonicalize()`, `std::env::current_dir()`, or a tool like `git` that returns resolved paths.

**Returns** the logical equivalent if translation succeeds, or the input path unchanged as a fallback.

**Falls back to the input when**:

- No active mapping exists
- The path doesn't start with the canonical prefix
- The path is relative
- Round-trip validation fails
- The path doesn't exist on disk (required for validation)

**Example**:

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

let ctx = LogicalPathContext::detect();

// If $PWD=/workspace/project and CWD=/mnt/wsl/workspace/project:
let canonical = Path::new("/mnt/wsl/workspace/project/src/main.rs");
let logical = ctx.to_logical(canonical);
// logical == "/workspace/project/src/main.rs" (if mapping is active)
// logical == "/mnt/wsl/workspace/project/src/main.rs" (if no mapping)
```

**Idempotence**: Calling `to_logical()` on a path that is already in logical form returns the path unchanged (it won't match the canonical prefix, so the fallback applies).

### `LogicalPathContext::to_canonical()`

```rust
#[must_use]
pub fn to_canonical(&self, path: &Path) -> PathBuf
```

Translate a logical (symlink-preserving) path to its canonical (symlink-resolved) equivalent.

**Parameters**:

- `path` — A logical path, typically from `$PWD` or user input.

**Returns** the canonical equivalent if translation succeeds, or the input path unchanged as a fallback.

**Falls back to the input when**:

- No active mapping exists
- The path doesn't start with the logical prefix
- The path is relative
- Round-trip validation fails
- The path doesn't exist on disk (required for validation)

**Example**:

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

let ctx = LogicalPathContext::detect();

// If $PWD=/workspace/project and CWD=/mnt/wsl/workspace/project:
let logical = Path::new("/workspace/project/src/main.rs");
let canonical = ctx.to_canonical(logical);
// canonical == "/mnt/wsl/workspace/project/src/main.rs" (if mapping is active)
// canonical == "/workspace/project/src/main.rs" (if no mapping)
```

### `Default` Implementation

```rust
let ctx = LogicalPathContext::default();
assert!(!ctx.has_mapping());
```

Returns a context with no active mapping. Useful as a placeholder or in tests where no symlink translation is needed. Equivalent to calling `detect()` in an environment with no symlinks.

## Common Patterns

### Detect Once, Reuse Everywhere

```rust
use logical_path::LogicalPathContext;
use std::sync::OnceLock;

static CTX: OnceLock<LogicalPathContext> = OnceLock::new();

fn get_ctx() -> &'static LogicalPathContext {
    CTX.get_or_init(LogicalPathContext::detect)
}
```

### Translating Multiple Paths

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

fn display_paths(paths: &[&Path]) {
    let ctx = LogicalPathContext::detect();
    for path in paths {
        let display = ctx.to_logical(path);
        println!("{}", display.display());
    }
}
```

### Conditional Logic Based on Mapping State

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

fn emit_cd(target: &Path) -> String {
    let ctx = LogicalPathContext::detect();
    let display_path = ctx.to_logical(target);

    if ctx.has_mapping() {
        format!("cd {} # (translated from {})", display_path.display(), target.display())
    } else {
        format!("cd {}", display_path.display())
    }
}
```

## Important Notes

### Path Existence Requirement

Both `to_logical()` and `to_canonical()` call `std::fs::canonicalize()` internally for round-trip validation. This means **the path must exist on disk** for translation to succeed. If the path doesn't exist, the fallback (input unchanged) is returned.

This is a deliberate design choice — validating correctness is more important than translating hypothetical paths.

### Relative Paths

Relative paths (e.g., `src/main.rs`, `../README.md`) are always returned unchanged. The crate does not resolve relative paths against the current directory. Only absolute paths are eligible for prefix translation.

### Non-UTF-8 Paths

All path operations use `OsStr`/`Path` types. No intermediate conversion to `String` is performed. The crate handles non-UTF-8 path components without panicking.
