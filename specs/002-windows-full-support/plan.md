# Implementation Plan: Windows Full Support

**Branch**: `002-windows-full-support` | **Date**: 2026-04-09 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/002-windows-full-support/spec.md`

## Summary

Implement full Windows support for the `logical-path` crate by replacing the current no-op Windows `detect()` with a detection mechanism based on comparing `std::env::current_dir()` (preserves junctions, subst drives, mapped drives) against `std::fs::canonicalize()` (resolves to physical path). The `\\?\` Extended Length Path prefix returned by `canonicalize()` on Windows must be stripped before any comparison or prefix matching. The existing suffix-matching divergence algorithm is reused. Path comparison on Windows must be case-insensitive. All Unix behavior remains unchanged. Trace-level diagnostics are added via the `log` crate.

## Technical Context

**Language/Version**: Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml)
**Primary Dependencies**: None at runtime (current). New: `log` crate for trace diagnostics. `windows-sys` may be needed behind `#[cfg(windows)]` if OS APIs beyond std are required.
**Storage**: N/A (pure library, no persistent state)
**Testing**: `cargo test` + `tempfile` dev-dependency. Windows tests require NTFS junction/subst creation (likely via `std::process::Command` calling `mklink /J` and `subst`).
**Target Platform**: Linux, macOS, Windows (all three, cross-platform library)
**Project Type**: Library crate (`lib.rs` only, no binary targets)
**Performance Goals**: No explicit latency bound. Detection and translation are fast OS calls.
**Constraints**: Zero unconditional new dependencies. Platform-gated `windows-sys` acceptable. No `unsafe` without documented justification.
**Scale/Scope**: Single-file library (~300 LOC). Feature adds ~200-400 LOC of platform-specific code + tests.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
| --------- | ------ | ----- |
| I. Library-First Design | PASS | No binary targets added. Public API surface unchanged (`detect()`, `to_logical()`, `to_canonical()`, `has_mapping()`). |
| II. Correctness & Safety | PASS | Five-step algorithm (Detect → Map → Translate → Validate → Fall back) preserved. Round-trip validation on every translation. `\\?\` stripping ensures correct prefix matching. No `unsafe` planned (std library calls sufficient). |
| III. Test-First (TDD) | PASS | Tests written before implementation. Windows tests gated with `#[cfg(windows)]`. Platform tests use temp dirs for isolation. |
| IV. Cross-Platform Discipline | PASS | All new Windows code behind `#[cfg(windows)]`. Existing Unix paths untouched and gated with `#[cfg(not(windows))]`. CI must pass on all three platforms. |
| V. Semantic Versioning & MSRV | PASS | Feature addition = MINOR bump (0.1.0 → 0.2.0). MSRV unchanged at 1.85.0. `log` crate is lightweight and widely used. |

**Gate result: PASS** — no violations.

## Project Structure

### Documentation (this feature)

```text
specs/002-windows-full-support/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── public-api.md
└── tasks.md             # Phase 2 output (NOT created by /speckit.plan)
```

### Source Code (repository root)

```text
src/
└── lib.rs               # All library code (single file, platform-gated sections)

tests/
└── integration.rs       # Integration tests (platform-gated sections)
```

**Structure Decision**: Maintain the existing single-file library structure. Windows-specific code is added to `src/lib.rs` using `#[cfg(windows)]` blocks alongside the existing `#[cfg(not(windows))]` blocks. No new source files needed — the codebase is small enough (~300 LOC) that splitting into modules would add complexity without benefit.

## Complexity Tracking

> No constitution violations — this section is intentionally empty.
