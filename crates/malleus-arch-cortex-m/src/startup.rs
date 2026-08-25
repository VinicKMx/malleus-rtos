//! First-stage Cortex-M reset entry.
//!
//! This must remain assembly: calling Rust before `.data` and `.bss` are valid
//! would make the compiler, rather than this module, part of the startup
//! contract. Boards provide the linker symbols and materialize the entry in
//! their binary with the hidden macro below.

use core::mem::MaybeUninit;

const DATA_PATTERN: u32 = 0x4441_5441;
const EVIDENCE_OK: u32 = 0x4d41_4c32;
const EVIDENCE_DATA_FAILED: u32 = 0x4441_5441;
const EVIDENCE_BSS_FAILED: u32 = 0x4253_5321;
const EVIDENCE_BOTH_FAILED: u32 = 0x424f_5448;

// SAFETY: the board linker contract loads this non-zero word from Flash into
// the word-aligned `.data` range before any Rust code reads it.
#[unsafe(link_section = ".data.boot_probe")]
static mut DATA_SENTINEL: u32 = DATA_PATTERN;

// SAFETY: the board linker contract places this word in the word-aligned,
// `NOLOAD` `.bss` range, which the reset stub clears before Rust reads it.
#[unsafe(link_section = ".bss.boot_probe")]
static mut BSS_SENTINEL: u32 = 0;

// This section is intentionally excluded from startup clearing. The reset
// probe overwrites the word before a debugger is allowed to interpret it.
// SAFETY: the symbol has one writer during single-core reset and is read only
// by an externally halted debugger.
#[unsafe(link_section = ".uninit.boot_evidence")]
#[unsafe(no_mangle)]
static mut MALLEUS_BOOT_EVIDENCE: u32 = 0;

// Reset writes this word before touching `.data` or `.bss`. Fault assembly
// consults it before entering Rust, so a startup fault cannot cross the Rust
// boundary while RAM is still invalid.
// SAFETY: assembly and the fault handler access this aligned word only through
// volatile loads/stores during single-core bootstrap.
#[unsafe(link_section = ".uninit.fault_state")]
#[unsafe(no_mangle)]
pub(crate) static mut MALLEUS_FAULT_CAPTURE_STATE: MaybeUninit<u32> = MaybeUninit::uninit();

/// First Rust entry after the assembly reset stub establishes valid RAM.
///
/// # Safety
///
/// Only the `Reset` stub may call this symbol, after copying the complete
/// linker-defined `.data` range and clearing the complete `.bss` range.
#[doc(hidden)]
#[unsafe(no_mangle)]
unsafe extern "C" fn __malleus_start() -> ! {
    // SAFETY: the reset stub completed `.data` initialization before this
    // function was called; volatile access makes the hardware observation
    // explicit and prevents constant-folding the probe away.
    let data_ok = unsafe { (&raw const DATA_SENTINEL).read_volatile() == DATA_PATTERN };
    // SAFETY: the reset stub completed `.bss` initialization before this
    // function was called; the linker keeps this word inside that range.
    let bss_ok = unsafe { (&raw const BSS_SENTINEL).read_volatile() == 0 };

    let evidence = match (data_ok, bss_ok) {
        (true, true) => EVIDENCE_OK,
        (false, true) => EVIDENCE_DATA_FAILED,
        (true, false) => EVIDENCE_BSS_FAILED,
        (false, false) => EVIDENCE_BOTH_FAILED,
    };

    // SAFETY: reset is single-core and no interrupt is enabled by this image;
    // the `.uninit` word is aligned, valid, and exclusively owned here.
    unsafe { (&raw mut MALLEUS_BOOT_EVIDENCE).write_volatile(evidence) };

    #[cfg(all(feature = "cortex-m7", target_arch = "arm"))]
    crate::fault::arm_capture();

    #[cfg(all(
        feature = "cortex-m7",
        feature = "bootstrap-fault-probe",
        target_arch = "arm"
    ))]
    crate::fault::run_bootstrap_probe();

    #[cfg(not(all(
        feature = "cortex-m7",
        feature = "bootstrap-fault-probe",
        target_arch = "arm"
    )))]
    loop {
        core::hint::spin_loop();
    }
}

/// Materialize the Cortex-M reset entry in the consuming board image.
///
/// This is a link-time integration detail, not a stable port API.
#[doc(hidden)]
#[macro_export]
macro_rules! __malleus_cortex_m_reset {
    () => {
        core::arch::global_asm!(
            r#"
    .syntax unified
    .thumb

    .section .text.Reset, "ax", %progbits
    .global Reset
    .type Reset, %function
    .thumb_func
Reset:
    /* Keep faults in assembly until Rust memory initialization is complete. */
    ldr r0, =MALLEUS_FAULT_CAPTURE_STATE
    ldr r1, =0x52535430
    str r1, [r0]

    /* Copy the word-aligned `.data` image from Flash into RAM. */
    ldr r0, =__sidata
    ldr r1, =__sdata
    ldr r2, =__edata
.Lcopy_data:
    cmp r1, r2
    bhs .Lzero_bss_start
    ldr r3, [r0], #4
    str r3, [r1], #4
    b .Lcopy_data

.Lzero_bss_start:
    /* Clear every word in `.bss`; `.uninit` deliberately remains untouched. */
    ldr r1, =__sbss
    ldr r2, =__ebss
    movs r3, #0
.Lzero_bss:
    cmp r1, r2
    bhs .Lenter_rust
    str r3, [r1], #4
    b .Lzero_bss

.Lenter_rust:
    /* No Rust code runs until both memory operations have completed. */
    bl __malleus_start

    /* The Rust entry is diverging. Fail closed if that contract is violated. */
.Lunexpected_return:
    b .Lunexpected_return

    .size Reset, . - Reset
"#
        );
    };
}
