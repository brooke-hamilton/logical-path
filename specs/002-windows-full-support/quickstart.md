# Quickstart: Windows Full Support

## What Changes

After this feature, `logical-path` detects path indirections on Windows (NTFS junctions, directory symlinks, subst drives, mapped network drives) in addition to the existing Unix symlink detection.

## Usage (unchanged API)

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

let ctx = LogicalPathContext::detect();

if ctx.has_mapping() {
    // On Windows with junction C:\workspace → D:\projects\workspace:
    let canonical = Path::new(r"D:\projects\workspace\src\main.rs");
    let logical = ctx.to_logical(canonical);
    // logical == r"C:\workspace\src\main.rs"
    println!("Logical: {}", logical.display());

    // Reverse direction:
    let back = ctx.to_canonical(&logical);
    // back == r"D:\projects\workspace\src\main.rs"
    println!("Canonical: {}", back.display());
}
```

## Platform Behavior Summary

| Platform | Detection Source | Logical Path Source | Canonical Path Source |
| -------- | --------------- | ------------------- | -------------------- |
| Linux | `$PWD` vs `getcwd()` | `$PWD` | `getcwd()` |
| macOS | `$PWD` vs `getcwd()` | `$PWD` | `getcwd()` |
| Windows | `current_dir()` vs `canonicalize()` | `GetCurrentDirectoryW` | `GetFinalPathNameByHandleW` (with `\\?\` stripped) |

## Adding Diagnostics (optional)

```rust
// In your application's main() or setup code:
// Install any `log`-compatible logger to see trace diagnostics
env_logger::init(); // or any other log implementation

// Then call detect() as normal — diagnostics are emitted at trace level
let ctx = LogicalPathContext::detect();
```

## Building and Testing

```bash
# Build
cargo build

# Run tests (current platform)
cargo test

# Run tests with trace diagnostics visible
RUST_LOG=trace cargo test -- --nocapture

# Clippy
cargo clippy -- --deny warnings

# Format check
cargo fmt --check
```

## Windows-Specific Test Setup

Windows integration tests create and clean up their own junctions and subst drives. No manual setup is required. Tests that need junction/subst support skip gracefully if the commands are unavailable.
