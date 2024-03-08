# ADR-0005 — Isolation via MPU/PMP, gated by declared capabilities

> **Status:** Accepted · **Checkpoint:** 0 (design) · 3 (working)

## Context

A recurring argument in Rust embedded circles: *if the code is safe Rust, why do
you need an MPU?*

The argument is wrong, and it is worth being precise about why, because the
answer shapes the whole design.

Rust's guarantees hold for safe code. They do not extend to:

- `unsafe` blocks — every kernel and every HAL has them;
- **DMA engines**, which write to addresses without consulting anything;
- vendor C libraries and binary drivers;
- a peripheral register misconfigured into writing wherever it likes;
- memory corruption originating in hardware — a bit flip, a marginal supply.

A DMA controller programmed with a bad descriptor will happily write over
another task's stack. No amount of type safety in the code that programmed it
changes that, because by the time the hardware acts, the type system is not
present.

Beyond memory: even with perfectly safe code, a task can deadlock, overrun its
deadline, or panic. Isolation is what makes those *local* events.

## Decision

**Tasks run unprivileged, behind hardware memory protection, with access
determined by capabilities declared in the manifest.**

```toml
[[task]]
name = "telemetry"
capabilities = ["net.mqtt", "ipc.sensor-data"]
```

That task's protection regions cover its own stack and data, the Ethernet
peripheral, and the `sensor-data` channel storage. Nothing else. It cannot reach
the motor PWM register — not by convention, but because the MPU raises a fault.

Supporting decisions:

- **`isolated = true` is the default.** Isolation is opt-out, never opt-in. A
  safe default that must be requested is not a safe default.
- **Three declared tiers** (see [fault model](../design/05-fault-model.md)), and
  the tier in use is always reported.
- **On a tier-0 board, `restart = "on-fault"` is a build error.** Restarting a
  task you cannot contain is a false promise.
- **Region sets are computed at build time**, not derived at runtime. The switch
  path installs a pre-computed set; the allocator has already proved it fits.
- **The kernel runs privileged.** It is small and heavily reviewed precisely
  because it is the layer with no containment above it.

## Consequences

### What this buys

- A fault in one task cannot corrupt another — the precondition for everything
  in the fault model.
- A DMA descriptor built by an isolated task cannot target memory outside that
  task's regions (`ARCH-MPU-003`).
- Capability declarations are *documentation that is enforced*. The manifest says
  what a task can reach, and that statement is true by construction.
- Isolation makes selective restart meaningful rather than theatrical.

### What this costs

- **Context-switch cost.** Installing a region set on every switch is real, and
  it appears in `BENCH-002` rather than being excluded from it.
- **Region scarcity.** ARMv7-M gives 8. Kernel code, kernel data, task stack,
  task data, and peripherals consume most of that, so a task's capability budget
  is genuinely tight. This is the design's sharpest practical constraint.
- **Alignment padding on ARMv7-M.** A 3 KiB stack occupies 4 KiB. Reported per
  task (M0012) so it is a decision, not a surprise.
- **Syscall overhead for IPC.** Isolated tasks cannot share a pointer, so every
  cross-task message goes through the kernel.
- **Driver structure changes.** A driver becomes a task holding a capability,
  not a library any task can call.

### What it forecloses

Zero-copy sharing of arbitrary buffers between isolated tasks. Bulk transfer
requires either a shared region declared by both parties — which weakens the
boundary and must be explicit — or a copy. There is no way around this; it is
what a hardware boundary means.

## Alternatives considered

**Type safety only, no MPU** (the Embassy/RTIC position). Cheaper, simpler,
faster switches. Rejected for the reasons in Context: it does not cover DMA,
`unsafe`, or C. It is a perfectly reasonable choice for systems where a fault
means "reset and move on" — but that is exactly the outcome Malleus exists to
avoid.

**MPU only around the kernel**, tasks mutually trusting. A common middle ground:
protects the kernel from application bugs at much lower cost. Rejected because
the interesting failure is not "the app corrupted the kernel" but "the network
stack corrupted the control loop", and this does nothing about that.

**Process isolation like Tock**, with dynamically loadable processes. Stronger
and more general. Rejected as heavier than needed: Malleus's task set is static,
so the boundary can be static too, which is cheaper and analysable. Tock's model
is right for its goal of running untrusted third-party applications, which is
explicitly not a Malleus goal.

**Software fault isolation** (compiler-inserted bounds checks instead of
hardware). Works without an MPU. Rejected: significant runtime cost, does not
cover DMA or C code, and requires trusting the compiler for a security property
— which is a much larger claim than trusting it for a correctness property.

## Revisit when

- Measurement shows region installation dominates the context switch on
  realistic workloads. The fix would be region-set caching or coarser task
  grouping, not abandoning isolation.
- The 8-region ARMv7-M budget proves unable to express realistic industrial task
  sets. That is a genuine falsifier for the design as it stands, and is called
  out as such in [ADR-0001](0001-target-architecture.md).
