# Examples

Real-world usage patterns for the `logical-path` crate.

## Shell Integration (cd Directives)

The most common use case: a Rust CLI tool that emits `cd` directives for shell integration (e.g., a directory jumper, a project switcher, or a git worktree tool).

Without `logical-path`, the emitted path resolves all symlinks, moving the user out of their logical directory tree:

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

fn emit_cd_directive(target: &Path) {
    let ctx = LogicalPathContext::detect();

    // Translate the canonical path to preserve the user's symlink structure.
    let display_path = ctx.to_logical(target);

    // Emit a cd directive that keeps the user in their logical directory.
    println!("cd {}", display_path.display());
}
```

## Displaying Paths to Users

Any tool that shows filesystem paths in its output (status bars, error messages, file listings) should translate canonical paths to logical paths for a consistent user experience:

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

struct App {
    ctx: LogicalPathContext,
}

impl App {
    fn new() -> Self {
        App {
            ctx: LogicalPathContext::detect(),
        }
    }

    fn display_file(&self, canonical_path: &Path) {
        let display = self.ctx.to_logical(canonical_path);
        println!("  {}", display.display());
    }
}
```

## Git Worktree Path Comparison

`git worktree list` returns canonical paths. If your tool compares worktree paths against the user's `$PWD`, the paths won't match without translation:

```rust
use logical_path::LogicalPathContext;
use std::path::{Path, PathBuf};

fn find_current_worktree(worktrees: &[PathBuf], current_dir: &Path) -> Option<PathBuf> {
    let ctx = LogicalPathContext::detect();

    // Translate each worktree path to logical form for comparison
    // against the user's perceived current directory.
    for worktree in worktrees {
        let logical = ctx.to_logical(worktree);
        if current_dir.starts_with(&logical) {
            return Some(logical);
        }
    }
    None
}
```

## Global Context with `OnceLock`

For applications that need the context in multiple places, initialize it once and share it globally:

```rust
use logical_path::LogicalPathContext;
use std::sync::OnceLock;

static PATH_CTX: OnceLock<LogicalPathContext> = OnceLock::new();

fn path_ctx() -> &'static LogicalPathContext {
    PATH_CTX.get_or_init(LogicalPathContext::detect)
}
```

## Translating Paths from External Tools

When receiving paths from external tools (e.g., `git`, `cargo`, LSP servers), those paths are typically canonical. Translate them before displaying to the user:

```rust
use logical_path::LogicalPathContext;
use std::path::PathBuf;

fn process_tool_output(canonical_paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let ctx = LogicalPathContext::detect();
    canonical_paths
        .iter()
        .map(|p| ctx.to_logical(p))
        .collect()
}
```

## Normalizing User Input

When the user provides a logical path that needs to be passed to a filesystem API, translate it to canonical form:

```rust
use logical_path::LogicalPathContext;
use std::path::Path;

fn open_file(user_path: &Path) -> std::io::Result<String> {
    let ctx = LogicalPathContext::detect();

    // Translate to canonical for filesystem operations.
    let fs_path = ctx.to_canonical(user_path);

    std::fs::read_to_string(&fs_path)
}
```

## Diagnostic Output

Use `has_mapping()` and `Debug` formatting for diagnostics:

```rust
use logical_path::LogicalPathContext;

fn print_diagnostics() {
    let ctx = LogicalPathContext::detect();

    if ctx.has_mapping() {
        println!("Symlink prefix mapping detected:");
        println!("  Context: {:?}", ctx);
    } else {
        println!("No symlink prefix mapping detected.");
        println!("  $PWD and getcwd() are consistent (or $PWD is unset).");
    }
}
```

## Integration with `clap` CLI Applications

A typical pattern for a `clap`-based CLI tool:

```rust
use logical_path::LogicalPathContext;
use std::path::PathBuf;

struct Cli {
    target: PathBuf,
}

fn run(cli: Cli) {
    let ctx = LogicalPathContext::detect();

    // Translate the target path for display.
    let display_path = ctx.to_logical(&cli.target);
    println!("Working in: {}", display_path.display());

    // Use the original (canonical) path for filesystem operations.
    // Or use to_canonical() if the input was a logical path from the user.
}
```
