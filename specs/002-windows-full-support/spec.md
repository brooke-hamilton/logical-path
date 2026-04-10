# Feature Specification: Windows Full Support

**Feature Branch**: `002-windows-full-support`
**Created**: 2026-04-09
**Status**: Draft
**Input**: User description: "Implement full support for Windows. Remove these limitations: NTFS junctions not detected, subst drives not detected, and Extended Length Path prefix handling."

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Detect Logical Path via NTFS Junctions (Priority: P1)

A developer on Windows creates an NTFS junction (e.g., `C:\workspace` pointing to `D:\projects\workspace`) and opens a terminal inside the junction path. When a CLI tool built with `logical-path` calls `LogicalPathContext::detect()`, the library discovers the junction relationship by comparing the working directory path (from `GetCurrentDirectoryW`, which preserves junctions) against its fully resolved canonical path (from `canonicalize()` → `GetFinalPathNameByHandleW`, which resolves through junctions). The context records the mapping so that canonical paths under the junction target can be translated back to the junction-based logical path.

> **Key insight — logical path source on Windows**: On Unix, `$PWD` provides the logical path and `getcwd()` provides the canonical path. On Windows, the roles are different: `GetCurrentDirectoryW` (i.e., `std::env::current_dir()`) preserves junctions, subst drives, and mapped drives — making it the logical path source. `GetFinalPathNameByHandleW` (i.e., `std::fs::canonicalize()`) resolves all indirections to the physical path — making it the canonical path source. This asymmetry is why the existing `$PWD`-based approach does not apply to Windows, but a `current_dir()` vs `canonicalize()` comparison achieves the same result.

**Why this priority**: NTFS junctions are the most common form of directory symlink on Windows. They are used by developers, build systems, and tools like `mklink /J`. Supporting junctions is the single most impactful Windows capability.

**Independent Test**: Can be fully tested by creating an NTFS junction in a controlled test environment, setting the current directory to a path under the junction, calling `detect()`, and asserting that the returned context contains the correct prefix mapping.

**Acceptance Scenarios**:

1. **Given** an NTFS junction `C:\workspace` → `D:\projects\workspace` and the process working directory is `C:\workspace\myproject`, **When** `LogicalPathContext::detect()` is called, **Then** the context records a mapping (e.g., canonical prefix `D:\projects\workspace`, logical prefix `C:\workspace`) and `has_mapping()` returns `true`.
2. **Given** the detected mapping from scenario 1 and a canonical path `D:\projects\workspace\myproject\src\main.rs`, **When** `ctx.to_logical()` is called, **Then** the returned path is `C:\workspace\myproject\src\main.rs`.
3. **Given** a Windows environment with no junctions or subst drives in the current directory path, **When** `LogicalPathContext::detect()` is called, **Then** the context reports no active mapping and `has_mapping()` returns `false`.
4. **Given** a junction that has been removed after `detect()` was called, **When** `ctx.to_logical()` is called on a path under the former junction target, **Then** the round-trip validation fails and the original path is returned unchanged.

---

### User Story 2 — Detect Logical Path via Subst Drives (Priority: P1)

