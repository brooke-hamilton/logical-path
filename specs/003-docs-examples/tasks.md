# Tasks: Runnable Example Projects

**Input**: Design documents from `/specs/003-docs-examples/`
**Prerequisites**: plan.md (required), spec.md (required), research.md, data-model.md, quickstart.md

**Tests**: Not requested — these are educational example projects, not library code. Verified by `cargo build` and manual `cargo run`.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the two standalone Cargo binary project directories and manifests

- [x] T001 Create Unix example project directory structure at `docs/example-unix/src/`
- [x] T002 [P] Create Unix example Cargo manifest at `docs/example-unix/Cargo.toml` with edition 2024, rust-version 1.85.0, and `logical-path = { path = "../../" }` dependency
- [x] T003 [P] Create Windows example project directory structure at `docs/example-windows/src/`
- [x] T004 [P] Create Windows example Cargo manifest at `docs/example-windows/Cargo.toml` with edition 2024, rust-version 1.85.0, and `logical-path = { path = "../../" }` dependency

**Checkpoint**: Two empty binary crate skeletons exist, each with valid `Cargo.toml`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: No shared foundational infrastructure is needed for this feature — the examples are independent standalone crates with no shared code between them. Proceed directly to user story phases.

**Checkpoint**: N/A — skip to Phase 3

---

## Phase 3: User Story 1 - Run Linux/macOS Example to See the Problem and Fix (Priority: P1) 🎯 MVP

**Goal**: A developer on Linux/macOS can clone the repo, navigate to `docs/example-unix/`, and run `cargo run` to see the broken `cd` behavior (symlinks resolved away) followed by the corrected behavior (symlinks preserved via `logical-path`).

**Independent Test**: Run `cargo build` in `docs/example-unix/` on a Unix system to verify compilation. Run `cargo run` in a symlinked directory to verify broken vs. fixed output. Run `cargo run` without a symlink to verify the no-mapping explanatory message.

### Implementation for User Story 1

- [x] T005 [US1] Implement Unix example main with `compile_error!` platform guard, `broken_cd_demo()`, `fixed_cd_demo()`, no-mapping handling, and `main()` entry point in `docs/example-unix/src/main.rs`
- [x] T006 [US1] Verify Unix example compiles by running `cargo build` in `docs/example-unix/`

**Checkpoint**: User Story 1 implementation is complete and compiles on the target platform

---

## Phase 4: User Story 2 - Run Windows Example to See the Problem and Fix (Priority: P1)

**Goal**: A developer on Windows can clone the repo, navigate to `docs/example-windows/`, and run `cargo run` to see the broken `cd` behavior (junctions/subst resolved away) followed by the corrected behavior (preserved via `logical-path`).

**Independent Test**: Run `cargo build` in `docs/example-windows/` on a Windows system to verify compilation. Run `cargo run` with an NTFS junction or subst drive to verify broken vs. fixed output. Run `cargo run` without a junction to verify the no-mapping message.

### Implementation for User Story 2

- [x] T007 [US2] Implement Windows example main with `compile_error!` platform guard, `broken_cd_demo()`, `fixed_cd_demo()`, no-mapping handling, and `main()` entry point in `docs/example-windows/src/main.rs`
- [x] T008 [US2] Verify Windows example compiles by running `cargo build` in `docs/example-windows/` on a Windows system

**Checkpoint**: User Story 2 implementation is complete and compiles on the target platform

---

## Phase 5: User Story 3 - Read the README to Understand the Example Without Running It (Priority: P2)

**Goal**: A developer browsing the repository on GitHub can read each README and fully understand the problem, the solution, and how to reproduce it — without running any code.

**Independent Test**: Open each README on GitHub (or locally) and verify it contains: problem description, code snippets for broken and fixed behavior, expected output blocks, setup instructions for creating symlinks/junctions, and the no-mapping case explanation.

### Implementation for User Story 3

