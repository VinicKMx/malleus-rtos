# Glossary

Terms as Malleus uses them. Several are used differently elsewhere, and where
that is true it is noted — an unmarked disagreement about vocabulary is how two
people have a long argument about nothing.

---

**Blocked** — A task waiting on a kernel object *held by another task*.
Distinguished from **Waiting** because blocked can indicate priority inversion
and waiting cannot. Many RTOSes collapse these; the distinction is the useful
one during diagnosis.

**Capability** — A declared permission for a task to reach a resource:
peripheral, channel, or memory region. Enforced at compile time by scoping and
at run time by the MPU. See [capabilities](09-capabilities.md).

**Channel** — A bounded, typed, unidirectional message queue between two tasks,
declared in the manifest. See [IPC](08-ipc.md).

**Conformance suite** — The checklist an architecture port must pass, on
hardware, to be declared supported. In `malleus_arch::conformance`.

**Crash dump** — A fixed-size record written from the fault handler, persisted
to flash, surviving reset. Read by `cargo malleus dump`.

**Critical section** — A region with interrupts masked. Malleus distinguishes
`enter()` (mask everything) from `enter_below(p)` (mask at or below priority
`p`); nearly all kernel code uses the latter, so hard real-time ISRs keep their
latency.

**Deadline** — The time by which a task's activation must complete, relative to
its release. Defaults to the period. **A missed deadline is a fault in Malleus**,
not a statistic.

**Degraded mode** — A declared, visible system state in which a subsystem has
been shed after repeated failure, and the device continues with reduced
function. Visible to operators and reported in telemetry — never silent.

**Executor** — An `async` scheduler running futures cooperatively within one
priority level. Malleus runs several, one per priority level. An executor does
not preempt.

**Fault** — Any of: memory access violation, stack overflow, illegal operation,
arithmetic error, panic, **deadline miss**, watchdog timeout, or kernel
invariant violation.

**Firm real-time** — A late result is worthless but not dangerous. Contrast
**hard** (a late result is a system failure) and **soft** (a late result is
degraded but useful).

**Idle priority** — Priority 0, reserved for the kernel's idle task. The
validator rejects an application task declared there (M0004).

**Instant** — A point on the monotonic timeline, in ticks since boot, as a
`u64`.

**Isolation tier** — 0 (none, type safety only), 1 (ARMv7-M MPU, power-of-two
regions), 2 (ARMv8-M MPU or RISC-V PMP, fine-grained). **Always reported, never
assumed.**

**MalleusRT** — The kernel proper. **Malleus RTOS** is the whole platform:
kernel, runtime, analyser, tooling, board support. The distinction matters
because most of what makes the platform worth using lives outside the kernel.

**Manifest** — `malleus.toml`, the declarative description of a system. See
[system manifest](07-system-manifest.md).

**Monotonic** — Never runs backwards, never jumps, unaffected by wall-clock
adjustment, continues across tickless idle. Resets only on reboot, and the
reboot is recorded.

**MPU / PMP** — Memory Protection Unit (ARM) / Physical Memory Protection
(RISC-V). Hardware enforcing which addresses unprivileged code may access.
Neither provides virtual memory.

**Overflow policy** — What a full channel does to an arriving sender: `block`,
`reject`, `drop-oldest`, `drop-newest`. **No default** — every channel declares
one.

**Priority ceiling** — A mutex protocol raising an acquiring task immediately to
the highest priority of any task that may acquire it. Bounds each task to one
block per activation and prevents deadlock among ceiling-protected mutexes.

**Priority inheritance** — A mutex protocol temporarily raising a holder to the
priority of its highest-priority waiter. Cheaper than ceiling, needs no
declaration, does not prevent deadlock.

**Priority inversion** — A high-priority task waiting on a resource held by a
lower-priority one. Bounded by inheritance or ceiling. When it is visible in the
declaration itself, the validator reports it as inversion *by construction*
(M0018).

**Ready set** — The bitmap of priorities that currently have runnable work. One
word; `highest()` is one `CLZ` instruction.

**Response time** — Worst-case elapsed time from a task's release to its
completion, including all preemption and blocking. **Distinct from WCET**, which
is CPU time alone. A task can need 8 ms of CPU and have a 17 ms response time,
and confusing the two is the most common error in reading a schedulability
report.

**Restart policy** — `never`, `on-fault` with a budget and window, or
`escalate-to-reset`. Declared per task.

**Schedulability** — Whether every task provably meets its deadline. Computed by
exact response-time analysis. Verdicts: `PASS`, `FAIL`, `UNKNOWN`.

**Tick** — The unit of the monotonic timeline. Rate is board-declared; the
default is 1 MHz.

**Tickless** — No periodic timer interrupt. The timer is programmed for the next
deadline, so an idle system sleeps as long as it genuinely can.

**UNKNOWN** — A schedulability verdict meaning not enough was declared to
decide. A **first-class outcome**, not an error: it names what to measure next.
Malleus will not print `PASS` for a system it cannot check.

**WCET** — Worst-case execution time: CPU time for one activation, excluding
preemption. In Malleus it is **declared, not derived** — deriving a sound WCET on
a cached Cortex-M7 is a research problem, not a build step. See
[real-time model](04-realtime-model.md#4-the-wcet-problem).

**Waiting** — A task waiting for time to pass or for an external event.
Contrast **Blocked**.
