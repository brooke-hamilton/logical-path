# FAQ

Frequently asked questions about the `logical-path` crate.

## General

### What problem does this crate solve?

Rust's standard library resolves symlinks in filesystem paths. When a user is working in a directory reached through a symlink (e.g., `/workspace/project` → `/mnt/wsl/workspace/project`), tools that call `std::fs::canonicalize()` or `std::env::current_dir()` silently switch to the physical path. This crate translates those canonical paths back to the logical paths the user expects.

### Why not just avoid calling `canonicalize()`?

Sometimes you can't. Many tools and APIs return canonical paths:

- `std::env::current_dir()` always returns the canonical path.
- `git worktree list`, `cargo metadata`, and many other tools return canonical paths.
- Some filesystem operations require canonical paths for correctness.

The unserved problem is *undoing* canonicalization after it has already happened.

### How is this different from `dunce`, `path-absolutize`, or `path-dedot`?

Those crates help you *avoid* canonicalization or clean up path syntax. This crate *reverses* canonicalization by detecting the symlink mapping from the environment and translating paths through it.

| Crate | Purpose | Symlink awareness |
| ----- | ------- | ----------------- |
| `logical-path` | Reverse canonicalization via `$PWD` detection | Yes — detects and translates through symlinks |
| `dunce` | Strip `\\?\` prefix on Windows | No symlink handling |
| `path-absolutize` | Make paths absolute without resolving symlinks | Lexical only, no `$PWD` awareness |
| `path-dedot` | Remove `.`/`..` from paths | Pure string manipulation |

## Usage

### When should I call `detect()`?

Call `detect()` once at program startup and reuse the returned `LogicalPathContext` for the lifetime of the process. The detection reads `$PWD` and `getcwd()`, which are unlikely to change during a single program execution.

If your tool changes the current directory during execution, call `detect()` again after the change.

### Is `detect()` expensive?

No. It reads one environment variable, calls `getcwd()`, performs one `canonicalize()` call for validation, and does a linear scan over path components. This is a few microseconds at most.

### Can I share the context across threads?

Yes. `LogicalPathContext` is `Send + Sync`. You can wrap it in `Arc` or store it in `OnceLock` / `lazy_static` for global access.

### What happens if `$PWD` is not set?

`detect()` returns a context with no active mapping. All translations return the input path unchanged. No error or panic occurs.

### What happens if `$PWD` is stale or wrong?

If `$PWD` points to a directory that doesn't exist or doesn't resolve to the same canonical CWD, `detect()` returns a context with no active mapping. The crate validates `$PWD` before accepting it.

## Edge Cases

### What about paths that don't exist on disk?

Translation requires both the original and translated paths to exist on disk, because the round-trip validation step calls `std::fs::canonicalize()`. If either path doesn't exist, the fallback (input unchanged) is returned.

This means you cannot translate hypothetical paths or paths to files that haven't been created yet.

### What about relative paths?

Relative paths (e.g., `src/main.rs`, `../README.md`) are always returned unchanged. Only absolute paths are eligible for prefix translation. The crate does not resolve relative paths against the current directory.

### What about non-UTF-8 paths?

The crate uses `OsStr` and `Path` throughout. No conversion to `String` is performed. Non-UTF-8 path components are handled without panicking.

### What about nested symlinks?

The crate detects one prefix mapping per `LogicalPathContext`: the divergence between `$PWD` and `getcwd()`. If there are nested symlinks (symlinks within symlinks), only the outermost divergence is captured. Symlinks within the common suffix are not separately translated.

### What about trailing slashes or redundant separators?

`Path::components()` normalizes these. `/workspace/project/` and `/workspace/project` are treated as equivalent. Redundant separators like `/workspace//project` are also normalized.

### What about `.` and `..` in paths?

`Path::components()` normalizes `.` (current directory) components. However, `..` (parent directory) is preserved as a literal component — it is not resolved against the filesystem during suffix matching. Paths with `..` may not match as expected. For best results, pass clean absolute paths.

### What about case-insensitive filesystems (macOS)?

The crate performs exact byte comparison. On case-insensitive filesystems (macOS APFS default), `$PWD` and `getcwd()` will use consistent casing, so detection works correctly. However, if you compare translated paths against paths from other sources, you may need to normalize casing yourself.

## Platform Questions

### Does it work on Windows?

The crate compiles and runs on Windows, but `detect()` always returns no mapping because Windows has no direct `$PWD` equivalent. All translations pass through unchanged. It is safe to use the crate on Windows — it simply does nothing.

### Does it work on macOS?

Yes, and macOS is one of the primary motivating platforms. macOS has system-level symlinks (`/var` → `/private/var`, `/tmp` → `/private/tmp`) that cause canonical/logical path divergence even without user-created symlinks. The crate handles these automatically.

### Does it work in containers/Docker?

Yes, as long as the container's shell sets `$PWD` correctly and there are symlinks in effect. The detection algorithm works purely from environment state — it doesn't depend on any specific OS feature beyond `$PWD` and `getcwd()`.

## Design Questions

### Why no `Result` return types?

The crate's fallback guarantee means every call produces a usable path. Returning `Result` would force callers to add error-handling boilerplate for a situation where the fallback is always correct. The design goal is to make the crate safe to adopt unconditionally with zero ceremony.

### Why does translation require the path to exist?

The round-trip validation step (`canonicalize(translated) == canonicalize(original)`) is the crate's correctness guarantee. Without it, a broad prefix mapping could silently mistranslate unrelated paths. The existence requirement is a tradeoff: correctness over convenience.

### Why only one prefix mapping?

The crate detects the divergence between `$PWD` and `getcwd()`, which yields exactly one prefix pair. Multiple independent symlink chains would require a more complex detection mechanism. The single-mapping design covers the vast majority of real-world use cases (WSL mounts, macOS system symlinks, workspace symlinks).

### Can the mapping become stale during execution?

The `LogicalPathContext` is immutable — it captures a snapshot of the environment at the time `detect()` is called. If the user changes directories or the symlink target changes, the context may become stale. Call `detect()` again to refresh.
