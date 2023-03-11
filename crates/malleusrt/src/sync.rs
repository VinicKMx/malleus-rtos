//! Synchronisation primitives with declared, bounded blocking behaviour.
//!
//! # The contract block
//!
//! Every primitive in this module carries a machine-readable contract in its
//! documentation:
//!
//! ```text
//! Mutex::lock()
//!   complexity:   O(waiters)
//!   allocates:    no
//!   blocks:       yes
//!   isr-safe:     no
//!   timeout:      yes
//!   inheritance:  priority
//! ```
//!
//! These blocks are extracted by `cargo malleus analyze` and cross-checked
//! against the call sites in your tasks. Calling a blocking operation from a
//! task declared as hard real-time, without a bounded timeout, is a build
//! error — not a code-review comment somebody might miss.
//! See `docs/design/04-realtime-model.md`.

/// How a mutex bounds priority inversion.
///
/// Both options are offered because they are genuinely different trade-offs,
/// and an RTOS that picks one silently is making a systems decision on the
/// engineer's behalf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InversionControl {
    /// Priority inheritance: a task holding the mutex is temporarily raised to
    /// the priority of the highest-priority waiter.
    ///
    /// Cheap when uncontended, requires no declaration, and bounds inversion to
    /// the length of the critical section. It does not prevent deadlock and
    /// does not bound *chained* blocking as tightly as ceiling does.
    Inheritance,
    /// Immediate priority ceiling: a task acquiring the mutex is raised at once
    /// to the declared ceiling — the highest priority of any task that may
    /// acquire it.
    ///
    /// Bounds each task to at most one block per activation, prevents deadlock
    /// among ceiling-protected mutexes outright, and makes response-time
    /// analysis tractable. The cost is that the ceiling must be declared, and
    /// the analyser verifies it against the actual set of users found in the
    /// manifest.
    Ceiling {
        /// The declared ceiling priority.
        ceiling: crate::sched::Priority,
    },
}

/// Why a task was woken from a blocking wait.
///
/// Returned rather than folded into an error type, because "the thing I waited
/// for happened" and "I gave up waiting" are both normal outcomes that a
/// control loop must distinguish, and `Result` invites treating one as
/// exceptional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeReason {
    /// The awaited condition became true.
    Signalled,
    /// The timeout expired first.
    Expired,
    /// The wait was cancelled — the future was dropped, or the supervisor is
    /// tearing the task down.
    Cancelled,
    /// The task holding the resource faulted. The resource is in an unknown
    /// state and the waiter must decide what to do, exactly as a poisoned lock
    /// forces a decision in `std`.
    HolderFaulted,
}

/// Capacity of a bounded synchronisation object.
///
/// A newtype rather than a bare `usize`, so that "how many can it hold" cannot
/// be swapped with "how many does it hold" at a call site. Zero is rejected:
/// a zero-capacity channel is a rendezvous, which has different blocking
/// semantics and is a separate type rather than a degenerate case of this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capacity(u16);

impl Capacity {
    /// Construct a capacity.
    ///
    /// # Panics
    ///
    /// Panics if `n` is zero. Only reachable from `const` generated code, so
    /// this fails the build.
    #[must_use]
    pub const fn new(n: u16) -> Self {
        assert!(
            n > 0,
            "capacity must be non-zero; use a Rendezvous for unbuffered handoff"
        );
        Self(n)
    }

    /// The capacity as a count.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0 as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sched::Priority;

    #[test]
    fn ceiling_carries_the_declared_priority() {
        let control = InversionControl::Ceiling {
            ceiling: Priority::new_const(9),
        };
        match control {
            InversionControl::Ceiling { ceiling } => assert_eq!(ceiling.level(), 9),
            InversionControl::Inheritance => panic!("wrong variant"),
        }
    }

    #[test]
    fn capacity_is_a_count() {
        assert_eq!(Capacity::new(8).get(), 8);
    }
}
