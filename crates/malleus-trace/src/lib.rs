//! Wire format for trace streams and crash dumps.
//!
//! # Status: format defined, encoder not yet implemented — Checkpoint 2
//!
//! # Why the format is its own crate
//!
//! Because both ends need it and they are built for different machines: the
//! device encodes (`no_std`, no allocation, inside a fault handler), the host
//! decodes (`std`, whenever it likes). Putting the format in a shared crate
//! makes it impossible for the two to drift, and makes it possible for someone
//! else to write a decoder — a wire format only one tool can read is a captive
//! format, and captive formats are how observability data ends up unused.
//!
//! # Constraints the format has to satisfy
//!
//! - **Encodable from a fault handler.** No allocation, no locks, no calls back
//!   into the kernel. The system is already broken when this runs.
//! - **Bounded cost per event.** Tracing that perturbs timing tells you about a
//!   system that no longer exists. Every event has a fixed, documented cost,
//!   published alongside the benchmarks.
//! - **Lossy under overload, and honest about it.** When the buffer fills,
//!   events are dropped and the gap is *recorded*. A trace with a silent hole
//!   is worse than one with a hole labelled "127 events lost here".
//! - **Self-describing.** A stream carries the architecture, tick rate, and
//!   task table it was produced with, so a dump from a device in the field is
//!   readable without reconstructing the exact build.
//!
//! # Event categories
//!
//! | Category | Examples |
//! |----------|----------|
//! | Scheduling | switch, ready, block, preempt |
//! | Timing | activation, deadline met, deadline missed |
//! | Synchronisation | lock acquire/release, priority inheritance applied |
//! | IPC | send, receive, queue depth, overflow drop |
//! | Interrupts | entry, exit, latency sample |
//! | Faults | fault raised, supervisor decision, task restart |
//! | Power | idle entry/exit, sleep depth |

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

/// Wire format version.
///
/// Bumped on any incompatible change. The host decoder refuses a stream whose
/// version it does not know, rather than misinterpreting it — a misdecoded
/// trace is worse than no trace, because it looks like data.
pub const FORMAT_VERSION: u16 = 1;

/// Magic bytes at the start of every stream and dump: `MLRT`.
pub const MAGIC: [u8; 4] = *b"MLRT";

/// What kind of artefact a stream is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StreamKind {
    /// A live trace, streamed while the system runs.
    Trace = 1,
    /// A post-mortem crash dump, written once at fault time.
    Dump = 2,
    /// A periodic health record: uptime, reset reason, task statistics.
    Health = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_identifies_the_format() {
        assert_eq!(&MAGIC, b"MLRT");
    }

    #[test]
    fn stream_kinds_have_stable_discriminants() {
        // These go on the wire. Changing one silently reinterprets every
        // artefact ever produced, so the values are pinned by test.
        assert_eq!(StreamKind::Trace as u8, 1);
        assert_eq!(StreamKind::Dump as u8, 2);
        assert_eq!(StreamKind::Health as u8, 3);
    }
}
