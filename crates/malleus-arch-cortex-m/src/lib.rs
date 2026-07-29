//! ARM Cortex-M port of the Malleus architecture layer.
//!
//! # Status: bring-up in progress — Checkpoint 1
//!
//! This crate is a declared shape, not a working port. It is in the repository
//! from day one so that the [`malleus_arch`] contract is exercised by a real
//! consumer and so the porting surface is visible to anyone evaluating the
//! project. It contains no stubs that pretend to work: the port either exists
//! or the build tells you it does not.
//!
//! # Planned variants
//!
//! | Feature | Core | Isolation tier | Board |
//! |---------|------|----------------|-------|
//! | `cortex-m7` | Cortex-M7, ARMv7E-M | 1 (ARMv7-M MPU) | Nucleo-F767ZI |
//! | `cortex-m33` | Cortex-M33, ARMv8-M | 2 (ARMv8-M MPU) | planned |
//! | `cortex-m4` | Cortex-M4F, ARMv7E-M | 1 | planned |
//!
//! # Why Cortex-M7 first, when M33 is the strategic target
//!
//! Because it is the board on the bench. Bring-up on hardware you can probe,
//! reset, and stare at with a logic analyser is worth more than bring-up on the
//! architecturally preferable part you have to order and wait for. The M7 also
//! forces the harder MPU problem — power-of-two, naturally-aligned regions —
//! which means the region allocator is designed against the constrained case
//! rather than retrofitted to it later.
//!
//! The trade is explicit and recorded in
//! `docs/adr/0001-target-architecture.md`: the M7 has no TrustZone, so the
//! secure/non-secure split that ARMv8-M enables is deferred, not designed
//! around.
//!
//! # What Checkpoint 1 must deliver here
//!
//! - Reset vector and `.data`/`.bss` initialisation
//! - Exception and interrupt entry, including a fault handler that captures
//!   enough state to attribute the fault to a task
//! - `PendSV`-based deferred context switch with correct lazy-FPU handling
//! - `BASEPRI`-based critical sections that do not mask the highest priorities
//! - `SysTick` or a general-purpose timer as the monotonic source, tickless
//! - MPU region programming with a stack guard region below every task stack
//!
//! Each item maps to a requirement in [`malleus_arch::conformance`], and none
//! is considered done until it passes on hardware.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

#[cfg(any(feature = "cortex-m4", feature = "cortex-m7", feature = "cortex-m33"))]
mod startup;

/// Cortex-M7 (ARMv7E-M) port.
///
/// Empty until Checkpoint 1. See the module documentation for the delivery
/// list and `docs/internals/porting.md` for the walkthrough.
#[cfg(feature = "cortex-m7")]
pub mod m7 {}

/// Number of NVIC priority bits implemented, by vendor.
///
/// This is not an architectural constant: ARM specifies up to 8 bits and each
/// vendor wires up a subset. Getting it wrong makes priority comparisons
/// silently incorrect for a subset of levels, which produces a system that
/// mostly works. The board crate supplies the true value and the kernel never
/// guesses.
pub mod priority_bits {
    /// STM32F7 series, including the STM32F767ZI on the Nucleo-F767ZI.
    pub const STM32F7: u8 = 4;
    /// STM32H5 series.
    pub const STM32H5: u8 = 4;
    /// Nordic nRF52 and nRF53 series.
    pub const NRF5X: u8 = 3;
    /// Raspberry Pi RP2040 and RP2350.
    pub const RP2XXX: u8 = 2;

    /// Number of distinct hardware priority levels for a given bit count.
    #[must_use]
    pub const fn levels(bits: u8) -> u16 {
        1u16 << bits
    }
}

#[cfg(test)]
mod tests {
    use super::priority_bits::{self, levels};

    #[test]
    fn priority_bits_map_to_level_counts() {
        assert_eq!(levels(priority_bits::STM32F7), 16);
        assert_eq!(levels(priority_bits::NRF5X), 8);
        assert_eq!(levels(priority_bits::RP2XXX), 4);
    }

    #[test]
    fn no_vendor_exceeds_the_architectural_maximum() {
        for bits in [
            priority_bits::STM32F7,
            priority_bits::STM32H5,
            priority_bits::NRF5X,
            priority_bits::RP2XXX,
        ] {
            assert!(bits <= 8, "ARM specifies at most 8 NVIC priority bits");
            assert!(
                bits >= 2,
                "fewer than 2 bits cannot express a usable priority scheme"
            );
        }
    }
}