A developer on Windows uses `subst S: C:\long\path\to\source` to create a virtual drive letter `S:` mapped to a deeply nested directory. When working inside `S:\myproject`, a CLI tool calls `LogicalPathContext::detect()`. The same `current_dir()` vs `canonicalize()` comparison used for junctions detects the subst mapping: `current_dir()` returns `S:\myproject` (preserving the drive letter) while `canonicalize()` returns the resolved physical path. The library records the prefix mapping, and subsequent calls to `to_logical()` translate canonical paths back to the `S:\` drive letter form.

**Why this priority**: `subst` drives are a widely-used Windows convenience for shortening long paths. They are the second most common source of logical/canonical path divergence on Windows, alongside junctions. Both are detected by the same underlying mechanism (`current_dir()` vs `canonicalize()`), but subst drives are called out separately because their user-visible behavior (drive letter mapping) differs from junctions (directory-level links).

**Independent Test**: Can be fully tested by creating a subst mapping in a controlled test environment, setting the current directory to the substituted drive, calling `detect()`, and verifying the prefix mapping.

**Acceptance Scenarios**:

1. **Given** a subst mapping `S:` → `C:\long\path\to\source` and the process working directory is `S:\myproject`, **When** `LogicalPathContext::detect()` is called, **Then** the context records a mapping (e.g., canonical prefix `C:\long\path\to\source`, logical prefix `S:\`) and `has_mapping()` returns `true`.
2. **Given** the detected mapping from scenario 1 and a canonical path `C:\long\path\to\source\myproject\README.md`, **When** `ctx.to_logical()` is called, **Then** the returned path is `S:\myproject\README.md`.
3. **Given** a subst mapping and a canonical path that does NOT fall under the substituted target (e.g., `D:\other\file.txt`), **When** `ctx.to_logical()` is called, **Then** the original path is returned unchanged.
4. **Given** a subst mapping that has been removed after `detect()` was called, **When** `ctx.to_logical()` is called, **Then** round-trip validation fails and the original path is returned unchanged.

---

### User Story 3 — Strip Extended Length Path Prefix (Priority: P1)

`std::fs::canonicalize()` on Windows returns paths with the `\\?\` Extended Length Path prefix (e.g., `\\?\C:\Users\dev\project`). When the library performs canonicalization as part of detection or round-trip validation, these prefixed paths must be normalized to their standard form (e.g., `C:\Users\dev\project`) so that prefix matching and path comparison work correctly. Without this normalization, no junction or subst mapping would ever match because the canonical path has the `\\?\` prefix while the logical path does not.

**Why this priority**: This is a prerequisite for both junction and subst detection. If `\\?\` prefixes are not stripped, all prefix comparisons fail and Windows detection is non-functional. This is infrastructure that the other stories depend on.

**Independent Test**: Can be tested independently by calling `std::fs::canonicalize()` on a known Windows path, passing it through the normalization logic, and asserting the `\\?\` prefix is removed while the rest of the path is preserved.

**Acceptance Scenarios**:

1. **Given** a canonicalized path `\\?\C:\Users\dev\project\src\main.rs`, **When** the library normalizes it, **Then** the result is `C:\Users\dev\project\src\main.rs`.
2. **Given** a canonicalized UNC path `\\?\UNC\server\share\folder`, **When** the library normalizes it, **Then** the result is `\\server\share\folder`.
3. **Given** a path that does not have the `\\?\` prefix (e.g., `C:\Users\dev\project`), **When** the library normalizes it, **Then** the path is returned unchanged.
4. **Given** a path with the `\\?\` prefix that also contains a junction-resolved target, **When** detection runs, **Then** the prefix is stripped before suffix-matching so the junction mapping is correctly identified.

---

### User Story 4 — Translate Logical-to-Canonical on Windows (Priority: P2)

A developer has a logical path from user input or a configuration file (e.g., `S:\myproject\config.toml` or `C:\workspace\src\lib.rs` where `C:\workspace` is a junction). They need the canonical form for filesystem operations. Calling `ctx.to_canonical()` replaces the logical prefix with the canonical prefix and validates the result via round-trip canonicalization.

**Why this priority**: The reverse translation direction. Less commonly needed than `to_logical()` but important for completeness and for tools that need to normalize user-provided paths.

**Independent Test**: Can be tested by constructing a context with a known Windows prefix mapping and calling `to_canonical()` with logical paths inside and outside the mapped prefix.

**Acceptance Scenarios**:

1. **Given** an active mapping (canonical prefix `D:\projects\workspace`, logical prefix `C:\workspace`) and a logical path `C:\workspace\myproject\build.rs`, **When** `ctx.to_canonical()` is called, **Then** the returned path is `D:\projects\workspace\myproject\build.rs`.
2. **Given** an active mapping and a logical path that does NOT begin with the logical prefix, **When** `ctx.to_canonical()` is called, **Then** the original path is returned unchanged.
3. **Given** a context with no active mapping on Windows, **When** `ctx.to_canonical()` is called with any path, **Then** the input path is returned unchanged.

---

### User Story 5 — Graceful Fallback on Windows (Priority: P1)

In all conditions where the library cannot confidently translate a path on Windows — no junction or subst drive detected, mapping stale, path outside the mapped prefix, or round-trip validation failure — the library returns the input path unchanged. The caller always receives a usable path. No operation in the public API panics or returns an error due to Windows-specific path resolution state.

**Why this priority**: The safety guarantee is what allows cross-platform tools to call `detect()` unconditionally. Breaking this contract would force callers to add Windows-specific conditional logic.

**Independent Test**: Can be tested by calling `detect()` and translation methods in a variety of adverse Windows conditions (no junctions, removed subst drives, paths on different drives) and asserting fallback behavior.

**Acceptance Scenarios**:

1. **Given** a Windows environment with no junctions or subst drives, **When** `LogicalPathContext::detect()` is called, **Then** the context reports no active mapping and all translation calls return the input unchanged.
2. **Given** any context state on Windows, **When** `to_logical()` or `to_canonical()` is called with a relative path, **Then** the input is returned unchanged.
3. **Given** a detected mapping where round-trip validation fails (e.g., junction was removed after detection), **When** `to_logical()` is called, **Then** the original path is returned unchanged without panicking.

---

### User Story 6 — Backward Compatibility with Existing Unix Behavior (Priority: P1)

All existing Linux and macOS detection and translation behavior remains unchanged. The `$PWD`-based detection on Unix platforms continues to work exactly as before. The Windows-specific detection mechanisms (junction resolution, subst drive detection, `\\?\` prefix stripping) are confined to Windows code paths and do not affect Unix behavior.

**Why this priority**: Existing users on Linux and macOS must not experience regressions. This is a non-negotiable constraint.

**Independent Test**: The existing test suite for Linux and macOS continues to pass without modification. Any new shared logic is verified to not alter Unix code paths.

**Acceptance Scenarios**:

1. **Given** a Linux environment with a symlink-based `$PWD`, **When** `LogicalPathContext::detect()` is called, **Then** behavior is identical to the current implementation.
2. **Given** a macOS environment with `/var` → `/private/var` system symlinks, **When** `LogicalPathContext::detect()` is called, **Then** the `/private` prefix mapping is detected exactly as before.
3. **Given** any Unix environment, **When** the library is compiled and tests are run, **Then** all existing tests pass without modification.

---

### Edge Cases

- What happens when a junction target path no longer exists?
  The canonicalization step during detection or round-trip validation fails; the fallback path is returned unchanged.
- What happens when a subst drive letter is reassigned to a different target after `detect()`?
  The round-trip validation will fail because the canonical resolution no longer matches the stored mapping; fallback applies.
- What happens when a junction, subst drive, or mapped drive is retargeted (not just removed) between `detect()` and `to_logical()`?
  The TOCTOU window means the stored mapping may be stale. Round-trip validation detects the mismatch and returns the original path unchanged. The library provides best-effort, non-atomic guarantees.
- What happens when multiple subst drives or junctions are nested?
  The library detects only the outermost mapping (the divergence between the working directory path and its fully resolved form). Nested indirections within the resolved path are not separately tracked.
- What happens with UNC paths (e.g., `\\server\share\folder`)?
  UNC paths are treated as regular absolute paths. If a junction or subst drive resolves to a UNC path, the prefix mapping captures the UNC path as the canonical prefix.
- What happens when a path mixes forward slashes and backslashes?
  The library relies on the OS path normalization provided by the standard library's `Path` and `Component` types, which handle both separator styles on Windows.
- What happens when the `\\?\` prefix appears on a path passed directly to `to_logical()`?
  The library normalizes `\\?\` prefixes during internal canonicalization steps. Paths passed by the caller with `\\?\` prefixes are processed through the same normalization before prefix matching.
- What happens with case differences in Windows paths (e.g., `c:\workspace` vs `C:\Workspace`)?
  Windows path comparison during suffix matching uses case-insensitive comparison for path components, matching NTFS and the Windows OS behavior.
- What happens when the working directory is on a network drive mapped via `net use`?
  Network drive letters mapped via `net use` are similar to subst drives. The library attempts to resolve the drive letter to its UNC target and detect any mapping. If resolution is not possible, no mapping is detected and fallback applies.

---

### Out of Scope

The following items are explicitly excluded from this feature:

- **`\\.\` device paths**: Device namespace paths (e.g., `\\.\PhysicalDrive0`, `\\.\COM1`) are not handled by the `\\?\` prefix stripping logic and are not relevant to directory-level path indirection.
- **File-level symbolic links**: Symlinks to individual files (as opposed to directory-level links) are resolved by `std::fs::canonicalize()` during validation but are not separately detected or tracked as mappings.
- **WSL path interop**: Translation between WSL Linux paths (`/mnt/c/...`) and Windows paths (`C:\...`) is not in scope. The library operates within a single OS context.
- **Drive mapping authentication/credentials**: The library does not manage, store, or handle credentials for `net use` mapped drives. It only detects the path mapping if the OS resolves it.
- **Locale-specific case folding**: Path comparison uses ordinal case-insensitive comparison on Windows. Locale-aware or Unicode-normalized case folding is not performed.

## Clarifications

### Session 2026-04-09

- Q: Should Windows directory symlinks (`mklink /D`) be explicitly in-scope? → A: Yes, explicitly in-scope — add to FR-002 wording and add a test scenario.
- Q: Should the spec set an explicit latency bound for `detect()`? → A: No explicit target. The operations are inherently fast OS calls; no benchmark test required.
- Q: Should the spec explicitly acknowledge the TOCTOU race window between `detect()` and `to_logical()`/`to_canonical()`? → A: Yes. Document that detection is best-effort and non-atomic, with round-trip validation as mitigation.
- Q: Should the spec include an explicit out-of-scope section? → A: Yes. Add out-of-scope section listing `\\.\` device paths, file-level symlinks, WSL interop, and drive auth/credentials.
- Q: Should the library emit structured diagnostics for detection outcomes? → A: Yes. Add trace-level diagnostics via `log` or `tracing` for detection steps and fallback reasons.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: On Windows, `LogicalPathContext::detect()` MUST resolve the current working directory's canonical path by querying the OS for the fully resolved physical path, stripping `\\?\` Extended Length Path prefixes from the result.
- **FR-002**: On Windows, the library MUST detect path indirections (NTFS junctions, Windows directory symlinks (`mklink /D`), `subst` drives, and `net use` mapped drives) by comparing `current_dir()` (which preserves indirections) against `canonicalize(current_dir())` (which resolves to the physical path), then stripping `\\?\` prefixes from the canonicalized result. When the two paths differ, the divergence point is identified using the same suffix-matching algorithm used on Unix. This single mechanism handles all forms of Windows directory-level indirection, including directory symlinks.
- **FR-003**: On Windows, the `$PWD`-based staleness validation used on Unix (checking that `canonicalize($PWD) == canonical_cwd`) MUST NOT be applied. Since `current_dir()` is maintained by the OS (not a user-controlled environment variable), it is always current by definition. The detection step MUST compare `current_dir()` against `canonicalize(current_dir())` directly without an intermediate staleness check.
- **FR-004**: On Windows, `\\?\` Extended Length Path prefixes returned by `std::fs::canonicalize()` MUST be stripped before any path comparison or prefix matching. The stripping logic MUST handle both `\\?\C:\...` (local paths) and `\\?\UNC\server\share\...` (UNC paths, converted to `\\server\share\...`).
- **FR-005**: On Windows, path component comparison during suffix matching MUST be case-insensitive to match NTFS and Windows OS behavior. This applies to the divergence-point algorithm only, not to the returned path values (which preserve original casing).
- **FR-006**: The existing `$PWD`-based detection on Unix (Linux and macOS) MUST remain unchanged. Windows-specific detection logic MUST be gated behind `#[cfg(windows)]` and MUST NOT alter any Unix code paths.
- **FR-007**: `to_logical()` and `to_canonical()` on Windows MUST perform the same Translate → Validate → Fallback steps as on Unix: replace the source prefix with the target prefix, validate via round-trip canonicalization (with `\\?\` stripping applied to canonicalized results), and return the original path if validation fails.
- **FR-008**: The `\\?\` prefix stripping MUST be applied internally during detection and round-trip validation. The library MUST NOT require callers to pre-strip `\\?\` prefixes from paths passed to `to_logical()` or `to_canonical()`.
- **FR-009**: On Windows, when no junctions, subst drives, or other path indirections are in effect, `detect()` MUST return a context with no active mapping, preserving the existing fallback behavior.
- **FR-010**: All existing public API contracts MUST be preserved: `detect()` returns `LogicalPathContext`, `to_logical()` and `to_canonical()` return `PathBuf`, `has_mapping()` returns `bool`, and no method panics or returns an error type.
- **FR-011**: All platform-specific code paths MUST be gated with conditional compilation attributes (`#[cfg(windows)]`, `#[cfg(not(windows))]`, `#[cfg(unix)]`, etc.).
- **FR-012**: The library MUST compile and all tests MUST pass on Linux, macOS, and Windows.
- **FR-013**: The library MUST emit trace-level diagnostic messages (via the `log` crate) at key detection and translation decision points. At minimum, diagnostics MUST cover: (a) the `current_dir()` and `canonicalize()` values compared during detection, (b) whether a mapping was detected and the prefix pair, (c) fallback reasons when round-trip validation fails or no mapping applies. These diagnostics MUST be at `trace` or `debug` level and incur zero overhead when no subscriber/logger is active.

### Key Entities

- **NTFS Junction**: A Windows filesystem feature that creates a directory-level link (reparse point) from one location to another. Similar to a Unix directory symlink. Created via `mklink /J`. The library detects junctions by comparing the junction path against its resolved target.
- **Windows Directory Symlink**: A directory-level symbolic link created via `mklink /D`. Distinct from NTFS junctions: directory symlinks can target remote paths and require elevated privileges or Developer Mode (Windows 10+). They are a separate reparse point type but are resolved by the same `current_dir()` vs `canonicalize()` detection mechanism.
- **Subst Drive**: A virtual drive letter created by the Windows `subst` command, mapped to an existing directory path. The library detects subst drives by resolving the drive letter to its underlying target.
- **Extended Length Path Prefix (`\\?\`)**: A Windows path prefix that bypasses the `MAX_PATH` (260 character) limit. Returned by `std::fs::canonicalize()` on Windows. Must be stripped for correct path comparison and prefix matching.
- **UNC Path**: A Windows network path in the form `\\server\share\path`. When `\\?\` stripping encounters `\\?\UNC\...`, it converts to `\\server\share\...`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A developer on Windows can create an NTFS junction, run a tool built with `logical-path`, and see the junction-based logical path displayed instead of the resolved physical path — verified by an integration test that creates a junction, calls `detect()`, calls `to_logical()`, and asserts the junction path is returned.
- **SC-002**: A developer on Windows can use a `subst` drive and see paths translated to the drive-letter form — verified by an integration test that creates a subst mapping, calls `detect()`, calls `to_logical()`, and asserts the drive-letter path is returned.
- **SC-003**: All paths returned by internal canonicalization on Windows are free of `\\?\` prefixes — verified by targeted unit tests that pass `\\?\`-prefixed paths through the stripping logic and assert clean output.
- **SC-004**: Every call to `to_logical()` or `to_canonical()` on Windows returns a usable, non-empty path — the function never panics and never returns an empty path, regardless of junction state, subst state, or `\\?\` prefix presence.
- **SC-005**: The round-trip property holds for translated paths on Windows: translating canonical to logical and back yields the original canonical path — verified by parameterized tests covering junctions and subst drives.
- **SC-006**: All existing Linux and macOS tests continue to pass without modification after the Windows changes are merged.
- **SC-007**: Case-insensitive path comparison on Windows correctly identifies mappings even when casing differs between the working directory path and the resolved path — verified by a test with mixed-case junction or subst paths.
- **SC-008**: The library adds no new unconditional dependencies for the Windows detection logic. Platform-gated conditional dependencies (e.g., `windows-sys` behind `#[cfg(windows)]`) are acceptable if needed for OS API access. A lightweight logging facade (`log`) is acceptable as a cross-platform diagnostic dependency (FR-013). Beyond `log`, the dependency footprint on non-Windows platforms MUST remain unchanged.
- **SC-009**: Trace-level diagnostics are emitted during detection and fallback — verified by enabling a `tracing` or `log` subscriber in a test, running `detect()` and `to_logical()`, and asserting that relevant diagnostic messages are captured.

## Assumptions

- NTFS junctions and Windows directory symlinks (`mklink /D`) are both explicitly in-scope as primary forms of directory-level indirection. They are detected by the same `current_dir()` vs `canonicalize()` mechanism and must have dedicated test coverage. File-level symbolic links (symlinks to individual files) are not separately handled; they are resolved by `std::fs::canonicalize()` during validation.
- NTFS junctions, Windows directory symlinks (`mklink /D`), `subst` drives, and `net use` mapped drives are all detected by the same mechanism: comparing `current_dir()` against `canonicalize(current_dir())`. No separate per-indirection-type detection logic (e.g., `QueryDosDevice`) is required, though such APIs may be used as an optimization or fallback if the primary mechanism proves insufficient.
- The `\\?\` prefix stripping covers the two standard forms: `\\?\X:\...` for local paths and `\\?\UNC\server\share\...` for UNC paths. Other extended-length path forms (e.g., `\\.\` device paths) are not in scope.
- Windows path comparison during suffix matching uses the OS-level case-insensitive behavior (ordinal case-insensitive comparison). Locale-specific case folding is not performed.
- Only one prefix mapping is active per process context, consistent with the existing Unix behavior. If a path traverses both a subst drive and a junction, the outermost indirection (the one visible at the working directory level) is captured.
- Detection is **best-effort and non-atomic**. A TOCTOU (time-of-check-time-of-use) window exists between `detect()` and subsequent `to_logical()`/`to_canonical()` calls: the underlying junction, subst mapping, or drive mapping could be removed, retargeted, or replaced during this window. The round-trip validation step in `to_logical()` and `to_canonical()` serves as the safety net — if the mapping is no longer valid at translation time, the original path is returned unchanged. The library does not provide atomic or transactional path-resolution guarantees.
- Network drives mapped via `net use` are handled by the same `current_dir()` vs `canonicalize()` comparison. If `canonicalize()` resolves the mapped drive to its UNC target, the mapping is detected. If `canonicalize()` does not resolve through the mapping (some network configurations), no mapping is detected and fallback applies.
- On Windows, `current_dir()` (via `GetCurrentDirectoryW`) is always current because the OS maintains it — unlike Unix `$PWD`, which is a user-controlled environment variable that can become stale. Therefore, the staleness validation step used on Unix is unnecessary on Windows.
- The minimum supported Rust version (MSRV) and edition constraints from the existing crate apply. No new MSRV bump is required unless Windows-specific standard library APIs demand it.
- Existing cross-platform CI (Linux, macOS, Windows) is in place and will validate that no regressions are introduced.
- No explicit performance latency target is defined for `detect()` or translation methods. The underlying OS calls (`GetCurrentDirectoryW`, `GetFinalPathNameByHandleW`) are inherently fast (microsecond-scale on local filesystems). The library performs no network I/O, recursive directory traversal, or blocking operations beyond single-path canonicalization.

## Dependencies & Constraints

- **No `unsafe` code**: Unless a specific, documented justification exists (e.g., calling a Windows API that requires `unsafe`). Any `unsafe` usage must be minimal, well-documented, and have no safe alternative.
- **Minimal new dependencies**: Windows-specific OS API access should use the standard library or the `windows-sys` crate (commonly used for Windows API bindings in the Rust ecosystem). A `log` or `tracing` facade dependency is acceptable for diagnostic output (must be behind a feature flag or use the lightweight `log` facade). No heavyweight dependencies.
- **Backward compatibility**: The public API surface does not change. `detect()`, `to_logical()`, `to_canonical()`, and `has_mapping()` retain their existing signatures and semantics.
- **Conditional compilation**: All Windows-specific code must be behind `#[cfg(windows)]` gates. All existing Unix code must remain behind `#[cfg(not(windows))]` or equivalent gates.
- **Quality gates**: All pull requests must pass `cargo test`, `cargo clippy -- --deny warnings`, `cargo fmt --check`, `cargo doc --no-deps`, and MSRV build on all three platforms.
- **TDD workflow**: Tests must be written before implementation, following the Red-Green-Refactor cycle.
