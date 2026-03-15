# Quickstart: logical-path

## Add the dependency

```toml
[dependencies]
logical-path = "0.1"
```

## Basic usage

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

fn main() {
    // Detect the active symlink prefix mapping from $PWD vs getcwd().
    let ctx = LogicalPathContext::detect();

    // Check if a mapping was found.
    if ctx.has_mapping() {
        println!("Symlink mapping detected!");
    }

    // Translate a canonical path to its logical equivalent.
    let canonical = Path::new("/mnt/wsl/workspace/project/src/main.rs");
    let logical = ctx.to_logical(canonical);
    println!("Logical path: {}", logical.display());

    // Translate a logical path to its canonical equivalent.
    let logical_input = Path::new("/workspace/project/README.md");
    let canonical_out = ctx.to_canonical(logical_input);
    println!("Canonical path: {}", canonical_out.display());
}
```

## Shell integration example

```rust
use logical_path::LogicalPathContext;
use std::path::PathBuf;

fn emit_cd_directive(target: &PathBuf) {
    let ctx = LogicalPathContext::detect();
    let logical = ctx.to_logical(target);
    println!("cd {}", logical.display());
}
```

## Build and test

```bash
# Build the library
cargo build

# Run tests
cargo test

# Check for lint warnings
cargo clippy -- --deny warnings

# Verify formatting
cargo fmt --check

# Build documentation
cargo doc --no-deps --open
```

## Platform notes

- **Linux/macOS**: Full symlink detection and translation via `$PWD`.
- **macOS**: System symlinks like `/var` → `/private/var` are handled automatically.
- **Windows**: `detect()` returns no active mapping; all translations return input unchanged.
