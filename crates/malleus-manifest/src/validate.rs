//! Whole-system validation of a manifest.
//!
//! # On error messages
//!
//! Adoption is decided in the first hour, and the first hour is mostly spent
//! reading error messages. A diagnostic here is required to say three things:
//! what is wrong, where, and what to do about it. "invalid priority" says one
//! of the three. This module treats a diagnostic without a suggestion as an
//! incomplete diagnostic, and the tests enforce it.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::{Duration, Manifest};

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The build proceeds; the engineer should know something anyway.
    Warning,
    /// The build stops.
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// One finding about a manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// How serious.
    pub severity: Severity,
    /// Stable code, e.g. `"M0007"`, so it can be looked up, suppressed by
    /// policy, and referenced in documentation.
    pub code: &'static str,
    /// What is wrong.
    pub message: String,
    /// Where — a task name, channel name, or `[system]`.
    pub location: String,
    /// What to do about it. Never empty.
    pub suggestion: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]: {}\n  --> {}\n  help: {}",
            self.severity, self.code, self.message, self.location, self.suggestion
        )
    }
}

/// The result of validating a manifest.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// Every finding, in the order produced.
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// Number of error-severity diagnostics.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    /// Number of warning-severity diagnostics.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count()
    }

    /// Whether the manifest describes a buildable system.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.error_count() == 0
    }

    fn push(
        &mut self,
        severity: Severity,
        code: &'static str,
        location: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            message: message.into(),
            location: location.into(),
            suggestion: suggestion.into(),
        });
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for d in &self.diagnostics {
            writeln!(f, "{d}\n")?;
        }
        Ok(())
    }
}

