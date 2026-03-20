<!--
================================================================================
SYNC IMPACT REPORT
================================================================================
Version change: (none) → 1.0.0  (initial adoption)

Modified principles: N/A — first adoption; no prior principles to rename.

Added sections:
  - Core Principles (5 principles: Library-First Design, Correctness & Safety,
    Test-First (TDD), Cross-Platform Discipline, Semantic Versioning & MSRV Policy)
  - Quality Gates (Section 2)
  - Contribution Workflow (Section 3)
  - Governance

Removed sections: N/A — initial adoption.

Templates reviewed for consistency:
  ✅ .specify/templates/plan-template.md
       — "Constitution Check" gate list is intentionally left to the per-feature
         speckit.plan agent; no hard-coded principle names to update.
       — Language/Version examples already include "Rust 1.75"; testing example
         already includes "cargo test". No changes required.
  ✅ .specify/templates/spec-template.md
       — Fully generic; no constitution-specific references. No changes required.
  ✅ .specify/templates/tasks-template.md
       — Phase/test structure is language-agnostic. "cargo test" already appears
         in examples. No changes required.

Follow-up TODOs:
  - TODO(MSRV): Pin the concrete MSRV value in Cargo.toml `rust-version` field
    and README "Minimum Supported Rust Version" section once the initial
    implementation stabilises and dependency MSRV requirements are known.
================================================================================
-->

# logical-path Constitution

## Core Principles

### I. Library-First Design

The `logical-path` crate MUST remain a pure library crate (`lib.rs` only;
no `main.rs`, no binary targets). The public API surface MUST be minimal and
deliberately designed: every public type, function, and trait MUST earn its
place. API signatures MUST be ergonomic for callers (accepting `&Path`-like
inputs, returning `PathBuf` or `Option`/`Result` as appropriate) and MUST NOT
leak internal implementation details. Stability is a first-class concern:
once a symbol is published on crates.io at a non-zero major version, removing
or incompatibly changing it requires a SemVer MAJOR bump (see Principle V).

**Rationale**: Downstream CLI tools depend on a stable, composable library.
Keeping the crate purely a library prevents scope creep, keeps compile times
low, and lets consumers integrate without unwanted binary artefacts.

### II. Correctness & Safety

Path translation MUST be semantically correct. Specifically:

- The five-step algorithm (Detect → Map → Translate → Validate → Fall back)
  is non-negotiable; no step MAY be skipped or short-circuited.
- The Validate step (round-trip `canonicalize(translated) == canonicalize(original)`)
  MUST be executed for every translation to prevent broad-prefix mis-mappings.
- The Fall-back step MUST always return a usable path (original canonical path)
  rather than an error, preserving caller correctness under edge cases.
- `unsafe` code is PROHIBITED unless a specific, documented justification is
  provided in the PR and a safe alternative has been explicitly ruled out.
- All public functions dealing with path state MUST handle `OsStr`/`Path`
  inputs that are not valid UTF-8 without panicking.

**Rationale**: Incorrect path translation silently corrupts user workflows
(e.g., mis-targeted `cd` directives). Correctness and graceful fall-back are
the core value proposition of this library; they MUST never be traded for
convenience or performance.

### III. Test-First (TDD)

Tests MUST be written before implementation code is merged. The Red-Green-
Refactor cycle is mandatory:

1. Write a failing test that captures the intended behaviour.
2. Obtain reviewer acknowledgement that the test correctly specifies the
   requirement.
3. Implement until the test passes.
4. Refactor without breaking the green suite.

`cargo test` MUST pass on all supported platforms before any PR is merged.
Platform-specific tests MUST be gated with `#[cfg(target_os = "...")]` or
`#[cfg(unix)]` / `#[cfg(windows)]` as appropriate. Tests that depend on
filesystem state MUST create and clean up their own temporary directories
(e.g., via `tempfile` or manual setup/teardown) and MUST NOT rely on the
developer's local environment.

**Rationale**: Path-resolution edge cases are numerous and platform-dependent.
Writing tests first ensures the specification is unambiguous, and platform
guards prevent CI failures on unrelated systems.

### IV. Cross-Platform Discipline

