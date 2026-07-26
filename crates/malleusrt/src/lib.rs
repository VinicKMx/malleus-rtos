//! **MalleusRT** — the real-time kernel of Malleus RTOS.
//!
//! Malleus RTOS is the whole platform: kernel, runtime, build-time analyser,
//! host tooling, board support. MalleusRT is the kernel proper — the part that
//! schedules, switches, protects, and reports. Keeping the two names distinct
//! is not vanity: most of what makes the platform worth using lives *outside*
//! the kernel, and a name that blurs them invites the usual mistake of judging
//! an RTOS solely by its scheduler.
//!
//! # What this crate promises
//!
//! Every public operation in this crate documents its time complexity, whether
//! it allocates, whether it can block, and whether it is callable from an
//! interrupt handler. That is not a documentation convention — it is the
//! product. An RTOS whose API hides a potentially unbounded operation behind a
//! pleasant signature is not usable for hard real-time work, however elegant it
//! reads. See `docs/design/04-realtime-model.md`.
//!
//! # What this crate does not do
//!
//! - It does not allocate. There is no allocator, and `alloc` is not a
//!   dependency. Every object the kernel manages is placed by the build-time
//!   code generator into statically sized storage.
//!   See `docs/adr/0003-no-kernel-heap.md`.
//! - It does not create tasks at runtime. The task set is fixed at build time
//!   and derived from `malleus.toml`.
//!   See `docs/adr/0002-static-system-definition.md`.
//! - It does not contain drivers. Drivers are ordinary tasks or libraries
//!   outside the kernel, holding capabilities granted by the manifest.
//!   See `docs/adr/0005-memory-isolation.md`.
//!
//! # Module map
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`sched`] | Fixed-priority preemptive scheduling, O(1) ready-set |
//! | [`task`]  | Task control blocks, states, and the static task table |
//! | [`time`]  | Deadlines, delays, and the timer wheel |
//! | [`sync`]  | Mutexes with priority inheritance, semaphores, event flags |
//! | [`ipc`]   | Bounded, typed message channels between protection domains |
//! | [`fault`] | Fault capture, supervision policy, crash dumps |

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
// The kernel's arithmetic must not panic and its indexing must not be
// unchecked. A `+` that can overflow inside a scheduler is a system that stops
// scheduling; the compiler is a cheaper place to catch that than a field
// return. Every deviation is a local `#[allow]` with a written justification.
#![deny(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

pub mod fault;
pub mod ipc;
pub mod sched;
pub mod sync;
pub mod task;
pub mod time;

pub use sched::Priority;
pub use task::TaskId;

/// Errors returned by kernel operations.
///
/// The kernel never panics on a recoverable condition and never returns a
/// heap-allocated error. This enum is `Copy` and fits in a register so it can
/// cross the syscall boundary and be stored in a crash dump unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The operation would block, and the caller asked not to block.
    WouldBlock,
    /// A bounded wait expired before the operation could complete.
    TimedOut,
    /// The operation is not permitted from an interrupt handler.
    NotFromInterrupt,
    /// The target queue, channel, or pool is full.
    Full,
    /// The target is empty.
    Empty,
    /// The caller does not hold the capability this resource requires.
    ///
    /// In a correct build this is unreachable: capabilities are checked at
    /// compile time by generated code. It exists for the C FFI boundary, where
    /// the compiler cannot enforce the check.
    PermissionDenied,
    /// The addressed task exists but is not in a state that accepts this
    /// operation — typically it has faulted and is awaiting supervision.
    PeerUnavailable,
    /// The operation was cancelled, e.g. a future was dropped while pending.
    Cancelled,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::WouldBlock => "operation would block",
            Self::TimedOut => "operation timed out",
            Self::NotFromInterrupt => "operation not permitted from an interrupt handler",
            Self::Full => "destination full",
            Self::Empty => "source empty",
            Self::PermissionDenied => "missing capability",
            Self::PeerUnavailable => "peer task unavailable",
            Self::Cancelled => "operation cancelled",
        };
        f.write_str(s)
    }
}

impl core::error::Error for Error {}

/// A kernel result.
pub type Result<T> = core::result::Result<T, Error>;
