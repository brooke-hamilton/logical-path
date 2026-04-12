# Windows Example: Junction-Aware cd Directives

This standalone Rust project demonstrates why CLI tools need junction/subst-aware path handling on Windows, and how the `logical-path` crate fixes the problem.

## The Problem

On Windows, several features create path indirections that `std::fs::canonicalize()` resolves away:

- **NTFS junctions** (`mklink /J`) — directory-level hard links
- **Subst drives** (`subst`) — drive letters mapped to directories
- **Directory symlinks** (`mklink /D`) — symbolic links to directories

When a CLI tool calls `std::fs::canonicalize()` on the current directory, it gets the *physical* path with all indirections resolved. If the tool emits a `cd` directive using this path, the user ends up in a different location than they expect.

**Example scenario:**

```text
# Create a junction
mklink /J C:\workspace D:\projects\workspace

# Navigate via the junction
cd C:\workspace\my-project\src

# current_dir() preserves the junction:
#   C:\workspace\my-project\src

# But canonicalize() resolves it to the physical target:
#   D:\projects\workspace\my-project\src
```

A naive CLI tool would emit `cd D:\projects\workspace\my-project\src` — sending the user to the physical path instead of the junction-based path they expect.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024, MSRV 1.85.0)
- A Windows system
- An NTFS junction, subst drive, or directory symlink in your working directory path (see Setup below)

## Setup: Using an NTFS Junction

Create a junction to reproduce the problem:

```cmd
:: Create a target directory
mkdir D:\projects\workspace

:: Create a junction pointing to it
mklink /J C:\workspace D:\projects\workspace

:: Navigate into the project via the junction
cd C:\workspace
```

### Alternative: Using a Subst Drive

```cmd
:: Map drive Z: to an existing directory
subst Z: D:\projects\workspace

:: Navigate via the subst drive
cd Z:\my-project\src
```

Then clone and build the example:

```cmd
git clone https://github.com/brooke-hamilton/logical-path.git
cd logical-path\docs\example-windows
cargo run
```

## The Broken Behavior

The `broken_cd_demo()` function shows what happens without `logical-path`:

```rust
fn broken_cd_demo() {
    let cwd = std::env::current_dir().unwrap();
    let canonical_cwd = std::fs::canonicalize(&cwd).unwrap();
    // Strip the \\?\ prefix for readability

    // canonicalize() resolves junctions, subst drives, and symlinks
    // If current_dir() differs from canonicalize(), an indirection is in effect
    println!("The tool emits: cd {}", canonical_cwd.display());
    // WRONG — this is the canonical path, not where the user thinks they are
}
```

**Expected output (with junction active):**

```text
=== The Problem: Broken cd directive (without logical-path) ===

A naive CLI tool uses std::fs::canonicalize(current_dir()) to find where you are.
On Windows, canonicalize() resolves NTFS junctions, subst drives, and symlinks.

current_dir() returns:   C:\workspace\my-project\src
canonicalize() returns:  D:\projects\workspace\my-project\src

The tool emits: cd D:\projects\workspace\my-project\src
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                   WRONG! This is the canonical path, not where you think you are.
```

## The Fix

The `fixed_cd_demo()` function shows the corrected behavior using `logical-path`:

```rust
use logical_path::LogicalPathContext;

fn fixed_cd_demo() {
    let ctx = LogicalPathContext::detect();

    if ctx.has_mapping() {
        let canonical_cwd = std::fs::canonicalize(
            std::env::current_dir().unwrap()
        ).unwrap();
        let logical_cwd = ctx.to_logical(&canonical_cwd);
        println!("The tool emits: cd {}", logical_cwd.display());
        // CORRECT — preserves the junction/subst-based path
    }
}
```

`LogicalPathContext::detect()` compares `current_dir()` (which preserves junctions and subst drives) against `canonicalize()` (which resolves them) to discover the prefix mapping. Then `to_logical()` translates any canonical path back to its logical equivalent.

**Expected output (with junction active):**

```text
=== The Fix: Corrected cd directive (with logical-path) ===

Using LogicalPathContext::detect() + to_logical():

The tool emits: cd C:\workspace\my-project\src
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                   CORRECT! This preserves your junction/subst-based directory structure.
```

## What Happens Without a Junction

If you run the example from a directory that does not traverse any junctions, subst drives, or symlinks, `current_dir()` and `canonicalize()` return the same path. The example detects this and prints an explanatory message:

```text
=== The Problem: Broken cd directive (without logical-path) ===

No junction or subst mapping detected. current_dir() and canonicalize() agree:
  Current directory: C:\Users\dev\project\src

Both paths are identical — no junction or subst drive is in effect.
In a real scenario with a junction or subst drive, these would diverge.

=== The Fix: Corrected cd directive (with logical-path) ===

No mapping active — to_logical() returns the input path unchanged.
The tool emits: cd C:\Users\dev\project\src
```

## How It Works

1. **Detection**: `LogicalPathContext::detect()` calls `current_dir()` (which preserves junctions and subst drives on Windows) and `canonicalize()` (which resolves them to physical paths). If they differ, it finds the divergence point — the junction/subst prefix.
2. **Translation**: `to_logical()` takes a canonical path, checks if it starts with the canonical prefix, and replaces that prefix with the logical prefix. It validates the result via round-trip canonicalization.
3. **No-op safety**: If no mapping is active, `to_logical()` returns the input path unchanged — no errors, no surprises.
