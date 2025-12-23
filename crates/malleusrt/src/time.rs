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
            // Round *up*: we need the smallest whole number of periods that
            // carries `next` to at least `now`. The previous
            // `floor(behind / interval) + 1` added a period that was already
            // accounted for whenever `behind` was an exact multiple, so a task
            // finishing precisely on a grid point was charged one phantom miss
            // and lost a real activation slot. `now` exactly on an activation
            // instant means that activation is due, not missed — the same rule
            // the `now == next` path above already applies.
            //
            // A zero interval is rejected by the manifest validator (M0026),
            // but `div_ceil` panics on a zero divisor, so the kernel still
            // guards it rather than trusting an upstream check at runtime.
            let skipped = if self.interval == 0 {
                1
            } else {
                behind.div_ceil(self.interval)
            };
            // Saturate the width conversion rather than casting it. `skipped`
            // is a `u64`; `as u32` keeps only the low word, so an overrun of
            // exactly 2^32 activations reported *zero* missed — the counter
            // silently reading healthy at the precise moment it matters most.
            let skipped_count = if skipped > u32::MAX as u64 {
                u32::MAX
            } else {
                skipped as u32
            };
            self.missed = self.missed.saturating_add(skipped_count);
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

    /// The counter must saturate, never wrap. `skipped as u32` discarded the
    /// high word, so an overrun of exactly 2^32 activations reported zero
    /// misses — the one reading that would stop anyone investigating.
    #[test]
    fn a_huge_overrun_saturates_the_miss_counter_instead_of_wrapping() {
        let mut period = Period::starting_at(Instant::ZERO, 1);
        // Chosen so the skip count is an exact multiple of 2^32, whose low
        // word is zero.
        period.advance(Instant::from_ticks((1u64 << 32) + 2));
        assert_eq!(
            period.missed_activations(),
            u32::MAX,
            "a four-billion-activation overrun must saturate, not wrap to a small number"
        );
    }

    /// Finishing exactly on an activation instant must be treated the same way
    /// whether the task is on time or several periods late. The old
    /// `floor + 1` charged a phantom miss and skipped a real activation slot
    /// at every exact grid point, so `missed` drifted upward on precisely the
    /// tick-aligned periods the validator encourages.
    #[test]
    fn an_overrun_landing_on_the_grid_is_not_charged_a_phantom_miss() {
        // On time, finishing exactly at the next activation: due now.
        let mut on_time = Period::starting_at(Instant::ZERO, 10);
        assert_eq!(on_time.advance(Instant::from_ticks(20)).as_ticks(), 20);
        assert_eq!(on_time.missed_activations(), 0);

        // One full period late, again exactly on the grid. Activation 20 was
        // missed; activation 30 is due now, not missed as well.
        let mut late = Period::starting_at(Instant::ZERO, 10);
        assert_eq!(late.advance(Instant::from_ticks(30)).as_ticks(), 30);
        assert_eq!(
            late.missed_activations(),
            1,
            "the activation due at `now` must not be counted as missed"
        );

        // Two full periods late.
        let mut later = Period::starting_at(Instant::ZERO, 10);
        assert_eq!(later.advance(Instant::from_ticks(40)).as_ticks(), 40);
        assert_eq!(later.missed_activations(), 2);
    }

    /// Just past a grid point is a genuine miss, and must stay one.
    #[test]
    fn an_overrun_just_past_the_grid_still_counts() {
        let mut period = Period::starting_at(Instant::ZERO, 10);
        assert_eq!(period.advance(Instant::from_ticks(21)).as_ticks(), 30);
        assert_eq!(period.missed_activations(), 1);
    }

    #[test]
    fn catch_up_is_arithmetic_not_iterative() {
        // A one-million-period overrun must resolve in constant time and land
        // in the future. If this ever hangs, `advance` has grown a loop.
        let mut period = Period::starting_at(Instant::ZERO, 10);
        let now = Instant::from_ticks(10_000_000);
        let next = period.advance(now);
        // `>=`, not `>`: 10_000_000 is itself on the activation grid, so the
        // correct answer is "due now", not "due one period from now". The
        // requirement is that `advance` never returns an instant in the past.
        assert!(next.as_ticks() >= now.as_ticks());
        assert!(period.missed_activations() > 0);
    }
}
