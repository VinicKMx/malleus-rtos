//! Bounded-time mutual exclusion against interrupts.

/// A scoped interrupt mask.
///
/// Malleus deliberately exposes *two* flavours of critical section, because
/// blanket interrupt disabling is the single most common way an RTOS quietly
/// destroys its own interrupt latency guarantee:
///
/// - [`CriticalSection::enter`] masks every maskable interrupt. Reserved for
///   the handful of kernel operations that touch state shared with the
///   fault handler.
/// - [`CriticalSection::enter_below`] masks only interrupts at or below a
///   given priority. Everything else in the kernel uses this, so a hard
///   real-time ISR keeps running while lower-priority kernel work proceeds.
///
/// # Safety
///
/// Implementors must guarantee that entering and leaving is correctly nested
/// and that the previous mask is restored exactly. A section that leaks a
/// disabled state silently converts the system from real-time to not.
pub unsafe trait CriticalSection: Sized {
    /// Mask all maskable interrupts. Returns a token that restores the
    /// previous mask on drop.
    ///
    /// # Contract
    ///
    /// - O(1), a bounded and documented number of cycles.
    /// - Nestable.
    ///
    /// Every use of this in the kernel is individually justified in
    /// `docs/internals/critical-sections.md`, with a measured worst-case
    /// duration. That document is checked by CI against the trace output — a
    /// section that grows past its declared budget fails the build.
    fn enter() -> Self;

    /// Mask only interrupts at or below `priority`, leaving higher-priority
    /// (more urgent) interrupts live.
    ///
    /// # Contract
    ///
    /// - O(1).
    /// - Does not affect interrupt latency for priorities above `priority`.
    fn enter_below(priority: u8) -> Self;

    /// The interrupt priority currently in effect, or `None` if unmasked.
    fn current_mask() -> Option<u8>;

    /// Whether the caller is executing inside an interrupt handler.
    ///
    /// The kernel uses this to reject blocking operations from ISR context at
    /// runtime, backing up the compile-time `IsrSafe` marker in
    /// `malleus-kernel`.
    fn in_interrupt() -> bool;
}
