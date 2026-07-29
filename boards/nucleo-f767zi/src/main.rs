//! Reset and memory-initialization probe for the Nucleo-F767ZI.
//!
//! The Cortex-M reset stub copies `.data` and clears `.bss` before calling the
//! architecture crate's minimal Rust entry. A debugger can then read
//! `MALLEUS_BOOT_EVIDENCE` to distinguish success from either initialization
//! failure without requiring a HAL or clock configuration.

#![no_main]
#![no_std]

use core::panic::PanicInfo;

malleus_arch_cortex_m::__malleus_cortex_m_reset!();

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
