# ADR-0007 — Faults are contained, recorded, and supervised

> **Status:** Accepted · **Checkpoint:** 0 (design) · 3 (demonstrable)

## Context

An embedded device fails in the field. Today, the typical outcomes are:

1. **Watchdog reset.** The device reboots. Whatever it was controlling stops. No
   record of why.
2. **Silent corruption.** The fault damages unrelated state and the device
   misbehaves in a way nobody connects to the cause.
3. **Hang.** The device stops responding and stays that way until someone
   power-cycles it.

All three share a property: **the field engineer gets no evidence.** A device
that "just reboots sometimes" is one of the most expensive things in embedded
engineering, because diagnosing it requires reproducing it, and it does not
reproduce on a bench.

The full argument is in the [fault model](../design/05-fault-model.md). This ADR
records the decision.

## Decision

**A fault is attributed, contained, recorded, and recovered from according to a
declared policy.**

Four commitments:

**1 · Attribution.** Every fault names the task, the address, the program
counter, and the stack pointer, where the hardware captures them
(`ARCH-FAULT-001`).

**2 · Containment.** With protection hardware, a fault stops one task. The
`may_have_escaped(isolated)` predicate decides whether restart is safe; a memory
fault on a tier-0 board escalates rather than restarting.

**3 · Recording.** A crash dump is written from the fault handler, persisted to
flash, survives reset, and is read later by `cargo malleus dump`. No allocation,
no locks, bounded time, corruption-tolerant — a partially written dump must
still yield its readable prefix, because the interesting case is the device that
died mid-write.

**4 · Supervision.** Declared per task:

```toml
restart = "never"
restart = { on-fault = { budget = 5, window = "60s" } }
restart = "escalate-to-reset"
```

Two further decisions that are easy to get wrong:

- **A deadline miss is a fault**, not a statistic. In a control system it is a
  failure of the same seriousness as a bad pointer; only the consequences arrive
  later.
- **Peers are notified on restart.** A waiter on a resource held by a faulted
  task gets `WakeReason::HolderFaulted` and must decide, exactly as a poisoned
  lock forces a decision in `std`. A peer that continues as though nothing
  happened — holding a stale session or sequence number — is a rich source of
  second-order bugs that appear only after a field restart.

## Consequences

### What this buys

- The product keeps working through a failure in a non-essential subsystem.
- Field failures produce evidence.
- Reboot loops are diagnosable rather than merely observable.
- Degraded mode is a declared, visible state an operator can see.

### What this costs

- **Fault-handler code is the hardest code in the system.** It runs when the
  system is already broken, cannot allocate, cannot lock, cannot call the
  kernel, and must terminate before the watchdog. It will be the most reviewed
  and most tested code in the project, and it will still be where the subtle
  bugs are.
- **Flash wear** from crash dumps. Bounded by a dump budget; a device faulting
  often enough to wear its flash has a bigger problem, but the bound must exist.
- **Restart semantics are genuinely subtle.** "Restart the task" is easy to say
  and hard to make correct in the presence of peers, in-flight DMA, and
  half-completed protocol exchanges.
- **RAM for dump buffers**, reserved permanently for an event that may never
  happen.
- Requires protection hardware to mean anything, and says so on boards without.

### What it forecloses

Nothing. It does mean the kernel must be small enough to review as trusted code,
because a fault *inside* the kernel escalates to reset — there is no containment
above it.

## Alternatives considered

**Watchdog reset only** — the status quo. Simple, robust, and it does work in
the sense that the device comes back. Rejected because it stops whatever the
device was controlling and produces no evidence.

**Panic-and-halt.** Rust's default. Rejected as unacceptable for a controller:
halting with a motor energised is a safety problem, not a debugging convenience.

**Restart everything on any fault.** Simpler than selective restart and avoids
all the subtlety around peers. Rejected because it discards the main benefit —
the motor loop surviving a network failure is the entire point.

**Let the application handle faults** via a hook. Maximum flexibility. Rejected
because fault handling is exactly the code an application team is least equipped
to write and least able to test, and because a fault hook running in application
context cannot be trusted to be intact.

**Erlang-style supervision trees.** Richer and well-proven. Rejected as too much
machinery for a microcontroller with 64 tasks — but the *ideas* (let it crash,
supervise, restart, escalate) are directly borrowed, and the intellectual debt
should be acknowledged.

## Revisit when

- The Checkpoint 3 demonstration is attempted. It will surface everything wrong
  with this design, and it is scheduled early for exactly that reason.
- Restart semantics prove insufficient for a real reference application — most
  likely around in-flight DMA or a protocol exchange that cannot simply be
  abandoned.
