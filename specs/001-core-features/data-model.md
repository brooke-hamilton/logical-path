# Data Model: logical-path Core Library

**Date**: 2026-03-15 | **Spec**: [spec.md](spec.md)

## Entities

### `LogicalPathContext` (Public)

The primary public type. Encapsulates zero or one active prefix mappings. Immutable after construction. Thread-safe (no interior mutability).

| Field | Type | Description |
|-------|------|-------------|
| `mapping` | `Option<PrefixMapping>` | The detected prefix mapping, or `None` if no active symlink mapping exists. |

**Derives**: `Debug`, `Clone`, `PartialEq`, `Eq`

**Construction**: Via `LogicalPathContext::detect()` associated function only. No public constructor exposes `PrefixMapping` internals. Internally, `detect()` delegates to a `pub(crate) fn detect_from(pwd: Option<&OsStr>, canonical_cwd: &Path) -> LogicalPathContext` helper that accepts its inputs as parameters rather than reading from global state, enabling unit tests to exercise the detection logic without modifying process environment variables.

**Methods**:

| Method | Signature | Description |
|--------|-----------|-------------|
| `detect()` | `fn detect() -> LogicalPathContext` | Reads `$PWD` and `getcwd()`, computes prefix mapping. Never fails. |
| `to_logical()` | `fn to_logical(&self, path: &Path) -> PathBuf` | Translates canonical → logical. Falls back to input unchanged. |
| `to_canonical()` | `fn to_canonical(&self, path: &Path) -> PathBuf` | Translates logical → canonical. Falls back to input unchanged. |
| `has_mapping()` | `fn has_mapping(&self) -> bool` | Returns `true` if an active prefix mapping was detected. |

All four methods are annotated with `#[must_use]` to prevent callers from accidentally discarding return values.

**Thread Safety**: `Send + Sync` (auto-derived — no interior mutability, all fields are `PathBuf` / `Option`).

---

### `PrefixMapping` (Internal)

An internal (non-public) type representing the divergence point between the logical and canonical paths. Not exposed in the public API.

| Field | Type | Description |
|-------|------|-------------|
| `canonical_prefix` | `PathBuf` | The canonical (symlink-resolved) prefix, e.g., `/mnt/wsl/workspace`. |
| `logical_prefix` | `PathBuf` | The logical (symlink-preserving) prefix, e.g., `/workspace`. |

**Derives**: `Debug`, `Clone`, `PartialEq`, `Eq`

**Validation Rules**:

- Both prefixes must be non-empty absolute paths.
- The canonical prefix must differ from the logical prefix (otherwise no mapping is needed).
- The suffixes (path components after the divergence point) must match between the `$PWD` and `getcwd()` paths.

**State Transitions**: None — immutable after construction.

---

## Relationships

```text
LogicalPathContext
  └── mapping: Option<PrefixMapping>
                  ├── canonical_prefix: PathBuf
                  └── logical_prefix: PathBuf
```

- `LogicalPathContext` **has zero or one** `PrefixMapping`.
- `PrefixMapping` is always created inside `detect()` and never modified.
- No circular references. No shared ownership needed.

## Path Types (Standard Library)

These are not custom types but are central to the data model:

| Type | Role |
|------|------|
| `&Path` | Input parameter type for `to_logical()` and `to_canonical()` |
| `PathBuf` | Return type; owned path value |
| `OsString` | Used internally for `$PWD` value from `var_os()` |
| `Component` | Used internally for suffix-matching (path component enumeration) |

## Invariants

1. **Immutability**: Once a `LogicalPathContext` is constructed, its mapping never changes. Callers who need to refresh the mapping must call `detect()` again.
2. **Fallback guarantee**: Every call to `to_logical()` or `to_canonical()` returns a non-empty `PathBuf`. If translation cannot be performed, the input path is returned as-is.
3. **Round-trip correctness**: For any successfully translated path `p'`, `canonicalize(p') == canonicalize(p)` where `p` is the original input.
4. **Existence requirement**: Round-trip validation calls `std::fs::canonicalize()`, which requires paths to exist on disk. Translation of paths to non-existent files will always fall back to returning the input unchanged.
5. **No-panic guarantee**: No method on `LogicalPathContext` panics under any input, including non-UTF-8 paths, missing env vars, or stale symlinks.
