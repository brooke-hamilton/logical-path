# Research: Runnable Example Projects

**Feature**: 003-docs-examples
**Date**: 2026-04-11

## R-001: Platform-Gating Binary Crates with `compile_error!`

**Decision**: Use module-level `compile_error!` with `#[cfg(not(unix))]` / `#[cfg(not(windows))]` to produce clear compile-time errors on wrong platforms.

**Rationale**: This is the idiomatic Rust pattern for platform-gated binaries. Placing `compile_error!` at module level (outside `main()`) halts compilation immediately with a custom message before any function bodies are analyzed. This avoids cryptic "main function not found" errors that would result from `#[cfg]`-gating `main()` itself.

**Pattern for Unix-only binary**:

```rust
#[cfg(not(unix))]
compile_error!("This example requires Linux or macOS (unix target).");

fn main() {
    // unix-only logic
}
```

**Pattern for Windows-only binary**:

```rust
#[cfg(not(windows))]
compile_error!("This example requires Windows.");

fn main() {
    // windows-only logic
}
```

**Key facts**:

- `cfg(unix)` matches both Linux and macOS (and other POSIX-like targets)
- `compile_error!` is a macro invoked at module scope, not inside a function
- WASM and other non-unix/non-windows targets also hit the error (desired behavior)

**Alternatives considered**:

- `#[cfg]`-gating `main()` away: Produces cryptic "main function not found" error
- Runtime `panic!()`: Compiles everywhere, fails only at runtime
- Cargo feature flags: User must remember to set flags; doesn't prevent cross-compilation

## R-002: Standalone Example Crates with Relative Path Dependencies

**Decision**: Each example project uses `logical-path = { path = "../../" }` in its `Cargo.toml` to reference the parent crate.

**Rationale**: Relative path dependencies are the standard Cargo mechanism for depending on a local crate without publishing. Since the example directories are at `docs/example-unix/` and `docs/example-windows/`, the relative path `../../` correctly resolves to the repository root where the `logical-path` crate's `Cargo.toml` lives.

**Key facts**:

- The root `Cargo.toml` defines a `[package]` (not a `[workspace]` with `members`), so these example crates are NOT auto-discovered as workspace members
- `cargo build` at the repo root only builds the `logical-path` library; example projects require explicit `cd docs/example-unix && cargo build`
- Path dependencies are resolved relative to the directory containing the `Cargo.toml` that declares them
- Each example crate needs its own `edition` and `rust-version` fields matching the parent crate (edition 2024, MSRV 1.85.0)

**Alternatives considered**:

- Git dependencies (`logical-path = { git = "..." }`): Requires published repo, doesn't work for local development
- crates.io version (`logical-path = "0.1"`): Requires the crate to be published first
- Workspace members: Would cause `cargo build` at root to build examples, violating the independence requirement

## R-003: Demonstrating the "Broken" Behavior (canonicalize vs PWD)

**Decision**: The "broken" demo uses `std::fs::canonicalize(std::env::current_dir())` to get the canonical path and emits a `cd` directive with it, showing that symlinks/junctions are resolved away.

**Rationale**: This directly demonstrates the real-world problem. When a CLI tool calls `canonicalize()` on the current directory (or any path within it), the returned path has all symlinks resolved. If the tool then emits this path as a `cd` directive, the user's shell would navigate to the physical location instead of the logical location they expect.

**Unix scenario**:

- User is in `/workspace/project/src` (where `/workspace` is a symlink to `/mnt/wsl/workspace`)
- `std::env::current_dir()` returns `/mnt/wsl/workspace/project/src` (canonical, from `getcwd()`)
- A naive `cd` directive would use this canonical path, moving the user out of `/workspace/`
- The `logical-path` fix: `LogicalPathContext::detect()` detects the mapping, and `to_logical()` translates back to `/workspace/project/src`

**Windows scenario**:

- User is in `C:\workspace\project\src` (where `C:\workspace` is an NTFS junction to `D:\projects\workspace`)
- `std::fs::canonicalize(current_dir())` returns `D:\projects\workspace\project\src` (with `\\?\` prefix stripped)
- A naive `cd` directive would use this resolved path, moving the user to `D:\`
- The `logical-path` fix: same API, detects junction mapping, translates back to `C:\workspace\project\src`

**No-mapping case**: When no symlink/junction is active, `current_dir()` and `canonicalize()` return the same path. The example detects this via `ctx.has_mapping()` and prints an explanatory message.
