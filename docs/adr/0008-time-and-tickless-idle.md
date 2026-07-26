# ADR-0008 — Monotonic 64-bit ticks with tickless idle

> **Status:** Accepted · **Checkpoint:** 0 (design) · 1 (working)

## Context

Time is the substrate of a real-time system, and three choices shape everything
built on it: the unit, the width, and whether a periodic tick interrupt runs.

The classic RTOS design is a fixed periodic tick — 1 kHz is traditional — with
time counted in ticks since boot. It is simple, and it has two costs that matter
here: resolution is limited to the tick period (1 ms is far too coarse for a
500 µs deadline), and the tick interrupt fires forever, preventing deep sleep.

## Decision

**Monotonic time, 64-bit, counted in ticks of a board-declared rate, with a
programmable one-shot alarm and no periodic tick.**

- **Default 1 MHz**, giving microsecond resolution. Boards may declare otherwise.
- **`u64`.** At 1 MHz this does not wrap for 584,000 years. Wrap handling is a
  classic source of bugs that appear after 49.7 days of uptime — precisely the
  bug you cannot find on a bench, and precisely the uptime an industrial
  controller reaches. Eight bytes is a cheap way to delete an entire failure
  class.
- **Ticks, not nanoseconds.** Converting on a Cortex-M without a hardware
  divider costs more than the abstraction saves. Conversion happens once, in the
  code generator, where it is free.
- **Tickless.** The timer is programmed for the next deadline. No periodic
  interrupt, so an idle system sleeps as long as it genuinely can.
- **Monotonic means monotonic**: never backwards, never a jump, unaffected by
  wall-clock adjustment, continuing across tickless idle. It resets only on
  reboot, and the reboot is recorded so a host tool can stitch timelines across
  a restart.
- **`saturating_since`**, not wrapping subtraction. A negative duration in a
  deadline calculation is always a bug; saturating turns it into a visible "zero
  time left" instead of a 584,000-year timeout.
- **Periods must be exact tick multiples**, warned by the validator (M0009). A
  fractional ratio produces systematic jitter that looks like a hardware fault.
- **Periodic activation does not drift.** `Period::advance` moves an absolute
  reference rather than sleeping for an interval. Missed activations are
  counted, not silently skipped, and the catch-up is arithmetic — never a loop,
  so a badly overrunning task cannot take unbounded kernel time at the moment
  you can least afford it.

## Consequences

### What this buys

- Microsecond deadlines are expressible and measurable.
- Deep sleep is possible; idle power is a published benchmark rather than a
  casualty of the design.
- No wrap-handling bugs.
- No drift in periodic tasks.
- Overruns are visible instead of quietly absorbed.

### What this costs

- **Tickless is harder to get right than a periodic tick.** The classic race —
  a deadline that expires between the decision to sleep and the programming of
  the alarm — is subtle and produces a hang, not a glitch. It is
  `ARCH-TIME-002` in the conformance suite, tested explicitly, because
  "an alarm in the past must fire immediately rather than being lost" is the
  kind of requirement that is obvious once stated and easy to miss otherwise.
- **64-bit arithmetic on a 32-bit core** costs cycles on every deadline
  computation.
- **Reading a wrapping hardware counter monotonically** requires care under
  concurrent access from a task and an ISR (`ARCH-TIME-001`, tested over two
  full wraps).
- **Some timers cannot express long intervals.** `max_alarm_ticks()` exists so
  the kernel programs an intermediate wake-up and re-arms rather than silently
  truncating.
- **1 MHz costs more interrupts than 1 kHz** when timers are actually in use.
  Tickless recovers most of it, but not all.

### What it forecloses

Nothing significant. A board that genuinely wants a periodic tick can declare a
rate and set alarms at fixed intervals; it simply gains nothing by doing so.

## Alternatives considered

**Fixed periodic tick, e.g. 1 kHz.** Simple, well-understood, easy to get right.
Rejected: 1 ms resolution cannot express a 500 µs deadline, and the perpetual
interrupt forecloses deep sleep. These are exactly the two properties the target
domain needs.

**Nanoseconds as the unit.** Universal, no board-specific rate, no alignment
warnings. Rejected because of conversion cost on the device. `u64` nanoseconds
also wraps in 584 years, so the width argument is unchanged — this is purely
about arithmetic cost on a core without a divider.

**32-bit ticks with explicit wrap handling.** Cheaper arithmetic. Rejected: wrap
bugs are a well-known, recurring, hard-to-reproduce failure class, and 4 extra
bytes per timestamp is not a trade worth making.

**Two clocks: a coarse tick for scheduling and a fine counter for measurement.**
Used by some systems. Rejected as more machinery than value, and as an
invitation to the bug where the two disagree.

## Revisit when

- Power measurement shows 1 MHz is too expensive at idle for battery-powered
  targets. The fix is a lower default rate per board, which the design already
  supports — not a change to the model.
- A target's timer hardware cannot support the model cleanly. That would be
  useful evidence about the abstraction's portability and should be recorded
  here.
