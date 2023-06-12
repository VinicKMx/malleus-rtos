//! Human-written units for durations and sizes.
//!
//! The manifest says `"500us"` and `"2KiB"`, not `500` and `2048`. This is not
//! sugar. A bare number in a config file has an implied unit that lives only in
//! the reader's head, and the two most expensive unit confusions in embedded
//! systems are milliseconds versus microseconds and KB versus KiB. Requiring
//! the unit at the point of writing removes both, and the parser is strict:
//! there is no default unit to fall back on.

use std::fmt;

/// Failure to parse a unit-carrying value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseUnitError {
    /// No numeric part.
    #[error("`{0}` has no numeric part")]
    MissingNumber(String),
    /// No unit suffix. There is deliberately no default.
    #[error("`{0}` has no unit; write e.g. `500us`, `1ms`, `2KiB` — Malleus never assumes one")]
    MissingUnit(String),
    /// The suffix is not one this parser knows.
    #[error("`{unit}` is not a valid {kind} unit (expected one of: {expected})")]
    UnknownUnit {
        /// The offending suffix.
        unit: String,
        /// What was being parsed: `duration` or `size`.
        kind: &'static str,
        /// The accepted suffixes.
        expected: &'static str,
    },
    /// The value overflowed.
    #[error("`{0}` is too large to represent")]
    Overflow(String),
}

/// A duration, stored in nanoseconds.
///
/// Nanoseconds internally, because the manifest is a host-side artefact where
/// the cost of 64-bit arithmetic is irrelevant, and converting to ticks is a
/// board-specific step that happens exactly once, in the code generator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u64);

impl Duration {
    /// Construct from nanoseconds.
    #[must_use]
    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Value in nanoseconds.
    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    /// Convert to ticks at `tick_hz`, rounding up.
    ///
    /// Rounds up so a declared deadline is never quietly tightened by rounding.
    #[must_use]
    pub const fn to_ticks(self, tick_hz: u64) -> u64 {
        if tick_hz == 0 {
            return 0;
        }
        // ns * hz / 1e9, computed to avoid overflow for realistic inputs.
        let seconds = self.0 / 1_000_000_000;
        let remainder = self.0 % 1_000_000_000;
        seconds
            .saturating_mul(tick_hz)
            .saturating_add(remainder.saturating_mul(tick_hz).div_ceil(1_000_000_000))
    }

    /// Whether this duration is an exact multiple of one tick at `tick_hz`.
    ///
    /// The analyser requires this for periodic tasks. A period that is not an
    /// exact tick multiple produces systematic jitter that looks like a
    /// hardware problem and is not.
    #[must_use]
    pub const fn is_tick_aligned(self, tick_hz: u64) -> bool {
        if tick_hz == 0 {
            return false;
        }
        let tick_nanos = 1_000_000_000 / tick_hz;
        tick_nanos != 0 && self.0.is_multiple_of(tick_nanos)
    }
}

impl std::str::FromStr for Duration {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (value, unit) = split_value_unit(s)?;
        let multiplier = match unit {
            "ns" => 1u64,
            "us" | "µs" => 1_000,
            "ms" => 1_000_000,
            "s" => 1_000_000_000,
            other => {
                return Err(ParseUnitError::UnknownUnit {
                    unit: other.to_owned(),
                    kind: "duration",
                    expected: "ns, us, ms, s",
                });
            }
        };
        value
            .checked_mul(multiplier)
            .map(Self)
            .ok_or_else(|| ParseUnitError::Overflow(s.to_owned()))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        if n >= 1_000_000_000 && n.is_multiple_of(1_000_000_000) {
            write!(f, "{}s", n / 1_000_000_000)
        } else if n >= 1_000_000 && n.is_multiple_of(1_000_000) {
            write!(f, "{}ms", n / 1_000_000)
        } else if n >= 1_000 && n.is_multiple_of(1_000) {
            write!(f, "{}us", n / 1_000)
        } else {
            write!(f, "{n}ns")
        }
    }
}

/// A size in bytes.
///
/// Binary units only — `KiB`, `MiB`. `KB` is rejected rather than silently
/// interpreted, because on a part with 512 KiB of RAM the difference between
/// 1000 and 1024 compounds into a linker error whose cause is not obvious.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteSize(u64);

impl ByteSize {
    /// Construct from a byte count.
    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    /// Value in bytes.
    #[must_use]
    pub const fn as_bytes(self) -> u64 {
        self.0
    }

    /// Round up to the next power of two.
    ///
    /// Needed for ARMv7-M protection regions, which must be a power of two and
    /// naturally aligned. The difference between this and [`Self::as_bytes`] is
    /// the padding the analyser reports as a line item.
    #[must_use]
    pub const fn to_power_of_two(self) -> Self {
        if self.0 <= 1 {
            return Self(1);
        }
        Self(self.0.next_power_of_two())
    }
}

