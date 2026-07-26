//! `cargo malleus analyze` — schedulability and utilisation from the manifest.

use std::process::ExitCode;

use anyhow::{Result, bail};
use malleus_analyzer::{TaskTiming, Verdict, analyse};
use malleus_manifest::{Duration, Manifest};

/// Analyse `source` and print the schedulability report.
pub(crate) fn run(source: &str) -> Result<ExitCode> {
    let manifest = Manifest::parse(source)?;
    let report = manifest.validate();
    if report.error_count() > 0 {
        bail!("manifest is invalid; run `cargo malleus check` for details");
    }

    let tick_hz = parse_tick_hz(&manifest.system.tick_rate)?;
    let mut timings = Vec::with_capacity(manifest.tasks.len());
    for task in &manifest.tasks {
        timings.push(TaskTiming {
            name: task.name.clone(),
            priority: task.priority,
            period: to_ticks(task.period.as_deref(), tick_hz)?,
            deadline: to_ticks(task.deadline.as_deref(), tick_hz)?,
            wcet: to_ticks(task.wcet.as_deref(), tick_hz)?,
            // Shared-resource declarations arrive with the mutex work in
            // Checkpoint 2. Until then blocking is zero, and the report says so
            // rather than letting the reader assume it was accounted for.
            blocking: 0,
        });
    }

    let analysis = analyse(&timings);

    println!("Schedulability — fixed-priority preemptive, exact response-time analysis");
    println!(
        "System: {}   tick rate: {}\n",
        manifest.system.name, manifest.system.tick_rate
    );
    println!(
        "  {:<22} {:>5} {:>10} {:>10} {:>10}  Verdict",
        "Task", "Prio", "Period", "Deadline", "WCET"
    );

    for (timing, (name, verdict)) in timings.iter().zip(&analysis.verdicts) {
        let verdict_text = match verdict {
            Verdict::Pass { response, slack } => {
                format!(
                    "PASS   response {}, slack {}",
                    ticks(*response),
                    ticks(*slack)
                )
            }
            Verdict::Fail { response, overrun } => {
                format!(
                    "FAIL   response {}, over by {}",
                    ticks(*response),
                    ticks(*overrun)
                )
            }
            Verdict::Unknown { reason } => format!("UNKNOWN  {reason}"),
        };
        println!(
            "  {:<22} {:>5} {:>10} {:>10} {:>10}  {}",
            name,
            timing.priority,
            optional(timing.period),
            optional(timing.deadline),
            optional(timing.wcet),
            verdict_text
        );
    }

    let utilisation = analysis.utilisation_ppm as f64 / 10_000.0;
    println!("\n  CPU utilisation (periodic tasks): {utilisation:.1}%");
    println!("  Blocking terms: not yet modelled (shared resources arrive in Checkpoint 2)");

    if analysis.has_unknowns() {
        println!("\n  Verdict: UNKNOWN — at least one task lacks a declared WCET.");
        println!("  This is not a failure. It is the honest answer, and it names what to");
        println!("  measure next. Malleus will not print PASS for a system it cannot check.");
        return Ok(ExitCode::FAILURE);
    }

    if analysis.is_schedulable() {
        println!("\n  Verdict: PASS");
        Ok(ExitCode::SUCCESS)
    } else {
        println!("\n  Verdict: FAIL — at least one task provably misses its deadline.");
        Ok(ExitCode::FAILURE)
    }
}

fn optional(ticks: Option<u64>) -> String {
    ticks.map_or_else(|| "-".to_owned(), |t| t.to_string())
}

fn ticks(t: u64) -> String {
    format!("{t}t")
}

fn to_ticks(text: Option<&str>, tick_hz: u64) -> Result<Option<u64>> {
    match text {
        None => Ok(None),
        Some(t) => {
            let duration: Duration = t.parse()?;
            Ok(Some(duration.to_ticks(tick_hz)))
        }
    }
}

fn parse_tick_hz(text: &str) -> Result<u64> {
    let trimmed = text.trim();
    let Some(split) = trimmed.find(|c: char| !c.is_ascii_digit()) else {
        bail!("tick_rate `{text}` has no unit; write e.g. `1MHz`");
    };
    let (number, unit) = trimmed.split_at(split);
    let value: u64 = number.parse()?;
    match unit.trim() {
        "Hz" => Ok(value),
        "kHz" => Ok(value * 1_000),
        "MHz" => Ok(value * 1_000_000),
        other => bail!("tick_rate unit `{other}` is not valid; use Hz, kHz, or MHz"),
    }
}
