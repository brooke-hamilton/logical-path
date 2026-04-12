# Unix Example: Symlink-Aware cd Directives

This standalone Rust project demonstrates why CLI tools need symlink-aware path handling on Linux and macOS, and how the `logical-path` crate fixes the problem.

## The Problem

On Unix systems, shells track two versions of the current directory:

- **`$PWD`** — the *logical* path, which preserves symlinks as the user navigated them
- **`getcwd()`** — the *canonical* path, which resolves all symlinks to their physical targets

When a CLI tool calls `std::env::current_dir()` (which uses `getcwd()` internally), it gets the canonical path with all symlinks resolved. If the tool emits a `cd` directive using this path, the user ends up in a different location than they expect.

**Example scenario:**

```text
# User creates a symlink
ln -s /mnt/wsl/workspace /workspace

# User navigates via the symlink
cd /workspace/project/src

# Shell tracks both paths:
echo $PWD           # /workspace/project/src        (logical)
pwd                 # /workspace/project/src        (logical, shell built-in)

# But getcwd() resolves the symlink:
python3 -c "import os; print(os.getcwd())"
                    # /mnt/wsl/workspace/project/src (canonical)
```

A naive CLI tool would emit `cd /mnt/wsl/workspace/project/src` — sending the user to the physical path instead of the symlink-based path they expect.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024, MSRV 1.85.0)
- A Linux or macOS system
- A symlink somewhere in your working directory path (see Setup below)

## Setup

Create a symlink to reproduce the problem:

```bash
# Create a target directory
mkdir -p /tmp/example-target

# Create a symlink pointing to it
ln -s /tmp/example-target /tmp/example-link

# Navigate into the project via the symlink
cd /tmp/example-link
```

Then clone and build the example:

```bash
git clone https://github.com/brooke-hamilton/logical-path.git
cd logical-path/docs/example-unix
cargo run
```

## The Broken Behavior

The `broken_cd_demo()` function shows what happens without `logical-path`:

```rust
fn broken_cd_demo() {
    let canonical_cwd = std::env::current_dir().unwrap();
    let pwd = std::env::var("PWD").ok();

    // current_dir() calls getcwd(), which resolves symlinks
    // If $PWD differs from getcwd(), a symlink is in effect
    println!("The tool emits: cd {}", canonical_cwd.display());
    // WRONG — this is the canonical path, not where the user thinks they are
}
```

**Expected output (with symlink active):**

```text
=== The Problem: Broken cd directive (without logical-path) ===

A naive CLI tool uses std::env::current_dir() to find where you are.
On Unix, current_dir() calls getcwd(), which resolves all symlinks.

Your shell's $PWD:       /workspace/project/src
getcwd() returns:        /mnt/wsl/workspace/project/src

The tool emits: cd /mnt/wsl/workspace/project/src
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                   WRONG! This is the canonical path, not where you think you are.
```

## The Fix

The `fixed_cd_demo()` function shows the corrected behavior using `logical-path`:

```rust
use logical_path::LogicalPathContext;

fn fixed_cd_demo() {
    let ctx = LogicalPathContext::detect();

    if ctx.has_mapping() {
        let canonical_cwd = std::env::current_dir().unwrap();
        let logical_cwd = ctx.to_logical(&canonical_cwd);
        println!("The tool emits: cd {}", logical_cwd.display());
        // CORRECT — preserves the symlink-based path
    }
}
```

`LogicalPathContext::detect()` compares `$PWD` against `getcwd()` to discover the symlink prefix mapping. Then `to_logical()` translates any canonical path back to its logical equivalent.

**Expected output (with symlink active):**

```text
=== The Fix: Corrected cd directive (with logical-path) ===

Using LogicalPathContext::detect() + to_logical():

The tool emits: cd /workspace/project/src
                   ^^^^^^^^^^^^^^^^^^^^^^
                   CORRECT! This preserves your symlink-based directory structure.
```

## What Happens Without a Symlink

If you run the example from a directory that does not traverse any symlinks, `$PWD` and `getcwd()` return the same path. The example detects this and prints an explanatory message:

```text
=== The Problem: Broken cd directive (without logical-path) ===

No symlink mapping detected. $PWD and getcwd() agree:
  Current directory: /home/user/project/src

Both paths are identical — no symlink is in effect.
In a real scenario with a symlink, these would diverge.

=== The Fix: Corrected cd directive (with logical-path) ===

No mapping active — to_logical() returns the input path unchanged.
The tool emits: cd /home/user/project/src
```

## How It Works

1. **Detection**: `LogicalPathContext::detect()` reads `$PWD` (logical path from the shell) and calls `getcwd()` (canonical path from the OS). If they differ, it finds the divergence point — the symlink prefix.
2. **Translation**: `to_logical()` takes a canonical path, checks if it starts with the canonical prefix, and replaces that prefix with the logical prefix. It validates the result via round-trip canonicalization.
3. **No-op safety**: If no symlink mapping is active, `to_logical()` returns the input path unchanged — no errors, no surprises.
