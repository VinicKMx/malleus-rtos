//! Generates the task table, IPC endpoints, and memory layout from a manifest.
//!
//! # Status: not yet implemented — Checkpoint 3
//!
//! # The rule this crate lives under
//!
//! **Everything it generates must be readable.**
//!
//! `cargo malleus expand` writes the generated code to a file you can open,
//! diff, and step through in a debugger. `cargo malleus inspect --layout`
//! prints the memory map, the protection regions, and the IPC graph. None of it
//! is hidden inside a proc macro whose output you can only infer from error
//! messages.
//!
//! This is not a stylistic preference. In a system where a wrong protection
//! region means a fault that does not happen — an isolation failure that leaves
//! no trace — the engineer has to be able to *check*. A tool that asks to be
//! trusted, in that position, has not earned it. Magic is a liability here.
//! See `docs/adr/0006-typed-ipc.md` and `docs/design/07-system-manifest.md`.
//!
//! # What is generated
//!
//! | Artefact | Purpose |
//! |----------|---------|
//! | `tasks.rs` | The `static` task table: priorities, stacks, entry points, restart policy |
//! | `ipc.rs` | One typed endpoint per channel, visible only to tasks holding its capability |
//! | `regions.rs` | Protection region sets, one per task, pre-computed for the switch path |
//! | `memory.x` | Linker script sections placing stacks and channel storage |
//! | `layout.md` | Human-readable memory report, committed to the repo and diffed in review |
//! | `architecture.md` | Task and IPC graph, generated so it cannot go stale |
//!
//! That last row is deliberate. Architecture diagrams rot because they are
//! maintained by hand, separately from the thing they describe. This one is
//! derived from the same manifest the firmware is built from, so it is either
//! correct or the build is broken.

/// Artefacts the generator produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Artefact {
    /// The `static` task table.
    TaskTable,
    /// Typed IPC endpoints.
    IpcEndpoints,
    /// Per-task memory protection region sets.
    Regions,
    /// Linker script fragment.
    LinkerScript,
    /// Human-readable memory report.
    MemoryReport,
    /// Generated architecture documentation.
    ArchitectureDoc,
}

impl Artefact {
    /// The file this artefact is written to, relative to the output directory.
    #[must_use]
    pub const fn filename(self) -> &'static str {
        match self {
            Self::TaskTable => "tasks.rs",
            Self::IpcEndpoints => "ipc.rs",
            Self::Regions => "regions.rs",
            Self::LinkerScript => "memory.x",
            Self::MemoryReport => "layout.md",
            Self::ArchitectureDoc => "architecture.md",
        }
    }

    /// Whether this artefact is meant to be committed and reviewed.
    ///
    /// Generated *code* is build output and stays out of the repository.
    /// Generated *reports* are committed, so that a pull request which quietly
    /// grows a stack by 4 KiB or adds an IPC edge shows up as a diff a reviewer
    /// can see. Making the consequences of a manifest change visible in review
    /// is worth the small noise of a checked-in generated file.
    #[must_use]
    pub const fn is_reviewed(self) -> bool {
        matches!(self, Self::MemoryReport | Self::ArchitectureDoc)
    }

    /// Every artefact.
    pub const ALL: [Self; 6] = [
        Self::TaskTable,
        Self::IpcEndpoints,
        Self::Regions,
        Self::LinkerScript,
        Self::MemoryReport,
        Self::ArchitectureDoc,
    ];
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_artefact_has_a_distinct_filename() {
        let names: HashSet<&str> = Artefact::ALL.iter().map(|a| a.filename()).collect();
        assert_eq!(
            names.len(),
            Artefact::ALL.len(),
            "two artefacts would overwrite each other"
        );
    }

    #[test]
    fn generated_code_is_not_committed_but_reports_are() {
        assert!(!Artefact::TaskTable.is_reviewed());
        assert!(!Artefact::IpcEndpoints.is_reviewed());
        assert!(!Artefact::LinkerScript.is_reviewed());
        assert!(Artefact::MemoryReport.is_reviewed());
        assert!(Artefact::ArchitectureDoc.is_reviewed());
    }
}
