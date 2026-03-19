# Tasks: logical-path Core Library

**Input**: Design documents from `specs/001-core-features/`
**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/public-api.md ✅

**Tests**: Included — TDD is mandated by the project constitution (Principle III).

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization, dependency configuration, and scaffolding

- [ ] T001 Add `tempfile` dev-dependency to Cargo.toml
- [ ] T002 Replace placeholder code in src/lib.rs with module-level doc comment, public struct `LogicalPathContext` (empty), and internal struct `PrefixMapping` (empty), with `Debug`, `Clone`, `PartialEq`, `Eq` derives per data-model.md
- [ ] T003 [P] Create tests/integration.rs with an empty test module and `use logical_path::LogicalPathContext;` import

**Checkpoint**: Project compiles with `cargo build` and `cargo test` (no real tests yet).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Internal data structures and helpers that ALL user stories depend on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [ ] T004 Implement `PrefixMapping` struct fields (`canonical_prefix: PathBuf`, `logical_prefix: PathBuf`) and derive `Debug, Clone, PartialEq, Eq` in src/lib.rs
- [ ] T005 Implement `LogicalPathContext` struct field (`mapping: Option<PrefixMapping>`) with private fields in src/lib.rs
- [ ] T006 Implement `has_mapping(&self) -> bool` method on `LogicalPathContext` that returns `self.mapping.is_some()` in src/lib.rs
- [ ] T007 Add unit test for `has_mapping()` returning `false` when mapping is `None` and `true` when mapping is `Some` in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T007a Add compile-time `Send + Sync` assertion test for `LogicalPathContext` in src/lib.rs `#[cfg(test)] mod tests` to prevent accidental regressions
- [ ] T008 Implement internal helper function `find_divergence_point(canonical: &Path, logical: &Path)` that uses `Path::components()` to find the longest common suffix, and returns `Option<(PathBuf, PathBuf)>` where the first element is the canonical prefix and the second is the logical prefix in src/lib.rs
- [ ] T009 Add unit tests for `find_divergence_point` covering: identical paths (returns None), paths with common suffix and different prefixes, paths with no common components, paths with trailing slashes/redundant separators, and paths containing `.` and `..` components in src/lib.rs `#[cfg(test)] mod tests`

**Checkpoint**: Foundation ready — `PrefixMapping`, `LogicalPathContext`, `has_mapping()`, and suffix-matching algorithm are implemented and tested.

---

## Phase 3: User Story 1 — Detect Active Symlink Mapping (Priority: P1) 🎯 MVP

**Goal**: `LogicalPathContext::detect()` reads `$PWD` vs `getcwd()`, computes the prefix mapping, and returns a context. Never panics, never errors.

**Independent Test**: Call `detect()` with controlled `$PWD` env var values and assert correct mapping or no-mapping states.

### Tests for User Story 1

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T010 [US1] Write unit test: `detect_from()` with `pwd` matching canonical CWD returns context with `has_mapping() == false` in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T011 [US1] Write unit test: `detect_from()` with `pwd` as `None` returns context with `has_mapping() == false` in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T012 [US1] Write integration test: `detect()` inside a real symlink directory returns context with `has_mapping() == true` and correct prefix pair in tests/integration.rs
- [ ] T012a [US1] Write integration test on Unix (`#[cfg(unix)]`): `detect()` with nested symlinks (symlink pointing through another symlink) detects only the outermost divergence mapping in tests/integration.rs
- [ ] T013 [US1] Write unit test: `detect_from()` with stale `pwd` (non-existent path) returns context with `has_mapping() == false` in src/lib.rs `#[cfg(test)] mod tests`

### Implementation for User Story 1

