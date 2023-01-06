//! Per-task CPU state and the context-switch contract.

/// Saved CPU state for one task.
///
/// The kernel treats this as an opaque blob it stores inside the task control
/// block. Only the architecture layer knows its shape.
///
/// # Safety
///
/// A `Context` describes where a task's registers and stack live. Constructing
/// one that points at memory the task does not own, or restoring one whose
/// stack has been freed, is undefined behaviour. Implementors must document
/// exactly which registers are saved by hardware on exception entry and which
/// the switch routine saves manually — the crash-dump decoder depends on that
/// layout being accurate.
pub unsafe trait Context: Sized + Send {
    /// Build the initial context for a task that has never run.
    ///
    /// On first dispatch the task begins executing `entry(arg)`. If `entry`
    /// ever returns, control lands on the architecture's task-exit trampoline,
    /// which raises a fault rather than falling off the end of the stack.
    ///
    /// # Safety
    ///
    /// `stack` must be a region exclusively owned by this task for its entire
    /// lifetime, aligned to the architecture's stack alignment, and large
    /// enough for the architecture's minimum exception frame. `entry` must be
    /// a valid function pointer for the task's execution mode (privileged or
    /// unprivileged).
    unsafe fn initialise(
        stack: &'static mut [u8],
        entry: unsafe extern "C" fn(usize) -> !,
        arg: usize,
        privileged: bool,
    ) -> Self;

    /// Current stack pointer of a suspended task.
    ///
    /// Used for stack high-water measurement and for walking the stack when
    /// building a crash dump. Meaningless while the task is running.
    fn stack_pointer(&self) -> usize;

    /// Request a switch to `next` at the earliest safe point.
    ///
    /// This does **not** switch inline. It marks the target as pending and
    /// triggers the architecture's lowest-priority "deferred switch" exception
    /// (`PendSV` on Cortex-M). The actual register swap therefore always
    /// happens in a known exception context, never in the middle of arbitrary
    /// kernel code.
    ///
    /// # Contract
    ///
    /// - O(1), no allocation, no unbounded loops.
    /// - Safe to call from an interrupt handler.
    /// - Idempotent: requesting a switch twice before it lands is harmless.
    ///
    /// # Safety
    ///
    /// `current` and `next` must both point to live contexts owned by the
    /// scheduler, and the caller must hold the scheduler lock.
    unsafe fn request_switch(current: *mut Self, next: *mut Self);
}
