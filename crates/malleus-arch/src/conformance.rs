//! The conformance suite every architecture port must pass.
//!
//! A port is not "supported" because it compiles. It is supported when it
//! passes every check listed here, **on hardware**, in CI. This module holds
//! the machine-readable checklist; the executable tests live in
//! `tests/hil/conformance/` and are driven by `cargo malleus test --hil`.
//!
//! The list is here, in the contract crate, on purpose: adding a requirement to
//! the kernel means adding it here, which immediately marks every port as
//! having an unmet requirement until it is re-run. There is no way to add a
//! kernel guarantee without confronting what it costs each port.

/// One requirement a port must satisfy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    /// Stable identifier, referenced from the requirement–test matrix in
    /// `docs/internals/traceability.md`.
    pub id: &'static str,
    /// What must hold.
    pub description: &'static str,
    /// Whether a port can be declared supported without this.
    pub level: Level,
}

/// How binding a requirement is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Must pass. A port failing this is not merged.
    Required,
    /// Must pass for isolation tier 1 or above; ports without protection
    /// hardware are exempt and are labelled tier 0.
    RequiredWithProtection,
    /// Measured and published, but not pass/fail. Benchmarks live here.
    Measured,
}

/// The full conformance checklist.
///
/// Kept as a `const` array so a port's build script can iterate it and emit a
/// coverage report at compile time.
pub const REQUIREMENTS: &[Requirement] = &[
    Requirement {
        id: "ARCH-CTX-001",
        description: "A task's registers are bit-identical across a suspend/resume cycle, \
                      including FPU state when the FPU is enabled.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-CTX-002",
        description: "A task whose entry function returns lands on the exit trampoline and \
                      raises a fault; it never falls off the end of its stack.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-CTX-003",
        description: "Lazy FPU stacking does not leak one task's floating-point registers \
                      into another. Verified with a task that never touches the FPU.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-CS-001",
        description: "Critical sections nest correctly to depth 8 and restore the exact \
                      prior interrupt mask.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-CS-002",
        description: "enter_below(p) leaves interrupts above priority p live: an ISR above p \
                      fires during the section with unchanged latency.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-TIME-001",
        description: "now() is monotonic across the hardware counter's wrap point, sampled \
                      concurrently from a task and an ISR for at least two full wraps.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-TIME-002",
        description: "An alarm set for a deadline already in the past fires immediately \
                      rather than being lost.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-TIME-003",
        description: "No ticks are lost across a tickless idle period, verified against an \
                      external time reference over one hour.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-MPU-001",
        description: "An unprivileged task reading or writing outside its declared regions \
                      faults, and the fault is attributed to the correct task.",
        level: Level::RequiredWithProtection,
    },
    Requirement {
        id: "ARCH-MPU-002",
        description: "No stale region survives a context switch: task B cannot reach task A's \
                      stack immediately after A is suspended.",
        level: Level::RequiredWithProtection,
    },
    Requirement {
        id: "ARCH-MPU-003",
        description: "A task cannot reach a peripheral it did not declare a capability for, \
                      including via a DMA descriptor it constructed itself.",
        level: Level::RequiredWithProtection,
    },
    Requirement {
        id: "ARCH-FAULT-001",
        description: "Every hardware fault reaches the kernel fault handler with the faulting \
                      task, address, and instruction pointer recoverable.",
        level: Level::Required,
    },
    Requirement {
        id: "ARCH-FAULT-002",
        description: "A stack overflow is detected before it corrupts adjacent memory, not \
                      after.",
        level: Level::Required,
    },
    Requirement {
        id: "BENCH-001",
        description: "Interrupt latency: hardware event to first instruction of the handler. \
                      Report min, mean, p99.9, and observed maximum.",
        level: Level::Measured,
    },
    Requirement {
        id: "BENCH-002",
        description: "Context switch: last instruction of the outgoing task to first \
                      instruction of the incoming one.",
        level: Level::Measured,
    },
    Requirement {
        id: "BENCH-003",
        description: "Wake-up latency: ISR signals a task to that task running.",
        level: Level::Measured,
    },
    Requirement {
        id: "BENCH-004",
        description: "Periodic task jitter over one million activations at 1 kHz.",
        level: Level::Measured,
    },
    Requirement {
        id: "BENCH-005",
        description: "Longest observed critical section inside the kernel.",
        level: Level::Measured,
    },
];

/// Requirements that block a port from being declared supported.
pub fn blocking(with_protection: bool) -> impl Iterator<Item = &'static Requirement> {
    REQUIREMENTS.iter().filter(move |r| match r.level {
        Level::Required => true,
        Level::RequiredWithProtection => with_protection,
        Level::Measured => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_ids_are_unique() {
        for (i, a) in REQUIREMENTS.iter().enumerate() {
            for b in REQUIREMENTS.iter().skip(i + 1) {
                assert_ne!(a.id, b.id, "duplicate conformance requirement id");
            }
        }
    }

    #[test]
    fn protection_tier_changes_the_blocking_set() {
        let tier0 = blocking(false).count();
        let tier1 = blocking(true).count();
        assert!(
            tier1 > tier0,
            "isolation requirements must be blocking when protection hardware is present"
        );
    }
}
