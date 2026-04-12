# Feature Specification: Runnable Example Projects

**Feature Branch**: `003-docs-examples`
**Created**: 2026-04-11
**Status**: Draft
**Input**: User description: "I want a set of examples created in the docs folder. There should be two runnable rust projects that show code that causes cd functions to unexpectedly result in the user being in the wrong folder. One of the projects will be specific to linux/mac, and the other project will be specific to Windows. The code in each project will have an example of what happens without using the logical-path crate, i.e., the user ends up in the wrong folder, and an example of using the logical-path create to fix the problem. Each example will have a comprehensive readme with code snippets showing the important code."

## Clarifications

### Session 2026-04-11

- Q: What should happen when someone builds an example on the wrong platform? → A: Use `compile_error!` macro for a clear compile-time error message.
- Q: What are the exact directory names for the example projects? → A: `docs/example-unix/` and `docs/example-windows/`.
- Q: Should examples simulate a realistic CLI tool or print raw path comparisons? → A: Simulate a realistic CLI tool that emits `cd` directives, showing wrong output first then corrected output.

## User Scenarios & Testing

### User Story 1 - Run Linux/macOS Example to See the Problem and Fix (Priority: P1)

A developer on Linux or macOS wants to understand why symlink-aware path handling matters. They clone the `logical-path` repository, navigate to the Linux/macOS example project, and run it. The example first demonstrates the broken behavior (a `cd` directive that resolves symlinks and lands the user in the wrong folder), then demonstrates the fix using the `logical-path` crate.

**Why this priority**: This is the primary use case — most Rust developers evaluating this crate will be on Linux or macOS and need to see the problem firsthand before understanding the value proposition.

**Independent Test**: Can be fully tested by running `cargo run` inside the Linux/macOS example project directory on a system with a symlink in the working directory path. The output clearly shows the broken path vs. the corrected path.

**Acceptance Scenarios**:

1. **Given** a developer has cloned the repository on Linux or macOS, **When** they navigate to the Linux/macOS example project and run `cargo run`, **Then** the program compiles and runs without errors.
2. **Given** the example is running on a system where the working directory traverses a symlink (e.g., `$PWD` differs from `getcwd()`), **When** the "without logical-path" section runs, **Then** the output shows a `cd` directive pointing to the canonical (symlink-resolved) path, demonstrating the user would end up in the wrong folder.
3. **Given** the example is running on the same symlinked system, **When** the "with logical-path" section runs, **Then** the output shows a `cd` directive pointing to the logical (symlink-preserving) path, demonstrating the user stays in the expected folder.
4. **Given** a developer is on a system without symlinks in the working directory path, **When** they run the example, **Then** the program still runs successfully and the output explains that both paths are identical because no symlink mapping is active.

---

### User Story 2 - Run Windows Example to See the Problem and Fix (Priority: P1)

A developer on Windows wants to understand why junction/subst-aware path handling matters. They clone the repository, navigate to the Windows example project, and run it. The example demonstrates the broken behavior (a `cd` directive that resolves NTFS junctions/subst drives and lands the user in the wrong folder), then demonstrates the fix using the `logical-path` crate.

**Why this priority**: Windows support is equally important — NTFS junctions, subst drives, and mapped network drives are common in enterprise Windows development environments.

**Independent Test**: Can be fully tested by running `cargo run` inside the Windows example project directory on a system with an NTFS junction or subst drive in the working directory path.

**Acceptance Scenarios**:

1. **Given** a developer has cloned the repository on Windows, **When** they navigate to the Windows example project and run `cargo run`, **Then** the program compiles and runs without errors.
2. **Given** the example is running on a system where the working directory uses an NTFS junction, subst drive, or directory symlink (e.g., `current_dir()` differs from `canonicalize()`), **When** the "without logical-path" section runs, **Then** the output shows a `cd` directive pointing to the canonical (resolved) path.
3. **Given** the same system, **When** the "with logical-path" section runs, **Then** the output shows a `cd` directive pointing to the logical (junction/subst-preserving) path.
4. **Given** a developer is on a Windows system without junctions or subst drives in the working directory path, **When** they run the example, **Then** the program still runs successfully and explains that both paths are identical.

---

### User Story 3 - Read the README to Understand the Example Without Running It (Priority: P2)

A developer browsing the repository on GitHub (or locally) wants to understand the problem and solution without running the code. They read the comprehensive README in each example project, which contains code snippets, expected output, and explanations.

**Why this priority**: Many developers evaluate libraries by reading documentation before running code. The README must stand alone as a learning resource.

**Independent Test**: Can be tested by reading the README on GitHub and verifying that all code snippets, expected output blocks, and explanations are present and coherent.

**Acceptance Scenarios**:

1. **Given** a developer opens the Linux/macOS example's README, **When** they read through it, **Then** they find: a description of the problem, a code snippet showing the broken behavior, expected output showing the wrong path, a code snippet showing the fix with `logical-path`, and expected output showing the correct path.
2. **Given** a developer opens the Windows example's README, **When** they read through it, **Then** they find equivalent content tailored to Windows scenarios (NTFS junctions, subst drives).
3. **Given** a developer reads either README, **When** they look for setup instructions, **Then** they find clear prerequisites and step-by-step instructions for creating the symlink/junction needed to reproduce the problem.

