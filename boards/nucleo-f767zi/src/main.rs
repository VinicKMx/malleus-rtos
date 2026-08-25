//! Reset, memory-initialization, and exception probe for the Nucleo-F767ZI.
//!
//! The Cortex-M reset stub copies `.data` and clears `.bss` before calling the
//! architecture crate's minimal Rust entry. The entry records startup status,
//! plants an undefined instruction, and leaves a fixed fault snapshot for a
//! debugger without requiring a HAL or clock configuration.

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
