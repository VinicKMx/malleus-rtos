//! Tasks: identity, state, and the statically generated task table.
//!
//! Malleus has no `spawn`. The task set is closed at build time, generated from
//! `malleus.toml` into a `static` table. This is the single decision that makes
//! everything else in the system analysable: if the task set can change at
//! runtime, then worst-case memory, worst-case CPU utilisation, and
//! schedulability are all unknowable at build time, and the analyser could only
//! ever produce a guess. See `docs/adr/0002-static-system-definition.md`.

use crate::sched::Priority;

/// Maximum number of tasks in a system.
///
/// A hard ceiling, not a default. It bounds the size of every kernel table so
/// they can live in `.bss` with a known size. Systems that genuinely need more
/// than 64 tasks on a microcontroller are usually describing state machines as
/// tasks; the manifest linter says so and suggests the alternative.
pub const MAX_TASKS: usize = 64;

/// A task's identity.
///
/// Generated code produces one `const` `TaskId` per declared task, so task
/// references are resolved and type-checked at compile time. There is no
/// "look up a task by name" API at runtime — a name lookup that can fail is a
/// runtime error class that a static system does not need to have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(u8);

impl TaskId {
    /// Construct from a raw index.
    ///
    /// Returns `None` when out of range. Used by generated code and by the C
    /// FFI shim, where indices arrive from outside the type system.
    #[must_use]
    pub const fn new(index: u8) -> Option<Self> {
        if (index as usize) < MAX_TASKS {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Construct from a raw index in a `const` context.
    ///
    /// # Panics
    ///
    /// Panics if `index >= MAX_TASKS`, failing the build rather than the
    /// device.
    #[must_use]
    pub const fn new_const(index: u8) -> Self {
        assert!((index as usize) < MAX_TASKS, "task index out of range");
        Self(index)
    }

    /// Index into the kernel's task tables.
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// What a task is currently doing.
///
/// This enum is what the `cargo malleus inspect` view renders, and what a crash
/// dump records for every task at the moment of the fault. The distinction
/// between [`Blocked`](TaskState::Blocked) and [`Waiting`](TaskState::Waiting)
/// matters for diagnosis: blocked means "another task holds what I need", which
/// can indicate a priority-inversion problem; waiting means "the outside world
/// has not spoken yet", which cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TaskState {
    /// Executing on the CPU.
    Running,
    /// Runnable, waiting only for the CPU.
    Ready,
    /// Waiting on a kernel synchronisation object held by another task.
    Blocked,
    /// Waiting for time to pass, or for an external event such as an
    /// interrupt or an incoming message.
    Waiting,
    /// Suspended by the supervisor after a fault, awaiting a restart decision.
    Faulted,
    /// Permanently stopped. Reached when the supervision policy is
    /// `restart = "never"`, or the restart budget is exhausted.
    Stopped,
}

impl TaskState {
    /// Whether the scheduler would consider dispatching this task.
    #[must_use]
    pub const fn is_schedulable(self) -> bool {
        matches!(self, Self::Running | Self::Ready)
    }
}

/// What the supervisor does when a task faults.
///
/// Declared per task in the manifest. The analyser cross-checks it against the
/// board's isolation tier: `OnFault` on a board without memory protection is a
/// build error, because a task that can corrupt its neighbours cannot be
/// meaningfully restarted in isolation.
/// See `docs/design/05-fault-model.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    /// Never restart. The task stops and the system continues without it.
    /// Correct for a task whose partial operation would be more dangerous than
    /// its absence.
    Never,
    /// Restart on fault, up to `budget` times within `window_ticks`. Beyond
    /// that the task is stopped and the system enters degraded mode.
    OnFault {
        /// Maximum restarts allowed inside the window.
        budget: u8,
        /// Sliding window length, in kernel ticks.
        window_ticks: u64,
    },
    /// The task is essential: its fault escalates to a full system reset with a
    /// crash dump persisted first. Correct for a safety supervisor whose
    /// absence means the system cannot be trusted at all.
    EscalateToReset,
}

/// Immutable, build-time description of one task.
///
/// Generated into a `static` array by `malleus-codegen`. Everything here is
/// known before the device powers on, which is what lets the analyser reason
/// about the system as a whole.
#[derive(Debug, Clone, Copy)]
pub struct TaskDescriptor {
    /// Name as declared in the manifest. Used in traces, dumps, and tooling.
    pub name: &'static str,
    /// Scheduling priority.
    pub priority: Priority,
    /// Stack size in bytes, after alignment and protection-region rounding.
    pub stack_bytes: usize,
    /// Declared activation period in ticks, for periodic tasks.
    pub period_ticks: Option<u64>,
    /// Declared relative deadline in ticks. Defaults to the period when a
    /// period is declared and no deadline is given.
    pub deadline_ticks: Option<u64>,
    /// Declared worst-case execution time in ticks, if the engineer supplied
    /// one. The analyser needs this for response-time analysis; without it,
    /// schedulability is reported as `UNKNOWN` rather than `PASS`.
    pub wcet_ticks: Option<u64>,
    /// Supervision policy.
    pub restart: RestartPolicy,
    /// Whether the task runs unprivileged behind memory protection.
    pub isolated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_ids_are_bounded() {
        assert!(TaskId::new(0).is_some());
        assert!(TaskId::new(MAX_TASKS as u8 - 1).is_some());
        assert_eq!(TaskId::new(MAX_TASKS as u8), None);
    }

    #[test]
    fn only_running_and_ready_are_schedulable() {
        assert!(TaskState::Running.is_schedulable());
        assert!(TaskState::Ready.is_schedulable());
        for state in [
            TaskState::Blocked,
            TaskState::Waiting,
            TaskState::Faulted,
            TaskState::Stopped,
        ] {
            assert!(!state.is_schedulable(), "{state:?} must not be schedulable");
        }
    }
}
