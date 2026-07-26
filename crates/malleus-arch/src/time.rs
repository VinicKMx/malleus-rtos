//! Monotonic time and one-shot alarms.
//!
//! Malleus keeps time in **ticks of a fixed, board-declared rate**, not in
//! nanoseconds, and stores them in a `u64`. The reasoning is in
//! `docs/adr/0008-time-and-tickless-idle.md`; the short version is that
//! converting to nanoseconds on a Cortex-M without a divider costs more than
//! the tick abstraction saves, and a 64-bit tick counter at 1 MHz does not wrap
//! for 584,000 years.

/// A point on the kernel's monotonic timeline.
///
/// Monotonic means: never runs backwards, never jumps, unaffected by wall-clock
/// adjustments, and keeps counting across tickless idle. It resets only on
/// reboot — and the reboot is recorded, so a host tool can stitch timelines
/// across a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant(u64);

impl Instant {
    /// The instant the kernel started counting.
    pub const ZERO: Self = Self(0);

    /// Construct from a raw tick count.
    #[must_use]
    pub const fn from_ticks(ticks: u64) -> Self {
        Self(ticks)
    }

    /// Raw tick count since boot.
    #[must_use]
    pub const fn as_ticks(self) -> u64 {
        self.0
    }

    /// Ticks elapsed from `earlier` to `self`, saturating at zero.
    ///
    /// Saturating rather than wrapping is deliberate: a negative duration in a
    /// deadline calculation is always a bug, and saturating turns it into a
    /// visible "zero time left" instead of a 584,000-year timeout.
    #[must_use]
    pub const fn saturating_since(self, earlier: Self) -> u64 {
        self.0.saturating_sub(earlier.0)
    }

    /// `self` advanced by `ticks`, saturating at the end of the timeline.
    #[must_use]
    pub const fn saturating_add_ticks(self, ticks: u64) -> Self {
        Self(self.0.saturating_add(ticks))
    }
}

/// Tick frequency of a board's monotonic timer, in hertz.
///
/// Declared by the board support crate and validated at build time: the
/// analyser rejects a configuration whose shortest declared task period is not
/// an exact multiple of the tick period, because a non-integer ratio produces
/// systematic jitter that is very hard to diagnose in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickRate(u32);

impl TickRate {
    /// Construct a tick rate.
    ///
    /// # Panics
    ///
    /// Panics if `hz` is zero. This is a const-evaluable panic in board
    /// configuration, so it fails the build, never the device.
    #[must_use]
    pub const fn from_hz(hz: u32) -> Self {
        assert!(hz > 0, "tick rate must be non-zero");
        Self(hz)
    }

    /// Frequency in hertz.
    #[must_use]
    pub const fn as_hz(self) -> u32 {
        self.0
    }

    /// Convert microseconds to ticks, rounding up.
    ///
    /// Rounds up so that a requested delay is never *shorter* than asked. For
    /// a deadline, being early is a correctness bug; being one tick late is a
    /// rounding artefact the analyser accounts for.
    #[must_use]
    pub const fn ticks_from_micros(self, micros: u64) -> u64 {
        let hz = self.0 as u64;
        micros.saturating_mul(hz).div_ceil(1_000_000)
    }
}

/// A monotonic clock with a programmable one-shot alarm.
///
/// # Safety
///
/// The kernel's entire notion of time rests on this. An implementation that
/// loses ticks across tickless idle, or that lets `now()` run backwards under
/// concurrent access from an ISR, breaks every timing guarantee the system
/// makes and does so intermittently.
pub unsafe trait MonotonicTimer {
    /// This timer's tick rate.
    fn tick_rate() -> TickRate;

    /// Current time.
    ///
    /// # Contract
    ///
    /// - O(1), lock-free, safe from any context including ISRs.
    /// - Monotonic under concurrent calls: two calls never observe time
    ///   moving backwards, even if the hardware counter wraps between them.
    fn now() -> Instant;

    /// Schedule a wake-up at `deadline`.
    ///
    /// A deadline in the past must fire immediately rather than being lost —
    /// this is the classic tickless-idle race and the conformance suite tests
    /// it explicitly.
    ///
    /// # Contract
    ///
    /// - O(1). Replaces any previously programmed alarm.
    ///
    /// # Errors
    ///
    /// [`crate::ArchError::DeadlineOutOfRange`] if the deadline exceeds the
    /// hardware's maximum programmable interval. The kernel handles this by
    /// programming an intermediate wake-up and re-arming, so it is a hint, not
    /// a failure.
    fn set_alarm(deadline: Instant) -> Result<(), crate::ArchError>;

    /// Cancel a pending alarm. Idempotent.
    fn clear_alarm();

    /// Longest interval this timer can be programmed for, in ticks.
    ///
    /// Used by the tickless-idle logic to decide how deep a sleep it can enter
    /// in one step.
    fn max_alarm_ticks() -> u64;
}
