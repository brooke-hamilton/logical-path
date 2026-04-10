# Tasks: Windows Full Support

**Input**: Design documents from `/specs/002-windows-full-support/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, contracts/public-api.md, quickstart.md

**Tests**: TDD is mandatory per the project constitution (Principle III). Tests are written before implementation in each phase, following the Red-Green-Refactor cycle.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing. US3 (\\?\ prefix stripping) is placed in the Foundational phase since it is a blocking prerequisite for all other Windows user stories.

**Task ID namespace**: IDs in this file (T001–T036, plus sub-IDs like T010a/T016a) are scoped to the 002-windows-full-support spec. They are independent of identically numbered IDs referenced in the 001-core-features spec.

**CI-untestable coverage**: `net use` mapped drives (FR-002) are detected by the same `current_dir()` vs `canonicalize()` mechanism as junctions and subst drives, but cannot be reliably created/torn down in CI. Coverage is by design review and the shared detection code path. If CI infrastructure for network drives becomes available, add dedicated integration tests.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the `log` dependency and prepare the crate for Windows-aware code

- [ ] T001 Add `log = "0.4"` to `[dependencies]` in Cargo.toml
- [ ] T002 [P] Update crate-level doc comment (`//!` block) in src/lib.rs to describe Windows detection via `current_dir()` vs `canonicalize()` instead of "returns no active mapping"

---

## Phase 2: Foundational — \\?\ Prefix Stripping & Cross-Platform Infrastructure

**Purpose**: Core infrastructure that MUST be complete before ANY Windows user story can be implemented. Implements US3 (Extended Length Path prefix stripping), cross-platform divergence algorithm, Windows detection skeleton, and trace diagnostics.

**CRITICAL**: No user story work (Phase 3+) can begin until this phase is complete.

### Tests for \\?\ Prefix Stripping (TDD: write first, must fail)

- [ ] T003 [P] Write `#[cfg(windows)]` unit tests for `strip_extended_length_prefix()` in src/lib.rs covering: `\\?\C:\Users\dev` → `C:\Users\dev`, `\\?\UNC\server\share\folder` → `\\server\share\folder`, `C:\Users\dev` (no prefix) → unchanged, empty path → unchanged

### Implementation for \\?\ Prefix Stripping

