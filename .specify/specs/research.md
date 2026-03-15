# Research: logical-path Core Library

**Date**: 2026-03-15 | **Spec**: [spec.md](spec.md)

## R-001: Environment Variable Access for `$PWD`

**Decision**: Use `std::env::var_os("PWD")` to read the logical path from the environment.

**Rationale**: `var_os()` returns `Option<OsString>`, which handles both the "unset" case (`None`) and non-UTF-8 values without panicking. Using `var()` would return `Result<String, VarError>` and fail on non-UTF-8 content, violating FR-009 (no panics on non-UTF-8 path bytes).

**Alternatives considered**:
- `std::env::var("PWD")` — returns `String`, fails on non-UTF-8 content. Rejected because FR-009 requires handling non-UTF-8 paths.

## R-002: Canonical CWD Retrieval

**Decision**: Use `std::env::current_dir()` to obtain the OS-reported canonical current working directory.

**Rationale**: Returns `Result<PathBuf, io::Error>`, maps to `getcwd(2)` on Unix and `GetCurrentDirectoryW` on Windows. Always returns the physical (symlink-resolved) path. `PathBuf` is `OsStr`-backed, so non-UTF-8 safe. If the call fails (e.g., CWD deleted), `detect()` returns a context with no active mapping.

**Alternatives considered**: None — this is the only standard library API for this purpose.

## R-003: Suffix-Matching Algorithm for Divergence Point

**Decision**: Use `Path::components()` collected into `Vec<Component>`, then iterate from the end to find the common suffix between the `$PWD` path and the canonical CWD.

**Rationale**: `components()` normalizes separators and handles `.`/`..`, yielding clean `Component` enum values. Collecting into a `Vec` allows reverse iteration to find the longest common suffix of path components. The divergence point separates the canonical prefix from the logical prefix. This is component-level matching (FR-004), not byte-level string prefix matching.

**Alternatives considered**:
- `Path::ancestors()` — yields complete `&Path` at each level, but comparing full ancestor paths is less direct for suffix-matching than comparing component sequences.
- Manual string splitting — rejected because it would bypass the platform-aware normalization that `components()` provides.

## R-004: Round-Trip Validation

**Decision**: For every translation, call `std::fs::canonicalize()` on both the translated path and the original input, then compare the results. If they match, the translation is valid. If `canonicalize()` fails on either path (e.g., path doesn't exist on disk), fall back to returning the original path unchanged.

**Rationale**: `canonicalize()` resolves all symlinks and normalizes the path, making it the ground truth for filesystem identity. Comparing canonical forms catches broad-prefix mis-mappings where the translation produces a syntactically valid but semantically wrong path (FR-007).

**Alternatives considered**:
- Skip validation for performance — rejected because correctness is the core value proposition (Constitution Principle II).
- Validate only `to_logical()` — rejected because `to_canonical()` has the same mis-mapping risk.

## R-005: Platform-Specific Behavior

**Decision**: Use `#[cfg(unix)]` and `#[cfg(windows)]` for platform-gated code paths. On Windows, `detect()` always returns no active mapping.

**Rationale**:
- **Linux**: `$PWD` is the authoritative logical path source. `canonicalize()` is clean (no prefix quirks). Full algorithm applies.
- **macOS**: `$PWD` is the logical path source. `canonicalize()` adds `/private` prefix for system symlinks (`/var` → `/private/var`). The generic suffix-matching algorithm handles this without special-casing — the suffix match naturally identifies `/private/var` vs `/var` as the divergence point.
- **Windows**: `$PWD` has no direct OS-level equivalent. `var_os("PWD")` returns `None` in most shells. `canonicalize()` adds `\\?\` Extended Length Path prefix. `detect()` returns no active mapping, and all translations fall back (FR-014).

**Alternatives considered**:
- Special-casing macOS `/private` — rejected because the generic algorithm handles it correctly.
- Attempting Windows `subst` drive detection — explicitly deferred to future work per spec assumptions.

## R-006: `canonicalize()` Platform Quirks

**Decision**: On Windows, the `\\?\` prefix from `canonicalize()` does not affect correctness because Windows `detect()` always returns no mapping, so `canonicalize()` is never used in the translation path on Windows. On macOS, the `/private` prefix is handled by the generic suffix-matching algorithm.

**Rationale**: Keeping the algorithm generic across Unix platforms avoids maintenance burden and ensures new platform symlink patterns are handled automatically.

**Alternatives considered**: None — this is a documentation decision, not an implementation choice.

## R-007: Error Handling Strategy

**Decision**: No public API function returns `Result` or `Err`. `detect()` returns `LogicalPathContext` (always succeeds). `to_logical()` and `to_canonical()` return `PathBuf` (always succeed, falling back to the input path). Internal errors (failed `canonicalize`, missing env var) are handled silently by falling back.

**Rationale**: FR-009 requires that no public API function panics or returns an unrecoverable error. The fallback guarantee (FR-008) means every call always produces a usable path. This makes the library safe to adopt unconditionally without error-handling boilerplate.

**Alternatives considered**:
- Returning `Result<PathBuf, E>` — rejected because the fallback path is always correct and callers shouldn't need to handle errors for a "best-effort translation" API.
- Logging internal errors — deferred; the library should not impose a logging framework. Callers who want visibility can compare input and output to detect fallback.

## R-008: Test Infrastructure

**Decision**: Use `tempfile` crate for integration tests that create real symlinks. Unit tests for the suffix-matching algorithm use constructed `LogicalPathContext` values with known prefix pairs (no filesystem interaction). Platform-specific tests gated with `#[cfg(target_os = "...")]`.

**Rationale**: `tempfile` provides automatic cleanup and avoids test pollution. Separating unit tests (pure logic) from integration tests (filesystem interaction) keeps the test suite fast and reliable.

**Alternatives considered**:
- Manual temp directory management — rejected because `tempfile` is the Rust ecosystem standard and handles cleanup on panic.
- Mocking `canonicalize()` — rejected for v0.1; integration tests with real symlinks are more trustworthy for a filesystem library.