impl Manifest {
    /// Check the manifest for whole-system problems.
    ///
    /// This is the build-time half of the real-time story. It does not need
    /// hardware and it does not need the code to run — every check here is a
    /// property of the declaration itself.
    #[must_use]
    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();
        self.check_task_names(&mut report);
        self.check_priorities(&mut report);
        self.check_timing(&mut report);
        self.check_stacks(&mut report);
        self.check_channels(&mut report);
        self.check_capabilities(&mut report);
        report
    }

    fn check_task_names(&self, report: &mut ValidationReport) {
        let mut seen: HashSet<&str> = HashSet::new();
        for task in &self.tasks {
            if !seen.insert(task.name.as_str()) {
                report.push(
                    Severity::Error,
                    "M0001",
                    &task.name,
                    format!("task name `{}` is declared more than once", task.name),
                    "task names identify endpoints in generated code and traces; give each \
                     task a distinct name",
                );
            }
            if task.name.is_empty() {
                report.push(
                    Severity::Error,
                    "M0002",
                    "[[task]]",
                    "task name is empty",
                    "give the task a short kebab-case name, e.g. `motor-control`",
                );
            }
        }
    }

    fn check_priorities(&self, report: &mut ValidationReport) {
        const MAX_PRIORITY: u8 = 31;
        for task in &self.tasks {
            if task.priority > MAX_PRIORITY {
                report.push(
                    Severity::Error,
                    "M0003",
                    &task.name,
                    format!(
                        "priority {} is out of range (the kernel provides 0..={MAX_PRIORITY})",
                        task.priority
                    ),
                    "lower the priority, or reconsider the design: needing more than 32 \
                     distinct urgency levels usually means priorities are encoding something \
                     else",
                );
            }
            if task.priority == 0 {
                report.push(
                    Severity::Error,
                    "M0004",
                    &task.name,
                    "priority 0 is reserved for the kernel idle task",
                    "use priority 1 for the least urgent application task",
                );
            }
        }

        // Two hard real-time tasks sharing a priority cannot both be guaranteed
        // their deadline: within a level, scheduling is FIFO, so one waits for
        // the other's full execution.
        let mut by_priority: HashMap<u8, Vec<&str>> = HashMap::new();
        for task in &self.tasks {
            if task.deadline.is_some() {
                by_priority
                    .entry(task.priority)
                    .or_default()
                    .push(&task.name);
            }
        }
        for (priority, names) in by_priority {
            if names.len() > 1 {
                report.push(
                    Severity::Warning,
                    "M0005",
                    names.join(", "),
                    format!(
                        "{} tasks with declared deadlines share priority {priority}",
                        names.len()
                    ),
                    "tasks at the same priority run FIFO, so each one's worst case includes \
                     the others' execution; give them distinct priorities, or accept the \
                     coupling and check the response-time report",
                );
            }
        }
    }

    fn check_timing(&self, report: &mut ValidationReport) {
        let tick_hz = parse_tick_hz(&self.system.tick_rate);

        for task in &self.tasks {
            let period = parse_duration(task.period.as_deref(), &task.name, "period", report);
            let deadline = parse_duration(task.deadline.as_deref(), &task.name, "deadline", report);
            let wcet = parse_duration(task.wcet.as_deref(), &task.name, "wcet", report);

            if let (Some(period), Some(deadline)) = (period, deadline)
                && deadline > period
            {
                report.push(
                    Severity::Error,
                    "M0006",
                    &task.name,
                    format!(
                        "deadline {deadline} is longer than period {period}; the task would \
                         still be running when its next activation is due"
                    ),
                    "shorten the deadline to at most the period, or lengthen the period",
                );
            }

            if let (Some(wcet), Some(deadline)) = (wcet, deadline)
                && wcet > deadline
            {
                report.push(
                    Severity::Error,
                    "M0007",
                    &task.name,
                    format!(
                        "declared worst-case execution time {wcet} exceeds deadline \
                         {deadline}; this task cannot meet its deadline even alone on the CPU"
                    ),
                    "reduce the work per activation, split it across activations, or relax \
                     the deadline",
                );
            }

            if task.deadline.is_some() && task.wcet.is_none() {
                report.push(
                    Severity::Warning,
                    "M0008",
                    &task.name,
                    "task declares a deadline but no worst-case execution time, so \
                     schedulability cannot be computed",
                    "measure the task's worst case with `cargo malleus trace` and declare it \
                     as `wcet`; until then this task's verdict is UNKNOWN, not PASS",
                );
            }

            if let (Some(period), Some(hz)) = (period, tick_hz)
                && !period.is_tick_aligned(hz)
            {
                report.push(
                    Severity::Warning,
                    "M0009",
                    &task.name,
                    format!(
                        "period {period} is not an exact multiple of the {} tick",
                        self.system.tick_rate
                    ),
                    "round the period to a tick multiple, or raise `tick_rate`; a fractional \
                     period produces systematic jitter that looks like a hardware fault",
                );
            }
        }
    }

    fn check_stacks(&self, report: &mut ValidationReport) {
        for task in &self.tasks {
            let Ok(stack) = task.stack.parse::<crate::ByteSize>() else {
                report.push(
                    Severity::Error,
                    "M0010",
                    &task.name,
                    format!("stack `{}` is not a valid size", task.stack),
                    "write a binary size with an explicit unit, e.g. `2KiB`",
                );
                continue;
            };

            // Below this, the architecture's exception frame alone dominates
            // and any real call depth overflows immediately.
            const MIN_STACK: u64 = 256;
            if stack.as_bytes() < MIN_STACK {
                report.push(
                    Severity::Error,
                    "M0011",
                    &task.name,
                    format!(
                        "stack {stack} is below the {MIN_STACK}-byte minimum needed for an \
                         exception frame"
                    ),
                    "reserve at least 512B; use `cargo malleus analyze --stacks` for a \
                     measured figure",
                );
            }

            let padded = stack.to_power_of_two();
            if padded != stack {
                report.push(
                    Severity::Warning,
                    "M0012",
                    &task.name,
                    format!(
                        "stack {stack} will occupy {padded} on an ARMv7-M MPU, wasting {} bytes \
                         to alignment",
                        padded.as_bytes() - stack.as_bytes()
                    ),
                    format!(
                        "round the stack up to `{padded}` to make the cost explicit, or move \
                         to an ARMv8-M part where regions are 32-byte granular"
                    ),
                );
            }
        }
    }

    fn check_channels(&self, report: &mut ValidationReport) {
        let names: HashSet<&str> = self.tasks.iter().map(|t| t.name.as_str()).collect();
        let mut seen: HashSet<&str> = HashSet::new();

        for channel in &self.channels {
            if !seen.insert(channel.name.as_str()) {
                report.push(
                    Severity::Error,
                    "M0013",
                    &channel.name,
                    format!("channel name `{}` is declared more than once", channel.name),
                    "give each channel a distinct name",
                );
            }

            for (role, task) in [("from", &channel.from), ("to", &channel.to)] {
                if !names.contains(task.as_str()) {
                    report.push(
                        Severity::Error,
                        "M0014",
                        &channel.name,
                        format!("`{role}` names task `{task}`, which is not declared"),
                        format!(
                            "declare a `[[task]]` named `{task}`, or correct the spelling — \
                             known tasks: {}",
                            sorted_list(&names)
                        ),
                    );
                }
            }

            if channel.from == channel.to {
                report.push(
                    Severity::Error,
                    "M0015",
                    &channel.name,
                    format!("task `{}` sends to itself", channel.from),
                    "a task does not need IPC to talk to itself; use an ordinary local queue",
                );
            }

            if channel.capacity == 0 {
                report.push(
                    Severity::Error,
                    "M0016",
                    &channel.name,
                    "capacity 0 is not a valid queue depth",
                    "use capacity 1 for a single-slot mailbox; unbuffered handoff is a \
                     `[[rendezvous]]`, which has different blocking semantics",
                );
            }

            const VALID_OVERFLOW: [&str; 4] = ["block", "reject", "drop-oldest", "drop-newest"];
            if !VALID_OVERFLOW.contains(&channel.overflow.as_str()) {
                report.push(
                    Severity::Error,
                    "M0017",
                    &channel.name,
                    format!("`{}` is not a valid overflow policy", channel.overflow),
                    format!(
                        "choose one of: {}. There is no default: the right answer depends on \
                         whether losing a message is worse than delaying one",
                        VALID_OVERFLOW.join(", ")
                    ),
                );
            }

            // Sending from a high-priority task into a blocking channel served
            // by a lower-priority one is priority inversion by construction.
            if channel.overflow == "block"
                && let (Some(from), Some(to)) = (self.task(&channel.from), self.task(&channel.to))
                && from.priority > to.priority
            {
                report.push(
                    Severity::Warning,
                    "M0018",
                    &channel.name,
                    format!(
                        "`{}` (priority {}) blocks on a channel drained by `{}` (priority {})",
                        from.name, from.priority, to.name, to.priority
                    ),
                    "this is priority inversion by construction: the urgent sender waits for \
                     the less urgent receiver. Use `drop-oldest` if the data is sampled state, \
                     or raise the receiver's priority",
                );
            }
        }
    }

    fn check_capabilities(&self, report: &mut ValidationReport) {
        // A capability naming a channel must match a declared channel, and the
        // task must actually be an endpoint of it.
        let channels: HashMap<&str, (&str, &str)> = self
            .channels
            .iter()
            .map(|c| (c.name.as_str(), (c.from.as_str(), c.to.as_str())))
            .collect();

        for task in &self.tasks {
            let mut seen: HashSet<&str> = HashSet::new();
            for capability in &task.capabilities {
                if !seen.insert(capability.as_str()) {
                    report.push(
                        Severity::Warning,
                        "M0019",
                        &task.name,
                        format!("capability `{capability}` is listed twice"),
                        "remove the duplicate",
                    );
                }

                let Some(channel_name) = capability.strip_prefix("ipc.") else {
                    continue;
                };
                match channels.get(channel_name) {
                    None => report.push(
                        Severity::Error,
                        "M0020",
                        &task.name,
                        format!("capability `{capability}` names an undeclared channel"),
                        format!(
                            "declare `[[channel]] name = \"{channel_name}\"`, or correct the \
                             spelling"
                        ),
                    ),
                    Some((from, to)) if *from != task.name && *to != task.name => report.push(
                        Severity::Error,
                        "M0021",
                        &task.name,
                        format!(
                            "task holds `{capability}` but is neither endpoint of channel \
                             `{channel_name}` (which runs `{from}` → `{to}`)"
                        ),
                        "grant the capability to an endpoint, or add a channel connecting this \
                         task",
                    ),
                    Some(_) => {}
                }
            }
        }

        // A declared channel whose endpoints never took the capability is dead
        // weight: it costs RAM and appears on the architecture diagram.
        for channel in &self.channels {
            let capability = format!("ipc.{}", channel.name);
            let claimed = self
                .tasks
                .iter()
                .filter(|t| t.name == channel.from || t.name == channel.to)
                .filter(|t| t.capabilities.contains(&capability))
                .count();
            if claimed == 0 {
                report.push(
                    Severity::Warning,
                    "M0022",
                    &channel.name,
                    "channel is declared but neither endpoint holds its capability, so no \
                     task can use it",
                    format!(
                        "add `\"{capability}\"` to the capabilities of `{}` and `{}`, or \
                         delete the channel to reclaim its RAM",
                        channel.from, channel.to
                    ),
                );
            }
        }
    }
}