- [ ] T014 [US1] Implement `LogicalPathContext::detect()` in src/lib.rs: create a `pub(crate) fn detect_from(pwd: Option<&OsStr>, canonical_cwd: &Path) -> LogicalPathContext` internal helper that takes `$PWD` and canonical CWD as parameters (for testability without modifying global process state). The public `detect()` reads `$PWD` via `std::env::var_os("PWD")`, gets canonical CWD via `std::env::current_dir()`, and delegates to `detect_from()`. `detect_from()` calls `find_divergence_point` and constructs `LogicalPathContext` with the resulting mapping. On Windows (`#[cfg(windows)]`), always return no-mapping context. Handle all error cases by returning no-mapping context.
- [ ] T015 [US1] Add doc comment and `#[must_use]` to `detect()` per contracts/public-api.md including platform behavior, panic guarantee, and fallback cases in src/lib.rs
- [ ] T016 [US1] Verify all US1 tests pass with `cargo test`

**Checkpoint**: `LogicalPathContext::detect()` is fully functional and independently testable.

---

## Phase 4: User Story 2 — Translate Canonical Path to Logical (Priority: P1)

**Goal**: `ctx.to_logical(&canonical_path)` replaces the canonical prefix with the logical prefix, validates via round-trip canonicalization, and falls back to input if anything fails.

**Independent Test**: Construct a `LogicalPathContext` with a known prefix pair, call `to_logical()` with paths inside and outside the prefix, and verify correct translations or fallbacks.

### Tests for User Story 2

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T017 [US2] Write unit test: `to_logical()` with active mapping and path under canonical prefix returns translated path in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T018 [US2] Write unit test: `to_logical()` with active mapping and path NOT under canonical prefix returns input unchanged in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T019 [US2] Write unit test: `to_logical()` with no active mapping returns input unchanged in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T020 [US2] Write integration test: `to_logical()` with real symlink environment performs correct translation with round-trip validation in tests/integration.rs

### Implementation for User Story 2

- [ ] T021 [US2] Implement `to_logical(&self, path: &Path) -> PathBuf` in src/lib.rs: check for active mapping, check path starts with canonical prefix via `Path::starts_with()`, strip canonical prefix and prepend logical prefix, run round-trip validation (`canonicalize(translated) == canonicalize(original)`), fall back to input on any failure
- [ ] T022 [US2] Add doc comment and `#[must_use]` to `to_logical()` per contracts/public-api.md including fallback cases and panic guarantee in src/lib.rs
- [ ] T023 [US2] Verify all US2 tests pass with `cargo test`

**Checkpoint**: `to_logical()` is fully functional. Combined with US1, paths can be detected and translated canonical → logical.

---

## Phase 5: User Story 3 — Translate Logical Path to Canonical (Priority: P2)

**Goal**: `ctx.to_canonical(&logical_path)` replaces the logical prefix with the canonical prefix, validates via round-trip, and falls back to input if anything fails.

**Independent Test**: Construct a `LogicalPathContext` with a known prefix pair, call `to_canonical()` with paths inside and outside the prefix, and verify correct translations or fallbacks.

### Tests for User Story 3

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T024 [US3] Write unit test: `to_canonical()` with active mapping and path under logical prefix returns translated path in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T025 [US3] Write unit test: `to_canonical()` with active mapping and path NOT under logical prefix returns input unchanged in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T026 [US3] Write unit test: `to_canonical()` with no active mapping returns input unchanged in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T027 [US3] Write integration test: `to_canonical()` with real symlink environment performs correct translation with round-trip validation in tests/integration.rs

### Implementation for User Story 3

- [ ] T028 [US3] Implement `to_canonical(&self, path: &Path) -> PathBuf` in src/lib.rs: check for active mapping, check path starts with logical prefix via `Path::starts_with()`, strip logical prefix and prepend canonical prefix, run round-trip validation, fall back to input on any failure
- [ ] T029 [US3] Add doc comment and `#[must_use]` to `to_canonical()` per contracts/public-api.md including fallback cases and panic guarantee in src/lib.rs
- [ ] T030 [US3] Verify all US3 tests pass with `cargo test`
- [ ] T030a Add parameterised round-trip test covering ≥10 distinct canonical/logical path structures per platform to satisfy SC-003, using a test table or `proptest` dev-dependency in src/lib.rs `#[cfg(test)] mod tests` or tests/integration.rs

