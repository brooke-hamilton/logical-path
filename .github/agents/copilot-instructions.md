# logical-path Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-11

## Active Technologies
- Rust edition 2024 (MSRV 1.85.0, matching parent crate) + `logical-path` (via relative path `../../`), `log` 0.4 (transitive) (003-docs-examples)
- N/A (filesystem read-only; no persistence) (003-docs-examples)

- Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml) + None at runtime (current). New: `log` crate for trace diagnostics. `windows-sys` may be needed behind `#[cfg(windows)]` if OS APIs beyond std are required. (002-windows-full-support)

## Project Structure

```text
src/
tests/
```

## Commands

- `cargo test`
- `cargo clippy -- --deny warnings`
- `cargo fmt --check`

## Code Style

Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml): Follow standard conventions

## Recent Changes
- 003-docs-examples: Added Rust edition 2024 (MSRV 1.85.0, matching parent crate) + `logical-path` (via relative path `../../`), `log` 0.4 (transitive)

- 002-windows-full-support: Added Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml) + None at runtime (current). New: `log` crate for trace diagnostics. `windows-sys` may be needed behind `#[cfg(windows)]` if OS APIs beyond std are required.

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
