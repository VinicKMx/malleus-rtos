//! The Malleus system manifest: schema, parser, and validator.
//!
//! `malleus.toml` is the single declarative description of a system — its
//! tasks, their timing contracts, their stacks, the resources each may touch,
//! and the channels between them. Everything downstream is derived from it:
//! the task table, the protection-region layout, the typed IPC endpoints, the
//! linker configuration, the schedulability report, and the architecture
//! diagram in the generated documentation.
//!
//! ```toml
//! [system]
//! name  = "motion-controller"
//! board = "nucleo-f767zi"
//!
//! [[task]]
//! name     = "motor-control"
//! priority = 7
//! period   = "1ms"
//! deadline = "500us"
//! wcet     = "180us"      # measured, and checked against reality on hardware
//! stack    = "2KiB"
//! restart  = "never"      # a half-running motor loop is worse than none
//! capabilities = ["timer.control", "spi.encoder", "pwm.motor"]
//!
//! [[task]]
//! name     = "telemetry"
//! priority = 2
//! stack    = "4KiB"
//! restart  = { on-fault = { budget = 5, window = "60s" } }
//! capabilities = ["ipc.sensor-data", "net.mqtt"]
//!
//! [[channel]]
//! name     = "sensor-data"
//! from     = "sensor-acquisition"
//! to       = "telemetry"
//! capacity = 16
//! overflow = "drop-oldest"  # fresh samples supersede stale ones
//! ```
//!
//! # Why a manifest, and not just Rust
//!
//! Because the interesting questions are about the system, not about any one
//! task. Whether the task set is schedulable, whether the protection regions
//! fit the MPU, whether total stack plus channels plus kernel exceeds RAM, and
//! whether the IPC graph has a cycle are all *whole-system* properties. Rust's
//! type system reasons brilliantly about one crate at a time; it has nothing to
//! say about whether the sum of your stacks fits in 512 KiB.
//!
//! The manifest is deliberately data, not code. It can be diffed, reviewed,
//! generated, and analysed by tools that are not the compiler — including tools
//! nobody has written yet. See `docs/adr/0002-static-system-definition.md` and
//! `docs/design/07-system-manifest.md`.

use serde::{Deserialize, Serialize};

mod units;
mod validate;

pub use units::{ByteSize, Duration, ParseUnitError};
pub use validate::{Diagnostic, Severity, ValidationReport};

/// A parsed system manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// System-wide settings.
    pub system: System,
    /// Task declarations.
    #[serde(default, rename = "task")]
    pub tasks: Vec<Task>,
    /// Channel declarations.
    #[serde(default, rename = "channel")]
    pub channels: Vec<Channel>,
}

/// System-wide settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct System {
    /// System name. Appears in the firmware image and in telemetry.
    pub name: String,
    /// Board support crate to build against.
    pub board: String,
    /// Monotonic tick rate. Defaults to 1 MHz — fine enough for microsecond
    /// deadlines, coarse enough that a 64-bit counter never wraps in practice.
    #[serde(default = "default_tick_rate")]
    pub tick_rate: String,
}

fn default_tick_rate() -> String {
    "1MHz".to_owned()
}

/// One task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    /// Unique task name.
    pub name: String,
    /// Scheduling priority; higher is more urgent.
    pub priority: u8,
    /// Stack reservation, e.g. `"2KiB"`.
    pub stack: String,
    /// Activation period for periodic tasks, e.g. `"1ms"`.
    #[serde(default)]
    pub period: Option<String>,
    /// Relative deadline. Defaults to the period when omitted.
    #[serde(default)]
    pub deadline: Option<String>,
    /// Declared worst-case execution time.
    ///
    /// Optional, because an honest `None` is better than an invented number.
    /// Its absence is not fatal — it downgrades the schedulability verdict from
    /// `PASS` to `UNKNOWN`, which is the truthful answer.
    #[serde(default)]
    pub wcet: Option<String>,
    /// Resources this task may touch. Anything not listed is unreachable.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Whether the task runs unprivileged behind memory protection.
    #[serde(default = "default_true")]
    pub isolated: bool,
}

fn default_true() -> bool {
    true
}

/// One IPC channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    /// Unique channel name.
    pub name: String,
    /// Sending task.
    pub from: String,
    /// Receiving task.
    pub to: String,
    /// Queue depth.
    pub capacity: u16,
    /// Behaviour when full: `block`, `reject`, `drop-oldest`, `drop-newest`.
    pub overflow: String,
}

/// Failure to read a manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The TOML was malformed.
    #[error("malformed manifest: {0}")]
    Syntax(#[from] toml::de::Error),
    /// The manifest parsed but describes an invalid system.
    #[error("manifest describes an invalid system: {0} error(s)")]
    Invalid(usize),
}

impl Manifest {
    /// Parse a manifest from TOML source, without validating it.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Syntax`] if the input is not well-formed TOML
    /// matching the schema.
    pub fn parse(source: &str) -> Result<Self, ManifestError> {
        Ok(toml::from_str(source)?)
    }

    /// Parse and validate.
    ///
    /// Returns the manifest together with its report, so that a caller can show
    /// warnings even on success. Warnings are shown, never swallowed: a warning
    /// nobody sees is a bug nobody fixes.
    ///
    /// # Errors
    ///
    /// Returns [`ManifestError::Syntax`] for malformed input, or
    /// [`ManifestError::Invalid`] when validation produced any error-severity
    /// diagnostic.
    pub fn parse_and_validate(source: &str) -> Result<(Self, ValidationReport), ManifestError> {
        let manifest = Self::parse(source)?;
        let report = manifest.validate();
        if report.error_count() > 0 {
            return Err(ManifestError::Invalid(report.error_count()));
        }
        Ok((manifest, report))
    }

    /// Look up a task by name.
    #[must_use]
    pub fn task(&self, name: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
        [system]
        name  = "demo"
        board = "nucleo-f767zi"

        [[task]]
        name     = "control"
        priority = 7
        stack    = "2KiB"
    "#;

    #[test]
    fn a_minimal_manifest_parses() {
        let manifest = Manifest::parse(MINIMAL).expect("minimal manifest must parse");
        assert_eq!(manifest.system.name, "demo");
        assert_eq!(manifest.tasks.len(), 1);
        assert_eq!(manifest.tasks[0].priority, 7);
    }

    #[test]
    fn tick_rate_and_isolation_have_safe_defaults() {
        let manifest = Manifest::parse(MINIMAL).unwrap();
        assert_eq!(manifest.system.tick_rate, "1MHz");
        assert!(
            manifest.tasks[0].isolated,
            "isolation must be opt-out, never opt-in: the safe default is the protected one"
        );
    }

    #[test]
    fn unknown_keys_are_rejected_rather_than_ignored() {
        // A typo'd key that is silently ignored is how a system ends up running
        // with a stack half the size the engineer thought they asked for.
        let source = r#"
            [system]
            name  = "demo"
            board = "nucleo-f767zi"

            [[task]]
            name     = "control"
            priority = 7
            stack    = "2KiB"
            stak     = "8KiB"
        "#;
        assert!(matches!(
            Manifest::parse(source),
            Err(ManifestError::Syntax(_))
        ));
    }

    #[test]
    fn tasks_are_addressable_by_name() {
        let manifest = Manifest::parse(MINIMAL).unwrap();
        assert!(manifest.task("control").is_some());
        assert!(manifest.task("nonexistent").is_none());
    }
}
