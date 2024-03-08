# Architecture Decision Records

An ADR records **one decision**: what was decided, what forces led there, what
was rejected, and what it costs. Not what the code does — the code says that.
An ADR exists so that in two years, when someone asks "why on earth is it done
this way", the answer is written down rather than reconstructed from memory or,
worse, silently reversed.

## Rules

1. **One decision per ADR.** If it needs "and", it is two ADRs.
2. **Immutable once accepted.** Changed your mind? Write a new ADR that
   supersedes this one, and mark this one superseded. Editing history to look
   consistent destroys the record's only value.
3. **State the cost.** An ADR listing only advantages is a sales pitch. Every
   decision here costs something; say what.
4. **Name what was rejected, and why.** The alternatives are usually the more
   informative half.
5. **Link to the code.** An ADR describing something that does not exist should
   say so, with its checkpoint.

## Status values

| Status | Meaning |
|---|---|
| **Proposed** | Under discussion; not binding |
| **Accepted** | Binding. Contradicting code is a bug. |
| **Superseded by ADR-NNNN** | Replaced; kept for the historical record |
| **Deprecated** | No longer applies, nothing replaced it |

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-target-architecture.md) | Cortex-M7 for bring-up, ARMv8-M as the strategic target | Accepted |
| [0002](0002-static-system-definition.md) | The task set is static, declared in a manifest | Accepted |
| [0003](0003-no-kernel-heap.md) | The kernel does not allocate | Accepted |
| [0004](0004-scheduling-policy.md) | Fixed-priority preemptive, with async executors inside priority levels | Accepted |
| [0005](0005-memory-isolation.md) | Isolation via MPU/PMP, gated by declared capabilities | Accepted |
| [0006](0006-typed-ipc.md) | IPC is typed and generated from the manifest | Accepted |
| [0007](0007-fault-model.md) | Faults are contained, recorded, and supervised | Accepted |
| [0008](0008-time-and-tickless-idle.md) | Monotonic 64-bit ticks, tickless idle | Accepted |
| [0009](0009-ecosystem-interoperability.md) | Reuse the Rust embedded ecosystem; build no alternatives | Accepted |
| [0010](0010-unsafe-code-policy.md) | Every `unsafe` block is documented and inventoried | Accepted |
| [0011](0011-toolchain-and-msrv-policy.md) | Pinned toolchain, declared MSRV, reviewed bumps | Accepted |
| [0012](0012-licensing.md) | Dual MIT OR Apache-2.0 | Accepted |

## Template

```markdown
# ADR-NNNN — Title in the imperative

> **Status:** Proposed | Accepted | Superseded by ADR-NNNN
> **Date:** YYYY-MM-DD · **Checkpoint:** N

## Context
The forces at play. What makes this a decision rather than an obvious choice.

## Decision
What we are doing. Imperative, specific, testable.

## Consequences
### What this buys
### What this costs
### What it forecloses

## Alternatives considered
Each with why it was rejected. Be fair to them.

## Revisit when
The concrete signal that would justify reopening this.
```

That last section is unusual and deliberate. A decision without a stated
revisit condition tends to become permanent by default, long after the forces
that produced it have changed.
