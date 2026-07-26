# Porting guide

> **Status:** the contract is defined; no port exists yet. The first
> (Cortex-M7) lands in Checkpoint 1, and this document will be corrected by the
> experience of writing it.

## The contract

A port implements one trait, `malleus_arch::Arch`, which bundles four
independent concerns:

| Associated type | Responsibility |
|---|---|
| `Context` | Saving and restoring a task's CPU state |
| `Critical` | Bounded-time mutual exclusion against interrupts |
| `Timer` | A monotonic clock and a programmable one-shot alarm |
| `Memory` | Hardware memory protection, or `NoProtection` |

Plus four constants: `NAME`, `STACK_ALIGN`, `STACK_GROWS_DOWN`, and
`PRIORITY_LEVELS`.

That is the entire surface. If it compiles against these traits and passes the
conformance suite, the kernel runs on it.

## A port is not "supported" because it compiles

It is supported when it passes every `Required` item in
`malleus_arch::conformance` **on hardware**. Not in QEMU — QEMU does not model
cache effects, flash wait states, or real interrupt latency, which is precisely
the domain this project claims to serve.

The checklist lives in the contract crate on purpose: adding a kernel guarantee
means adding a requirement there, which immediately marks every port as having
an unmet requirement. There is no way to add a guarantee without confronting
what it costs each port.

## What the requirements are actually checking

Most are obvious. Four are not, and they are the ones that catch real ports:

**`ARCH-CTX-003` — lazy FPU stacking must not leak.** On Cortex-M with lazy
floating-point stacking, the hardware defers saving FPU registers. Get the
interaction with context switching wrong and one task's floating-point state
appears in another. Tested with a task that never touches the FPU, because that
is the task where the leak is visible.

**`ARCH-CS-002` — `enter_below(p)` must actually leave higher priorities live.**
The easy implementation masks everything and satisfies the type signature. It
also silently converts the system from real-time to not, and nothing else in the
suite would notice.

**`ARCH-TIME-002` — an alarm set in the past must fire immediately.** The
classic tickless race: a deadline expires between deciding to sleep and
programming the alarm. Getting this wrong produces a hang, not a glitch, and it
happens rarely enough to reach production.

**`ARCH-MPU-002` — no stale region survives a context switch.** Task B must not
reach task A's stack immediately after A is suspended. An implementation that
programs the new regions without invalidating the old ones passes every other
isolation test and provides no isolation.

That last one generalises: **an isolation failure leaves no trace.** Every other
bug in this system announces itself; a protection boundary that was never
programmed correctly produces a system that appears to work perfectly, until the
day containment is needed. That is why these run on hardware and why the region
layout is a reviewed artefact.

## Declaring honestly

**Isolation tier.** A board without protection hardware uses
`memory::NoProtection`, which is tier 0. This is not a silent fallback: the
analyser reports it, and any task declaring `restart = "on-fault"` becomes a
build error. Restarting a task you cannot contain is a false promise.

**NVIC priority bits.** ARM specifies up to 8; each vendor wires up a subset —
4 on STM32, 3 on many nRF parts, 2 on RP2xxx. The board crate supplies the true
value. The kernel never guesses, because guessing makes priority comparisons
silently incorrect for a subset of levels, which produces a system that mostly
works.

**What is untested.** Say so. A port with an honest gap is more useful than one
with an implied guarantee.

## Steps

1. Implement the four associated types.
2. Write the reset vector, `.data`/`.bss` initialisation, and exception entry.
3. Implement the deferred context switch (`PendSV` on Cortex-M) — the switch
   always happens in a known exception context, never inline in kernel code.
4. Implement `enter_below` using a priority-masking register (`BASEPRI`), not a
   global disable.
5. Implement the monotonic timer and alarm, tickless.
6. Implement MPU region programming, with a guard region below every stack.
7. Route every hardware fault to the kernel fault handler with the task,
   address, and PC recoverable.
8. Run the conformance suite on hardware.
9. Publish the benchmarks — min, mean, p99.9, and observed maximum. Not just the
   mean; in real-time work the mean is close to useless.

## Adding a board on an existing architecture

Much smaller: declare the memory map, the NVIC priority bits, the tick source
and its rate, the peripherals and their capability names, and the isolation
tier. Then run the conformance suite anyway — the architecture is shared, the
silicon is not, and errata are real.
