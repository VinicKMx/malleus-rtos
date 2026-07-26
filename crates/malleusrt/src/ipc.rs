//! Typed, bounded inter-task messaging.
//!
//! # Why IPC is a kernel concern
//!
//! Once tasks are isolated behind memory protection, they cannot share a
//! pointer. Everything that crosses a protection boundary has to go through the
//! kernel, which makes the message channel the load-bearing abstraction of the
//! whole system rather than a convenience.
//!
//! Malleus therefore does not offer a `send(task_id, &[u8])` primitive as its
//! public surface. Channels are declared in `malleus.toml` and generated into
//! typed endpoints:
//!
//! ```ignore
//! // Generated from the manifest. `motor` exists only in tasks that declared
//! // the `ipc.motor-command` capability — an undeclared task cannot even name
//! // it, so the check is a compile error rather than a runtime denial.
//! motor.send(SetSpeed { rpm: 1_500 }, Timeout::Ticks(10)).await?;
//! ```
//!
//! The generated endpoint enforces, at compile time: who may talk to whom,
//! which message types are legal on the channel, the maximum message size, and
//! the overflow policy. It enforces at runtime only what genuinely cannot be
//! known statically: whether the receiver is currently alive.
//! See `docs/design/08-ipc.md`.

use crate::sync::Capacity;

/// What happens when a bounded channel is full and a sender arrives.
///
/// There is no default. Every channel declares one, because the right answer is
/// entirely application-specific and the wrong answer is invisible until the
/// day the system is under load — which is the day it matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Block the sender until space appears, subject to its timeout.
    ///
    /// Correct for a command path where every message matters. Dangerous on a
    /// hard real-time sender: the analyser requires such a sender to supply a
    /// bounded timeout, and rejects `Timeout::Forever`.
    Block,
    /// Fail immediately with [`crate::Error::Full`].
    ///
    /// Correct when the sender has something better to do than wait, and can
    /// meaningfully report or retry.
    Reject,
    /// Discard the oldest queued message to make room.
    ///
    /// Correct for telemetry and sampled state, where fresh data supersedes
    /// stale data. Wrong for commands, where dropping the oldest silently
    /// reorders intent. The manifest linter warns when this policy is used on a
    /// channel whose message type is named like a command.
    DropOldest,
    /// Discard the incoming message.
    ///
    /// Correct for logging, where the first messages after an event are usually
    /// the informative ones.
    DropNewest,
}

impl Overflow {
    /// Whether this policy can make a sender block.
    ///
    /// Used by the analyser to decide whether a send on this channel
    /// contributes to a task's worst-case blocking time.
    #[must_use]
    pub const fn can_block(self) -> bool {
        matches!(self, Self::Block)
    }

    /// Whether this policy can lose messages.
    ///
    /// Surfaced in the generated architecture documentation, so that a lossy
    /// path is visible on the IPC graph rather than buried in a config file.
    #[must_use]
    pub const fn is_lossy(self) -> bool {
        matches!(self, Self::DropOldest | Self::DropNewest)
    }
}

/// Build-time description of one channel, generated from the manifest.
#[derive(Debug, Clone, Copy)]
pub struct ChannelDescriptor {
    /// Channel name as declared, e.g. `"motor-command"`.
    pub name: &'static str,
    /// Queue depth.
    pub capacity: Capacity,
    /// Maximum encoded message size in bytes.
    ///
    /// Fixed at build time so the backing storage is a `static` array. Message
    /// types larger than this fail to compile against the generated endpoint.
    pub max_message_bytes: u16,
    /// Behaviour when full.
    pub overflow: Overflow,
    /// Sending task.
    pub sender: crate::TaskId,
    /// Receiving task.
    pub receiver: crate::TaskId,
}

impl ChannelDescriptor {
    /// Static RAM this channel consumes, in bytes.
    ///
    /// Reported per channel in the memory report from `cargo malleus analyze`.
    /// A generously sized channel is one of the easiest ways to run out of RAM
    /// on a microcontroller, and the cheapest place to find the waste is a
    /// build-time table rather than a debugger session.
    #[must_use]
    pub const fn static_bytes(&self) -> usize {
        self.capacity
            .get()
            .saturating_mul(self.max_message_bytes as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_block_can_block() {
        assert!(Overflow::Block.can_block());
        for policy in [Overflow::Reject, Overflow::DropOldest, Overflow::DropNewest] {
            assert!(!policy.can_block(), "{policy:?} must not block a sender");
        }
    }

    #[test]
    fn dropping_policies_are_marked_lossy() {
        assert!(Overflow::DropOldest.is_lossy());
        assert!(Overflow::DropNewest.is_lossy());
        assert!(!Overflow::Block.is_lossy());
        // Reject is not lossy: the sender is told, and owns the decision.
        assert!(!Overflow::Reject.is_lossy());
    }

    #[test]
    fn static_footprint_is_capacity_times_message_size() {
        let channel = ChannelDescriptor {
            name: "sensor-data",
            capacity: Capacity::new(16),
            max_message_bytes: 32,
            overflow: Overflow::DropOldest,
            sender: crate::TaskId::new_const(1),
            receiver: crate::TaskId::new_const(2),
        };
        assert_eq!(channel.static_bytes(), 512);
    }
}
