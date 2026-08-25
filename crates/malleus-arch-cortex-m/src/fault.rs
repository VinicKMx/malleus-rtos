//! ARMv7-M exception entry and bootstrap fault evidence.
//!
//! The assembly boundary selects the pre-exception MSP or PSP from
//! `EXC_RETURN`, prevents recursive entry into Rust, and passes only raw values
//! to the capture routine. The Rust side snapshots fixed-size architectural
//! state and then halts; task attribution and recovery belong to later phases.

#[cfg(target_arch = "arm")]
use core::{
    arch::{asm, global_asm},
    mem::MaybeUninit,
    sync::atomic::{Ordering, compiler_fence},
};

#[cfg(target_arch = "arm")]
const CAPTURE_ARMED: u32 = 0x4152_4d44;
#[cfg(target_arch = "arm")]
const CAPTURE_COMPLETE: u32 = 0x444f_4e45;
#[cfg(target_arch = "arm")]
const EVIDENCE_MAGIC: u32 = 0x4d41_4c46;
#[cfg(target_arch = "arm")]
const EVIDENCE_VERSION: u32 = 1;

const EXC_RETURN_HANDLER_MSP_BASIC: u32 = 0xffff_fff1;
const EXC_RETURN_THREAD_MSP_BASIC: u32 = 0xffff_fff9;
const EXC_RETURN_THREAD_PSP_BASIC: u32 = 0xffff_fffd;
const EXC_RETURN_HANDLER_MSP_EXTENDED: u32 = 0xffff_ffe1;
const EXC_RETURN_THREAD_MSP_EXTENDED: u32 = 0xffff_ffe9;
const EXC_RETURN_THREAD_PSP_EXTENDED: u32 = 0xffff_ffed;

const EXTENDED_FRAME_PREFIX_WORDS: usize = 18;
const STACKING_ERROR_MASK: u32 = 0x0000_3838;
#[cfg(target_arch = "arm")]
const MMFAR_VALID: u32 = 1 << 7;
#[cfg(target_arch = "arm")]
const BFAR_VALID: u32 = 1 << 15;

#[cfg(target_arch = "arm")]
const FRAME_VALID: u32 = 1 << 0;
const FRAME_EXTENDED: u32 = 1 << 1;
const FRAME_USED_PSP: u32 = 1 << 2;
#[cfg(target_arch = "arm")]
const FRAME_MMFAR_VALID: u32 = 1 << 3;
#[cfg(target_arch = "arm")]
const FRAME_BFAR_VALID: u32 = 1 << 4;

#[cfg(target_arch = "arm")]
const SHCSR_ADDRESS: usize = 0xe000_ed24;
#[cfg(target_arch = "arm")]
const CFSR_ADDRESS: usize = 0xe000_ed28;
#[cfg(target_arch = "arm")]
const HFSR_ADDRESS: usize = 0xe000_ed2c;
#[cfg(target_arch = "arm")]
const MMFAR_ADDRESS: usize = 0xe000_ed34;
#[cfg(target_arch = "arm")]
const BFAR_ADDRESS: usize = 0xe000_ed38;

#[cfg(target_arch = "arm")]
const SHCSR_MEMFAULT_ENABLE: u32 = 1 << 16;
#[cfg(target_arch = "arm")]
const SHCSR_BUSFAULT_ENABLE: u32 = 1 << 17;
#[cfg(target_arch = "arm")]
const SHCSR_USAGEFAULT_ENABLE: u32 = 1 << 18;

#[derive(Clone, Copy)]
#[repr(C)]
struct ExceptionFrame {
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
}

impl ExceptionFrame {
    #[cfg(target_arch = "arm")]
    const UNAVAILABLE: Self = Self {
        r0: 0,
        r1: 0,
        r2: 0,
        r3: 0,
        r12: 0,
        lr: 0,
        pc: 0,
        xpsr: 0,
    };
}

