# Architecture contract unsafe review record

**Policy:** [ADR-0010](../../adr/0010-unsafe-code-policy.md) and
[ADR-0013](../../adr/0013-single-maintainer-unsafe-review.md).
**Date:** 2026-09-08. **Reviewing maintainer:** Vinicius Pedrosa;
explicit dispositions remain pending as listed below.
**Reviewed source revision:** `6abbcc4`.

## Inventory and provenance

OBSERVED: `cargo xtask unsafe` reports 15 constructs in `malleus-arch`:
14 legacy constructs plus one test-only block added in `6abbcc4`. Each is
listed below, with its own invariant rows. No production unsafe was added.
The legacy constructs were all introduced in
`eb1bbd3cd3a5422a75f2146b63edcf876492fb0c` (`feat(arch): specify the hardware
architecture contract`), as confirmed by `git blame -l` and the introducing
commit. The later authors-metadata change did not introduce these constructs.

The abstract traits have no executable port implementation here. Testing a
mock or implementing a timer, critical section, context switch or MPU would
not retrospectively test the declaration and would exceed bootstrap scope.
`NOT TESTABLE` below is restricted to runtime behavior of an unimplemented
abstract declaration; it is not an exemption for future implementations.

## ARCH-01 — Context

**Source:** [context.rs](../../../crates/malleus-arch/src/context.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Register/stack layout describes live task-owned state; Send must remain sound | Port implementor; context/stack owner | Dangling, aliased or misdecoded task state | FACT: # Safety contract only; no Context implementation exists here. ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-02 — Context::initialise

**Source:** [context.rs](../../../crates/malleus-arch/src/context.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Exclusive static stack lifetime, ABI alignment and minimum frame; valid entry for mode | Caller owns stack; port builds frame | Out-of-bounds initial frame or entry in wrong execution mode | FACT: documented preconditions. No executable implementation; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-03 — Context::request_switch

**Source:** [context.rs](../../../crates/malleus-arch/src/context.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Both context pointers live and scheduler lock held; switch deferred and bounded | Scheduler caller; port deferred exception implementation | Dangling pointers, concurrent context writes, inline unsafe switch | FACT: documented preconditions/contract. No implementation; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-04 — CriticalSection

**Source:** [critical.rs](../../../crates/malleus-arch/src/critical.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Masks nest and restore exact previous state; priority-bounded mask does not block higher urgency | Port implementor; scoped token lifetime | Leaked disable state or priority latency violation | FACT: # Safety and enter/enter_below contracts. No implementation; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-05 — Arch

**Source:** [lib.rs](../../../crates/malleus-arch/src/lib.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Associated implementations uphold contracts; real required conformance precedes support claim | Port implementor; maintainer publishing support | Unsound switch/protection implementation called supported | FACT: trait documentation and associated bounds. No Arch implementation; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-06 — MemoryProtection

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Correct region replacement, no stale grants and true hardware conformance | Port implementor; region allocator/caller entitlement | Silent isolation bypass | FACT: contract only for protected ports; NoProtection separately recorded. ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-07 — MemoryProtection::enable

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Programmed regions cover kernel before enable | Caller configures complete regions; port enables unit | Kernel access faults or invalid protection state | FACT: abstract method # Safety. Protected implementation absent; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-08 — MemoryProtection::disable

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Re-enable protection before any return to unprivileged execution | Fault handler/caller; port register writer | Execution continues without containment | FACT: abstract method # Safety. No protected implementation; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-09 — MemoryProtection::install

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Regions belong to incoming task; valid count/alignment/size; replace complete set | Caller capability policy; port validator | Over-grant or stale previous grants | FACT: documented authorization/error contract; no protected implementation. HOST-NP tests only NoProtection rejection; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-10 — unsafe impl MemoryProtection for NoProtection

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Zero regions and no isolation claim; methods do not program hardware | NoProtection implementation; board/analyser tier selection | False claim of protection or silent acceptance of regions | OBSERVED HOST-NP: REGION_COUNT=0, nonempty fails, empty succeeds; FACT: methods contain no hardware operations. |
| Tier-0 warning/restart rejection are not implemented by this trait | Manifest tooling and integrator | Assuming a runtime no-op enforces build-time containment policy | FACT: responsibility outside this type; no physical isolation is claimed; ARCH-P2. |

**Negative evidence and limits:** HOST-NP regression rejects a nonempty set; accepting it in the isolated mutant fails the test. This does not demonstrate hardware isolation.

## ARCH-11 — NoProtection::enable

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| No hardware state is changed and no protection is asserted | NoProtection implementation; caller selects tier 0 | Accidental access/claim of active hardware protection | FACT: empty method body; there is no memory operation to justify. Method-specific negative evidence ARCH-P2. |

**Negative evidence and limits:** The no-op body is inspected directly; HOST-NP checks the type-level rejection guarantee but not these two bodies. Method-specific negative coverage remains ARCH-P2.

## ARCH-12 — NoProtection::disable

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| No hardware state is changed; tier 0 remains explicitly unprotected | NoProtection implementation; tier selection owner | Accidental hardware side effect or implied restored protection | FACT: empty method body. Method-specific negative evidence ARCH-P2. |

**Negative evidence and limits:** The no-op body is inspected directly; HOST-NP checks the type-level rejection guarantee but not these two bodies. Method-specific negative coverage remains ARCH-P2.

## ARCH-13 — NoProtection::install

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Empty set only; nonempty returns UnsupportedRegion without dereferencing addresses | NoProtection implementation; caller owns regions | False success suggesting installed protection | OBSERVED HOST-NP and mutant failure; FACT: branch checks only slice emptiness; no pointer access. |
| Caller-supplied region entitlement remains a documented caller obligation | Calling integrator; host test owns its buffer | Future implementation relies on unauthorized region descriptors | FACT: inherited unsafe contract. Test supplies owned storage; not an MPU proof. |

**Negative evidence and limits:** HOST-NP regression rejects a nonempty set; accepting it in the isolated mutant fails the test. This does not demonstrate hardware isolation.

## ARCH-14 — MonotonicTimer

**Source:** [time.rs](../../../crates/malleus-arch/src/time.rs), symbol/block above.
**Introduced:** `eb1bbd3cd3a5422a75f2146b63edcf876492fb0c`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Time monotonic across ISR/concurrency/idle; alarm not lost; bounded operations | Port implementor and hardware timer | Time reversal, lost wake-up or unbounded ISR work | FACT: documented # Safety and methods. No timer implementation; ARCH-P1. |

**Negative evidence and limits:** NOT TESTABLE as runtime behavior of this declaration alone: it has no executable body/port here. Source contract and caller/implementor owners are recorded; disposition ARCH-P1 is pending.

## ARCH-15 — tests::no_protection_rejects_regions_instead_of_claiming_isolation — unsafe calls

**Source:** [memory.rs](../../../crates/malleus-arch/src/memory.rs), symbol/block above.
**Introduced:** `6abbcc4`. Shared revision/date/reviewing maintainer above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Region and slices remain live, region names only test-owned storage | Test stack storage and slice lifetimes | Dangling descriptor or unauthorized range in unsafe call | FACT: local array outlives both calls; no references to unrelated memory. OBSERVED HOST-NP. |
| Only NoProtection is called; no hardware or descriptor dereference | Concrete callee NoProtection, not a dynamic port | Unsafe host call reaches hardware implementation | FACT: statically resolved methods; OBSERVED HOST-NP/mutant rejection. |

**Negative evidence and limits:** HOST-NP regression rejects a nonempty set; accepting it in the isolated mutant fails the test. This does not demonstrate hardware isolation.

## HOST-NP — negative behavior of NoProtection

**OBSERVED (host, Rust 1.95.0):**

```sh
cargo test -p malleus-arch --all-features
cargo xtask ci
cargo xtask unsafe
```

Three architecture tests pass; the workspace has 91 host tests. The new test
asserts zero representable regions, success for the empty slice and
`Err(UnsupportedRegion)` for a nonempty slice. It passes a descriptor naming
only its live 32-byte local buffer; the callee does not access that buffer.

The following isolated mutant was executed, with rustc success (warnings from
the reduced harness) and regression exit 101: actual `Ok(())` versus expected
`Err(UnsupportedRegion)`. No production source was modified by the experiment.

```sh
mkdir -p /tmp/malleus-no-protection-negative
python3 - <<'PY'
from pathlib import Path
root = Path('/tmp/malleus-no-protection-negative')
source = Path('crates/malleus-arch/src/memory.rs').read_text()
(root / 'memory.rs').write_text(source.replace('if regions.is_empty() {', 'if true {'))
(root / 'test.rs').write_text(
    '#[derive(Debug, PartialEq)] pub enum ArchError { UnsupportedRegion }\n'
    '#[path = "memory.rs"] mod memory;\n')
PY
rustc --edition=2024 --test /tmp/malleus-no-protection-negative/test.rs -o /tmp/malleus-no-protection-negative/test
/tmp/malleus-no-protection-negative/test no_protection_rejects_regions_instead_of_claiming_isolation
```

Provenance can be checked using `git log --reverse --format='%H %s' --
crates/malleus-arch` and `git blame -l` on the source locators above. All 14
legacy construct declaration lines map to `eb1bbd3`; ARCH-15 maps to `6abbcc4`.
This record does not attribute a later test to the introducing C0 commit.

## Outstanding dispositions

Both items are **PENDING MAINTAINER DISPOSITION**. No second review is being
requested in place of the accepted policy; the missing gate is explicit
disposition of these evidence limitations.

| ID | Evidence state and limitation | Proposed disposition |
|---|---|---|
| ARCH-P1 | NOT TESTABLE: runtime behavior of abstract declarations without implementations. Applies to ARCH-01..09 and ARCH-14. Deliberately violating raw-pointer/stack preconditions is not a valid Rust test and can invoke UB. Conformance metadata is not execution. | Accept declaration-only contract/ownership review for C0, require each future concrete implementation to supply its own negative and hardware evidence before support. This proposal is pending approval; no future implementation is added here. |
| ARCH-P2 | NOT PERFORMED: method-specific negative coverage for no-op enable/disable (ARCH-11/12), and end-to-end board/analyser tier-0 policy coupling (ARCH-10). Empty method source has no hardware operation; HOST-NP demonstrates rejection, not all surrounding policy. | Consider accepting direct source inspection of empty methods plus HOST-NP for this minimal type; retain separate enforcement obligations for tooling/integrators. Acceptance is pending, not implied by a passing test. |

No new hardware behavior is claimed for C0. In particular this record does not
implement or validate critical sections, time, context switching or protection.
