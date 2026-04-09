# How It Works

This document explains the five-step algorithm that `logical-path` uses to detect symlink prefix mappings and translate paths between their canonical and logical forms.

## Background

When you `cd` into a directory through a symlink, your shell tracks two different paths to the same location:

- **`$PWD`** (logical path) — The path you typed, preserving symlinks. Maintained by the shell itself.
- **`getcwd()`** (canonical path) — The physical path with all symlinks resolved. Returned by the OS.

For example, if `/workspace` is a symlink to `/mnt/wsl/workspace`:

```text
$PWD         = /workspace/project/src
getcwd()     = /mnt/wsl/workspace/project/src
```

Rust's standard library only provides access to the canonical path. This crate bridges the gap.

## The Five-Step Algorithm

### Step 1: Detect

`LogicalPathContext::detect()` reads two values:

1. **`$PWD`** via `std::env::var_os("PWD")` — Returns `Option<OsString>`, handling both unset and non-UTF-8 cases without panicking.
2. **Canonical CWD** via `std::env::current_dir()` — Returns `Result<PathBuf>`, mapping to `getcwd(2)` on Unix.

If either value is unavailable, detection returns no mapping immediately.

### Step 2: Map (Suffix-Matching)

If `$PWD` and the canonical CWD differ, the algorithm finds where they diverge by comparing path components from the end (longest common suffix):

```text
Canonical: /mnt/wsl/workspace/project/src
               ↑         ↑        ↑      ↑     ↑
Logical:              /workspace/project/src
                          ↑        ↑      ↑

Common suffix: workspace / project / src
Canonical prefix: /mnt/wsl
Logical prefix: /
```

The algorithm uses `Path::components()` to decompose each path, collects them into vectors, and iterates from the end to find the longest common suffix. Everything before the common suffix is the prefix pair.

**Key properties**:

- Component-level matching, not byte-level string matching. This handles trailing slashes, `.` components, and redundant separators correctly.
- Works generically across all Unix platforms — no special-casing for macOS `/private` prefixes.

### Step 3: Validate Detection

Before accepting a mapping, the algorithm validates that `$PWD` is not stale:

```rust
canonicalize($PWD) == canonical_cwd
```

If `$PWD` points to a deleted directory, has been reassigned, or doesn't resolve to the same canonical CWD, the mapping is rejected. This prevents incorrect translations from an out-of-date environment.

### Step 4: Translate

When `to_logical()` or `to_canonical()` is called with a path:

1. **Check mapping** — If no mapping exists, return input unchanged.
2. **Check absolute** — If the path is relative, return input unchanged.
3. **Strip prefix** — If the path starts with the source prefix, strip it.
4. **Prepend prefix** — Prepend the destination prefix to the remaining suffix.

```text
to_logical("/mnt/wsl/workspace/project/src/main.rs"):
  Strip canonical prefix:  /mnt/wsl  →  workspace/project/src/main.rs
  Prepend logical prefix:  /         →  /workspace/project/src/main.rs
```

### Step 5: Validate Translation (Round-Trip)

Every translated path is validated before being returned:

```rust
canonicalize(translated_path) == canonicalize(original_path)
```

This catches cases where the prefix mapping is too broad and would mistranslate unrelated paths. If validation fails — or if either path doesn't exist on disk — the original input path is returned unchanged.

## Visual Walkthrough

Here is a complete example of the algorithm applied to a WSL environment:

```text
Environment:
  $PWD     = /workspace/project
  getcwd() = /mnt/wsl/workspace/project

Step 1 (Detect):
  pwd = Some("/workspace/project")
  canonical_cwd = "/mnt/wsl/workspace/project"
  → They differ, proceed to mapping.

Step 2 (Map):
  Components (canonical): [/, mnt, wsl, workspace, project]
  Components (logical):   [/, workspace, project]
  Reverse scan:
    project == project ✓ (suffix_len = 1)
    workspace == workspace ✓ (suffix_len = 2)
    wsl != / → stop
  canonical_prefix = /mnt/wsl  (5 - 2 = 3 components → /, mnt, wsl)
  logical_prefix = /            (3 - 2 = 1 component  → /)

Step 3 (Validate Detection):
  canonicalize("/workspace/project") → "/mnt/wsl/workspace/project" ✓

Step 4 (Translate — to_logical):
  Input: /mnt/wsl/workspace/project/src/main.rs
  strip_prefix("/mnt/wsl") → "workspace/project/src/main.rs"
  join with "/" → "/workspace/project/src/main.rs"

Step 5 (Validate Translation):
  canonicalize("/workspace/project/src/main.rs")
    → "/mnt/wsl/workspace/project/src/main.rs" ✓ (matches original canonical)
  → Return: /workspace/project/src/main.rs
```

## macOS System Symlinks

macOS has built-in symlinks like `/var` → `/private/var` and `/tmp` → `/private/tmp`. The generic suffix-matching algorithm handles these without any special-casing:

```text
$PWD     = /var/folders/xyz/T/test
getcwd() = /private/var/folders/xyz/T/test

Common suffix: var / folders / xyz / T / test
Canonical prefix: /private
Logical prefix: /
```

This means the crate works out of the box on macOS for any tool that displays paths under `/var` or `/tmp`.

## Fallback Conditions

The algorithm falls back to returning the input unchanged in all of these cases:

| Condition | Stage | Reason |
| --------- | ----- | ------ |
| `$PWD` is unset | Detection | No logical path source |
| `$PWD` equals canonical CWD | Detection | No symlink in effect |
| `$PWD` is stale (deleted directory) | Detection validation | `canonicalize($PWD)` fails |
| `$PWD` doesn't resolve to CWD | Detection validation | Divergent `$PWD` assignment |
| No common suffix between paths | Mapping | Cannot determine divergence point |
| Path is relative | Translation | Only absolute paths are translated |
| Path doesn't start with source prefix | Translation | Path is outside the mapped tree |
| Path doesn't exist on disk | Round-trip validation | `canonicalize()` fails |
| Round-trip check fails | Round-trip validation | Mapping would produce incorrect result |
| Running on Windows | Detection | `$PWD` has no OS-level equivalent |

The fallback guarantee ensures the crate is always safe to use — you never get an error, and you always get a usable path.
