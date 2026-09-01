# ADR-0013 — Record unsafe review controls during single-maintainer development

> **Status:** Accepted
> **Date:** 2026-09-01 · **Checkpoint:** 1

## Context

[ADR-0010](0010-unsafe-code-policy.md) requires two reviewers for changes
involving `unsafe`. There is currently one maintainer. Its cost discussion
already acknowledges that this rule is least effective in a single-maintainer
project. Requiring an unavailable reviewer leaves integrated work with an
unresolved gate; calling self-review independent review would hide the gap.

The [startup](../../crates/malleus-arch-cortex-m/src/startup.rs) and
[fault capture](../../crates/malleus-arch-cortex-m/src/fault.rs) contain the C1
code affected by this decision. The extended-frame offset defect demonstrates
that a documented invariant can be wrong. A checklist and a test written from
the same wrong assumption can agree and still be wrong.

This decision defines a replacement for rule 5 only. It does not claim that
these controls provide the independence or assurance of a second reviewer.
Acceptance does not by itself complete the retroactive evidence records.

## Decision

Replace only item 5 of ADR-0010's Additional rules while the project has a
single maintainer. For every new or changed `unsafe` block in the kernel or an
architecture crate, require both of the following controls. Apply the same
record to unsafe functions, traits, implementations, and unsafe attributes
when they express the safety contract affected by a change.

1. **A recorded checklist per block or construct.** Identify every invariant,
   its owner (the caller, linker, hardware, or named software component that
   establishes it), failure conditions, and how that invariant is verified.
2. **Negative-test evidence.** Record a regression test that fails when the
   invariant is violated or the defect is reintroduced. For hardware-facing
   code, physical fault injection or observation of the relevant failure
   scenario counts as this test. Compilation alone does not.

All other ADR-0010 requirements remain unchanged: `SAFETY:`, `# Safety`,
confinement, minimal scope, named invariants, inventory, and lints. Two reviewers
become mandatory again when a second maintainer joins. There is no fixed expiry
or checkpoint trigger. Retain the recorded evidence when that happens.

### Public evidence record

Store the checklists in versioned `docs/internals/unsafe-review/<crate>.md`,
linked to the source and the inventory produced by `cargo xtask unsafe`. The
existing inventory is a count of constructs, not a completed review; reconcile
its scope with the records and explain any counting differences. The record
format is defined here; the retrospective records are separate from this ADR.

Each entry must contain:

- Stable record ID, source path and symbol/block locator, reviewed commit or
  source revision, date, and reviewing maintainer.
- One checklist row per invariant: invariant, owner, failure condition,
  verification method, result, and evidence reference.
- Negative scenario, expected rejection/failure, exact command or physical
  procedure, target/configuration, and observed result. A regression test must
  include evidence that reintroducing the defect makes it fail; a physical
  observation must distinguish the intended failure path from stale or
  debugger-created state.
- Limitations, unresolved items, and the maintainer's disposition. Distinguish
  host-tested, target-built, and hardware-tested evidence. Never label a
  source-only argument or an unexecuted test as observed behavior.

Review evidence must be public and versioned, not solely in a private plan or
local journal. Publish the relevant commands, observations, build hashes, and
invariant-to-evidence mapping; redact secrets without removing what is needed
to assess the claim. Private logs alone cannot satisfy this requirement.

### Hardware-only and untestable conditions

For pure MMIO or assembly barriers, first identify the observable condition:
for example, the configured fault reaching the expected handler after a
physical reset. Record the physical setup, firmware hash, stimulus and result,
and state which invariant the observation actually exercises. A planted fault
does not by itself verify every MMIO address, ordering property, or failure
condition. Disassembly can support an argument but is not physical behavior.

If neither a host test nor a relevant physical experiment can test an
invariant, record `NOT TESTABLE`, the concrete reason, alternatives considered,
source/manual or disassembly evidence, and the remaining risk. The maintainer
must explicitly accept that justification in the public record or leave the
item unresolved. This is a visible limitation of the negative-test control,
not an executed test or a silent exemption. Missing hardware or an unperformed
available test is `NOT PERFORMED`, not `NOT TESTABLE`.

### Retroactive scope

Apply both controls to the C1 unsafe code integrated in
`215816f`, `f926464`, and `ea5cb1d`. Those commits become regularized when their
records satisfy the two controls, including explicit disposition of any
untestable condition. No second review remains pending for those commits once
that application is complete. Acceptance alone does not complete it.

The maintainer decided on 2026-09-01 to include the 14 inventory constructs in
`malleus-arch` from C0 in the C1 retrospective work, under the same two controls.
They become regularized only when their records satisfy those controls; this
decision does not declare their evidence complete.

## Consequences

### What this buys

An auditable invariant-to-evidence record and explicit failure checks replace
an unavailable second review during single-maintainer development. Existing
safety documentation and enforcement continue to apply.

### What it costs

The author can miss the same reasoning error in the implementation, checklist,
and test. These controls do not detect an assumption the author does not
question; an independent reviewer might. This is reduced review independence,
not equivalent assurance. Writing and maintaining public evidence takes time,
and hardware experiments may be unavailable or leave explicitly accepted gaps.

### What it forecloses

A passing build, a private checklist, or a delayed self-review cannot be called
a second review or substitute for the two recorded controls. This decision does
not certify code or weaken any other unsafe requirement.

## Alternatives considered

- **Keep ADR-0010 intact and wait for a reviewer.** Preserves independence but
  leaves the current single-maintainer work blocked without an available
  reviewer; rejected as the operating policy for this period.
- **Self-review after a waiting interval.** Time can improve attention but
  does not supply an independent reasoning model or require falsifying a
  safety assumption; rejected as the replacement control.
- **Checklist without a negative test.** Can repeat the implementation's
  assumptions without challenging them; rejected.
- **Remove the review gate without replacement evidence.** Makes progress
  possible by losing accountability for unsafe invariants; rejected.

## Revisit when

A second maintainer joins: restore ADR-0010's two-reviewer rule. Reassess the
controls if a defect escapes both the checklist and its negative evidence;
that reassessment does not introduce an automatic expiry or checkpoint gate.
