//! Architecture abstraction contract for the Malleus RTOS kernel.
//!
//! The kernel is written once, against the traits in this crate. Every
//! supported architecture provides one implementation. This is the *entire*
//! surface a new port has to satisfy — if a port compiles against these traits
//! and passes the conformance suite in [`conformance`], the kernel runs on it.
//!
//! # Design rule
//!
//! This crate contains **no** `#[cfg(target_arch)]`. It is pure contract, and
//! it compiles on the host so that the kernel's logic can be unit tested
//! against a simulated architecture. Anything that needs to know what CPU it is
//! running on belongs in an `malleus-arch-*` crate instead.
//!
//! # Porting guide
//!
//! See `docs/internals/porting.md` for the full walkthrough. In short, a port
//! implements [`Arch`], which bundles four independent concerns:
//!
//! - [`Arch::Context`]  — saving and restoring a task's CPU state,
//! - [`Arch::Critical`] — bounded-time mutual exclusion against interrupts,
//! - [`Arch::Timer`]    — a monotonic clock and a programmable one-shot alarm,
//! - [`Arch::Memory`]   — optional hardware memory protection.
//!
//! Ports without memory protection use [`memory::NoProtection`] and lose fault
//! containment, but nothing else. That degradation is explicit and reported by
//! `cargo malleus analyze`, never silent.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
// Same rule as the kernel: this crate runs on the context-switch and fault
// paths, where a panic has nowhere to go.
#![deny(clippy::arithmetic_side_effects, clippy::indexing_slicing)]

pub mod conformance;
pub mod context;
pub mod critical;
pub mod memory;
pub mod time;

pub use context::Context;
pub use critical::CriticalSection;
pub use memory::{MemoryProtection, Permissions, Region};
pub use time::{Instant, MonotonicTimer, TickRate};

/// The complete set of capabilities the kernel requires from an architecture.
///
/// # Safety
///
/// Implementations drive context switching and memory protection. An incorrect
/// implementation is unsound in ways the kernel cannot detect or defend
/// against. Implementors must uphold every contract documented on the
/// associated types, and must pass [`conformance`] on real hardware — not only
/// in simulation — before the port is declared supported.
pub unsafe trait Arch {
    /// Human-readable port name, e.g. `"cortex-m7"`. Recorded in crash dumps
    /// and trace streams so a host tool can decode them without guessing.
    const NAME: &'static str;

    /// Natural stack alignment in bytes. Stack allocations are rounded up to
    /// this. ARM AAPCS requires 8; some ABIs with vector state require 16.
    const STACK_ALIGN: usize;

    /// Whether the stack grows toward lower addresses. Every architecture
    /// Malleus targets today answers `true`, but the stack-usage analyser reads
    /// this rather than assuming.
    const STACK_GROWS_DOWN: bool;

    /// Number of distinct hardware interrupt priority levels usable by tasks.
    ///
    /// The scheduler maps its logical priorities onto these. When the kernel
    /// has more logical priorities than the hardware offers, the mapping is
    /// computed at build time and reported by the analyser.
    const PRIORITY_LEVELS: u8;

    /// Per-task CPU state.
    type Context: Context;
    /// Interrupt-masking primitive used for kernel critical sections.
    type Critical: CriticalSection;
    /// Monotonic time source and one-shot alarm.
    type Timer: MonotonicTimer;
    /// Hardware memory protection, or [`memory::NoProtection`].
    type Memory: MemoryProtection;
}

/// Errors an architecture layer can report to the kernel.
///
/// Deliberately small and non-allocating: the kernel maps these onto fault
/// records, and a crash dump must be able to carry one in a fixed-size field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArchError {
    /// A memory region could not be programmed: bad alignment, bad size, or
    /// more regions requested than the hardware provides.
    UnsupportedRegion,
    /// The requested alarm deadline is further out than the timer can express.
    DeadlineOutOfRange,
    /// The hardware is not in a state where this operation is legal, e.g.
    /// programming protection regions before the unit was enabled.
    InvalidState,
}

impl core::fmt::Display for ArchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::UnsupportedRegion => "memory region not representable by this MPU",
            Self::DeadlineOutOfRange => "alarm deadline outside timer range",
            Self::InvalidState => "operation illegal in the current hardware state",
        };
        f.write_str(s)
    }
}
