# Implementation Plan: logical-path Core Library

**Branch**: `core-features` | **Date**: 2026-03-15 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `specs/core-features/spec.md`

## Summary

Implement the core `logical-path` crate: a pure Rust library that translates canonical (symlink-resolved) filesystem paths back to their logical (symlink-preserving) equivalents. The primary public type is `LogicalPathContext`, which detects the active symlink prefix mapping by comparing `$PWD` against `getcwd()`, then provides `to_logical()` and `to_canonical()` translation methods with round-trip validation and graceful fallback. The implementation uses only `std` for core logic, targets Linux/macOS/Windows, and follows TDD per the constitution.

## Technical Context

**Language/Version**: Rust, edition 2024 (per `Cargo.toml`)
**Primary Dependencies**: None for core logic (std only); `tempfile` for test helpers
**Storage**: N/A
**Testing**: `cargo test`, `cargo test --doc`, `cargo clippy -- --deny warnings`, `cargo fmt --check`
**Target Platform**: Linux, macOS, Windows (cross-platform)
**Project Type**: Library crate (pure `lib.rs`, no binary targets)
**Performance Goals**: N/A — path translation is inherently fast (string prefix operations); no hot-loop or high-throughput requirements
**Constraints**: No `unsafe` code; no external dependencies for core logic; all public items documented; MSRV ≥ 1.85.0 (required by edition 2024); to be pinned in Cargo.toml post-stabilization
**Scale/Scope**: Single crate, ~5 public API items (`LogicalPathContext`, `detect()`, `to_logical()`, `to_canonical()`, `has_mapping()`) plus 1 `pub(crate)` testability helper (`detect_from()`)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Evidence |
| --------- | ------ | -------- |
| **I. Library-First Design** | ✅ PASS | Pure library crate, no `main.rs`, no binary targets. Minimal API: one public struct + 3-4 methods. Accepts `&Path`, returns `PathBuf`. No internal types leaked. |
| **II. Correctness & Safety** | ✅ PASS | Five-step algorithm (Detect→Map→Translate→Validate→Fall back) is mandatory per spec FR-003. Round-trip validation on every translation (FR-007). Fallback returns original path, never errors (FR-008, FR-009). No `unsafe`. `OsStr`/`Path` throughout — no UTF-8 panics. |
| **III. Test-First (TDD)** | ✅ PASS | Spec mandates TDD workflow. Acceptance scenarios provide concrete test cases. Platform-gated tests via `#[cfg()]`. `tempfile` for filesystem tests with automatic cleanup. |
| **IV. Cross-Platform Discipline** | ✅ PASS | Linux/macOS/Windows CI required. Platform-specific code gated with `#[cfg(unix)]`/`#[cfg(windows)]`. Windows: `detect()` returns no mapping. macOS: handles `/private` prefix via generic algorithm. |
| **V. Semantic Versioning & MSRV** | ✅ PASS | Starting at `0.1.0`. Edition 2024 requires MSRV ≥ 1.85.0. MSRV to be pinned in Cargo.toml once implementation stabilizes (per constitution TODO). |

**Result**: All gates pass. No violations. Proceeding to Phase 0.

### Post-Design Re-Check (Phase 1 complete)

| Principle | Status | Post-Design Evidence |
| --------- | ------ | ------------------- |
| **I. Library-First Design** | ✅ PASS | Data model confirms: one public struct (`LogicalPathContext`), four methods (`detect`, `to_logical`, `to_canonical`, `has_mapping`). Internal `PrefixMapping` is not exposed. All methods accept `&Path` / return `PathBuf`. |
| **II. Correctness & Safety** | ✅ PASS | Contract specifies round-trip validation for every translation. Fallback guarantee documented. No `unsafe`. Research confirms `OsStr`/`Path` used throughout — no UTF-8 conversions in hot path. |
| **III. Test-First (TDD)** | ✅ PASS | Spec acceptance scenarios map directly to test cases. Research R-008 confirms `tempfile` for integration tests, `#[cfg()]` for platform gating. |
| **IV. Cross-Platform Discipline** | ✅ PASS | Contract documents platform matrix. Windows: no-mapping context, always fallback. macOS: `/private` handled by generic algorithm (Research R-005). |
| **V. Semantic Versioning & MSRV** | ✅ PASS | Version 0.1.0 confirmed. No external dependencies for core (Research R-007). Edition 2024 requires MSRV ≥ 1.85.0; to be pinned in Cargo.toml post-stabilization. |

**Post-Design Result**: All gates pass. Design is constitution-compliant.

## Project Structure

### Documentation (this feature)

```text
specs/core-features/
├── spec.md              # Feature specification
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (public API contracts)
└── tasks.md             # Phase 2 output (created by /speckit.tasks)
```

### Source Code (repository root)

```text
src/
└── lib.rs               # All library code (single-module for v0.1)

tests/
└── integration.rs       # Integration tests with real symlinks (optional, per complexity)
```

**Structure Decision**: Single-module library (`src/lib.rs`) is appropriate for the scope of v0.1. The crate has one public type with three methods and a small internal helper. No sub-modules are needed until the codebase grows. Integration tests that require real filesystem symlinks go in `tests/`. Unit tests use `#[cfg(test)] mod tests` inside `lib.rs`. The `detect()` function delegates to an internal `pub(crate) detect_from(pwd, canonical_cwd)` helper to enable unit-testing the detection logic without modifying process-global state (`$PWD`, CWD).

## Complexity Tracking

No constitution violations to justify. All design decisions align with the five principles.