- [x] T009 [P] [US3] Write comprehensive README for Unix example at `docs/example-unix/README.md` with problem description, code snippets from `main.rs`, expected output for symlink and no-symlink cases, and step-by-step symlink setup instructions
- [x] T010 [P] [US3] Write comprehensive README for Windows example at `docs/example-windows/README.md` with problem description, code snippets from `main.rs`, expected output for junction and no-junction cases, and step-by-step junction/subst setup instructions

**Checkpoint**: Both READMEs are self-contained learning resources viewable on GitHub

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final validation across both examples

- [x] T011 Run `quickstart.md` verification checklist against both example projects
- [x] T012 Verify both example projects are NOT included in root workspace by running `cargo build` at repo root and confirming examples are not built

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: N/A — skipped
- **User Story 1 (Phase 3)**: Depends on T001 and T002 from Setup
- **User Story 2 (Phase 4)**: Depends on T003 and T004 from Setup
- **User Story 3 (Phase 5)**: Depends on T005 (Unix main.rs) and T007 (Windows main.rs) — READMEs reference code snippets from the source files
- **Polish (Phase 6)**: Depends on all user stories being complete

### User Story Dependencies

- **User Story 1 (P1)**: Depends only on Setup (T001, T002) — no dependency on other stories
- **User Story 2 (P1)**: Depends only on Setup (T003, T004) — no dependency on other stories
- **User Story 3 (P2)**: Depends on US1 (T005) and US2 (T007) for code snippets — but can start drafts earlier

### User Story 1 & 2 Parallelism

User Stories 1 and 2 are fully independent and can be implemented in parallel. They share no files, no code, and no dependencies on each other.

### Within Each User Story

- Setup tasks (Cargo.toml, directory) before source implementation
- Source implementation before README (README references code snippets)
- Build verification after source implementation

### Parallel Opportunities

- T002, T003, T004 can all run in parallel (different files, independent projects)
- T005 and T007 can run in parallel (different projects, no shared code)
- T009 and T010 can run in parallel (different README files)

---

## Parallel Example: Setup Phase

```text
# Launch all independent setup tasks together:
Task T002: "Create Unix example Cargo.toml at docs/example-unix/Cargo.toml"
Task T003: "Create Windows example directory at docs/example-windows/src/"
Task T004: "Create Windows example Cargo.toml at docs/example-windows/Cargo.toml"
```

## Parallel Example: User Stories 1 & 2

```text
# Both user stories can proceed simultaneously after their setup tasks:
Task T005: "Implement Unix example main.rs at docs/example-unix/src/main.rs"
Task T007: "Implement Windows example main.rs at docs/example-windows/src/main.rs"
```

## Parallel Example: User Story 3

```text
# Both READMEs can be written in parallel:
Task T009: "Write Unix example README at docs/example-unix/README.md"
Task T010: "Write Windows example README at docs/example-windows/README.md"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (T001-T004)
2. Complete Phase 3: User Story 1 (T005-T006)
3. **STOP and VALIDATE**: Run `cargo build` in `docs/example-unix/` on a Unix system
4. Deploy/demo if ready — Unix example is usable

### Incremental Delivery

1. Complete Setup → Both project skeletons ready
2. Add User Story 1 (Unix main.rs) → Build and validate → MVP!
3. Add User Story 2 (Windows main.rs) → Build and validate on Windows
4. Add User Story 3 (READMEs) → Review on GitHub for completeness
5. Polish → Final cross-project validation
6. Each story adds value without breaking previous stories

### Parallel Team Strategy

With multiple developers:

1. Setup phase is quick — one developer can complete all 4 tasks
2. Once Setup is done:
   - Developer A: User Story 1 (Unix example)
   - Developer B: User Story 2 (Windows example)
3. Once US1 and US2 are done:
   - Developer A: Unix README (T009)
   - Developer B: Windows README (T010)
4. Polish together

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- No tests requested — examples are verified by `cargo build` and manual `cargo run`
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