/// Fixed debugger-readable snapshot written by the terminal fault path.
///
/// `magic` is written last. A zero or different value means the remaining
/// words must be treated as absent or partially written.
#[derive(Clone, Copy)]
#[repr(C)]
struct FaultEvidence {
    magic: u32,
    version: u32,
    exception_number: u32,
    exc_return: u32,
    stack_pointer: u32,
    frame_flags: u32,
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
    cfsr: u32,
    hfsr: u32,
    shcsr: u32,
    mmfar: u32,
    bfar: u32,
}

#[cfg(target_arch = "arm")]
// SAFETY: the linker places this aligned object in DTCM `.uninit`. It has one
// writer after faults are armed and is inspected only by a halted debugger.
#[unsafe(link_section = ".uninit.fault_evidence")]
#[unsafe(no_mangle)]
static mut MALLEUS_FAULT_EVIDENCE: MaybeUninit<FaultEvidence> = MaybeUninit::uninit();

fn frame_layout(exc_return: u32) -> Option<(usize, u32)> {
    match exc_return {
        EXC_RETURN_HANDLER_MSP_BASIC | EXC_RETURN_THREAD_MSP_BASIC => Some((0, 0)),
        EXC_RETURN_THREAD_PSP_BASIC => Some((0, FRAME_USED_PSP)),
        EXC_RETURN_HANDLER_MSP_EXTENDED | EXC_RETURN_THREAD_MSP_EXTENDED => {
            Some((EXTENDED_FRAME_PREFIX_WORDS, FRAME_EXTENDED))
        }
        EXC_RETURN_THREAD_PSP_EXTENDED => {
            Some((EXTENDED_FRAME_PREFIX_WORDS, FRAME_EXTENDED | FRAME_USED_PSP))
        }
        _ => None,
    }
}

const fn stacking_failed(cfsr: u32) -> bool {
    cfsr & STACKING_ERROR_MASK != 0
}

#[cfg(target_arch = "arm")]
fn read_scb(address: usize) -> u32 {
    // SAFETY: callers pass one of the aligned, privileged 32-bit SCB register
    // addresses defined by the ARMv7-M memory map. Reads do not clear status.
    unsafe { (address as *const u32).read_volatile() }
}

#[cfg(target_arch = "arm")]
fn write_shcsr(value: u32) {
    // SAFETY: this is the aligned privileged SHCSR address. The read/modify/
    // write preserves status fields and changes only the handler enable bits.
    unsafe { (SHCSR_ADDRESS as *mut u32).write_volatile(value) }
}

#[cfg(target_arch = "arm")]
fn read_exception_frame(
    stack_pointer: *const u32,
    exc_return: u32,
    cfsr: u32,
) -> Option<(ExceptionFrame, u32)> {
    let (prefix_words, layout_flags) = frame_layout(exc_return)?;
    if stacking_failed(cfsr) || (stack_pointer as usize) & 0x3 != 0 {
        return None;
    }

    let frame_pointer = stack_pointer
        .wrapping_add(prefix_words)
        .cast::<ExceptionFrame>();
    // SAFETY: a recognized EXC_RETURN value identifies the selected hardware
    // stack and frame kind. With no stacking/lazy-stacking error recorded,
    // ARMv7-M guarantees that exception entry populated the aligned core frame;
    // every bit pattern is valid for this all-u32 representation.
    let frame = unsafe { frame_pointer.read_volatile() };
    Some((frame, layout_flags | FRAME_VALID))
}

#[cfg(target_arch = "arm")]
pub(crate) fn arm_capture() {
    // SAFETY: reset initialized this aligned `.uninit` word before RAM setup.
    // This single-core write transfers ownership to the exception assembly.
    unsafe {
        (&raw mut crate::startup::MALLEUS_FAULT_CAPTURE_STATE)
            .cast::<u32>()
            .write_volatile(CAPTURE_ARMED);
    }
}

