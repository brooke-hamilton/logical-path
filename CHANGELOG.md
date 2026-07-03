# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-07-03

### Changed

- Updated the `log` dependency to 0.4.32.

## [0.1.0] - 2026-05-03

### Added

- Initial release of `logical-path`.
- `LogicalPathContext::detect()` infers the active logical-vs-canonical path mapping for the current process.
- `LogicalPathContext::has_mapping()`, `to_logical()`, and `to_canonical()` translate between symlink-preserving and symlink-resolved paths.
- Linux and macOS support via `$PWD` comparison against `current_dir()`.
- Windows support for junctions, directory symlinks, and subst drives via `current_dir()` versus `canonicalize()` comparison, including handling of the `\\?\` prefix.
- Conservative no-op behavior when detection fails, the input is relative, the prefix does not match, or round-trip validation fails.
- Runnable example crates under `docs/example-unix/` and `docs/example-windows/`.
