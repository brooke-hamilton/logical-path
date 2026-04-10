# Research: Windows Full Support

## R-001: Windows CWD Preservation Behavior

**Decision**: `std::env::current_dir()` preserves junctions, subst drives, and mapped drives on Windows.

**Rationale**: On Windows, `current_dir()` calls `GetCurrentDirectoryW`, which returns the path as stored in the process's current directory state — it does not resolve junctions, subst mappings, or directory symlinks. If the process CDs into `C:\workspace` (a junction to `D:\projects\workspace`), `current_dir()` returns `C:\workspace`. This makes it the correct "logical path source" on Windows, analogous to `$PWD` on Unix.

**Alternatives considered**: Reading `$PWD` on Windows (not applicable — Windows shells do not maintain `$PWD`), `QueryDosDevice` (more complex, gives the same result for subst drives but not junctions).

## R-002: Windows Canonicalization Behavior

**Decision**: `std::fs::canonicalize()` always returns `\\?\`-prefixed paths on Windows and resolves through all directory indirections (junctions, directory symlinks, subst drives).

**Rationale**: On Windows, `canonicalize()` calls `GetFinalPathNameByHandle` with `FILE_NAME_NORMALIZED`, which resolves all reparse points and symlinks to the physical path and prepends the `\\?\` extended-length prefix. This is the correct "canonical path source" on Windows.

**Alternatives considered**: `GetLongPathName` (does not resolve junctions), manual `DeviceIoControl` queries (overly complex, not needed).

## R-003: Extended Length Path Prefix Stripping

**Decision**: Implement `\\?\` stripping inline (no external crate). Two patterns to handle.

**Rationale**: The stripping logic is straightforward and well-defined:

1. `\\?\C:\...` → `C:\...` — strip 4 characters when followed by a drive letter and colon
2. `\\?\UNC\server\share\...` → `\\server\share\...` — replace `\\?\UNC\` with `\\`

The `dunce` crate (v1.0.5) handles this correctly, but the logic is ~15 lines of code and adding a runtime dependency for it is unnecessary. Implementing inline avoids a dependency and keeps the stripping behavior fully visible in the codebase. The function operates on `Path` → `PathBuf` to avoid lifetime issues with `OsStr` slicing.

**Alternatives considered**: `dunce` crate — rejected because the stripping logic is trivial and the crate would be an unconditional dependency (it's useful on all platforms, but we only need it on Windows). Implementing inline keeps the dependency footprint at zero for this concern.

## R-004: Case-Insensitive Path Comparison

**Decision**: Use `OsStr::eq_ignore_ascii_case()` for path component comparison during suffix matching on Windows.

**Rationale**: NTFS is case-preserving but case-insensitive, using ordinal (non-locale-aware) comparison. `OsStr::eq_ignore_ascii_case()` was stabilized in Rust 1.79.0, and our MSRV is 1.85.0, so it is available. The spec explicitly states "ordinal case-insensitive comparison" (not Unicode case-folding), which aligns with ASCII-case comparison. Windows path components (drive letters, directory names) are overwhelmingly ASCII. The spec's out-of-scope section explicitly excludes locale-specific case folding.

**Alternatives considered**: Full Unicode case-folding via `to_uppercase()` — rejected per spec ("ordinal case-insensitive comparison"). Converting to `str` and using `str::eq_ignore_ascii_case` — unnecessary since `OsStr` has the method directly.

## R-005: Creating Junctions and Subst Drives in Tests

**Decision**: Use `std::process::Command` to call `cmd /c mklink /J` for junction creation and `subst` for drive mapping in test code.

**Rationale**: Both `mklink /J` (junctions) and `subst` work without elevation on modern Windows (Vista+). Using `std::process::Command` avoids adding a dev-dependency for a handful of test-setup calls. Junctions are created with `cmd /c mklink /J <link> <target>`, subst drives with `subst <letter>: <path>`, and cleaned up with `rd <link>` and `subst <letter>: /D` respectively. Tests must handle the case where these commands fail (e.g., in CI environments without Windows) by skipping gracefully.

**Alternatives considered**: `junction` crate as dev-dependency — a reasonable choice, but shelling out is simpler for the small number of test-setup calls needed and avoids explaining an extra dependency.

## R-006: Trace-Level Diagnostics

**Decision**: Use the `log` crate (v0.4.x) for trace-level diagnostics.

**Rationale**: The `log` crate is the standard Rust logging facade. When no logger is configured, the macros compile to a single integer comparison and conditional jump — effectively zero overhead. The `log` crate is an unconditional dependency, but it is tiny (no transitive dependencies), widely used, and the spec requires diagnostics on all platforms (FR-013 applies to detection and fallback on both Unix and Windows). Using `log::trace!()` and `log::debug!()` satisfies the requirement.

**Alternatives considered**: `tracing` crate — heavier dependency with more transitive deps, overkill for this use case. The spec mentions either `log` or `tracing`; `log` is the lighter choice.

## R-007: Divergence Algorithm Reuse

**Decision**: Reuse the existing `find_divergence_point()` suffix-matching algorithm on Windows, with case-insensitive component comparison.

**Rationale**: The spec (FR-002) states the "same suffix-matching algorithm used on Unix" with case-insensitive comparison on Windows. The existing function compares components with `==` (case-sensitive). On Windows, the comparison must be `OsStr::eq_ignore_ascii_case()`. The cleanest approach is to extract the comparison into a platform-conditional helper or parameterize the function. The algorithm logic (walk from end, count matching suffix, extract prefixes) is identical.

**Alternatives considered**: Separate Windows-only divergence function — rejected to avoid code duplication. A single function with platform-conditional comparison keeps the algorithm in one place.

## R-008: `detect()` on Windows — No Staleness Check

**Decision**: On Windows, `detect()` compares `current_dir()` vs `canonicalize(current_dir())` directly, with no `$PWD` staleness check.

**Rationale**: Per FR-003, the `$PWD`-based staleness validation used on Unix does not apply on Windows because `current_dir()` is maintained by the OS and is always current. The detection flow is: (1) call `current_dir()`, (2) call `canonicalize()` on that result, (3) strip `\\?\` prefix from the canonicalized result, (4) compare the two paths, (5) if different, run the suffix-matching divergence algorithm to extract the prefix mapping.

**Alternatives considered**: None — the spec is explicit that the `$PWD` staleness check must not be applied on Windows.

## R-009: New Dependencies Summary

| Dependency | Type | Platform Gate | Justification |
| ---------- | ---- | ------------- | ------------- |
| `log` | Runtime | Unconditional | FR-013 diagnostics. Zero overhead when unconfigured. No transitive deps. |

No other new runtime or dev-dependencies required. The `\\?\` stripping is implemented inline. Test junctions/subst use `std::process::Command`.
