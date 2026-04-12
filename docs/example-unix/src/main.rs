#[cfg(not(unix))]
compile_error!("This example requires Linux or macOS (unix target).");

use logical_path::LogicalPathContext;
use std::path::Path;

/// Demonstrate the broken behavior: a naive CLI tool that uses
/// `std::env::current_dir()` (which calls `getcwd()` on Unix) to find the
/// current directory. Because `getcwd()` resolves all symlinks, any `cd`
/// directive it produces will point to the *canonical* path — not the path
/// the user expects.
fn broken_cd_demo() {
    println!("=== The Problem: Broken cd directive (without logical-path) ===");
    println!();
    println!("A naive CLI tool uses std::env::current_dir() to find where you are.");
    println!("On Unix, current_dir() calls getcwd(), which resolves all symlinks.");
    println!();

    let canonical_cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("Error: could not determine current directory: {e}");
            return;
        }
    };

    let pwd = std::env::var("PWD").ok();

    match &pwd {
        Some(pwd_val) if Path::new(pwd_val) != canonical_cwd.as_path() => {
            println!("Your shell's $PWD:       {pwd_val}");
            println!("getcwd() returns:        {}", canonical_cwd.display());
            println!();
            println!("The tool emits: cd {}", canonical_cwd.display());
            println!(
                "                   {}",
                "^".repeat(canonical_cwd.display().to_string().len())
            );
            println!(
                "                   WRONG! This is the canonical path, not where you think you are."
            );
        }
        _ => {
            println!("No symlink mapping detected. $PWD and getcwd() agree:");
            println!("  Current directory: {}", canonical_cwd.display());
            println!();
            println!("Both paths are identical — no symlink is in effect.");
            println!("In a real scenario with a symlink, these would diverge.");
        }
    }
}

/// Demonstrate the fixed behavior: the same CLI tool scenario, but now using
/// `LogicalPathContext::detect()` and `to_logical()` to translate the
/// canonical path back to the logical (symlink-preserving) path.
fn fixed_cd_demo() {
    println!("=== The Fix: Corrected cd directive (with logical-path) ===");
    println!();

    let ctx = LogicalPathContext::detect();

    if ctx.has_mapping() {
        println!("Using LogicalPathContext::detect() + to_logical():");
        println!();

        let canonical_cwd = match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => {
                eprintln!("Error: could not determine current directory: {e}");
                return;
            }
        };

        let logical_cwd = ctx.to_logical(&canonical_cwd);
        println!("The tool emits: cd {}", logical_cwd.display());
        println!(
            "                   {}",
            "^".repeat(logical_cwd.display().to_string().len())
        );
        println!(
            "                   CORRECT! This preserves your symlink-based directory structure."
        );
    } else {
        let cwd = match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => {
                eprintln!("Error: could not determine current directory: {e}");
                return;
            }
        };
        println!("No mapping active — to_logical() returns the input path unchanged.");
        println!("The tool emits: cd {}", cwd.display());
    }
}

fn main() {
    broken_cd_demo();
    println!();
    fixed_cd_demo();
}
