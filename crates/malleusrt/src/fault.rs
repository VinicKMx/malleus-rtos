//! Fault capture, supervision, and crash dumps.
//!
//! # The thesis, in one module
//!
//! Rust removes a large class of memory bugs from safe code. It does not remove
//! them from `unsafe` blocks, from DMA engines that ignore the type system
//! entirely, from vendor C libraries, or from a peripheral misconfigured into
//! writing wherever it likes. Nor does it do anything about the bugs that are
//! not memory bugs at all: a task that overruns its deadline, deadlocks, or
//! divides by zero.
//!
//! So the question is not "can a task fail" — it can — but **what the product
//! does when one does**. Malleus's answer is that a fault is a local, recorded,
//! survivable event:
//!
//! ```text
//! MQTT client dereferences a null pointer
//!   → MPU raises a memory fault
//!   → kernel records who, where, and what they were doing
//!   → only that task is stopped; the 1 kHz motor loop never misses a tick
//!   → supervisor consults the restart policy and restarts it
//!   → crash dump persists to flash, readable later by `cargo malleus dump`
//!   → repeated failure escalates to degraded mode, not to a silent reboot loop
//! ```
//!
//! See `docs/design/05-fault-model.md`.

use malleus_arch::Instant;

use crate::TaskId;

/// What went wrong.
///
/// Deliberately a small, `Copy`, non-allocating enum: it is recorded inside a
/// fault handler, where the failing task's memory is not to be trusted and
/// there is no safe way to build anything more elaborate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FaultKind {
    /// Access outside the task's permitted regions. The isolation mechanism
    /// did its job.
    MemoryAccess {
        /// The address the task tried to touch, if the hardware captured it.
        address: Option<usize>,
        /// Whether the access was a write.
        write: bool,
    },
    /// The task's stack grew past its allocation. Detected by a guard region
    /// before adjacent memory is corrupted, not after.
    StackOverflow,
    /// Undefined instruction, misaligned access, or a bus error.
    IllegalOperation,
    /// Division by zero, or an arithmetic overflow with checks enabled.
    Arithmetic,
    /// The task called `panic!`.
    Panic {
        /// Source location, when the panic handler could recover it.
        location: Option<&'static core::panic::Location<'static>>,
    },
    /// The task missed a declared deadline by more than its allowed slack.
    ///
    /// Treating this as a fault rather than a statistic is a deliberate
    /// position: in a control system, a missed deadline is a failure of the
    /// same seriousness as a bad pointer, and the difference is only that the
    /// consequences arrive later.
    DeadlineMiss {
        /// By how many ticks the deadline was overrun.
        overrun_ticks: u64,
    },
    /// The task stopped feeding its watchdog.
    WatchdogTimeout,
    /// The kernel detected an invariant violation attributable to this task,
    /// e.g. a malformed argument crossing the C FFI boundary.
    KernelInvariant {
        /// Stable identifier of the violated invariant, cross-referenced in
        /// `docs/internals/invariants.md`.
        invariant: &'static str,
    },
}

impl FaultKind {
    /// Whether this fault indicates the task may have corrupted memory outside
    /// itself.
    ///
    /// Governs whether a restart is safe. A deadline miss is contained by
    /// definition; a memory fault on a board without protection hardware is
    /// not, and the supervisor escalates instead of restarting. Restarting a
    /// task that may have scribbled on a neighbour is theatre.
    #[must_use]
    pub const fn may_have_escaped(self, isolated: bool) -> bool {
        match self {
            Self::MemoryAccess { .. } | Self::StackOverflow => !isolated,
            Self::IllegalOperation => !isolated,
            Self::Arithmetic
            | Self::Panic { .. }
            | Self::DeadlineMiss { .. }
            | Self::WatchdogTimeout
            | Self::KernelInvariant { .. } => false,
        }
    }
}

/// A recorded fault.
///
/// Fixed size, no pointers into the failed task's memory, and safe to write to
/// flash from the fault handler.
#[derive(Debug, Clone, Copy)]
pub struct FaultRecord {
    /// Which task failed.
    pub task: TaskId,
    /// What happened.
    pub kind: FaultKind,
    /// When, on the monotonic timeline.
    pub at: Instant,
    /// Program counter at the fault, when recoverable.
    pub program_counter: Option<usize>,
    /// Stack pointer at the fault, for offline unwinding.
    pub stack_pointer: Option<usize>,
    /// How many times this task has faulted since boot.
    pub occurrence: u32,
}

/// The supervisor's decision after a fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Restart the task. Its stack is wiped, its channels are drained, and its
    /// peers are notified so they can reset their own view of the
    /// relationship — a restarted peer that is treated as if nothing happened
    /// is a rich source of second-order bugs.
    Restart,
    /// Leave the task stopped and continue without it.
    Stop,
    /// Enter degraded mode: stop the task, disable the subsystem it belongs to,
    /// and mark the system so that operators and telemetry can see it.
    Degrade {
        /// Which subsystem is being shed, for reporting.
        subsystem: &'static str,
    },
    /// Persist the dump and reset the system. The last resort, and always
    /// recorded, so a reboot loop is diagnosable from the field rather than
    /// merely observable.
    Reset,
}

/// Why the device last started.
///
/// Read at boot and reported in the health record. A device that cannot say why
/// it restarted cannot be debugged remotely, and remote debuggability is most
/// of what separates a product from a prototype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResetReason {
    /// Cold start from power-on.
    PowerOn,
    /// External reset pin.
    External,
    /// Hardware watchdog expired — nobody was left to record why.
    Watchdog,
    /// The kernel reset deliberately after a fault, with a dump preserved.
    FaultEscalation,
    /// Reset requested by the firmware-update process.
    SoftwareUpdate,
    /// The hardware reported a cause the port does not recognise.
    Unknown,
}

impl ResetReason {
    /// Whether this reason indicates something went wrong.
    #[must_use]
    pub const fn is_abnormal(self) -> bool {
        matches!(self, Self::Watchdog | Self::FaultEscalation | Self::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolation_determines_whether_a_memory_fault_is_contained() {
        let fault = FaultKind::MemoryAccess {
            address: Some(0xDEAD_0000),
            write: true,
        };
        assert!(
            !fault.may_have_escaped(true),
            "an isolated task's memory fault is contained by the MPU"
        );
        assert!(
            fault.may_have_escaped(false),
            "without protection hardware, a memory fault may have hit a neighbour"
        );
    }

    #[test]
    fn timing_and_logic_faults_are_always_contained() {
        for fault in [
            FaultKind::DeadlineMiss { overrun_ticks: 12 },
            FaultKind::WatchdogTimeout,
            FaultKind::Arithmetic,
            FaultKind::Panic { location: None },
        ] {
            assert!(
                !fault.may_have_escaped(false),
                "{fault:?} cannot corrupt a neighbour"
            );
            assert!(!fault.may_have_escaped(true));
        }
    }

    #[test]
    fn abnormal_reset_reasons_are_flagged() {
        assert!(ResetReason::Watchdog.is_abnormal());
        assert!(ResetReason::FaultEscalation.is_abnormal());
        assert!(ResetReason::Unknown.is_abnormal());
        assert!(!ResetReason::PowerOn.is_abnormal());
        assert!(!ResetReason::SoftwareUpdate.is_abnormal());
    }
}
