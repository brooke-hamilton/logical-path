# logical-path Development Guidelines

## Active Technologies

- Rust edition 2024 with MSRV 1.85.0.
- Single library crate: `logical-path`.
- Runtime dependency: `log = "0.4"` for trace/debug diagnostics.
- Dev dependency: `tempfile = "3"` for integration tests.
- Standalone example crates under `docs/example-unix/` and `docs/example-windows/` depend on the root crate via a relative path.

## Project Structure

```text
src/lib.rs                  # Entire public library implementation
tests/integration.rs        # Cross-platform integration coverage
docs/                       # User-facing documentation and examples
docs/example-unix/          # Runnable Unix example crate
docs/example-windows/       # Runnable Windows example crate
```

## Commands

- `cargo test`
- `cargo clippy -- --deny warnings`
- `cargo fmt --check`

## Public API

The public surface is intentionally small. Preserve it unless the task explicitly requires an API change.

- `LogicalPathContext::detect()`
- `LogicalPathContext::has_mapping()`
- `LogicalPathContext::to_logical()`
- `LogicalPathContext::to_canonical()`

`LogicalPathContext` stores at most one detected prefix mapping and otherwise behaves as a no-op translator.

## Implementation Notes

- `src/lib.rs` has `#![deny(missing_docs)]`; new public items must be documented.
- Unix detection compares `$PWD` against `current_dir()` and validates `$PWD` with `canonicalize()` before accepting a mapping.
- Windows detection compares `current_dir()` against `canonicalize(current_dir())` and strips the `\\?\` prefix before prefix matching.
- Translation must stay conservative: when detection fails, a path is relative, the prefix does not match, or round-trip validation fails, return the input unchanged.
- Keep path handling non-panicking and compatible with non-UTF-8 paths.

## Testing Expectations

- `tests/integration.rs` is the main behavior suite for Unix and Windows path-indirection scenarios.
- Unix tests serialize mutations to process-global state (`$PWD`, current directory) because environment mutation is `unsafe` in Rust 2024.
- Windows tests cover junction, symlink, subst-drive, and related current-directory behavior.
- When changing detection or translation semantics, update or extend integration tests before widening the implementation.

## Documentation Expectations

- Keep `README.md` aligned with the actual API and platform behavior.
- Update the focused docs in `docs/` when behavior, guarantees, or examples change.
- If you change user-visible behavior on Unix or Windows, check the runnable example crate in the corresponding `docs/example-*` directory.

## Current Behavior Highlights

- The crate translates canonical filesystem paths back to logical, symlink-preserving paths.
- Linux and macOS use shell-provided `$PWD` as the logical source of truth.
- Windows supports junctions, directory symlinks, subst drives, and mapped-drive style path divergence through `current_dir()` versus `canonicalize()` comparison.

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->
