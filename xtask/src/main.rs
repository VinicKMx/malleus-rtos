//! Repository automation.
//!
//! The point of an `xtask` is that CI and a contributor's laptop run the *same*
//! commands. A CI pipeline that diverges from what people run locally trains
//! everyone to push and hope, and a green local build that fails in CI is a
//! tax on every contribution. `cargo xtask ci` is exactly what the workflow
//! runs — nothing more, nothing hidden.

use std::process::{Command, ExitCode};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

/// Repository automation for Malleus RTOS.
#[derive(Parser)]
#[command(name = "xtask", bin_name = "cargo xtask", about)]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Run everything CI runs, in the same order.
    Ci,
    /// Format check.
    Fmt,
    /// Clippy across the workspace, warnings denied.
    Lint,
    /// Host test suite.
    Test,
    /// Cross-compile the bare-metal crates for every supported target.
    Cross,
    /// Verify that every `unsafe` block carries a `SAFETY:` comment.
    Unsafe,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Task::Ci => {
            fmt()?;
            lint()?;
            test()?;
            cross()?;
            unsafe_audit()
        }
        Task::Fmt => fmt(),
        Task::Lint => lint(),
        Task::Test => test(),
        Task::Cross => cross(),
        Task::Unsafe => unsafe_audit(),
    }
}

fn fmt() -> Result<()> {
    cargo(&["fmt", "--all", "--check"])
}

fn lint() -> Result<()> {
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])
}

fn test() -> Result<()> {
    cargo(&["test", "--workspace", "--all-features"])
}

/// Bare-metal crates must build for the targets they claim to support.
///
/// The kernel is `no_std` and portable, so it compiles for the host too — which
/// is what makes the host test suite possible. But "compiles for the host" is
/// not evidence it compiles for a Cortex-M, and the difference is exactly where
/// portability bugs live.
fn cross() -> Result<()> {
    const TARGETS: [&str; 2] = ["thumbv7em-none-eabihf", "thumbv8m.main-none-eabihf"];
    const CRATES: [&str; 4] = [
        "malleusrt",
        "malleus-arch",
        "malleus-arch-cortex-m",
        "malleus-runtime",
    ];

    for target in TARGETS {
        for krate in CRATES {
            cargo(&["build", "--target", target, "--package", krate])?;
        }
    }
    Ok(())
}

/// Every `unsafe` block must carry a `SAFETY:` comment.
///
/// Clippy's `undocumented_unsafe_blocks` enforces this at the block level and
/// is already denied in the workspace lints. This check is the belt to that
/// pair of braces: it also catches `unsafe` in files clippy is not run over,
/// and it produces the list that feeds the safety-evidence artefact described
/// in `docs/adr/0010-unsafe-code-policy.md`.
fn unsafe_audit() -> Result<()> {
    println!("\n$ ./ci/check-unsafe.sh\n");
    let status = Command::new("./ci/check-unsafe.sh").status()?;
    if !status.success() {
        bail!("unsafe audit failed");
    }
    Ok(())
}

fn cargo(args: &[&str]) -> Result<()> {
    println!("\n$ cargo {}\n", args.join(" "));
    let status = Command::new(env!("CARGO")).args(args).status()?;
    if !status.success() {
        bail!("`cargo {}` failed", args.join(" "));
    }
    Ok(())
}