#[cfg(target_arch = "arm")]
fn configure_probe_fault() {
    let current = read_scb(SHCSR_ADDRESS);
    let enabled = current | SHCSR_MEMFAULT_ENABLE | SHCSR_BUSFAULT_ENABLE;
    let configured = if cfg!(feature = "bootstrap-hardfault-escalation") {
        enabled & !SHCSR_USAGEFAULT_ENABLE
    } else {
        enabled | SHCSR_USAGEFAULT_ENABLE
    };
    write_shcsr(configured);

    // SAFETY: the barriers only order the preceding architectural register
    // write before the planted fault; they do not access memory or the stack.
    unsafe { asm!("dsb", "isb", options(nostack, preserves_flags)) };
}

#[cfg(target_arch = "arm")]
pub(crate) fn run_bootstrap_probe() -> ! {
    configure_probe_fault();

    // SAFETY: UDF intentionally transfers control to UsageFault, or to
    // HardFault when escalation is selected. Both vectors are installed and
    // terminal; execution can never resume past this instruction.
    unsafe { asm!("udf #0", options(noreturn, nomem, nostack)) }
}

#[cfg(target_arch = "arm")]
/// Capture one terminal ARMv7-M fault after assembly validates entry state.
///
/// # Safety
///
/// Only the fault-entry assembly may call this function. It must pass the
/// active IPSR exception number, the untouched hardware EXC_RETURN value, and
/// the corresponding pre-exception stack pointer after changing capture state
/// from armed to capturing.
#[unsafe(no_mangle)]
unsafe extern "C" fn __malleus_fault_capture(
    exception_number: u32,
    exc_return: u32,
    stack_pointer: *const u32,
) -> ! {
    // The ARM manual requires reading address registers before their validity
    // bits so a later, higher-priority fault cannot pair a new address with old
    // status. PRIMASK is already set, but this also handles a possible NMI.
    let raw_mmfar = read_scb(MMFAR_ADDRESS);
    let raw_bfar = read_scb(BFAR_ADDRESS);
    let cfsr = read_scb(CFSR_ADDRESS);
    let hfsr = read_scb(HFSR_ADDRESS);
    let shcsr = read_scb(SHCSR_ADDRESS);

    let (frame, mut frame_flags) = read_exception_frame(stack_pointer, exc_return, cfsr)
        .unwrap_or((ExceptionFrame::UNAVAILABLE, 0));

    let mmfar = if cfsr & MMFAR_VALID != 0 {
        frame_flags |= FRAME_MMFAR_VALID;
        raw_mmfar
    } else {
        0
    };
    let bfar = if cfsr & BFAR_VALID != 0 {
        frame_flags |= FRAME_BFAR_VALID;
        raw_bfar
    } else {
        0
    };

    let evidence = FaultEvidence {
        magic: 0,
        version: EVIDENCE_VERSION,
        exception_number,
        exc_return,
        stack_pointer: stack_pointer as u32,
        frame_flags,
        r0: frame.r0,
        r1: frame.r1,
        r2: frame.r2,
        r3: frame.r3,
        r12: frame.r12,
        lr: frame.lr,
        pc: frame.pc,
        xpsr: frame.xpsr,
        cfsr,
        hfsr,
        shcsr,
        mmfar,
        bfar,
    };

    // SAFETY: exception assembly changed the state to capturing before this
    // call, so it is the sole writer. The linker provides aligned DTCM storage.
    // The aggregate is written first, then magic and completion are published.
    unsafe {
        let evidence_pointer = (&raw mut MALLEUS_FAULT_EVIDENCE).cast::<FaultEvidence>();
        evidence_pointer.write_volatile(evidence);
        evidence_pointer
            .cast::<u32>()
            .write_volatile(EVIDENCE_MAGIC);
        (&raw mut crate::startup::MALLEUS_FAULT_CAPTURE_STATE)
            .cast::<u32>()
            .write_volatile(CAPTURE_COMPLETE);
    }

    loop {
        compiler_fence(Ordering::SeqCst);
        core::hint::spin_loop();
    }
}

