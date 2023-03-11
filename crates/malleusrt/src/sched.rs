//! Fixed-priority preemptive scheduling.
//!
//! # Policy
//!
//! Strictly preemptive, fixed priority, FIFO within a priority level. The
//! highest-priority runnable task runs. Always. There is no fair-share, no
//! ageing, and no anti-starvation heuristic — starving a low-priority task is
//! the *correct* behaviour when a high-priority one is runnable, and hiding
//! that with a heuristic destroys analysability.
//!
//! Priorities are assigned by the engineer in `malleus.toml` and checked by
//! `cargo malleus analyze` against a response-time analysis. The analyser will
//! tell you that your priority assignment misses a deadline; it will not
//! silently fix it at runtime. See `docs/adr/0004-scheduling-policy.md`.
//!
//! # Why a bitmap
//!
//! Finding the highest-priority runnable task must be O(1) with a small,
//! *constant* cost — not amortised, not average-case. A bitmap of ready
//! priorities plus a count-leading-zeros instruction gives exactly that: on
//! Cortex-M, `CLZ` is a single cycle, so the lookup is a handful of cycles
//! regardless of how many tasks exist.

use core::num::NonZeroU32;

/// Number of distinct scheduling priorities.
///
/// 32 fits the ready-set in one word, which keeps the scheduler lookup to a
/// single `CLZ`. Widening this to 64 costs a second word and a branch on every
/// scheduling decision; no reference application has needed it. Revisit with
/// data, not with taste.
pub const PRIORITY_LEVELS: usize = 32;

/// A scheduling priority. Higher numeric value means more urgent.
///
/// Note the direction: Malleus uses "higher number wins", matching FreeRTOS and
/// ordinary intuition, and inverts internally when programming hardware
/// interrupt priorities, where ARM uses "lower number wins". Getting this
/// backwards is a classic source of field failures, so the conversion happens
/// in exactly one place: [`Priority::to_hardware`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    /// The idle priority. Reserved for the kernel's idle task; the analyser
    /// rejects an application task declared at this priority.
    pub const IDLE: Self = Self(0);
    /// The most urgent priority available to application tasks.
    pub const MAX: Self = Self(PRIORITY_LEVELS as u8 - 1);

    /// Construct a priority.
    ///
    /// Returns `None` if `level` is out of range, so a bad value from a
    /// generated table or the C FFI is a handled error rather than a panic.
    #[must_use]
    pub const fn new(level: u8) -> Option<Self> {
        if (level as usize) < PRIORITY_LEVELS {
            Some(Self(level))
        } else {
            None
        }
    }

    /// Construct a priority, panicking on an out-of-range value.
    ///
    /// # Panics
    ///
    /// Panics if `level >= PRIORITY_LEVELS`. Intended only for `const` contexts
    /// in generated code, where the panic is a build failure, never a runtime
    /// one.
    #[must_use]
    pub const fn new_const(level: u8) -> Self {
        assert!((level as usize) < PRIORITY_LEVELS, "priority out of range");
        Self(level)
    }

    /// Numeric level.
    #[must_use]
    pub const fn level(self) -> u8 {
        self.0
    }

    /// Convert to a hardware interrupt priority, where lower means more urgent.
    ///
    /// `hw_levels` is the number of levels the interrupt controller actually
    /// implements, which on ARM is a function of how many priority bits the
    /// vendor wired up — 4 bits (16 levels) on STM32, 3 bits (8 levels) on many
    /// nRF parts. The board crate supplies it; the kernel never guesses.
    #[must_use]
    pub const fn to_hardware(self, hw_levels: u8) -> u8 {
        if hw_levels == 0 {
            return 0;
        }
        // `PRIORITY_LEVELS` is a power of two, so the scaling divide is a
        // shift. Written as a shift rather than a `/` so the operation is
        // visibly panic-free rather than relying on the divisor being non-zero.
        const SHIFT: u32 = PRIORITY_LEVELS.trailing_zeros();
        let scaled = ((self.0 as u16).saturating_mul(hw_levels as u16) >> SHIFT) as u8;
        hw_levels.saturating_sub(1).saturating_sub(scaled)
    }
}

/// The set of priorities that currently have at least one runnable task.
///
/// One bit per priority. Bit *n* set means priority *n* has work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReadySet(u32);

impl ReadySet {
    /// An empty ready set.
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Mark `priority` as having runnable work.
    ///
    /// # Contract
    ///
    /// O(1), no allocation, ISR-safe, cannot block.
    #[inline]
    pub const fn insert(&mut self, priority: Priority) {
        self.0 |= 1u32 << priority.0;
    }

