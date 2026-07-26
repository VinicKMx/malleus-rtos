//! Hardware memory protection.
//!
//! Malleus supports three tiers of isolation, and the tier in use is always
//! visible in the build report rather than assumed:
//!
//! | Tier | Hardware | Guarantee |
//! |------|----------|-----------|
//! | 0 | none ([`NoProtection`]) | Rust type safety only. A task with `unsafe` or a bad DMA descriptor can corrupt any other task. |
//! | 1 | ARMv7-M MPU (Cortex-M4/M7) | Task-granular isolation with power-of-two, naturally-aligned regions. Costs padding. |
//! | 2 | ARMv8-M MPU (Cortex-M33) or RISC-V PMP | Task-granular isolation with arbitrary 32-byte-granular regions. Little padding. |
//!
//! The ARMv7-M alignment constraint is a real cost, not a footnote: a task
//! needing 3 KiB of stack must be given a 4 KiB naturally-aligned region.
//! `cargo malleus analyze` reports that padding as a line item so the waste is
//! a decision, not a surprise. See `docs/adr/0005-memory-isolation.md`.

use crate::ArchError;

/// Access rights for a protection region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions {
    /// Readable by unprivileged (task) code.
    pub read: bool,
    /// Writable by unprivileged (task) code.
    pub write: bool,
    /// Executable.
    pub execute: bool,
    /// Device memory: strongly ordered, non-cacheable, no speculative access.
    /// Required for MMIO, and getting this wrong on a Cortex-M7 with its store
    /// buffer produces bugs that only appear at speed.
    pub device: bool,
}

impl Permissions {
    /// Read-only data.
    pub const RO: Self = Self {
        read: true,
        write: false,
        execute: false,
        device: false,
    };
    /// Read-write data. The default for a task's stack and `.bss`.
    pub const RW: Self = Self {
        read: true,
        write: true,
        execute: false,
        device: false,
    };
    /// Executable, read-only code.
    pub const RX: Self = Self {
        read: true,
        write: false,
        execute: true,
        device: false,
    };
    /// Memory-mapped peripheral registers.
    pub const MMIO: Self = Self {
        read: true,
        write: true,
        execute: false,
        device: true,
    };
}

/// A contiguous span of address space with uniform permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    /// Base address.
    pub base: usize,
    /// Size in bytes.
    pub size: usize,
    /// Access rights for unprivileged code.
    pub permissions: Permissions,
}

/// Programs the hardware memory-protection unit.
///
/// # Safety
///
/// This is the mechanism the entire fault-containment story depends on. An
/// implementation that programs the wrong region, forgets to invalidate on
/// switch, or leaves a stale region enabled silently removes isolation while
/// the build report still claims it is present — the worst possible failure
/// mode for a safety feature. Implementations must pass the memory-protection
/// section of [`crate::conformance`] on hardware.
pub unsafe trait MemoryProtection {
    /// Number of simultaneously programmable regions.
    ///
    /// The build-time region allocator uses this to decide whether a task's
    /// declared capabilities fit, and produces an actionable error naming the
    /// offending task when they do not.
    const REGION_COUNT: u8;

    /// Whether the hardware requires power-of-two, naturally-aligned regions.
    /// `true` on ARMv7-M, `false` on ARMv8-M and RISC-V PMP (NAPOT aside).
    const REQUIRES_POWER_OF_TWO: bool;

    /// Smallest programmable region size in bytes.
    const MIN_REGION_SIZE: usize;

    /// Enable protection. Until this is called, regions have no effect.
    ///
    /// # Safety
    ///
    /// The caller must have programmed a region set that covers the kernel's
    /// own code and data, or the next kernel access faults.
    unsafe fn enable();

    /// Disable protection entirely. Used only by the fault handler when
    /// building a crash dump, which needs to read a faulted task's stack.
    ///
    /// # Safety
    ///
    /// Removes all isolation. Callers must re-enable before returning to any
    /// unprivileged code.
    unsafe fn disable();

    /// Install a task's complete region set, replacing the previous one.
    ///
    /// # Contract
    ///
    /// - O(n) in `regions.len()`, which is bounded by [`Self::REGION_COUNT`],
    ///   so this is O(1) with a small constant. It sits on the context-switch
    ///   path and its cost is part of the published switch benchmark.
    ///
    /// # Errors
    ///
    /// [`ArchError::UnsupportedRegion`] if any region violates the alignment,
    /// size, or count constraints. In a shipped system this cannot happen: the
    /// build-time allocator has already proved the set is representable. The
    /// runtime check exists to catch a broken port, and is compiled out in
    /// release builds of ports that have passed conformance.
    ///
    /// # Safety
    ///
    /// The caller must ensure `regions` describes memory the incoming task is
    /// entitled to, per its declared capabilities.
    unsafe fn install(regions: &[Region]) -> Result<(), ArchError>;
}

/// The no-op implementation, for architectures with no protection hardware.
///
/// Choosing this is not a silent fallback. A board declaring `NoProtection`
/// makes `cargo malleus analyze` emit an explicit isolation-tier-0 warning, and
/// any task whose manifest requests `restart = "on-fault"` is rejected at build
/// time — restarting a task you cannot contain is a false promise.
#[derive(Debug, Clone, Copy)]
pub struct NoProtection;

// SAFETY: Every method is a no-op that touches no hardware and makes no
// isolation claim. `REGION_COUNT` is zero, so the build-time allocator refuses
// to place any region and the analyser reports isolation tier 0.
unsafe impl MemoryProtection for NoProtection {
    const REGION_COUNT: u8 = 0;
    const REQUIRES_POWER_OF_TWO: bool = false;
    const MIN_REGION_SIZE: usize = 0;

    unsafe fn enable() {}
    unsafe fn disable() {}

    unsafe fn install(regions: &[Region]) -> Result<(), ArchError> {
        if regions.is_empty() {
            Ok(())
        } else {
            Err(ArchError::UnsupportedRegion)
        }
    }
}
