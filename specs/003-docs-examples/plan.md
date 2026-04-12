# Implementation Plan: Runnable Example Projects

**Branch**: `003-docs-examples` | **Date**: 2026-04-11 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/003-docs-examples/spec.md`

**Note**: This template is filled in by the `/speckit.plan` command. See `.specify/templates/plan-template.md` for the execution workflow.

## Summary

Create two standalone, runnable Rust binary projects under `docs/example-unix/` and `docs/example-windows/` that demonstrate the symlink/junction path resolution problem and its fix using the `logical-path` crate. Each project simulates a realistic CLI tool emitting `cd` directives — first showing the broken behavior (canonical path), then the corrected behavior (logical path). Each includes a comprehensive README with code snippets, expected output, and setup instructions.

## Technical Context

**Language/Version**: Rust edition 2024 (MSRV 1.85.0, matching parent crate)
**Primary Dependencies**: `logical-path` (via relative path `../../`), `log` 0.4 (transitive)
**Storage**: N/A (filesystem read-only; no persistence)
**Testing**: Manual `cargo run` on target platform; `cargo build` compile check on all platforms
**Target Platform**: `docs/example-unix/` targets Linux and macOS; `docs/example-windows/` targets Windows
**Project Type**: Two standalone binary crate examples (educational, not library code)
**Performance Goals**: N/A (educational examples)
**Constraints**: Must NOT be part of the root workspace; each project builds independently with its own `Cargo.toml`
**Scale/Scope**: 2 small binary crates (~50-100 lines each), 2 READMEs

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
| --------- | ------ | ----- |
| I. Library-First Design | PASS | This feature adds standalone example binaries under `docs/`, not binary targets in the library crate. The library crate's `lib.rs` is unchanged. No `main.rs` added to library. |
| II. Correctness & Safety | PASS | Examples consume the public API (`LogicalPathContext::detect()`, `to_logical()`). No new unsafe code. The "broken" demo uses only stdlib functions. No path manipulation outside the crate's API. |
| III. Test-First (TDD) | JUSTIFIED DEVIATION | These are educational example programs, not library code. They demonstrate existing tested functionality. The examples themselves are verified by `cargo build` (compile check) and manual `cargo run`. No new library behavior is introduced that requires TDD. |
| IV. Cross-Platform Discipline | PASS | Each example uses `#[cfg()]` with `compile_error!` for wrong-platform builds. The Unix example compiles only on `unix`; the Windows example only on `windows`. |
| V. Semantic Versioning & MSRV | PASS | No changes to the library's public API or MSRV. Example projects match the parent crate's edition and MSRV. |
| Quality Gates | PASS | Example projects are independent crates not in the workspace. They do not affect `cargo test`, `cargo clippy`, or `cargo fmt` runs on the root crate. |

## Project Structure

### Documentation (this feature)

```text
specs/003-docs-examples/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (N/A - no external interfaces)
└── tasks.md             # Phase 2 output (/speckit.tasks command)
```

### Source Code (repository root)

```text
docs/
├── example-unix/
│   ├── Cargo.toml       # [package] binary crate, depends on logical-path = { path = "../../" }
│   ├── README.md        # Comprehensive README with problem, code snippets, setup instructions
│   └── src/
│       └── main.rs      # Unix-specific demo: broken cd (canonicalize) vs fixed cd (to_logical)
├── example-windows/
│   ├── Cargo.toml       # [package] binary crate, depends on logical-path = { path = "../../" }
│   ├── README.md        # Comprehensive README with problem, code snippets, setup instructions
│   └── src/
│       └── main.rs      # Windows-specific demo: broken cd (canonicalize) vs fixed cd (to_logical)
├── api-reference.md     # (existing, unchanged)
├── architecture.md      # (existing, unchanged)
├── examples.md          # (existing, unchanged)
├── FAQ.md               # (existing, unchanged)
├── how-it-works.md      # (existing, unchanged)
└── platform-behavior.md # (existing, unchanged)
```

**Structure Decision**: Two independent Cargo binary projects under `docs/`, each with `Cargo.toml`, `src/main.rs`, and `README.md`. They are NOT workspace members — building the root crate does not build or test these examples. Each depends on `logical-path` via relative path.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
| --------- | ---------- | ------------------------------------ |
| III. TDD deviation | Educational examples, not library code | Writing unit tests for example output would be over-engineering; the examples themselves *are* the demonstration |