fn parse_duration(
    text: Option<&str>,
    task: &str,
    field: &'static str,
    report: &mut ValidationReport,
) -> Option<Duration> {
    let text = text?;
    match text.parse::<Duration>() {
        Ok(d) => Some(d),
        Err(e) => {
            report.push(
                Severity::Error,
                "M0023",
                task,
                format!("`{field}` is invalid: {e}"),
                "write a duration with an explicit unit, e.g. `1ms`, `500us`, `250ns`",
            );
            None
        }
    }
}

fn parse_tick_hz(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    let split = trimmed.find(|c: char| !c.is_ascii_digit())?;
    let (number, unit) = trimmed.split_at(split);
    let value = number.parse::<u64>().ok()?;
    match unit.trim() {
        "Hz" => Some(value),
        "kHz" => Some(value * 1_000),
        "MHz" => Some(value * 1_000_000),
        _ => None,
    }
}

fn sorted_list(names: &HashSet<&str>) -> String {
    let mut v: Vec<&str> = names.iter().copied().collect();
    v.sort_unstable();
    v.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(body: &str) -> Manifest {
        let source = format!("[system]\nname = \"t\"\nboard = \"nucleo-f767zi\"\n{body}");
        Manifest::parse(&source).expect("test manifest must parse")
    }

    fn codes(report: &ValidationReport) -> Vec<&str> {
        report.diagnostics.iter().map(|d| d.code).collect()
    }

    #[test]
    fn a_sound_manifest_produces_no_errors() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "2KiB"
            period = "1ms"
            deadline = "500us"
            wcet = "180us"
            capabilities = ["ipc.samples"]

            [[task]]
            name = "telemetry"
            priority = 2
            stack = "4KiB"
            capabilities = ["ipc.samples"]

            [[channel]]
            name = "samples"
            from = "control"
            to = "telemetry"
            capacity = 16
            overflow = "drop-oldest"
        "#,
        );
        let report = m.validate();
        assert!(report.is_ok(), "unexpected errors: {report}");
    }

    #[test]
    fn deadline_longer_than_period_is_an_error() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "2KiB"
            period = "1ms"
            deadline = "2ms"
        "#,
        );
        assert!(codes(&m.validate()).contains(&"M0006"));
    }

    #[test]
    fn wcet_exceeding_deadline_is_an_error() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "2KiB"
            period = "1ms"
            deadline = "500us"
            wcet = "600us"
        "#,
        );
        assert!(codes(&m.validate()).contains(&"M0007"));
    }

    #[test]
    fn a_deadline_without_a_wcet_downgrades_to_unknown() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "2KiB"
            period = "1ms"
            deadline = "500us"
        "#,
        );
        let report = m.validate();
        assert!(codes(&report).contains(&"M0008"));
        assert!(
            report.is_ok(),
            "a missing WCET is a warning, not a build failure"
        );
    }

    #[test]
    fn the_idle_priority_is_reserved() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 0
            stack = "2KiB"
        "#,
        );
        assert!(codes(&m.validate()).contains(&"M0004"));
    }

    #[test]
    fn duplicate_task_names_are_rejected() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "2KiB"

            [[task]]
            name = "control"
            priority = 5
            stack = "2KiB"
        "#,
        );
        assert!(codes(&m.validate()).contains(&"M0001"));
    }

    #[test]
    fn a_channel_to_an_unknown_task_names_the_known_ones() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "2KiB"

            [[channel]]
            name = "samples"
            from = "control"
            to = "telemetri"
            capacity = 4
            overflow = "reject"
        "#,
        );
        let report = m.validate();
        let d = report
            .diagnostics
            .iter()
            .find(|d| d.code == "M0014")
            .expect("unknown endpoint must be reported");
        assert!(
            d.suggestion.contains("control"),
            "the suggestion must list the tasks that do exist: {}",
            d.suggestion
        );
    }

    #[test]
    fn non_power_of_two_stacks_report_their_armv7m_padding() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 7
            stack = "3KiB"
        "#,
        );
        let report = m.validate();
        let d = report
            .diagnostics
            .iter()
            .find(|d| d.code == "M0012")
            .expect("padding warning");
        assert!(
            d.message.contains("4KiB"),
            "must state the real cost: {}",
            d.message
        );
        assert!(
            d.message.contains("1024"),
            "must state the waste: {}",
            d.message
        );
    }

    #[test]
    fn blocking_from_high_to_low_priority_is_flagged_as_inversion() {
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 9
            stack = "2KiB"
            capabilities = ["ipc.cmd"]

            [[task]]
            name = "logger"
            priority = 2
            stack = "2KiB"
            capabilities = ["ipc.cmd"]

            [[channel]]
            name = "cmd"
            from = "control"
            to = "logger"
            capacity = 4
            overflow = "block"
        "#,
        );
        assert!(codes(&m.validate()).contains(&"M0018"));
    }

    #[test]
    fn a_capability_for_a_channel_the_task_is_not_part_of_is_rejected() {
        let m = manifest(
            r#"
            [[task]]
            name = "a"
            priority = 3
            stack = "2KiB"

            [[task]]
            name = "b"
            priority = 4
            stack = "2KiB"
            capabilities = ["ipc.link"]

            [[task]]
            name = "eavesdropper"
            priority = 5
            stack = "2KiB"
            capabilities = ["ipc.link"]

            [[channel]]
            name = "link"
            from = "a"
            to = "b"
            capacity = 4
            overflow = "reject"
        "#,
        );
        let report = m.validate();
        let d = report
            .diagnostics
            .iter()
            .find(|d| d.code == "M0021")
            .expect("M0021");
        assert_eq!(d.location, "eavesdropper");
    }

    #[test]
    fn an_unused_channel_is_reported_as_wasted_ram() {
        let m = manifest(
            r#"
            [[task]]
            name = "a"
            priority = 3
            stack = "2KiB"

            [[task]]
            name = "b"
            priority = 4
            stack = "2KiB"

            [[channel]]
            name = "link"
            from = "a"
            to = "b"
            capacity = 64
            overflow = "reject"
        "#,
        );
        assert!(codes(&m.validate()).contains(&"M0022"));
    }

    #[test]
    fn every_diagnostic_says_what_to_do_about_it() {
        // The contract this module exists to enforce. A diagnostic that only
        // states a problem sends the reader to the source; one that states a
        // fix keeps them working.
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 0
            stack = "3KiB"
            period = "1ms"
            deadline = "2ms"

            [[channel]]
            name = "x"
            from = "control"
            to = "control"
            capacity = 0
            overflow = "maybe"
        "#,
        );
        let report = m.validate();
        assert!(!report.diagnostics.is_empty());
        for d in &report.diagnostics {
            assert!(
                !d.suggestion.trim().is_empty(),
                "{} has no suggestion",
                d.code
            );
            assert!(!d.location.trim().is_empty(), "{} has no location", d.code);
            assert!(!d.message.trim().is_empty(), "{} has no message", d.code);
        }
    }

    #[test]
    fn diagnostic_codes_are_unique_per_condition() {
        // Guards against copy-paste reuse of a code for a different condition,
        // which would break suppression-by-code and documentation lookup.
        let m = manifest(
            r#"
            [[task]]
            name = "control"
            priority = 0
            stack = "2KiB"
        "#,
        );
        let report = m.validate();
        let mut codes: Vec<&str> = codes(&report);
        codes.sort_unstable();
        let before = codes.len();
        codes.dedup();
        assert_eq!(
            before,
            codes.len(),
            "one condition produced a duplicated code"
        );
    }
}
