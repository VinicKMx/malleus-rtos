//! Deadlines, delays, and periodic activation.

pub use malleus_arch::{Instant, TickRate};

/// How long an operation may wait.
///
/// Every blocking operation in Malleus takes one of these. There is no
/// "wait forever" default: an unbounded wait must be spelled
/// [`Timeout::Forever`] at the call site, so that a code reviewer can see it.
/// Most field hangs in embedded systems are an unbounded wait that nobody
/// noticed was unbounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeout {
    /// Do not block. Returns [`crate::Error::WouldBlock`] instead.
    None,
    /// Block for at most this many ticks.
    Ticks(u64),
    /// Block until an absolute instant.
    Until(Instant),
    /// Block indefinitely. Deliberately verbose.
    Forever,
}

impl Timeout {
    /// Resolve to an absolute deadline given the current time.
    ///
    /// # Contract
    ///
    /// O(1), no allocation, ISR-safe.
    #[must_use]
    pub const fn deadline_from(self, now: Instant) -> Option<Instant> {
        match self {
            Self::None => Some(now),
            Self::Ticks(t) => Some(now.saturating_add_ticks(t)),
            Self::Until(i) => Some(i),
            Self::Forever => None,
        }
    }

    /// Whether this timeout forbids blocking at all.
    #[must_use]
    pub const fn is_immediate(self) -> bool {
        matches!(self, Self::None)
    }
}

/// Tracks the activation instants of a periodic task without drift.
///
/// The naive `sleep(period)` loop accumulates every tick of execution time and
/// every scheduling delay into permanent phase drift. `Period` advances an
/// absolute reference instead, so a task activated late still targets the
/// original grid and the drift does not compound.
///
/// If the task is so late that it has missed an entire period, that is a
/// deadline miss and the kernel reports it rather than silently skipping. A
/// missed activation is information; swallowing it is how a control loop
/// quietly degrades for months before anyone notices.
#[derive(Debug, Clone, Copy)]
pub struct Period {
    next: Instant,
    interval: u64,
    missed: u32,
}

impl Period {
    /// Start a periodic schedule of `interval_ticks`, with the first activation
    /// one interval after `start`.
    #[must_use]
    pub const fn starting_at(start: Instant, interval_ticks: u64) -> Self {
        Self {
            next: start.saturating_add_ticks(interval_ticks),
            interval: interval_ticks,
            missed: 0,
        }
    }

    /// The instant of the next scheduled activation.
    #[must_use]
    pub const fn next_activation(&self) -> Instant {
        self.next
    }

    /// Number of activations missed since this schedule began.
    #[must_use]
    pub const fn missed_activations(&self) -> u32 {
        self.missed
    }

    /// Advance to the next activation given that the current one completed at
    /// `now`, returning the new target.
    ///
    /// Skipped activations are counted, not hidden.
    ///
    /// # Contract
    ///
    /// O(1) — the catch-up is computed arithmetically, never by looping over
    /// missed periods. A loop here would make a badly overrunning task take
    /// unbounded time inside the kernel, which is exactly when you can least
    /// afford it.
    pub const fn advance(&mut self, now: Instant) -> Instant {
        self.next = self.next.saturating_add_ticks(self.interval);
        if now.as_ticks() > self.next.as_ticks() {
            let behind = now.saturating_since(self.next);
            // `checked_div` rather than `/`: a zero interval is a degenerate
            // configuration the manifest validator rejects, but the kernel
            // still must not divide by it at runtime.
            let skipped = match behind.checked_div(self.interval) {
                Some(periods) => periods.saturating_add(1),
                None => 1,
            };
            self.missed = self.missed.saturating_add(skipped as u32);
            self.next = self
                .next
                .saturating_add_ticks(skipped.saturating_mul(self.interval));
        }
        self.next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_none_yields_an_already_expired_deadline() {
        let now = Instant::from_ticks(100);
        assert_eq!(Timeout::None.deadline_from(now), Some(now));
        assert!(Timeout::None.is_immediate());
    }

    #[test]
    fn timeout_forever_has_no_deadline() {
        assert_eq!(Timeout::Forever.deadline_from(Instant::ZERO), None);
        assert!(!Timeout::Forever.is_immediate());
    }

    #[test]
    fn periodic_activation_does_not_drift() {
        // A task that consistently takes 3 ticks of a 10-tick period must still
        // activate on the original grid: 10, 20, 30 — never 13, 26, 39.
        let mut period = Period::starting_at(Instant::ZERO, 10);
        assert_eq!(period.next_activation(), Instant::from_ticks(10));

        let mut now = Instant::from_ticks(13);
        assert_eq!(period.advance(now), Instant::from_ticks(20));

        now = Instant::from_ticks(23);
        assert_eq!(period.advance(now), Instant::from_ticks(30));

        assert_eq!(
            period.missed_activations(),
            0,
            "on-time work must not count as missed"
        );
    }

    #[test]
    fn overrun_past_a_full_period_is_counted_not_hidden() {
        let mut period = Period::starting_at(Instant::ZERO, 10);
        // The task blew through activations at 10 and 20, finishing at 25.
        let next = period.advance(Instant::from_ticks(25));
        assert!(
            next.as_ticks() > 25,
            "next activation must be in the future"
        );
        assert_eq!(period.missed_activations(), 1);
    }

    #[test]
    fn catch_up_is_arithmetic_not_iterative() {
        // A one-million-period overrun must resolve in constant time and land
        // in the future. If this ever hangs, `advance` has grown a loop.
        let mut period = Period::starting_at(Instant::ZERO, 10);
        let now = Instant::from_ticks(10_000_000);
        let next = period.advance(now);
        assert!(next.as_ticks() > now.as_ticks());
        assert!(period.missed_activations() > 0);
    }
}