impl std::str::FromStr for ByteSize {
    type Err = ParseUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (value, unit) = split_value_unit(s)?;
        let multiplier = match unit {
            "B" => 1u64,
            "KiB" => 1024,
            "MiB" => 1024 * 1024,
            other => {
                return Err(ParseUnitError::UnknownUnit {
                    unit: other.to_owned(),
                    kind: "size",
                    // Note the omission of KB/MB: decimal units are rejected on
                    // purpose, so the mistake is caught rather than absorbed.
                    expected: "B, KiB, MiB",
                });
            }
        };
        value
            .checked_mul(multiplier)
            .map(Self)
            .ok_or_else(|| ParseUnitError::Overflow(s.to_owned()))
    }
}

impl fmt::Display for ByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        if n >= 1024 * 1024 && n.is_multiple_of(1024 * 1024) {
            write!(f, "{}MiB", n / (1024 * 1024))
        } else if n >= 1024 && n.is_multiple_of(1024) {
            write!(f, "{}KiB", n / 1024)
        } else {
            write!(f, "{n}B")
        }
    }
}

/// Split `"500us"` into `(500, "us")`.
fn split_value_unit(s: &str) -> Result<(u64, &str), ParseUnitError> {
    let trimmed = s.trim();
    let split = trimmed
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| ParseUnitError::MissingUnit(trimmed.to_owned()))?;
    if split == 0 {
        return Err(ParseUnitError::MissingNumber(trimmed.to_owned()));
    }
    let (number, unit) = trimmed.split_at(split);
    let value = number
        .parse::<u64>()
        .map_err(|_| ParseUnitError::Overflow(trimmed.to_owned()))?;
    Ok((value, unit.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_parse_across_every_scale() {
        let cases = [
            ("250ns", 250u64),
            ("500us", 500_000),
            ("1ms", 1_000_000),
            ("60s", 60_000_000_000),
        ];
        for (text, nanos) in cases {
            let parsed: Duration = text.parse().expect(text);
            assert_eq!(parsed.as_nanos(), nanos, "parsing {text}");
        }
    }

    #[test]
    fn a_missing_unit_is_an_error_not_a_default() {
        assert!(matches!(
            "500".parse::<Duration>(),
            Err(ParseUnitError::MissingUnit(_))
        ));
        assert!(matches!(
            "4096".parse::<ByteSize>(),
            Err(ParseUnitError::MissingUnit(_))
        ));
    }

    #[test]
    fn decimal_size_units_are_rejected() {
        // `2KB` must not silently become 2000 or 2048 bytes.
        let err = "2KB".parse::<ByteSize>().unwrap_err();
        assert!(
            matches!(err, ParseUnitError::UnknownUnit { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn sizes_use_binary_units() {
        assert_eq!("2KiB".parse::<ByteSize>().unwrap().as_bytes(), 2048);
        assert_eq!("1MiB".parse::<ByteSize>().unwrap().as_bytes(), 1_048_576);
    }

    #[test]
    fn durations_round_trip_through_display() {
        for text in ["250ns", "500us", "1ms", "60s"] {
            let parsed: Duration = text.parse().unwrap();
            assert_eq!(parsed.to_string(), text);
        }
    }

    #[test]
    fn sizes_round_trip_through_display() {
        for text in ["512B", "2KiB", "1MiB"] {
            let parsed: ByteSize = text.parse().unwrap();
            assert_eq!(parsed.to_string(), text);
        }
    }

    #[test]
    fn tick_conversion_rounds_up_so_deadlines_are_never_tightened() {
        // At 1 MHz a tick is 1us. 1500ns is one and a half ticks and must
        // become 2, not 1 — a deadline rounded down is a deadline you did not
        // agree to.
        let d = Duration::from_nanos(1_500);
        assert_eq!(d.to_ticks(1_000_000), 2);
    }

    #[test]
    fn tick_conversion_is_exact_on_aligned_values() {
        let d: Duration = "1ms".parse().unwrap();
        assert_eq!(d.to_ticks(1_000_000), 1_000);
        assert!(d.is_tick_aligned(1_000_000));
    }

    #[test]
    fn misaligned_periods_are_detectable() {
        // 1500ns at a 1 MHz (1us) tick is not an exact multiple.
        assert!(!Duration::from_nanos(1_500).is_tick_aligned(1_000_000));
    }

    #[test]
    fn power_of_two_rounding_exposes_armv7m_padding() {
        // 3 KiB of stack costs 4 KiB of ARMv7-M protection region.
        let requested: ByteSize = "3KiB".parse().unwrap();
        let actual = requested.to_power_of_two();
        assert_eq!(actual.as_bytes(), 4096);
        assert_eq!(
            actual.as_bytes() - requested.as_bytes(),
            1024,
            "reported padding"
        );

        // Already a power of two: no padding.
        let exact: ByteSize = "2KiB".parse().unwrap();
        assert_eq!(exact.to_power_of_two().as_bytes(), 2048);
    }

    #[test]
    fn nonsense_input_is_rejected_cleanly() {
        assert!("us".parse::<Duration>().is_err());
        assert!("".parse::<Duration>().is_err());
        assert!("12parsecs".parse::<Duration>().is_err());
    }
}