---

### Edge Cases

- What happens when the example is run from a directory without any symlink/junction mapping? The examples should handle this gracefully, outputting a message that explains no mapping was detected and both paths are identical.
- What happens when the example is compiled on the wrong platform (e.g., the Linux example compiled on Windows)? Each project MUST use `compile_error!` to produce a clear, descriptive compile-time error message (e.g., "This example requires Linux or macOS") rather than failing silently or with confusing linker errors.
- What happens when the `logical-path` crate dependency cannot be resolved? Each example project's `Cargo.toml` should reference the parent crate via a relative path dependency so it works out of the box from the repository.

## Requirements

### Functional Requirements

- **FR-001**: The repository MUST contain a runnable Rust example project specific to Linux/macOS, located at `docs/example-unix/`.
- **FR-002**: The repository MUST contain a runnable Rust example project specific to Windows, located at `docs/example-windows/`.
- **FR-003**: Each example project MUST be a standalone Cargo project with its own `Cargo.toml` that depends on `logical-path` via relative path.
- **FR-004**: Each example project MUST demonstrate the broken behavior by simulating a realistic CLI tool (e.g., a fictional directory-jumper) that emits a `cd` directive using `std::fs::canonicalize` or `std::env::current_dir` without `logical-path`, showing the wrong (canonical) path.
- **FR-005**: Each example project MUST demonstrate the fixed behavior by simulating the same CLI tool scenario using `LogicalPathContext::detect()` and `to_logical()`, emitting a `cd` directive with the correct (logical) path.
- **FR-006**: The Linux/macOS example MUST use Unix-specific concepts: `$PWD` vs `getcwd()` divergence caused by symlinks.
- **FR-007**: The Windows example MUST use Windows-specific concepts: `current_dir()` vs `canonicalize()` divergence caused by NTFS junctions, subst drives, or directory symlinks.
- **FR-008**: Each example project MUST include a comprehensive README with: problem description, code snippets showing the important code, expected output, and setup instructions for reproducing the scenario.
- **FR-009**: Each example's README MUST include instructions for creating the necessary symlink (Linux/macOS) or junction/subst drive (Windows) to reproduce the problem.
- **FR-010**: Each example MUST handle the case where no symlink/junction mapping is active, outputting an explanatory message instead of failing.
- **FR-011**: Each example project MUST use conditional compilation (`#[cfg()]` attributes) to restrict compilation to the intended target platform, using the `compile_error!` macro to produce a clear, descriptive error message when built on the wrong platform (e.g., "This example requires Linux or macOS").

### Key Entities

- **Example Project**: A standalone Cargo binary project at `docs/example-unix/` or `docs/example-windows/`, each containing `Cargo.toml`, `src/main.rs`, and `README.md`.
- **Broken Behavior Demo**: Code that uses standard library path functions without `logical-path`, demonstrating the symlink/junction resolution problem.
- **Fixed Behavior Demo**: Code that uses `LogicalPathContext` from the `logical-path` crate to translate canonical paths back to logical paths.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A developer can clone the repository, navigate to either example project, and run it with `cargo run` on the appropriate platform without modification.
- **SC-002**: The output of each example clearly distinguishes between the "broken" path and the "fixed" path, making the value of `logical-path` immediately obvious.
- **SC-003**: Each README is self-contained — a developer reading it on GitHub can understand the problem, the solution, and how to reproduce it without consulting other documentation.
- **SC-004**: Both example projects compile and run successfully when tested on their respective target platforms.
- **SC-005**: A developer unfamiliar with the crate can understand the core value proposition within 5 minutes of reading either README.

## Assumptions

- Developers evaluating the crate have Rust and Cargo installed.
- The example projects will reference `logical-path` as a path dependency (e.g., `logical-path = { path = "../../" }`) so they work directly from the repository without publishing.
- The Linux/macOS example will use a symlink scenario (e.g., `/workspace` → `/mnt/wsl/workspace` or a custom symlink) as its demonstration case.
- The Windows example will use an NTFS junction or `subst` drive as its demonstration case.
- Each example project is a binary crate (not a library) with a `main.rs` entry point.
- The examples are educational — they demonstrate the problem and solution rather than being production-ready tools.

## Scope Boundaries

### In Scope

- Two standalone Cargo binary projects (one Linux/macOS, one Windows)
- Comprehensive README for each project
- "Before" (broken) and "after" (fixed) code in each project
- Setup instructions for reproducing the symlink/junction scenario
- Graceful handling when no mapping is active

### Out of Scope

- Cross-platform examples that work on all platforms in a single binary
- CI/CD integration for running examples automatically
- Video tutorials or interactive demos
- Integration with the existing `docs/examples.md` documentation page
- Benchmark or performance examples