Every code change MUST be verified on Linux, macOS, and Windows via CI before
merge. No platform-specific code path MAY be introduced without conditional
compilation (`#[cfg(...)]`). Platform limitations (e.g., Windows `$PWD`
absence, macOS case-insensitivity, `\\?\` UNC prefix quirks) MUST be
documented in the public API docs and in `README.md` under "Platform Notes".
`cargo test --target <platform>` green status is a hard merge gate for all
three platforms.

**Rationale**: The library's core use cases arise precisely from platform-
specific symlink behaviour (macOS `/var`→`/private/var`, WSL VHD mounts,
NFS). Untested platform code will silently regress on the platforms users
need most.

### V. Semantic Versioning & MSRV Policy

The crate MUST follow [SemVer](https://semver.org):

- **PATCH** bump: backward-compatible bug fixes, doc improvements, internal
  refactors with no API change.
- **MINOR** bump: backward-compatible new public API, new platform support,
  or MSRV increase.
- **MAJOR** bump: any removal or incompatible change to a stable public API.

The Minimum Supported Rust Version (MSRV) MUST be stated in both `Cargo.toml`
(`rust-version` field) and `README.md`. Increasing the MSRV requires at
minimum a MINOR version bump and MUST be called out explicitly in the
`CHANGELOG` entry. The MSRV MUST NOT be raised solely for convenience;
a concrete dependency requirement or language-feature need MUST be cited.

**Rationale**: Library consumers pin dependency versions in their projects.
Predictable versioning and a stable MSRV let downstream tools upgrade safely
and plan their own MSRV policies accordingly.

## Quality Gates

All pull requests MUST pass the following automated checks before merge. CI
failure on any gate is a hard block; no exceptions.

- **`cargo test`** — full test suite green on Linux, macOS, and Windows CI
  runners.
- **`cargo clippy -- --deny warnings`** — zero clippy warnings; all lints
  treated as errors.
- **`cargo fmt --check`** — code MUST be formatted with `rustfmt` using the
  repository's `rustfmt.toml` (or default settings if absent).
- **`cargo doc --no-deps`** — documentation MUST compile without warnings;
  every public item MUST have a doc comment.
- **Examples** — all code examples in `README.md` and in doc comments MUST
  compile (enforced via `cargo test --doc` and `cargo test --examples`).
- **MSRV check** — CI MUST include a job that builds and tests against the
  stated MSRV toolchain to detect unintentional MSRV regressions.

## Contribution Workflow

1. **Open an issue first** for any significant change: new API symbols,
   behaviour changes, platform support additions, or MSRV bumps. Discuss
   design and approach before investing implementation effort.
2. **Fork and branch** from `main`. Branch names SHOULD follow the pattern
   `<type>/<short-description>` (e.g., `feat/logical-path-context`,
   `fix/windows-pwd-fallback`).
3. **Write tests first** (Principle III). The PR description MUST reference
   the failing test commit.
4. **Implement** until all tests pass and all Quality Gates are green.
5. **Update documentation**: public API docs, `README.md` Platform Notes,
   and `CHANGELOG` entry (following [Keep a Changelog](https://keepachangelog.com)
   format).
6. **Submit PR** — link the originating issue. CI MUST be green before
   requesting review.
7. **Merge** only after at least one maintainer approval and all CI checks
   pass.

Bug reports, platform-specific test cases, and documentation improvements
are welcome without a prior issue; open the PR directly with a clear
description.

## Governance

This constitution supersedes all other project practices, style guides, and
informal conventions. In cases of conflict, the constitution governs.

**Amendment procedure**: Amendments require a pull request that (a) updates
this file with a version bump following the semantic versioning rules in
Principle V applied to governance changes, (b) states the rationale for the
change in the PR description, and (c) updates the `LAST_AMENDED_DATE` and
`CONSTITUTION_VERSION` fields below. The PR MUST be reviewed and approved by
at least one maintainer before merge.

**Versioning policy for the constitution itself**:

- MAJOR: removal of a principle or backward-incompatible governance change.
- MINOR: addition of a new principle or materially new governance section.
- PATCH: clarifications, wording improvements, typo fixes.

**Compliance review**: Adherence to this constitution MUST be verified on
every pull request. Reviewers are responsible for checking that the PR does
not violate any principle or bypass any quality gate. Non-compliant PRs MUST
NOT be merged regardless of other merit.

**Version**: 1.0.0 | **Ratified**: 2025-07-18 | **Last Amended**: 2025-07-18
