# Feature Specification: logical-path Core Library

**Feature Branch**: `copilot/create-spec-for-crate`  
**Created**: 2025-07-18  
**Status**: Draft  
**Input**: User description: "Core `logical-path` crate library — a Rust library that translates canonical (symlink-resolved) filesystem paths back to their logical (symlink-preserving) equivalents."

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Detect Active Symlink Mapping (Priority: P1)

A developer building a CLI tool calls `LogicalPathContext::detect()` at startup. The call compares the `$PWD` environment variable (the shell's logical path) against the OS-reported current working directory (the canonical, symlink-resolved path). When those two values differ, the library identifies the logical prefix and the canonical prefix at their point of divergence and stores the mapping internally. When they are identical — or when `$PWD` is unset — the context records that no active mapping exists.

**Why this priority**: All other functionality depends on a correctly detected mapping. Without a reliable detect step, every translation is either wrong or a no-op. This story delivers the foundation that every downstream story builds on.

**Independent Test**: Can be fully tested by invoking `LogicalPathContext::detect()` inside a controlled process environment where `$PWD` is set to a known symlink path and the canonical path is known. The test confirms the returned context encodes the correct prefix pair, the correct "no mapping" state, and that a stale or inconsistent `$PWD` results in no mapping being recorded.

**Acceptance Scenarios**:

1. **Given** a process where `$PWD` is `/workspace/project` and the canonical CWD is `/mnt/wsl/workspace/project`, **When** `LogicalPathContext::detect()` is called, **Then** the returned context records `canonical_prefix = /mnt/wsl/workspace` and `logical_prefix = /workspace` (or the appropriate divergence point), and reports an active mapping.
2. **Given** a process where `$PWD` equals the canonical CWD (no active symlink), **When** `LogicalPathContext::detect()` is called, **Then** the returned context reports no active mapping.
3. **Given** a process where `$PWD` is unset, **When** `LogicalPathContext::detect()` is called, **Then** the returned context reports no active mapping and does not panic.
4. **Given** a process where `$PWD` is set to a path that no longer exists on disk (stale), **When** `LogicalPathContext::detect()` is called, **Then** the returned context reports no active mapping and does not panic or return an error to the caller.
5. **Given** a macOS environment where the canonical CWD has a `/private` prefix (e.g., `/private/var/folders/…`) and `$PWD` shows `/var/folders/…`, **When** `LogicalPathContext::detect()` is called, **Then** the returned context correctly identifies the `/private/var` → `/var` mapping.

---

### User Story 2 — Translate a Canonical Path to Its Logical Equivalent (Priority: P1)

A CLI tool has obtained a canonical path (e.g., from `std::fs::canonicalize()`, a git API, or an OS callback) and needs to display it to the user or write a `cd` directive for shell integration. The developer calls `ctx.to_logical(&canonical_path)`. If an active mapping applies, the library strips the canonical prefix and prepends the logical prefix. Before returning the translated path, the library validates the round-trip: re-canonicalising the translated path must produce the same canonical path as re-canonicalising the original input. If validation passes, the logical path is returned. If no mapping applies, or if validation fails, the original canonical path is returned unchanged.

**Why this priority**: This is the primary value-delivery operation of the crate. Without it, the detect step has no practical use.

**Independent Test**: Can be fully tested by constructing a `LogicalPathContext` with a known prefix pair (via a test helper or by driving `detect()` in a controlled environment), then calling `to_logical()` with canonical paths that are within the mapped prefix, outside the mapped prefix, or invalid. Each case is verified independently against expected output.

**Acceptance Scenarios**:

1. **Given** an active mapping (canonical prefix `/mnt/wsl/workspace`, logical prefix `/workspace`) and a canonical path `/mnt/wsl/workspace/project/src/main.rs`, **When** `ctx.to_logical()` is called, **Then** the returned path is `/workspace/project/src/main.rs`.
2. **Given** an active mapping and a canonical path that does NOT begin with the canonical prefix (e.g., `/home/user/notes.txt`), **When** `ctx.to_logical()` is called, **Then** the original path `/home/user/notes.txt` is returned unchanged.
3. **Given** an active mapping and a canonical path whose translation would fail round-trip validation (i.e., `canonicalize(translated) ≠ canonicalize(original)`), **When** `ctx.to_logical()` is called, **Then** the original canonical path is returned unchanged.
4. **Given** a context with no active mapping, **When** `ctx.to_logical()` is called with any path, **Then** the input path is returned unchanged.
5. **Given** an active mapping and a canonical path containing non-UTF-8 bytes in its components, **When** `ctx.to_logical()` is called, **Then** the function handles the path without panicking and returns a usable path.

---

### User Story 3 — Translate a Logical Path to Its Canonical Equivalent (Priority: P2)

A developer has a logical path (e.g., sourced from `$PWD` or user input) and needs its canonical form for filesystem operations such as file I/O or comparison with paths returned by OS APIs. The developer calls `ctx.to_canonical(&logical_path)`. If an active mapping applies, the library strips the logical prefix and prepends the canonical prefix. The same round-trip validation used in `to_logical()` is applied. If no mapping applies or validation fails, the original path is returned unchanged.

**Why this priority**: Important for callers who need to normalise user-provided paths before passing them to filesystem APIs. However, `to_logical()` is the more commonly needed direction, so this is P2.

**Independent Test**: Can be fully tested with the same controlled environment used for User Story 2, but calling `to_canonical()` with logical paths.

**Acceptance Scenarios**:

1. **Given** an active mapping (canonical prefix `/mnt/wsl/workspace`, logical prefix `/workspace`) and a logical path `/workspace/project/README.md`, **When** `ctx.to_canonical()` is called, **Then** the returned path is `/mnt/wsl/workspace/project/README.md`.
2. **Given** an active mapping and a logical path that does NOT begin with the logical prefix, **When** `ctx.to_canonical()` is called, **Then** the original path is returned unchanged.
3. **Given** a context with no active mapping, **When** `ctx.to_canonical()` is called with any path, **Then** the input path is returned unchanged.
4. **Given** an active mapping and a logical path containing non-UTF-8 bytes, **When** `ctx.to_canonical()` is called, **Then** the function handles the path without panicking.

---

### User Story 4 — Graceful Fallback Preserves Caller Correctness (Priority: P1)

In all conditions where the library cannot confidently translate a path — `$PWD` unset, mapping stale, path outside the mapped prefix, or round-trip validation failure — the library returns the input path unchanged. The caller always receives a usable path regardless of the environment's symlink topology. No operation in the public API returns an `Err` or panics due to symlink-resolution state.

**Why this priority**: The fall-back guarantee is the safety net that makes the library safe to adopt unconditionally. Without it, callers would need conditional logic around every call, which defeats the library's purpose.

**Independent Test**: Can be fully tested by calling both `to_logical()` and `to_canonical()` in a variety of adverse conditions (unset `$PWD`, stale mappings, non-matching paths, non-UTF-8 paths) and asserting that each call returns the original input path without panicking.

**Acceptance Scenarios**:

1. **Given** any context state (mapped or unmapped), **When** `to_logical()` or `to_canonical()` is called with a path that cannot be translated, **Then** the return value equals the input path and no panic or unhandled error occurs.
2. **Given** a corrupted or partially-resolved `$PWD`, **When** `LogicalPathContext::detect()` is called, **Then** the resulting context safely reports no active mapping rather than producing an incorrect mapping.

---

### User Story 5 — Cross-Platform Operation (Priority: P2)

The library operates correctly on Linux, macOS, and Windows. On Linux and macOS, `$PWD` is the source of logical-path information. On macOS, system-level symlinks (`/var` → `/private/var`, `/tmp` → `/private/tmp`) are handled without special-casing by the generic five-step algorithm. On Windows, where `$PWD` has no direct OS-level equivalent, the library detects this condition and returns no active mapping, ensuring the fall-back path is always taken; it does not produce incorrect translations.

**Why this priority**: The library's use cases arise specifically from platform symlink behaviour. Correctness on all three platforms is required, though Linux and macOS cover the majority of currently reported user pain.

**Independent Test**: Each platform is independently testable via CI runners. Linux and macOS tests exercise the full five-step algorithm with real or mock symlink environments. Windows tests verify that `detect()` reports no active mapping and that all translation calls return input unchanged.

**Acceptance Scenarios**:

1. **Given** a Linux environment with a user-created symlink (e.g., `/workspace` → `/mnt/data/workspace`), **When** `LogicalPathContext::detect()` is called from within a directory under that symlink, **Then** the correct prefix mapping is detected and translation succeeds.
2. **Given** a macOS environment where `$PWD` is `/var/folders/xyz` and the canonical CWD is `/private/var/folders/xyz`, **When** `LogicalPathContext::detect()` is called, **Then** the mapping `/private/var` → `/var` is detected.
3. **Given** a Windows environment, **When** `LogicalPathContext::detect()` is called, **Then** the context reports no active mapping and subsequent translation calls return inputs unchanged without panicking.

---

### Edge Cases

- What happens when `$PWD` contains a trailing slash or redundant separators?  
  The library normalises path components before comparison, treating `/workspace/` and `/workspace` as equivalent.
- What happens when the canonical path and `$PWD` diverge at the root level?  
  The mapping algorithm finds no valid suffix-match and records no active mapping; fall-back applies.
- What happens when path components contain non-UTF-8 bytes (e.g., filenames with arbitrary byte sequences)?  
  All path operations use `OsStr`/`Path` types throughout; no intermediate conversion to `String` is performed that could truncate or panic on non-UTF-8 input.
- What happens when `$PWD` points to a directory that has been deleted or remounted since the process started?  
  The canonicalize step during validation fails; the fall-back path is returned unchanged.
- What happens with deeply nested symlinks (symlinks within symlinks)?  
  The algorithm only resolves one prefix mapping per context (the outermost divergence point). Nested symlinks outside that prefix are not translated.
- What happens with path components that are empty strings or single dots?  
  Dot-components and empty components are treated as part of the raw path and are not specially handled by the prefix-matching logic; the OS-level `canonicalize` call handles their resolution.
- What happens on case-insensitive filesystems (macOS APFS)?  
  The library performs component-level comparison using the raw byte representation; callers on case-insensitive systems are responsible for normalising case before comparison if that behaviour is needed.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The library MUST provide a `LogicalPathContext` type as its primary public interface.
- **FR-002**: `LogicalPathContext` MUST expose a `detect()` associated function that reads `$PWD` from the process environment and the canonical current working directory from the OS, compares the two, and returns a `LogicalPathContext` value representing the detected mapping (if any).
- **FR-003**: `LogicalPathContext::detect()` MUST implement the full five-step algorithm: Detect → Map → Translate → Validate → Fall back. No step may be skipped.
- **FR-004**: The mapping step MUST use suffix-matching on path components (not byte-level string prefix matching) to identify the divergence point between the logical and canonical paths.
- **FR-005**: `LogicalPathContext` MUST expose a `to_logical(&self, path: &Path) -> PathBuf` method that translates a canonical path to its logical equivalent using the stored mapping, or returns the input unchanged if no mapping applies or validation fails.
- **FR-006**: `LogicalPathContext` MUST expose a `to_canonical(&self, path: &Path) -> PathBuf` method that translates a logical path to its canonical equivalent using the stored mapping, or returns the input unchanged if no mapping applies or validation fails.
- **FR-007**: Both `to_logical()` and `to_canonical()` MUST execute the Validate step (round-trip `canonicalize(translated) == canonicalize(original)`) before returning a translated path.
- **FR-008**: Both `to_logical()` and `to_canonical()` MUST return the input path unchanged (fall back) when: the context has no active mapping, the path does not begin with the relevant prefix, or the Validate step fails.
- **FR-009**: No public API function MUST panic or return an unrecoverable error due to symlink-resolution state, missing environment variables, or non-UTF-8 path bytes.
- **FR-009a**: `LogicalPathContext` MUST expose an `is_active() -> bool` method that returns `true` when an active prefix mapping exists and `false` otherwise. No accessor methods for the internal prefix pair are exposed; the mapping remains an opaque implementation detail.
- **FR-010**: The crate MUST be a pure library crate with no binary targets.
- **FR-011**: All public API functions MUST accept `&Path`-like inputs (not `String`) and return `PathBuf` or `Option<PathBuf>` / `Result<PathBuf, _>` as appropriate, without leaking internal implementation types.
- **FR-012**: All public types and functions MUST have doc comments, including documented behaviour for the fall-back case and platform-specific notes.
- **FR-013**: Platform-specific code paths MUST be gated with conditional compilation attributes (`#[cfg(unix)]`, `#[cfg(windows)]`, `#[cfg(target_os = "macos")]`, etc.).
- **FR-014**: On Windows, where `$PWD` has no direct OS-level equivalent, `LogicalPathContext::detect()` MUST report no active mapping rather than attempting an incorrect heuristic; `to_logical()` and `to_canonical()` MUST fall back to returning input unchanged.
- **FR-015**: The crate MUST compile and all tests MUST pass on Linux, macOS, and Windows.

### Key Entities

- **`LogicalPathContext`**: The central value type. Encapsulates zero or one active prefix mappings (a canonical prefix and its corresponding logical prefix). Created via `detect()`. Immutable after construction. Thread-safety properties follow from having no interior mutability. Exposes `is_active() -> bool` to query mapping state; the prefix pair itself is not publicly accessible.
- **Prefix Mapping**: An internal representation of the divergence point between `$PWD` and `getcwd()`. Consists of a canonical path prefix and a logical path prefix, both derived at construction time. Not directly exposed in the public API.
- **Canonical Path**: A fully resolved filesystem path with all symlinks removed, as returned by `std::fs::canonicalize()` or the OS. Input to `to_logical()`; output of `to_canonical()`.
- **Logical Path**: A symlink-preserving filesystem path as recorded in `$PWD`. Output of `to_logical()`; input to `to_canonical()`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer can integrate the library into a CLI tool and, within a WSL or macOS symlink environment, display the user's logical path instead of the canonical path — verifiable by a passing integration test that creates a real symlink, invokes `detect()`, calls `to_logical()`, and asserts the logical path is returned.
- **SC-002**: Every call to `to_logical()` or `to_canonical()` on any supported platform returns a usable, non-empty path — the function never panics and never returns an empty path, regardless of `$PWD` state or path content.
- **SC-003**: The round-trip property holds for every translated path: translating a canonical path to logical and then back to canonical yields the original canonical path — verified by a property-based or parameterised test suite covering at least 10 distinct path structures per platform.
- **SC-004**: All five steps of the algorithm (Detect, Map, Translate, Validate, Fall back) have independent, named test cases that pass on Linux, macOS, and Windows CI runners.
- **SC-005**: The library introduces zero additional dependencies on external crates for its core path-translation logic (the standard library is sufficient), keeping the compile-time and supply-chain footprint minimal.
- **SC-006**: All public items carry doc comments, and `cargo doc --no-deps` completes with zero warnings — confirming the API is self-documenting for downstream consumers.
- **SC-007**: A developer calling `ctx.to_logical()` on a path that falls outside the mapped prefix receives the original path back unchanged — no mis-translation — verifiable by a targeted test case.
- **SC-008**: The library handles paths with non-UTF-8 filenames without panicking, confirmed by a test that constructs such a path on Unix and asserts a clean return value.

## Clarifications

### Session 2026-03-15

- Q: Should `LogicalPathContext` expose a method to query whether an active mapping exists? → A: Expose `is_active() -> bool` method only (no accessor methods for prefix pair).

## Assumptions

- `$PWD` is the authoritative source of logical path information on Unix-like platforms. The library does not attempt to recover a logical path by other means (e.g., traversing `/proc/self/fd` symlinks) when `$PWD` is unset.
- Only one prefix mapping is active per process context. Nested or multiple independent symlink mappings in the same path are outside scope for v0.1.
- The library does not cache filesystem state between calls to `to_logical()` / `to_canonical()`; it assumes that the mapping established at `detect()` time remains valid for the lifetime of the context. Callers operating in long-lived processes in volatile mount environments should recreate the context as needed.
- Windows support is explicitly limited for v0.1: `$PWD` has no direct OS-level equivalent, so detection always reports no active mapping. `subst` drive and NTFS junction detection are acknowledged future work.
- Case-insensitive filesystem handling (macOS APFS, Windows NTFS) is not explicitly normalised by the library; the OS-level `canonicalize` call handles case resolution during validation.
- The MSRV will be pinned once initial implementation is complete and dependency requirements are known, per the constitution's TODO.

## Dependencies & Constraints

- **No `unsafe` code**: Prohibited unless a specific, documented justification is provided and a safe alternative has been explicitly ruled out (per Constitution Principle II).
- **No external dependencies for core logic**: The standard library's `std::path`, `std::env`, and `std::fs` modules are sufficient for the core algorithm. Test helpers may use `tempfile` or similar.
- **Pure library crate**: No `main.rs`, no binary targets (per Constitution Principle I).
- **Quality gates**: All pull requests must pass `cargo test`, `cargo clippy -- --deny warnings`, `cargo fmt --check`, `cargo doc --no-deps`, and an MSRV build job before merge (per Constitution Quality Gates).
- **TDD workflow**: Tests must be written before implementation code is merged; the Red-Green-Refactor cycle is mandatory (per Constitution Principle III).
