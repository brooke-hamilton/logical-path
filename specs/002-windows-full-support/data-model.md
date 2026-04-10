# Data Model: Windows Full Support

## Entities

### LogicalPathContext (existing — unchanged public API)

The primary public type. Holds zero or one active prefix mapping.

| Field | Type | Description |
| ----- | ---- | ----------- |
| `mapping` | `Option<PrefixMapping>` | The detected prefix mapping, if any. Private field. |

**Derives**: `Debug`, `Clone`, `PartialEq`, `Eq`

**Traits**: `Default` (no mapping), `Send + Sync` (auto-derived, immutable after construction)

**Public methods** (unchanged):

- `detect() -> LogicalPathContext` — detects active prefix mapping from process environment
- `has_mapping() -> bool` — whether a mapping was detected
- `to_logical(&self, path: &Path) -> PathBuf` — translate canonical → logical
- `to_canonical(&self, path: &Path) -> PathBuf` — translate logical → canonical

### PrefixMapping (existing — unchanged)

Internal type representing the divergence between logical and canonical path prefixes.

| Field | Type | Description |
| ----- | ---- | ----------- |
| `canonical_prefix` | `PathBuf` | The resolved physical path prefix (e.g., `D:\projects\workspace`) |
| `logical_prefix` | `PathBuf` | The user-facing path prefix (e.g., `C:\workspace` for junctions, `S:\` for subst) |

**Derives**: `Debug`, `Clone`, `PartialEq`, `Eq`

### TranslationDirection (existing — unchanged)

Internal enum for directing the translate helper.

| Variant | Source Prefix | Target Prefix |
| ------- | ------------- | ------------- |
| `ToLogical` | `canonical_prefix` | `logical_prefix` |
| `ToCanonical` | `logical_prefix` | `canonical_prefix` |

## New Internal Functions

### `strip_extended_length_prefix(path: &Path) -> PathBuf` — `#[cfg(windows)]`

Strips `\\?\` Extended Length Path prefixes from Windows paths.

| Input Pattern | Output |
| ------------- | ------ |
| `\\?\C:\Users\dev\project` | `C:\Users\dev\project` |
| `\\?\UNC\server\share\folder` | `\\server\share\folder` |
| `C:\Users\dev\project` (no prefix) | `C:\Users\dev\project` (unchanged) |

**Rules**:

1. If the path starts with `\\?\UNC\`, replace with `\\` and return
2. If the path starts with `\\?\` followed by a drive letter and `:\`, strip the first 4 characters
3. Otherwise, return unchanged

### `find_divergence_point(canonical: &Path, logical: &Path) -> Option<(PathBuf, PathBuf)>` — refactored

Currently `#[cfg(not(windows))]`. Must become cross-platform with platform-conditional comparison:

- **Unix**: Component comparison uses `==` (case-sensitive, as today)
- **Windows**: Component comparison uses `OsStr::eq_ignore_ascii_case()` (case-insensitive)

The algorithm is unchanged: walk from the end, count matching suffix components, extract divergent prefixes.

### `detect_from_cwd(cwd: &Path, canonical_cwd: &Path) -> LogicalPathContext` — `#[cfg(windows)]`

Internal testability helper for Windows (analogous to existing `detect_from` on Unix).

| Parameter | Source | Description |
| --------- | ------ | ----------- |
| `cwd` | `std::env::current_dir()` | The logical CWD (preserves junctions/subst) |
| `canonical_cwd` | `std::fs::canonicalize(cwd)` with `\\?\` stripped | The resolved physical CWD |

**Returns**: `LogicalPathContext` with a mapping if `cwd != canonical_cwd` and the divergence algorithm finds a prefix pair.

## State Transitions

```text
                     detect()
                        │
         ┌──────────────┴──────────────┐
         │                             │
    [Unix path]                  [Windows path]
         │                             │
    Read $PWD                    current_dir()
    current_dir()                canonicalize()
    Validate $PWD                Strip \\?\ prefix
    staleness                    (no staleness check)
         │                             │
         └──────────────┬──────────────┘
                        │
              find_divergence_point()
              (case-sensitive on Unix,
               case-insensitive on Windows)
                        │
                ┌───────┴───────┐
                │               │
          [divergent]     [identical]
                │               │
          PrefixMapping     mapping: None
          stored            (fallback context)
```

## Validation Rules

- `strip_extended_length_prefix` must never fail — returns input unchanged for unrecognized patterns
- `find_divergence_point` returns `None` when paths are identical, have no common suffix, or have equal prefixes
- Round-trip validation in `translate()` must apply `\\?\` stripping to canonicalized results on Windows before comparison
- Case-insensitive comparison applies only to suffix-matching during divergence detection and round-trip validation on Windows, not to the returned path values (which preserve original casing)

## Relationships

```text
LogicalPathContext
    └── Option<PrefixMapping>
            ├── canonical_prefix: PathBuf
            └── logical_prefix: PathBuf

detect() ──uses──> find_divergence_point()
                         │
         ┌───────────────┴───────────────┐
         │                               │
    [Unix: ==]              [Windows: eq_ignore_ascii_case]

translate() ──uses──> strip_prefix + join
                      canonicalize (round-trip)
                      strip_extended_length_prefix [Windows]
```
