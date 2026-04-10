# Public API Contract: Windows Full Support

## Overview

The public API surface of `logical-path` is **unchanged** by this feature. No new public types, methods, or traits are added. The existing API gains Windows-functional behavior where it previously returned no-op fallbacks.

## Public API Surface

### `LogicalPathContext`

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalPathContext { /* private fields */ }

impl Default for LogicalPathContext { /* returns no-mapping context */ }

impl LogicalPathContext {
    /// Detect the active prefix mapping from the process environment.
    ///
    /// - Unix: compares $PWD against getcwd()
    /// - Windows: compares current_dir() against canonicalize(current_dir())
    ///   with \\?\ prefix stripping
    ///
    /// Never panics. Returns no-mapping context on any failure.
    #[must_use]
    pub fn detect() -> LogicalPathContext;

    /// Returns true if a prefix mapping was detected.
    #[must_use]
    pub fn has_mapping(&self) -> bool;

    /// Translate canonical path → logical path.
    /// Returns input unchanged on failure or no mapping.
    /// Never panics.
    #[must_use]
    pub fn to_logical(&self, path: &Path) -> PathBuf;

    /// Translate logical path → canonical path.
    /// Returns input unchanged on failure or no mapping.
    /// Never panics.
    #[must_use]
    pub fn to_canonical(&self, path: &Path) -> PathBuf;
}
```

### Behavioral Contract Changes (Windows)

| Method | Before (Windows) | After (Windows) |
| ------ | ---------------- | --------------- |
| `detect()` | Always returns no-mapping context | Detects junctions, directory symlinks, subst drives, mapped drives via `current_dir()` vs `canonicalize()` comparison |
| `has_mapping()` | Always `false` | `true` when indirection detected, `false` otherwise |
| `to_logical()` | Always returns input unchanged | Translates canonical → logical when mapping active; returns input unchanged on failure |
| `to_canonical()` | Always returns input unchanged | Translates logical → canonical when mapping active; returns input unchanged on failure |

### Behavioral Contract Preserved (Unix)

All existing Unix behavior is unchanged:

- `detect()` reads `$PWD` and compares against `getcwd()`
- `$PWD` staleness validation is applied
- Path comparison is case-sensitive
- No `\\?\` prefix handling

### Error Contract

**Unchanged**: No method in the public API returns `Result` or panics. All failure modes produce fallback behavior (input returned unchanged).

### Thread Safety Contract

**Unchanged**: `LogicalPathContext` is `Send + Sync`. Immutable after construction.

### Dependency Contract

| Dependency | Type | Platform | Impact on Callers |
| ---------- | ---- | -------- | ----------------- |
| `log` (new) | Runtime | All | Callers may optionally configure a logger to see trace diagnostics. No logger required. Zero overhead when unconfigured. |

Non-Windows platforms: no new dependencies beyond `log`.

### Diagnostic Contract (new)

When a `log`-compatible logger is active at `trace` or `debug` level:

- `detect()` logs the values of `current_dir()` and `canonicalize()` being compared
- `detect()` logs whether a mapping was detected and the prefix pair
- `to_logical()` / `to_canonical()` log fallback reasons when round-trip validation fails or path is outside the mapped prefix

Diagnostic messages are implementation details and not part of the stable API. Message format may change between versions.