#[cfg(target_arch = "arm")]
global_asm!(
    r#"
    .syntax unified
    .thumb

    .section .text.malleus_fault_entry, "ax", %progbits
    .balign 4

    .global HardFault
    .type HardFault, %function
    .thumb_func
HardFault:
    b.w .Lmalleus_fault_entry
    .size HardFault, . - HardFault

    .global MemManage
    .type MemManage, %function
    .thumb_func
MemManage:
    b.w .Lmalleus_fault_entry
    .size MemManage, . - MemManage

    .global BusFault
    .type BusFault, %function
    .thumb_func
BusFault:
    b.w .Lmalleus_fault_entry
    .size BusFault, . - BusFault

    .global UsageFault
    .type UsageFault, %function
    .thumb_func
UsageFault:
    b.w .Lmalleus_fault_entry
    .size UsageFault, . - UsageFault

.Lmalleus_fault_entry:
    /* Prevent configurable interrupts and recursive entry into Rust. */
    cpsid i
    ldr r3, =MALLEUS_FAULT_CAPTURE_STATE
    ldr r12, [r3]
    ldr r2, =0x41524d44
    cmp r12, r2
    bne .Lmalleus_fault_terminal
    ldr r12, =0x43415054
    str r12, [r3]

    /* Pass IPSR, untouched EXC_RETURN, and the pre-exception MSP or PSP. */
    mrs r0, ipsr
    mov r1, lr
    tst r1, #4
    beq .Lmalleus_fault_msp
    mrs r2, psp
    b .Lmalleus_fault_dispatch
.Lmalleus_fault_msp:
    mrs r2, msp
.Lmalleus_fault_dispatch:
    b.w __malleus_fault_capture

.Lmalleus_fault_terminal:
    b .Lmalleus_fault_terminal
"#
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exc_return_selects_the_core_frame_and_stack() {
        let cases = [
            (EXC_RETURN_HANDLER_MSP_BASIC, 0, 0),
            (EXC_RETURN_THREAD_MSP_BASIC, 0, 0),
            (EXC_RETURN_THREAD_PSP_BASIC, 0, FRAME_USED_PSP),
            (
                EXC_RETURN_HANDLER_MSP_EXTENDED,
                EXTENDED_FRAME_PREFIX_WORDS,
                FRAME_EXTENDED,
            ),
            (
                EXC_RETURN_THREAD_MSP_EXTENDED,
                EXTENDED_FRAME_PREFIX_WORDS,
                FRAME_EXTENDED,
            ),
            (
                EXC_RETURN_THREAD_PSP_EXTENDED,
                EXTENDED_FRAME_PREFIX_WORDS,
                FRAME_EXTENDED | FRAME_USED_PSP,
            ),
        ];

        for (exc_return, expected_words, expected_flags) in cases {
            assert_eq!(
                frame_layout(exc_return),
                Some((expected_words, expected_flags))
            );
        }
        assert_eq!(frame_layout(0xffff_ffff), None);
        assert_eq!(frame_layout(0), None);
    }

    #[test]
    fn every_stacking_error_suppresses_frame_access() {
        for bit in [3, 4, 5, 11, 12, 13] {
            assert!(
                stacking_failed(1 << bit),
                "CFSR bit {bit} must reject the frame"
            );
        }
        assert!(
            !stacking_failed(1 << 16),
            "UNDEFINSTR leaves the frame valid"
        );
    }

    #[test]
    fn debugger_evidence_layout_is_stable_for_the_bootstrap_contract() {
        assert_eq!(core::mem::size_of::<ExceptionFrame>(), 8 * 4);
        assert_eq!(core::mem::size_of::<FaultEvidence>(), 19 * 4);
        assert_eq!(core::mem::offset_of!(FaultEvidence, magic), 0);
        assert_eq!(core::mem::offset_of!(FaultEvidence, pc), 12 * 4);
        assert_eq!(core::mem::offset_of!(FaultEvidence, cfsr), 14 * 4);
    }
}
