//! Minimal linkable image for the Nucleo-F767ZI target foundation.
//!
//! This image deliberately does not claim to boot. `Reset` is a placeholder
//! until C1.P1.2 establishes and verifies the reset and memory-initialization
//! contract on hardware.

#![no_main]
#![no_std]

use core::panic::PanicInfo;

#[allow(non_snake_case)]
// SAFETY: this binary owns the unique `Reset` export, and `link.x` is the sole
// consumer of its C ABI and diverging control-flow contract.
#[unsafe(no_mangle)]
extern "C" fn Reset() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