    /// Mark `priority` as having no runnable work.
    ///
    /// # Contract
    ///
    /// O(1), no allocation, ISR-safe, cannot block.
    #[inline]
    pub const fn remove(&mut self, priority: Priority) {
        self.0 &= !(1u32 << priority.0);
    }

    /// Whether `priority` has runnable work.
    #[inline]
    #[must_use]
    pub const fn contains(&self, priority: Priority) -> bool {
        self.0 & (1u32 << priority.0) != 0
    }

    /// Whether nothing at all is runnable.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// The highest priority with runnable work.
    ///
    /// # Contract
    ///
    /// O(1) — a single count-leading-zeros instruction on every architecture
    /// Malleus targets. Constant time regardless of task count. This is *the*
    /// hot path of the scheduler; its cost appears in `BENCH-002`.
    #[inline]
    #[must_use]
    pub const fn highest(&self) -> Option<Priority> {
        match NonZeroU32::new(self.0) {
            // `ilog2` on a non-zero value is the index of the most significant
            // set bit — the highest ready priority. It lowers to a single `CLZ`
            // on Cortex-M.
            Some(bits) => Some(Priority(bits.ilog2() as u8)),
            None => None,
        }
    }

    /// Raw bits. For trace emission and crash dumps.
    #[must_use]
    pub const fn bits(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_set_has_no_highest() {
        assert_eq!(ReadySet::new().highest(), None);
        assert!(ReadySet::new().is_empty());
    }

    #[test]
    fn highest_wins_regardless_of_insertion_order() {
        let mut set = ReadySet::new();
        set.insert(Priority::new(3).unwrap());
        set.insert(Priority::new(17).unwrap());
        set.insert(Priority::new(1).unwrap());
        assert_eq!(set.highest(), Priority::new(17));

        // Same members, reverse order, same answer.
        let mut other = ReadySet::new();
        other.insert(Priority::new(17).unwrap());
        other.insert(Priority::new(3).unwrap());
        other.insert(Priority::new(1).unwrap());
        assert_eq!(set, other);
    }

    #[test]
    fn removing_the_top_reveals_the_next() {
        let mut set = ReadySet::new();
        set.insert(Priority::new(5).unwrap());
        set.insert(Priority::MAX);
        assert_eq!(set.highest(), Some(Priority::MAX));
        set.remove(Priority::MAX);
        assert_eq!(set.highest(), Priority::new(5));
    }

    #[test]
    fn idle_priority_is_representable_and_lowest() {
        let mut set = ReadySet::new();
        set.insert(Priority::IDLE);
        assert_eq!(set.highest(), Some(Priority::IDLE));
        assert!(!set.is_empty());
        assert!(Priority::IDLE < Priority::MAX);
    }

    #[test]
    fn every_priority_round_trips() {
        for level in 0..PRIORITY_LEVELS as u8 {
            let p = Priority::new(level).expect("level is in range");
            let mut set = ReadySet::new();
            set.insert(p);
            assert_eq!(
                set.highest(),
                Some(p),
                "priority {level} did not round-trip"
            );
            assert!(set.contains(p));
            set.remove(p);
            assert!(set.is_empty());
        }
    }

    #[test]
    fn out_of_range_priority_is_rejected() {
        assert_eq!(Priority::new(PRIORITY_LEVELS as u8), None);
        assert!(Priority::new(PRIORITY_LEVELS as u8 - 1).is_some());
    }

    #[test]
    fn hardware_mapping_inverts_urgency() {
        // ARM NVIC: lower number is more urgent. Malleus: higher is more
        // urgent. The most urgent Malleus priority must map to the most urgent
        // hardware level, and the least to the least.
        let hw_levels = 16;
        let top = Priority::MAX.to_hardware(hw_levels);
        let bottom = Priority::IDLE.to_hardware(hw_levels);
        assert!(
            top < bottom,
            "urgency inversion is backwards: {top} !< {bottom}"
        );
        assert_eq!(bottom, hw_levels - 1);
    }

    #[test]
    fn hardware_mapping_is_monotonic_and_in_range() {
        for hw_levels in [4u8, 8, 16, 32] {
            let mut previous = u8::MAX;
            for level in 0..PRIORITY_LEVELS as u8 {
                let hw = Priority::new(level).unwrap().to_hardware(hw_levels);
                assert!(
                    hw < hw_levels,
                    "hardware priority {hw} exceeds {hw_levels} levels"
                );
                assert!(
                    hw <= previous,
                    "mapping must be monotonically non-increasing"
                );
                previous = hw;
            }
        }
    }
}
