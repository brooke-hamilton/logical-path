#[cfg(not(windows))]
compile_error!("This example requires Windows.");

use logical_path::LogicalPathContext;
use std::path::Path;

/// Strip the `\\?\` Extended Length Path prefix that `canonicalize()` adds
/// on Windows, so the output is human-readable.
fn strip_extended_prefix(path: &Path) -> std::path::PathBuf {
    let s = match path.to_str() {
        Some(s) => s,
        None => return path.to_path_buf(),
    };

    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return std::path::PathBuf::from(format!(r"\\{rest}"));
    }

    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return std::path::PathBuf::from(rest);
    }

    path.to_path_buf()
}

/// Demonstrate the broken behavior: a naive CLI tool that uses
/// `std::fs::canonicalize(std::env::current_dir())` to find the current
/// directory. On Windows, `canonicalize()` resolves NTFS junctions, subst
/// drives, and directory symlinks to their physical target, so any `cd`
/// directive it produces will point to the *canonical* path — not the path
/// the user expects.
fn broken_cd_demo() {
    println!("=== The Problem: Broken cd directive (without logical-path) ===");
    println!();
    println!("A naive CLI tool uses std::fs::canonicalize(current_dir()) to find where you are.");
    println!("On Windows, canonicalize() resolves NTFS junctions, subst drives, and symlinks.");
    println!();

    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => {
            eprintln!("Error: could not determine current directory: {e}");
            return;
        }
    };

    let canonical_cwd = match std::fs::canonicalize(&cwd) {
        Ok(c) => strip_extended_prefix(&c),
        Err(e) => {
            eprintln!("Error: could not canonicalize current directory: {e}");
            return;
        }
    };

    if cwd != canonical_cwd {
        println!("current_dir() returns:   {}", cwd.display());
        println!("canonicalize() returns:  {}", canonical_cwd.display());
        println!();
        println!("The tool emits: cd {}", canonical_cwd.display());
        println!(
            "                   {}",
            "^".repeat(canonical_cwd.display().to_string().len())
        );
        println!(
            "                   WRONG! This is the canonical path, not where you think you are."
        );
    } else {
        println!("No junction or subst mapping detected. current_dir() and canonicalize() agree:");
        println!("  Current directory: {}", cwd.display());
        println!();
        println!("Both paths are identical — no junction or subst drive is in effect.");
        println!("In a real scenario with a junction or subst drive, these would diverge.");
    }
}

/// Demonstrate the fixed behavior: the same CLI tool scenario, but now using
/// `LogicalPathContext::detect()` and `to_logical()` to translate the
/// canonical path back to the logical (junction/subst-preserving) path.
fn fixed_cd_demo() {
    println!("=== The Fix: Corrected cd directive (with logical-path) ===");
    println!();

    let ctx = LogicalPathContext::detect();

    if ctx.has_mapping() {
        println!("Using LogicalPathContext::detect() + to_logical():");
        println!();

        let cwd = match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => {
                eprintln!("Error: could not determine current directory: {e}");
                return;
            }
        };

        let canonical_cwd = match std::fs::canonicalize(&cwd) {
            Ok(c) => strip_extended_prefix(&c),
            Err(e) => {
                eprintln!("Error: could not canonicalize current directory: {e}");
                return;
            }
        };

        let logical_cwd = ctx.to_logical(&canonical_cwd);
        println!("The tool emits: cd {}", logical_cwd.display());
        println!(
            "                   {}",
            "^".repeat(logical_cwd.display().to_string().len())
        );
        println!("                   CORRECT! This preserves your junction/subst-based directory structure.");
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