**Checkpoint**: `to_canonical()` is fully functional. Full bidirectional translation is now available.

---

## Phase 6: User Story 4 — Graceful Fallback Preserves Caller Correctness (Priority: P1)

**Goal**: Verify and harden the fallback guarantee across all adverse conditions: unset `$PWD`, stale mappings, non-matching paths, non-UTF-8 paths. No API call ever panics or returns an error.

**Independent Test**: Call `detect()`, `to_logical()`, and `to_canonical()` under every adverse condition and assert the input is returned unchanged without panicking.

### Tests for User Story 4

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T031 [P] [US4] Write unit test: `to_logical()` and `to_canonical()` both return input unchanged when round-trip validation would fail in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T032 [P] [US4] Write unit test on Unix (`#[cfg(unix)]`): paths with non-UTF-8 bytes are handled without panicking by `to_logical()` and `to_canonical()` in src/lib.rs `#[cfg(test)] mod tests`
- [ ] T033 [P] [US4] Write unit test: `detect_from()` with corrupted/partially-resolved `pwd` returns no-mapping context in src/lib.rs `#[cfg(test)] mod tests`

### Implementation for User Story 4

- [ ] T034 [US4] Review and harden `detect()`, `to_logical()`, and `to_canonical()` for all fallback edge cases in src/lib.rs: confirm no `unwrap()` or `expect()` on fallible operations, confirm `OsStr`/`Path` used throughout (no UTF-8 conversions), confirm all error branches return input unchanged
- [ ] T035 [US4] Verify all US4 tests pass with `cargo test`

**Checkpoint**: Fallback guarantee is verified under all adverse conditions. No API call panics.

---

## Phase 7: User Story 5 — Cross-Platform Operation (Priority: P2)

**Goal**: Library compiles and behaves correctly on Linux, macOS, and Windows. Platform-specific code is gated with `#[cfg()]`. Windows always returns no-mapping context.

**Independent Test**: Platform-gated tests verify correct behavior on each platform.

### Tests for User Story 5

> **Write these tests FIRST, ensure they FAIL before implementation**

- [ ] T036 [P] [US5] Write platform-gated test (`#[cfg(target_os = "linux")]`): `detect()` with real symlink works on Linux in tests/integration.rs
- [ ] T037 [P] [US5] Write platform-gated test (`#[cfg(target_os = "macos")]`): `detect()` handles `/private/var` → `/var` mapping on macOS in tests/integration.rs
- [ ] T038 [P] [US5] Write platform-gated test (`#[cfg(windows)]`): `detect()` returns no-mapping and all translations return input unchanged on Windows in tests/integration.rs

### Implementation for User Story 5

- [ ] T039 [US5] Ensure all platform-specific code paths in `detect()` are gated with `#[cfg(unix)]` / `#[cfg(windows)]` in src/lib.rs
- [ ] T040 [US5] Verify `cargo test` passes (on current platform) and `cargo clippy -- --deny warnings` and `cargo fmt --check` pass

**Checkpoint**: Cross-platform operation is verified. All platform-gated code is correctly conditioned.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, quality gates, and final validation

