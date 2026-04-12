# Data Model: Runnable Example Projects

**Feature**: 003-docs-examples
**Date**: 2026-04-11

## Overview

This feature produces documentation artifacts (example projects), not library data types. No new types, structs, or enums are added to the `logical-path` crate. The "data model" here describes the structure and content of each deliverable.

## Deliverables

### Entity: Example Project (Unix)

**Location**: `docs/example-unix/`

| Artifact | Purpose | Content |
| -------- | ------- | ------- |
| `Cargo.toml` | Package manifest | Binary crate, edition 2024, MSRV 1.85.0, depends on `logical-path = { path = "../../" }` |
| `src/main.rs` | Executable source | Module-level `compile_error!` for non-unix, `main()` with broken demo then fixed demo |
| `README.md` | Documentation | Problem description, code snippets, expected output, setup instructions |

**main.rs structure**:

1. Module-level `#[cfg(not(unix))] compile_error!(...)` guard
2. `fn broken_cd_demo()` — uses `std::env::current_dir()` to get canonical path, emits wrong `cd` directive
3. `fn fixed_cd_demo()` — uses `LogicalPathContext::detect()` + `to_logical()`, emits correct `cd` directive
4. `fn main()` — calls both demos with labeled output; handles no-mapping case

**README.md structure**:

1. Title and one-line description
2. The Problem — explains `$PWD` vs `getcwd()` divergence
3. Prerequisites — Rust/Cargo, a symlink in the working directory path
4. Setup — step-by-step instructions for creating a symlink
5. The Broken Behavior — code snippet from `broken_cd_demo()`, expected output
6. The Fix — code snippet from `fixed_cd_demo()`, expected output
7. What Happens Without a Symlink — expected output when no mapping is active

### Entity: Example Project (Windows)

**Location**: `docs/example-windows/`

| Artifact | Purpose | Content |
| -------- | ------- | ------- |
| `Cargo.toml` | Package manifest | Binary crate, edition 2024, MSRV 1.85.0, depends on `logical-path = { path = "../../" }` |
| `src/main.rs` | Executable source | Module-level `compile_error!` for non-windows, `main()` with broken demo then fixed demo |
| `README.md` | Documentation | Problem description, code snippets, expected output, setup instructions |

**main.rs structure**:

1. Module-level `#[cfg(not(windows))] compile_error!(...)` guard
2. `fn broken_cd_demo()` — uses `std::fs::canonicalize(std::env::current_dir())` (with `\\?\` prefix stripped) to get canonical path, emits wrong `cd` directive
3. `fn fixed_cd_demo()` — uses `LogicalPathContext::detect()` + `to_logical()`, emits correct `cd` directive
4. `fn main()` — calls both demos with labeled output; handles no-mapping case

**README.md structure**:

1. Title and one-line description
2. The Problem — explains `current_dir()` vs `canonicalize()` divergence on Windows
3. Prerequisites — Rust/Cargo, an NTFS junction or subst drive
4. Setup — step-by-step instructions for creating a junction via `mklink /J` or `subst`
5. The Broken Behavior — code snippet from `broken_cd_demo()`, expected output
6. The Fix — code snippet from `fixed_cd_demo()`, expected output
7. What Happens Without a Junction — expected output when no mapping is active

## Relationships

```text
logical-path (library crate, unchanged)
├── docs/example-unix/     (binary crate, depends on logical-path via path)
│   └── uses: LogicalPathContext::detect(), to_logical(), has_mapping()
└── docs/example-windows/  (binary crate, depends on logical-path via path)
    └── uses: LogicalPathContext::detect(), to_logical(), has_mapping()
```

## Validation Rules

- Each `Cargo.toml` MUST specify `edition = "2024"` and `rust-version = "1.85.0"`
- Each `Cargo.toml` MUST use `logical-path = { path = "../../" }` as the dependency path
- Each `main.rs` MUST have a module-level `compile_error!` for wrong-platform builds
- Each `main.rs` MUST handle the no-mapping case (when `has_mapping()` returns `false`)
- Each README MUST contain at least one fenced code block for the broken demo and one for the fixed demo
