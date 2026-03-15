# Public API Contract: logical-path

**Version**: 0.1.0 | **Type**: Rust library crate

## Crate Interface

The crate exposes a single public type and its methods. No other types, traits, or functions are part of the public API.

### `LogicalPathContext`

```rust
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
#[derive(Debug, Clone)]
pub struct LogicalPathContext { /* private fields */ }
```

### Methods

#### `detect()`

```rust
impl LogicalPathContext {
    /// Detect the active symlink prefix mapping by comparing `$PWD` (logical)
    /// against `getcwd()` (canonical).
    ///
    /// Implements the Detect and Map steps of the five-step algorithm
    /// (Detect → Map → Translate → Validate → Fall back). The Translate,
    /// Validate, and Fall back steps are performed by [`to_logical()`] and
    /// [`to_canonical()`].
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
    pub fn detect() -> LogicalPathContext;
}
```

**Preconditions**: None.
**Postconditions**: Returns a valid `LogicalPathContext`. If a prefix mapping was detected, `has_mapping()` returns `true`.
**Error behavior**: Never errors. Gracefully falls back to no-mapping context.

#### `to_logical()`

```rust
impl LogicalPathContext {
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
    /// - Round-trip validation fails
    /// - Canonicalization of the translated path fails (e.g., path doesn't exist)
    ///
    /// # Panics
    ///
    /// This function never panics, even with non-UTF-8 path components.
    pub fn to_logical(&self, path: &Path) -> PathBuf;
}
```

**Preconditions**: `path` should be a canonical path for meaningful translation. Non-canonical paths are accepted but may fall back.
**Postconditions**: Returns a non-empty `PathBuf`. If translation succeeded, `canonicalize(result) == canonicalize(input)`.
**Error behavior**: Never errors. Falls back to returning `path.to_path_buf()`.

#### `to_canonical()`

```rust
impl LogicalPathContext {
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
    /// - Round-trip validation fails
    /// - Canonicalization of the translated path fails
    ///
    /// # Panics
    ///
    /// This function never panics, even with non-UTF-8 path components.
    pub fn to_canonical(&self, path: &Path) -> PathBuf;
}
```

**Preconditions**: `path` should be a logical path for meaningful translation. Non-logical paths are accepted but may fall back.
**Postconditions**: Returns a non-empty `PathBuf`. If translation succeeded, `canonicalize(result) == canonicalize(input)`.
**Error behavior**: Never errors. Falls back to returning `path.to_path_buf()`.

#### `has_mapping()`

```rust
impl LogicalPathContext {
    /// Returns `true` if an active prefix mapping was detected.
    ///
    /// When this returns `false`, `to_logical()` and `to_canonical()` will
    /// always return their input unchanged.
    pub fn has_mapping(&self) -> bool;
}
```

**Preconditions**: None.
**Postconditions**: Returns `true` iff `detect()` found a valid prefix mapping.

## Behavioral Contract

### Fallback Guarantee

For **all** inputs and **all** platform/environment states:

```text
to_logical(path)   → PathBuf   (never panics, never empty)
to_canonical(path)  → PathBuf   (never panics, never empty)
detect()            → LogicalPathContext (never panics)
```

### Round-Trip Property

For any path `p` where translation is applied (not fallen back):

```text
canonicalize(to_logical(canonical_path)) == canonicalize(canonical_path)
canonicalize(to_canonical(logical_path)) == canonicalize(logical_path)
```

### Idempotence

Calling `to_logical()` on an already-logical path returns the input unchanged (the logical prefix doesn't start with the canonical prefix, so no translation is attempted).

### Platform Matrix

| Platform | `detect()` | `to_logical()` / `to_canonical()` |
|----------|-----------|-----------------------------------|
| Linux | Reads `$PWD` vs `getcwd()` | Full translation with validation |
| macOS | Reads `$PWD` vs `getcwd()` | Full translation with validation (handles `/private` naturally) |
| Windows | Returns no-mapping context | Always returns input unchanged |

## Dependencies

- **Runtime**: `std` only (no external crates)
- **Dev/Test**: `tempfile` (for integration tests with real symlinks)
