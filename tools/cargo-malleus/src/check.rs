//! `cargo malleus check` — validate a manifest without building.

use std::process::ExitCode;

use anyhow::Result;
use malleus_manifest::{Manifest, Severity};

/// Validate `source` and print a report.
pub(crate) fn run(source: &str, deny_warnings: bool) -> Result<ExitCode> {
    let manifest = Manifest::parse(source)?;
    let report = manifest.validate();

    for diagnostic in &report.diagnostics {
        println!("{diagnostic}\n");
    }

    let errors = report.error_count();
    let warnings = report.warning_count();

    if errors > 0 {
        println!(
            "{}: {errors} error(s), {warnings} warning(s)",
            Severity::Error
        );
        return Ok(ExitCode::FAILURE);
    }

    if warnings > 0 {
        println!(
            "checked `{}`: {} task(s), {} channel(s) — {warnings} warning(s)",
            manifest.system.name,
            manifest.tasks.len(),
            manifest.channels.len()
        );
        if deny_warnings {
            println!("failing because --deny-warnings was given");
            return Ok(ExitCode::FAILURE);
        }
    } else {
        println!(
            "checked `{}`: {} task(s), {} channel(s) — no findings",
            manifest.system.name,
            manifest.tasks.len(),
            manifest.channels.len()
        );
    }

    Ok(ExitCode::SUCCESS)
}