- [ ] T041 [P] Add crate-level doc comment (`//!`) to src/lib.rs with usage examples per quickstart.md, platform notes, and `#![deny(missing_docs)]` attribute
- [ ] T042 [P] Verify `cargo doc --no-deps` completes with zero warnings
- [ ] T043 Run full quality gate: `cargo test && cargo clippy -- --deny warnings && cargo fmt --check && cargo doc --no-deps`
- [ ] T044 Run quickstart.md validation: verify the basic usage example compiles and the documented API matches the implementation
- [ ] T045 Remove placeholder `add()` function and its test from src/lib.rs if still present
- [ ] T046 [P] Create `.github/workflows/ci.yml` with matrix build for Linux, macOS, and Windows running `cargo test`, `cargo clippy -- --deny warnings`, `cargo fmt --check`, `cargo doc --no-deps`, and an MSRV build job
- [ ] T047 [P] Pin `rust-version = "1.85.0"` (minimum for edition 2024) in Cargo.toml and add "Minimum Supported Rust Version" section to README.md

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **US1 Detect (Phase 3)**: Depends on Foundational — BLOCKS US2, US3
- **US2 to_logical (Phase 4)**: Depends on US1
- **US3 to_canonical (Phase 5)**: Depends on US1. Independent of US2.
- **US4 Fallback (Phase 6)**: Depends on US1, US2, US3 (verifies behavior across all methods)
- **US5 Cross-Platform (Phase 7)**: Depends on US1, US2, US3 (verifies platform behavior of all methods)
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (P1)**: Can start after Foundational (Phase 2) — Foundation for all other stories
- **US2 (P1)**: Can start after US1 — Requires `detect()` and `LogicalPathContext` to exist
- **US3 (P2)**: Can start after US1 — Independent of US2, requires `detect()` and `LogicalPathContext`
- **US4 (P1)**: Cross-cutting — verifies fallback behavior of US1 + US2 + US3
- **US5 (P2)**: Cross-cutting — verifies platform behavior of US1 + US2 + US3

### Within Each User Story

- Tests MUST be written and FAIL before implementation (TDD, Constitution Principle III)
- Implementation follows Red-Green-Refactor cycle
- Story complete before moving to next priority

### Parallel Opportunities

- **Phase 1**: T003 can run in parallel with T001/T002
- **Phase 2**: T008/T009 can run in parallel with T007 (different functions)
- **Phase 4 & 5**: US2 and US3 can execute in parallel after US1 completes (independent translation directions)
- **Phase 6**: All US4 test tasks (T031, T032, T033) can run in parallel
- **Phase 7**: All US5 test tasks (T036, T037, T038) can run in parallel
- **Phase 8**: T041, T042, T046, and T047 can run in parallel

---

## Parallel Example: After US1 Completes

```text
# US2 and US3 can proceed in parallel since they are independent:
# Developer A: US2 (to_logical)
T017 → T018 → T019 → T020 → T021 → T022 → T023

# Developer B: US3 (to_canonical)
T024 → T025 → T026 → T027 → T028 → T029 → T030
```

---

## Implementation Strategy

### MVP First (User Stories 1 + 2)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL — blocks all stories)
3. Complete Phase 3: US1 — Detect
4. Complete Phase 4: US2 — to_logical
5. **STOP and VALIDATE**: `detect()` + `to_logical()` are independently testable
6. This delivers the primary value proposition of the crate

### Incremental Delivery

1. Setup + Foundational → Foundation ready
2. Add US1 (Detect) → Test independently → Core detection works
3. Add US2 (to_logical) → Test independently → **MVP!** Primary use case works
4. Add US3 (to_canonical) → Test independently → Bidirectional translation
5. Add US4 (Fallback) → Verify safety net across all methods
6. Add US5 (Cross-Platform) → Verify platform correctness
7. Polish → Documentation, quality gates, final validation
8. Each story adds value without breaking previous stories

---

## Notes

- All code lives in src/lib.rs (single-module design per plan.md)
- Integration tests requiring real symlinks go in tests/integration.rs
- No `unsafe` code permitted
- No external dependencies for core logic (`std` only)
- `tempfile` is a dev-dependency for integration tests only
- TDD is mandatory per constitution — every test must fail before implementation
- Unit tests for `detect()` logic use `detect_from()` to avoid process-global state mutations; this eliminates parallel test races on `$PWD`
- All public methods are annotated with `#[must_use]`
- Edition 2024 implies MSRV ≥ 1.85.0
