# Quickstart: Runnable Example Projects

**Feature**: 003-docs-examples
**Date**: 2026-04-11

## Build and Run

### Unix Example (Linux/macOS)

```bash
cd docs/example-unix
cargo run
```

### Windows Example

```cmd
cd docs\example-windows
cargo run
```

### Expected Output (Unix, with symlink active)

```text
=== The Problem: Broken cd directive (without logical-path) ===

A naive CLI tool uses std::env::current_dir() to find where you are.
On Unix, current_dir() calls getcwd(), which resolves all symlinks.

Your shell's $PWD:       /workspace/project/src
getcwd() returns:        /mnt/wsl/workspace/project/src

The tool emits: cd /mnt/wsl/workspace/project/src
                   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                   WRONG! This is the canonical path, not where you think you are.

=== The Fix: Corrected cd directive (with logical-path) ===

Using LogicalPathContext::detect() + to_logical():

The tool emits: cd /workspace/project/src
                   ^^^^^^^^^^^^^^^^^^^^^^
                   CORRECT! This preserves your symlink-based directory structure.
```

### Expected Output (no symlink active)

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

## Verification Checklist

1. Example compiles: `cargo build` succeeds on target platform
2. Wrong-platform guard: `cargo build` on wrong platform produces `compile_error!` message
3. With mapping: Output shows different broken vs. fixed paths
4. Without mapping: Output shows explanatory message, no errors
