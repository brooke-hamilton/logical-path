# logical-path Development Guidelines

Auto-generated from all feature plans. Last updated: 2026-04-09

## Active Technologies

- Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml) + None at runtime (current). New: `log` crate for trace diagnostics. `windows-sys` may be needed behind `#[cfg(windows)]` if OS APIs beyond std are required. (002-windows-full-support)

## Project Structure

```text
src/
tests/
```

## Commands

cargo test [ONLY COMMANDS FOR ACTIVE TECHNOLOGIES][ONLY COMMANDS FOR ACTIVE TECHNOLOGIES] cargo clippy

## Code Style

Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml): Follow standard conventions

## Recent Changes

- 002-windows-full-support: Added Rust 1.85.0 (edition 2024, MSRV pinned in Cargo.toml) + None at runtime (current). New: `log` crate for trace diagnostics. `windows-sys` may be needed behind `#[cfg(windows)]` if OS APIs beyond std are required.

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
