//! `cargo malleus` — the Malleus RTOS developer tool.
//!
//! # Why this exists at Checkpoint 0
//!
//! Adoption is decided by the first hour. If getting from an empty board to a
//! running, inspectable firmware takes an afternoon of linker scripts, most
//! evaluators stop before they reach the parts that make the project worth
//! choosing. So the tool ships alongside the kernel rather than after it, and
//! the commands that *can* work without a kernel — `check` and `analyze`, both
//! pure manifest analysis — work today.
//!
//! Commands that need a kernel say so, and say which checkpoint delivers them.
//! An honest "not yet" is worth more than a stub that prints something
//! plausible.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

mod analyze;
mod check;

/// Build, flash, analyse, trace, and debug Malleus RTOS systems.
#[derive(Parser)]
#[command(name = "cargo-malleus", bin_name = "cargo malleus", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate `malleus.toml` without building anything.
    Check {
        /// Path to the manifest.
        #[arg(long, default_value = "malleus.toml")]
        manifest: PathBuf,
        /// Treat warnings as errors.
        #[arg(long)]
        deny_warnings: bool,
    },
    /// Report schedulability, CPU utilisation, and memory for the system.
    Analyze {
        /// Path to the manifest.
        #[arg(long, default_value = "malleus.toml")]
        manifest: PathBuf,
    },
    /// Create a new Malleus project. (Checkpoint 2)
    New {
        /// Project name.
        name: String,
    },
    /// Build the firmware image. (Checkpoint 2)
    Build,
    /// Flash the firmware to a connected board. (Checkpoint 2)
    Flash,
    /// Run the test suite, on host, in QEMU, or on hardware. (Checkpoint 2)
    Test,
    /// Show live task state, CPU use, stacks, and deadlines. (Checkpoint 4)
    Inspect,
    /// Record and render an execution timeline. (Checkpoint 4)
    Trace,
    /// Decode a crash dump from a device. (Checkpoint 3)
    Dump,
    /// Write out the code generated from the manifest. (Checkpoint 3)
    Expand,
}

impl Command {
    /// The checkpoint that delivers a not-yet-implemented command.
    const fn checkpoint(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::Check { .. } | Self::Analyze { .. } => None,
            Self::New { .. } | Self::Build | Self::Flash | Self::Test => {
                Some(("2", "docs/ROADMAP.md#checkpoint-2--make-it-usable"))
            }
            Self::Dump | Self::Expand => Some((
                "3",
                "docs/ROADMAP.md#checkpoint-3--build-the-differentiator",
            )),
            Self::Inspect | Self::Trace => {
                Some(("4", "docs/ROADMAP.md#checkpoint-4--production-platform"))
            }
        }
    }
}

fn main() -> ExitCode {
    // Invoked as `cargo malleus ...`, cargo passes "malleus" as argv[1].
    // Strip it so clap sees the subcommand either way.
    let args = std::env::args_os().enumerate().filter_map(|(i, arg)| {
        if i == 1 && arg == "malleus" {
            None
        } else {
            Some(arg)
        }
    });

    let cli = Cli::parse_from(args);

    match run(&cli.command) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: &Command) -> Result<ExitCode> {
    if let Some((checkpoint, link)) = command.checkpoint() {
        eprintln!(
            "This command is not implemented yet. It is delivered by Checkpoint {checkpoint}.\n\
             \n\
             Malleus RTOS is early, and the roadmap is public precisely so that what is and\n\
             is not built is never a surprise:\n\
             \n    {link}\n\
             \n\
             Working today: `cargo malleus check` and `cargo malleus analyze`, which need no\n\
             hardware and no kernel — they reason about `malleus.toml` alone."
        );
        return Ok(ExitCode::FAILURE);
    }

    match command {
        Command::Check {
            manifest,
            deny_warnings,
        } => {
            let source = read(manifest)?;
            check::run(&source, *deny_warnings)
        }
        Command::Analyze { manifest } => {
            let source = read(manifest)?;
            analyze::run(&source)
        }
        _ => unreachable!("every other command reports its checkpoint above"),
    }
}

fn read(path: &PathBuf) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| {
        format!(
            "could not read `{}`. Run this from a directory containing a Malleus manifest, \
             or pass --manifest <PATH>",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn implemented_commands_report_no_checkpoint() {
        assert!(
            Command::Check {
                manifest: PathBuf::new(),
                deny_warnings: false
            }
            .checkpoint()
            .is_none()
        );
        assert!(
            Command::Analyze {
                manifest: PathBuf::new()
            }
            .checkpoint()
            .is_none()
        );
    }

    #[test]
    fn unimplemented_commands_name_their_checkpoint() {
        for command in [
            Command::Build,
            Command::Flash,
            Command::Trace,
            Command::Dump,
        ] {
            let (checkpoint, link) = command
                .checkpoint()
                .expect("unimplemented commands must state a checkpoint");
            assert!(!checkpoint.is_empty());
            assert!(
                link.starts_with("docs/ROADMAP.md#"),
                "link must point at the roadmap"
            );
        }
    }
}
