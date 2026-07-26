# ADR-0009 — Reuse the Rust embedded ecosystem; build no alternatives

> **Status:** Accepted · **Checkpoint:** 0 (policy) · 2+ (implementation)

## Context

A new RTOS faces a bootstrapping problem. Nobody adopts a system with no
drivers; nobody writes drivers for a system with no users.

There are two ways out. Build your own ecosystem and hope momentum arrives — the
path Zephyr took successfully, with an enormous engineering budget and years.
Or become compatible with an ecosystem that already exists.

Malleus does not have that budget. It also does not need it: the Rust embedded
ecosystem already has well-designed, widely-implemented trait crates, and a
large body of drivers written against them.

There is a related temptation worth naming. A project defining its own logging
format, its own debug protocol, its own bootloader, and its own network stack
gets a coherent, controlled experience — and spends most of its effort on
problems other people have already solved better, while making every one of its
users learn a private idiom.

## Decision

**Implement the ecosystem's traits. Build no alternatives to solved problems.
Contribute upstream instead.**

### Reuse

| Concern | We use | We do not build |
|---|---|---|
| Digital I/O, SPI, I²C | `embedded-hal`, `embedded-hal-async` | our own HAL traits |
| Byte streams | `embedded-io`, `embedded-io-async` | our own I/O traits |
| Flash and storage | `embedded-storage` | our own storage traits |
| CAN | `embedded-can` | — |
| Network | `embedded-nal`, `smoltcp` / `embassy-net` | our own TCP/IP stack |
| Logging | `defmt` | our own log format |
| Debug and flash | `probe-rs` | our own probe software |
| Bootloader, OTA | MCUboot | our own bootloader |
| Cryptography | established RustCrypto crates | our own crypto |

### The rule

> **A driver that works on Embassy should work on Malleus without being
> rewritten.**

This is a requirement with a test attached, not an aspiration. Malleus competes
on isolation, build-time analysis, and diagnostics — not by asking the community
to port its drivers twice.

### What we do build

Only what does not exist and is core to the thesis: the kernel, the manifest and
its analyser, the code generator, the capability model, the fault and supervision
machinery, and the trace format. The trace format is the one arguable case — it
is a wire format, and wire formats are normally a place to reuse. It is built
because it must be encodable from a fault handler with no allocation and no
locks, which existing formats do not target. That reasoning is in
`malleus-trace`, and if someone points at an existing format that meets the
constraint, we should adopt it.

### Contribute upstream

Where a trait crate is missing something Malleus needs, the first move is a
patch upstream, not a local fork. A trait we define privately is a trait the
ecosystem cannot use.

## Consequences

### What this buys

- Drivers exist on day one.
- `probe-rs` and `defmt` mean debugging works with tools people already have.
- Not owning a TCP stack, a bootloader, or a crypto library is an enormous
  reduction in both effort and *security surface*. Writing your own crypto is
  the classic mistake; writing your own TLS is the same mistake with more steps.
- Effort concentrates on the four things that are actually differentiating.

### What this costs

- **Upstream constraints.** `embedded-hal` was not designed with capability-gated
  peripheral access in mind. Fitting a driver holding an ownership token into a
  capability model may be awkward, and where it is, the awkwardness lands on us.
- **Version churn.** Tracking several trait crates across breaking releases is
  ongoing maintenance that never finishes.
- **Less control over the experience.** `defmt`'s model is `defmt`'s model.
- **Slower to fix upstream bugs**, and sometimes we will need a temporary fork —
  which must be temporary, with an upstream PR open.
- **Async trait maturity.** `embedded-hal-async` is younger than its blocking
  sibling; parts of it may not be stable enough when Checkpoint 2 needs them.

### What it forecloses

A fully controlled, self-consistent, private ecosystem. This is a real loss for
polish and the right trade anyway.

## Alternatives considered

**Build our own trait ecosystem.** Perfect fit to the capability model, full
control. Rejected: it is the single most reliable way to guarantee no adoption.

**Reuse only where convenient, fork where awkward.** Pragmatic-sounding, and the
default outcome if this ADR did not exist. Rejected because "awkward" is a
judgement made under deadline pressure, and the accumulated result is a private
ecosystem arrived at one reasonable decision at a time.

**Target Zephyr's driver model** via a compatibility layer. Access to a far
larger driver set. Rejected as an enormous amount of C interfacing that would
dominate the project and pull the design toward Zephyr's assumptions.

## Revisit when

- A trait crate proves fundamentally incompatible with capability-gated access.
  The response is an upstream RFC, and only if that fails, a documented
  divergence with a written rationale — recorded as a new ADR, not as a quiet
  exception.
- `embedded-hal-async` turns out to be insufficiently stable at Checkpoint 2.
  That would be a schedule problem, not a reason to build alternatives.
