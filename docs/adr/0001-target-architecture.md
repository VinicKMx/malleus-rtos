# ADR-0001 — Bring up on Cortex-M7; treat ARMv8-M as the strategic target

> **Status:** Accepted · **Checkpoint:** 0

## Context

Malleus needs a first architecture. The choice sets the shape of the memory
protection design, the context-switch code, and which trade-offs are visible
early enough to influence the design rather than being retrofitted.

The obvious choice is **Cortex-M33** (ARMv8-M Mainline): TrustZone, an MPU with
32-byte-granular regions and no alignment requirement, and a modern exception
model. It is where the industry is going and where Malleus's isolation story is
most compelling.

The board actually on the bench is a **Nucleo-F767ZI**: Cortex-M7 (ARMv7E-M),
216 MHz, 2 MB flash, 512 KB RAM, on-board Ethernet PHY. No TrustZone. An
ARMv7-M MPU with 8 regions that must be **powers of two and naturally aligned**.

Two forces pull against each other:

1. Bring-up on hardware you can probe, reset, and stare at with a logic analyser
   is worth far more than bring-up on the architecturally preferable part you
   have to order and wait for. Early kernel work is dominated by "why did it
   hard-fault", and that is answered on a bench, not in simulation.
2. Designing against the easier MPU first risks producing a region allocator
   that cannot be made to work on the constrained one.

## Decision

**Bring up on Cortex-M7 (Nucleo-F767ZI). Design every abstraction against the
ARMv7-M constraint. Treat ARMv8-M as the strategic target for isolation.**

Concretely:

- `malleus-arch` exposes `MemoryProtection::REQUIRES_POWER_OF_TWO` and
  `MIN_REGION_SIZE`, so the region allocator is written against the constrained
  case from the first line.
- The manifest validator reports ARMv7-M alignment padding as a warning (M0012)
  with the exact byte cost, on every board, so the cost is visible even to
  engineers targeting ARMv8-M.
- TrustZone is **not** designed around. It is a Checkpoint 4+ capability, and
  nothing in the core design may assume a secure/non-secure split exists.
- Cortex-M33 becomes the second port, and the point at which the architecture
  contract is proven to be a real abstraction rather than a description of one
  chip.
- RISC-V with PMP comes after both, and is the point at which the contract is
  proven across vendors.

## Consequences

### What this buys

- Kernel bring-up starts immediately, on hardware with Ethernet — which the
  flagship reference application needs anyway.
- The region allocator is designed against the *harder* constraint. An allocator
  that works on ARMv7-M generalises to ARMv8-M; one designed for ARMv8-M does
  not generalise backwards.
- The M7's caches, store buffer, and flash wait states force the WCET question
  to be confronted honestly and early, rather than on a simple M0 where
  optimistic numbers would go unpunished.
- Cortex-M4 and M7 remain enormously deployed in industrial equipment. This is
  not a legacy target.

### What this costs

- **Padding.** A 3 KiB stack occupies a 4 KiB region. On a 512 KB part this is
  affordable; on a 64 KB part it would not be. The tool reports it per task.
- **Only 8 regions.** With kernel code, kernel data, task stack, task data, and
  peripherals, a task's capability budget is genuinely tight. The build-time
  allocator must produce a clear error naming the offending task rather than a
  runtime failure.
- **No TrustZone**, so the secure-boot and key-isolation story waits for
  ARMv8-M hardware.
- Cortex-M7 lazy FPU stacking is subtle, and getting it wrong leaks one task's
  floating-point registers into another. This is `ARCH-CTX-003` in the
  conformance suite, tested with a task that never touches the FPU.

### What it forecloses

Nothing permanently. It does mean the isolation story is weaker on the
demonstration board than it will be on the strategic one, and any benchmark or
demo must say which tier it ran at.

## Alternatives considered

**Cortex-M33 first (order a board, wait).** Architecturally cleaner and the
better long-term target. Rejected because it delays bring-up by weeks and
because designing the region allocator against the permissive MPU first is the
likelier way to end up with something that cannot be back-ported.

**QEMU only, no hardware.** Fastest to start, and QEMU's `mps2-an505` gives a
Cortex-M33. Rejected as a *primary* target: QEMU does not model cache effects,
flash wait states, or real interrupt latency, which is precisely the domain of
this project. QEMU remains valuable for CI, and is used there.

**RISC-V first.** Attractive openness and a cleaner PMP model. Rejected because
the industrial installed base is overwhelmingly ARM today, and because the
hardware on the bench is ARM.

**Support M4, M7, and M33 simultaneously from day one.** Rejected: three ports
before any of them is proven produces three half-ports and an architecture
contract shaped by nothing. See [non-goals](../design/12-non-goals.md).

## Revisit when

- The Cortex-M33 port is complete and the contract has survived it — at which
  point the "strategic target" language should be replaced with a real support
  matrix.
- Someone demonstrates that the ARMv7-M region allocator cannot represent a
  realistic industrial task set within 8 regions. That would be evidence the
  isolation design needs rethinking, not just retuning.