- [ ] T004 Implement `strip_extended_length_prefix(path: &Path) -> PathBuf` as a `#[cfg(windows)]` function in src/lib.rs per data-model.md rules (strip `\\?\` before drive letter, convert `\\?\UNC\` to `\\`)

### Tests for Cross-Platform Divergence Algorithm

- [ ] T005 [P] Write `#[cfg(windows)]` unit tests for case-insensitive `find_divergence_point()` in src/lib.rs covering: Windows paths with matching components differing only in case → no divergence, junction-like paths `D:\Projects\Workspace\src` vs `C:\workspace\src` with case-insensitive suffix match → correct prefix pair, identical Windows paths → `None`

### Implementation for Cross-Platform Divergence Algorithm

- [ ] T006 Refactor `find_divergence_point()` in src/lib.rs: remove `#[cfg(not(windows))]` gate, use `#[cfg(windows)]` `OsStr::eq_ignore_ascii_case()` and `#[cfg(not(windows))]` `==` for component comparison, preserve all existing `#[cfg(not(windows))]` unit tests unchanged

### Windows Detection Infrastructure

- [ ] T007 Write `#[cfg(windows)]` unit tests for `detect_from_cwd(cwd, canonical_cwd)` in src/lib.rs covering: cwd equals canonical_cwd → no mapping, cwd differs from canonical_cwd with common suffix → mapping with correct prefix pair, cwd differs with no common suffix → no mapping
- [ ] T008 Implement `detect_from_cwd(cwd: &Path, canonical_cwd: &Path) -> LogicalPathContext` as a `#[cfg(windows)] pub(crate)` method on `LogicalPathContext` in src/lib.rs that calls `find_divergence_point()` and returns the prefix mapping
- [ ] T009 Replace the no-op `#[cfg(windows)]` body of `detect()` in src/lib.rs with: call `current_dir()`, call `canonicalize()` on the result, apply `strip_extended_length_prefix()`, then delegate to `detect_from_cwd()`
- [ ] T010 Update the `translate()` method in src/lib.rs to apply `strip_extended_length_prefix()` to both `original_canonical` and `translated_canonical` on Windows (`#[cfg(windows)]`) before the round-trip comparison
- [ ] T010a [P] [US3] Write `#[cfg(windows)]` unit test in src/lib.rs: construct context with a Windows prefix mapping, call `to_logical()` with a `\\?\`-prefixed canonical path (e.g., `\\?\D:\projects\workspace\src\main.rs`), assert the returned path uses the logical prefix (FR-008: callers MUST NOT be required to pre-strip `\\?\` prefixes)
- [ ] T010b [US3] Update the `translate()` method in src/lib.rs to apply `strip_extended_length_prefix()` to the caller-provided `path` argument on Windows (`#[cfg(windows)]`) before the `strip_prefix` check, so that `\\?\`-prefixed input paths match the stored prefix mapping

### Trace Diagnostics (FR-013)

- [ ] T011 Add `log::trace!()` and `log::debug!()` calls to `detect()`, `detect_from()` (Unix), `detect_from_cwd()` (Windows), and `translate()` in src/lib.rs covering: current_dir/canonicalize values compared, mapping detected with prefix pair, no mapping detected reason, translate fallback reasons

### Windows Integration Test Infrastructure

- [ ] T012 [P] Implement `#[cfg(windows)]` test helpers in tests/integration.rs: `create_junction(link, target)` via `cmd /c mklink /J`, `create_dir_symlink(link, target)` via `cmd /c mklink /D`, `remove_junction(link)` via `cmd /c rd`, `create_subst(letter, target)` via `subst`, `remove_subst(letter)` via `subst /D`, and a `WinEnvGuard` RAII struct that saves/restores CWD and cleans up junctions/subst drives on drop

**Checkpoint**: Foundation ready — all Windows infrastructure is in place, existing Unix tests pass unchanged. User story implementation and testing can now begin.

---

## Phase 3: User Story 1 — Detect Logical Path via NTFS Junctions and Directory Symlinks (Priority: P1) MVP

**Goal**: Verify that `detect()` discovers NTFS junction and directory symlink mappings and `to_logical()` translates canonical paths back to the junction-based form.

**Independent Test**: Create an NTFS junction in a temp directory, set CWD to a path under the junction, call `detect()`, and assert the correct prefix mapping and `to_logical()` translation.

- [ ] T013 [P] [US1] Write `#[cfg(windows)]` integration test in tests/integration.rs: create NTFS junction (`mklink /J`) from temp link dir to temp real dir, set CWD to link path, call `detect()`, assert `has_mapping()` returns `true`
- [ ] T014 [P] [US1] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active junction mapping from `detect()`, call `to_logical()` with a canonical path under the real dir, assert the returned path uses the junction prefix
- [ ] T015 [US1] Write `#[cfg(windows)]` integration test in tests/integration.rs: with no junction in CWD path, call `detect()` (end-to-end via real OS calls), assert `has_mapping()` returns `false` (integration-level validation of US1-AS3)
- [ ] T016 [US1] Write `#[cfg(windows)]` integration test in tests/integration.rs: create junction, call `detect()`, remove junction, call `to_logical()` on a path under the former target, assert original path returned unchanged (round-trip validation failure)
- [ ] T016a [P] [US1] Write `#[cfg(windows)]` integration test in tests/integration.rs: create a Windows directory symlink (`mklink /D`) from temp link dir to temp real dir, set CWD to link path, call `detect()`, assert `has_mapping()` returns `true` and `to_logical()` returns the symlink-based path (FR-002 explicit directory symlink coverage per clarification session)
- [ ] T016b [US1] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active junction mapping from `detect()`, call `to_logical(canonical)` then `to_canonical(result)`, assert the final path equals the original canonical path (SC-005 round-trip property for junctions)

**Checkpoint**: NTFS junction detection is fully functional and independently verified on Windows.

---

## Phase 4: User Story 2 — Detect Logical Path via Subst Drives (Priority: P1)

**Goal**: Verify that `detect()` discovers `subst` drive letter mappings and `to_logical()` translates canonical paths back to the drive-letter form.

**Independent Test**: Create a `subst` mapping to a temp directory, set CWD to the substituted drive, call `detect()`, and assert the correct prefix mapping and `to_logical()` translation.

- [ ] T017 [P] [US2] Write `#[cfg(windows)]` integration test in tests/integration.rs: create `subst` mapping (e.g., `Z:` → temp dir), set CWD to `Z:\`, call `detect()`, assert `has_mapping()` returns `true`
- [ ] T018 [P] [US2] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active subst mapping from `detect()`, call `to_logical()` with a canonical path under the physical dir, assert the returned path uses the subst drive letter prefix
- [ ] T019 [US2] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active subst mapping, call `to_logical()` with a canonical path on a different drive (outside the mapping), assert the original path is returned unchanged
- [ ] T020 [US2] Write `#[cfg(windows)]` integration test in tests/integration.rs: create subst mapping, call `detect()`, remove subst, call `to_logical()` on a path under the former target, assert original path returned unchanged
- [ ] T020a [US2] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active subst mapping from `detect()`, call `to_logical(canonical)` then `to_canonical(result)`, assert the final path equals the original canonical path (SC-005 round-trip property for subst drives)

**Checkpoint**: Subst drive detection is fully functional and independently verified on Windows.

---

## Phase 5: User Story 5 — Graceful Fallback on Windows (Priority: P1)

**Goal**: Verify that all failure modes on Windows return the input path unchanged without panicking.

**Independent Test**: Call `detect()` and translation methods in adverse conditions (no indirection, relative paths, stale mappings) and assert fallback behavior.

- [ ] T021 [P] [US5] Write `#[cfg(windows)]` unit test for `detect_from_cwd()` in src/lib.rs: pass identical `cwd` and `canonical_cwd` Windows paths, assert `has_mapping()` returns `false` and `to_logical()`/`to_canonical()` return input unchanged (unit-level validation of the internal helper, distinct from integration-level T015)
- [ ] T022 [P] [US5] Write `#[cfg(windows)]` unit test in src/lib.rs: construct context with Windows prefix mapping, call `to_logical()` and `to_canonical()` with a relative path (`src\main.rs`), assert input returned unchanged
- [ ] T023 [US5] Write `#[cfg(windows)]` integration test in tests/integration.rs: create junction, `detect()`, retarget junction to a different directory, call `to_logical()`, assert original path returned unchanged (TOCTOU round-trip validation catches stale mapping)

**Checkpoint**: Graceful fallback behavior is verified for all adverse Windows conditions.

---

## Phase 6: User Story 6 — Backward Compatibility with Existing Unix Behavior (Priority: P1)

**Goal**: Confirm that all existing Linux and macOS detection and translation behavior remains unchanged after the Windows changes.

**Independent Test**: Run the entire existing test suite on Unix and confirm no modifications were needed and all tests pass.

- [ ] T024 [US6] Run `cargo test` on Linux and verify all existing unit tests in src/lib.rs and integration tests in tests/integration.rs pass without modification (SC-006)
- [ ] T025 [US6] Audit src/lib.rs to confirm no existing `#[cfg(not(windows))]` or `#[cfg(unix)]` gates were removed or altered, and no Unix code paths were changed — document audit result as a comment in the PR
- [ ] T025a [US6] Audit the `#[cfg(windows)]` `detect()` code path in src/lib.rs to confirm no `$PWD` staleness validation is applied (FR-003) — document audit result as a comment in the PR

**Checkpoint**: Full backward compatibility confirmed. No Unix regressions.

---

## Phase 7: User Story 4 — Translate Logical-to-Canonical on Windows (Priority: P2)

**Goal**: Verify that `to_canonical()` correctly replaces the logical prefix with the canonical prefix on Windows, with round-trip validation.

**Independent Test**: Construct contexts with known Windows junction and subst prefix mappings, call `to_canonical()` with logical paths, and assert the canonical form is returned.

- [ ] T026 [P] [US4] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active junction mapping from `detect()`, call `to_canonical()` with a logical path under the junction, assert the returned path uses the physical/canonical prefix
- [ ] T027 [P] [US4] Write `#[cfg(windows)]` integration test in tests/integration.rs: with active junction mapping, call `to_canonical()` with a path that does NOT start with the logical prefix, assert the original path is returned unchanged
- [ ] T028 [US4] Write `#[cfg(windows)]` integration test in tests/integration.rs: with no active mapping (`detect()` in a plain directory), call `to_canonical()` with any path, assert the input is returned unchanged

**Checkpoint**: Bidirectional translation (logical↔canonical) is fully functional on Windows.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation updates, diagnostics verification, and quality gate compliance

- [ ] T029 Update `LogicalPathContext` struct doc comment and `detect()` method doc comment in src/lib.rs to describe Windows detection behavior (replace "Windows: Always reports no active mapping" with accurate description)
- [ ] T030 [P] Update README.md "Platform Behavior" / "Platform Notes" section to document Windows junction, directory symlink, subst drive, and mapped drive detection
- [ ] T031 [P] Write integration test in tests/integration.rs that enables a `log`-compatible test subscriber, calls `detect()` and `to_logical()`, and asserts trace-level diagnostic messages are emitted (SC-009)
- [ ] T032 [P] Remove or update the existing `#[cfg(windows)]` integration test `detect_returns_no_mapping_on_windows` in tests/integration.rs since Windows `detect()` now returns mappings when indirections exist
- [ ] T033 Run `cargo clippy -- --deny warnings` targeting src/lib.rs and tests/integration.rs and fix any warnings
- [ ] T034 Run `cargo fmt --check` and fix any formatting issues in src/lib.rs and tests/integration.rs
- [ ] T035 Run `cargo doc --no-deps` and fix any documentation warnings (ensure all public items have doc comments)
- [ ] T036 Run quickstart.md validation: `cargo build`, `cargo test`, `cargo clippy -- --deny warnings`, `cargo fmt --check` on all supported platforms

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup (T001 for `log` dep). BLOCKS all user stories.
- **User Stories (Phase 3–7)**: All depend on Foundational (Phase 2) completion
  - US1 (Phase 3) and US2 (Phase 4) can proceed in parallel — they use independent filesystem mechanisms (junctions vs subst)
  - US5 (Phase 5) depends on Phase 2 for detection infrastructure; T023 depends on T012 (junction helpers)
  - US6 (Phase 6) can start after Phase 2 — just verification of existing tests
  - US4 (Phase 7) depends on Phase 2 for translate() updates; integration tests reuse T012 helpers
- **Polish (Phase 8)**: Depends on all user stories being complete

### User Story Dependencies

- **US1 (P1)**: Depends on Phase 2 only. No dependency on other stories.
- **US2 (P1)**: Depends on Phase 2 only. No dependency on other stories. Can run in parallel with US1.
- **US5 (P1)**: Depends on Phase 2. T023 uses junction helpers from T012. Can start after Phase 2.
- **US6 (P1)**: Depends on Phase 2. Independent of all other stories. Can start after Phase 2.
- **US4 (P2)**: Depends on Phase 2. Integration tests use junction helpers from T012. Can start after Phase 2.

### Within Each Phase

- Tests MUST be written and FAIL before implementation (TDD)
- Implementation tasks make the failing tests pass
- Foundational infrastructure before story-specific tests
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- T001 and T002 are parallel (different files: Cargo.toml vs src/lib.rs)
- T003, T005, T010a, and T012 are all parallel (independent test groups in different scopes)
- Phase 3 (US1) and Phase 4 (US2) can proceed in parallel (independent filesystem mechanisms)
- T013, T014, and T016a are parallel (independent test cases)
- T016b depends on T013/T014 (needs active junction mapping)
- T017 and T018 are parallel (independent test cases)
- T020a depends on T017/T018 (needs active subst mapping)
- T021 and T022 are parallel (independent unit test cases)
- T026 and T027 are parallel (independent test cases)
- T029, T030, T031, and T032 are parallel (different files/scopes)

---

## Parallel Example: Phase 2 Foundational

```text
# Write all tests in parallel first:
T003: "Unit tests for strip_extended_length_prefix"  ─┐
T005: "Unit tests for case-insensitive divergence"    ├─ parallel (independent test groups)
T010a: "Unit test for \\?\ input path stripping"      │
T012: "Windows integration test helpers"              ─┘

# Then implement sequentially (each makes its tests pass):
T004: "Implement strip_extended_length_prefix"
T006: "Refactor find_divergence_point cross-platform"
T007 → T008: "detect_from_cwd tests → implementation"
T009: "Wire up detect() body"
T010: "Update translate() for Windows round-trip"
T010b: "Strip \\?\ from caller input in translate()"
T011: "Add log diagnostics"
```

## Parallel Example: User Stories After Phase 2

```text
# US1 and US2 can run in parallel:
Developer A: T013, T014, T015, T016, T016a, T016b (NTFS junctions + dir symlinks)
Developer B: T017, T018, T019, T020, T020a (Subst drives)

# US5 and US6 can start as soon as Phase 2 completes:
T021, T022, T023 (Fallback tests)
T024, T025, T025a (Backward compat verification + FR-003 audit)
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001–T002)
2. Complete Phase 2: Foundational (T003–T012) — CRITICAL, blocks all stories
3. Complete Phase 3: US1 — NTFS Junction & Directory Symlink Detection (T013–T016b)
4. **STOP and VALIDATE**: Test junction detection independently on Windows
5. Deploy/demo if ready — junctions are the highest-impact Windows capability

### Incremental Delivery

1. Setup + Foundational → Windows infrastructure ready
2. Add US1 (Junctions) → Test independently → MVP ready
3. Add US2 (Subst Drives) → Test independently → Second capability
4. Add US5 (Fallback) + US6 (Backward Compat) → Safety/regression validation
5. Add US4 (Logical-to-Canonical) → Full bidirectional translation
6. Polish → Documentation, diagnostics, quality gates

### Task Count by User Story

| Story | Phase | Tasks | Key Files |
| ----- | ----- | ----- | --------- |
| Setup | 1 | 2 | Cargo.toml, src/lib.rs |
| Foundational (US3 + infra) | 2 | 12 | src/lib.rs, tests/integration.rs |
| US1: NTFS Junctions + Dir Symlinks | 3 | 6 | tests/integration.rs |
| US2: Subst Drives | 4 | 5 | tests/integration.rs |
| US5: Graceful Fallback | 5 | 3 | src/lib.rs, tests/integration.rs |
| US6: Backward Compat | 6 | 3 | src/lib.rs, tests/integration.rs |
| US4: Logical-to-Canonical | 7 | 3 | tests/integration.rs |
| Polish | 8 | 8 | src/lib.rs, tests/integration.rs, README.md |
| **Total** | | **42** | |

---

## Notes

- [P] tasks = different files or independent test cases, no dependencies on incomplete tasks
- [Story] label maps task to specific user story for traceability
- All Windows-specific code uses `#[cfg(windows)]` gates (FR-011)
- All Windows tests use `#[cfg(windows)]` gates and skip gracefully if commands fail
- Existing Unix tests remain gated with `#[cfg(not(windows))]` or `#[cfg(unix)]` (FR-006)
- Round-trip validation with `\\?\` stripping ensures correctness on Windows (FR-007, FR-008)
- `log` crate has zero overhead when no logger is configured (FR-013, R-006)
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
