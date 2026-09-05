# Cortex-M bootstrap unsafe review record

**Policy:** [ADR-0010](../../adr/0010-unsafe-code-policy.md) and
[ADR-0013](../../adr/0013-single-maintainer-unsafe-review.md).
**Date:** 2026-09-04. **Reviewing maintainer:** Vinicius Pedrosa;
record disposition is pending where explicitly stated below.
**Reviewed source revision:** `7eda023`; production capture last changed in
`ea5cb1d`, introduced in `f926464`; reset introduced in `215816f`.

This is a retrospective checklist, not an assertion of independent review or
complete hardware conformance. Each numbered entry below is one block counted
by `cargo xtask unsafe`. Locators are source symbols plus the operation; line
numbers are deliberately not the identity.

## Inventory reconciliation

OBSERVED: `cargo xtask unsafe` reports 10 constructs in this crate: three
blocks in [startup.rs](../../../crates/malleus-arch-cortex-m/src/startup.rs)
and seven in [fault.rs](../../../crates/malleus-arch-cortex-m/src/fault.rs).
The regex does not count two `unsafe extern "C" fn` declarations or ten unsafe
attributes. Entries CM-01..03 also cover startup's seven attributes and
`__malleus_start`; CM-07/CM-10 cover the three fault attributes and
`__malleus_fault_capture`. The reset macro's assembly and fault-entry
`global_asm!` are covered as boundary owners, although absent from the count.
No inventory-tool change or new production unsafe is included in this record.

Rows distinguish FACT (source/linker contract), OBSERVED (executed evidence),
and INFERENCE (conclusion with remaining limits). Shared evidence IDs below
are part of each entry's verification record, not private references.

## CM-01 — startup::__malleus_start — DATA_SENTINEL read

**Introduced:** `215816f`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| RAM is initialized before the Rust call; word is aligned, live, loaded from Flash | Reset assembly; board linker .data symbols | Early Rust entry, wrong LMA/range, or misalignment | FACT: copy loop precedes call; linker assertions. OBSERVED H-BOOT: poisoned data restored. |
| Read has a sole bootstrap owner and observes memory | Single-core reset path; no application execution | Concurrent writes or optimizer replacing observation | FACT: volatile raw read, no reference; OBSERVED H-BOOT sentinel value. |

**Negative evidence and limits:** H-BOOT is the negative initial-state experiment. Direct entry into Rust with corrupt .data was NOT PERFORMED (CM-P1).

## CM-02 — startup::__malleus_start — BSS_SENTINEL read

**Introduced:** `215816f`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| BSS is zeroed and the u32 is aligned/live before Rust | Reset zero loop; linker .bss symbols | Uninitialized RAM read, invalid bounds/alignment | FACT: .bss NOLOAD and bounded word loop; OBSERVED H-BOOT: a5a5a5a5 became zero. |
| There is no concurrent writer | Reset bootstrap lifecycle | Interrupt or other writer changes sentinel | FACT: single-core terminal bootstrap; OBSERVED boot evidence, not a race test. |

**Negative evidence and limits:** H-BOOT falsifies omitted BSS clearing for this word. Direct BSS-failure branch injection NOT PERFORMED (CM-P1).

## CM-03 — startup::__malleus_start — MALLEUS_BOOT_EVIDENCE write

**Introduced:** `215816f`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Evidence storage is aligned, writable and excluded from zeroing | Linker .uninit; startup sole writer | Wrong section, invalid symbol, stale marker | FACT: linker placement; OBSERVED H-BOOT: two poisoned evidence markers replaced. |
| Symbol identity and entry ABI match assembly; memory valid before entry | no_mangle/section attributes, Reset assembly | Wrong symbol/ABI or Rust before RAM init | FACT: ELF symbols and reset flow; OBSERVED H-BOOT physical reset/cold boot. |

**Negative evidence and limits:** H-BOOT rejects stale success evidence. Failure evidence codes DATA/BSS/BOTH are source-inspected only; CM-P1.

## CM-04 — fault::read_scb — volatile u32 read

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Address is one of the aligned privileged SCB constants | Private read_scb callers; ARMv7-M register map | Arbitrary/unmapped access or unprivileged execution | FACT: fixed callers at ED24/28/2c/34/38; OBSERVED H-FAULT status values. |
| Address registers read before validity status; reads do not clear status | Capture ordering; SCB semantics | Mismatched address/status or destructive read | FACT: source/disassembly order; OBSERVED H-FAULT MMFAR/BFAR invalid and zeroed. |

**Negative evidence and limits:** H-FAULT plants UDF and distinguishes configurable/escalated status. Valid MMFAR/BFAR and interleaving NMI were NOT PERFORMED (CM-P2); no claim of full MMIO semantics.

## CM-05 — fault::write_shcsr — volatile u32 write

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Aligned privileged SHCSR write with deliberate handler enables | configure_probe_fault and architectural SCB | Wrong address or enable mask | FACT: constant ED24 and read/modify/write; OBSERVED H-FAULT UsageFault versus escalation. |
| Status fields preserved by configuration sequence | SCB register semantics and configure_probe_fault | Unintended status/configuration modification | OBSERVED H-FAULT SHCSR 00070008/00030000; other status combinations not injected. |

**Negative evidence and limits:** H-FAULT is a physical negative instruction experiment with enable/disable alternatives. Untested status combinations are CM-P2.

## CM-06 — fault::read_exception_frame — volatile core read

**Introduced:** `f926464; corrected ea5cb1d`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Recognized EXC_RETURN, aligned selected SP, no stacking/lazy errors | Fault-entry assembly; frame_layout; CFSR guard | Wrong stack/frame or reading incomplete frame | OBSERVED HOST-FRAME and HOST-REJECT; FACT: source guards; H-FAULT basic MSP. |
| Eight live u32 words begin at SP+0, PC word 6; all bit patterns valid | Hardware exception entry; host test buffer owner | 18-word offset decodes FP as core; corrupt SP may still fail | OBSERVED HOST-FRAME old-offset mutant fails; H-FAULT PC matches UDF. Extended hardware deferred (CM-P3). |

**Negative evidence and limits:** HOST-REJECT fails with a stacking-mask bit removed. It tests rejection results with live storage, not a memory-bus trace proving no earlier access. Corrupt stack and actual hardware stacking failure are CM-P4.

## CM-07 — fault::arm_capture — state publication

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| State u32 is aligned and writable; reset initializes it before RAM setup | Linker .uninit; Reset assembly | Capture crosses Rust boundary before memory is valid | FACT: first Reset writes RST0; arm_capture after RAM/sentinel reads; H-FAULT reaches DONE. |
| Single-core transfer to exception assembly is exclusive | Startup/capture state machine | Reentrant or pre-arm capture enters Rust | FACT: assembly accepts ARMD only; pre-arm/reentrant paths NOT PERFORMED (CM-P1). |

**Negative evidence and limits:** H-FAULT demonstrates armed path only. State corruption/pre-arm/reentry needs a controlled target experiment; no automatic acceptance of the guard.

## CM-08 — fault::configure_probe_fault — dsb/isb asm

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Barriers order handler configuration before UDF without using stack/memory operands | ARM barrier semantics; compiler asm boundary | Fault observes stale enable state | FACT: dsb/isb in disassembly; OBSERVED H-FAULT two intended exception outcomes. |

**Negative evidence and limits:** H-FAULT exercises the configured fault; it does not show that removing either barrier would fail on this execution. Host cannot execute these ARM barriers; an isolated target ordering experiment is NOT PERFORMED. CM-P5 records that limitation.

## CM-09 — fault::run_bootstrap_probe — udf asm

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| UDF is intentional and installed vectors terminate; never fall through | ARM exception entry; board vectors; run_bootstrap_probe | Missing vector, recursive failure, return into corrupted execution | OBSERVED H-FAULT exception numbers, stacked PCs at UDF, live core subsequently halted in terminal loop; FACT noreturn/nomem/nostack declaration. |

**Negative evidence and limits:** H-FAULT is deliberate fault injection in both configurations. Terminal behavior with damaged MSP is untested (CM-P4).

## CM-10 — fault::__malleus_fault_capture — evidence publication block

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Aligned live DTCM storage and a sole writer after ARMD to CAPT | Linker attributes; exception assembly; unsafe extern C entry | Concurrent/reentrant write or invalid storage | FACT: section/symbol/ABI and state guard; OBSERVED H-FAULT full snapshot. Reentry CM-P1. |
| Snapshot precedes magic MALF and state DONE; debugger reads after halt | Volatile writes in capture; halted debugger | Stale/torn snapshot interpreted as complete | OBSERVED H-FAULT poison replaced; FACT: disassembly writes magic/state last. Mid-publication interruption CM-P2. |
| Entry gets IPSR, untouched EXC_RETURN and corresponding stacked SP | Fault-entry global_asm and unsafe function caller contract | ABI clobber, wrong MSP/PSP or insufficient handler MSP | H-FAULT validates basic MSP only; H-DIS shows 108-byte handler use, CM-P3/CM-P4 remain. |

**Negative evidence and limits:** H-FAULT poison and physical reset distinguish stale capture. Fault during publication, actual recursive entry and MSP exhaustion not validated.

## Reproducible host evidence

**OBSERVED HOST-FRAME / HOST-REJECT (Rust 1.95.0):**

```sh
cargo test -p malleus-arch-cortex-m --all-features
cargo xtask ci
```

Seven Cortex-M tests pass, including `basic_and_extended_frames_read_the_core_at_sp`
and `invalid_frames_are_rejected_before_memory_access`. The latter covers six
EXC_RETURN encodings times six stacking-status bits, misalignment, and unknown
encodings. No host test invokes the hardware-only code or deliberately reads an
invalid pointer. `cargo xtask ci` passed with 90 host tests at `7eda023`.

The following isolated mutations were executed; each selected regression failed
(exit 101), without changing the repository's production source:

```sh
mkdir -p /tmp/malleus-frame-negative
python3 - <<'PY'
from pathlib import Path
source = Path('crates/malleus-arch-cortex-m/src/fault.rs').read_text()
root = Path('/tmp/malleus-frame-negative')
(root / 'offset.rs').write_text(source.replace(
    'let frame_pointer = stack_pointer.cast::<ExceptionFrame>();',
    'let frame_pointer = stack_pointer.wrapping_add(if layout_flags & FRAME_EXTENDED != 0 { 18 } else { 0 }).cast::<ExceptionFrame>();'))
(root / 'mask.rs').write_text(source.replace(
    'const STACKING_ERROR_MASK: u32 = 0x0000_3838;',
    'const STACKING_ERROR_MASK: u32 = 0x0000_3830;'))
PY
rustc --edition=2024 --test /tmp/malleus-frame-negative/offset.rs -o /tmp/malleus-frame-negative/offset
/tmp/malleus-frame-negative/offset basic_and_extended_frames_read_the_core_at_sp
rustc --edition=2024 --test /tmp/malleus-frame-negative/mask.rs -o /tmp/malleus-frame-negative/mask
/tmp/malleus-frame-negative/mask invalid_frames_are_rejected_before_memory_access
```

Offset mutation reads eight `f0f0f0f0` FP-storage words instead of the core;
mask mutation incorrectly accepts CFSR bit 3. These challenge different
assumptions; neither proves real extended-FPU stacking or bus-fault handling.

**OBSERVED target-built:** from `boards/nucleo-f767zi`, release builds with and
without `--features hardfault-escalation-probe`, and
`cargo clippy --all-features --target thumbv7em-none-eabihf -- -D warnings`
passed. These are compilation/static-analysis evidence only.

## Imported physical observations

### H-BOOT — reset with poisoned RAM

**OBSERVED (2026-08-21/25 retained bring-up record):** Nucleo-F767ZI MB1137 Rev B,
DBGMCU_IDCODE `10016451`, ST-LINK/V2-1 serial `0667FF495177514867234631`, debug
firmware `V2J39S27`, probe-rs 0.32.0, stlink-tools 1.8.0. Firmware source
`215816f`, Rust 1.95.0 release, target `thumbv7em-none-eabihf`.

- ELF SHA-256: `c3b49bc8b649bf34946e0f3d0a50832147c5ba1c2311f7daf249ddb7f5605147`.
- Flash (688 bytes) SHA-256: `cae011749dbba61305b4b40247c904c901ba601dc54446b3872090b3ae84c00d`.

The recorded flash command family was `st-flash --connect-under-reset --reset
write`, with read-back verification. The original filename argument was not
retained; no exact original filename is claimed here. The reproducible image
must be identified by hash, not by an invented historical path.

At `.data = 20000000`, `.bss = 20000004`, boot evidence `20000008`, poison
`deadbeef a5a5a5a5 cafebabe` became `44415441 00000000 4d414c32` after physical
reset. Replacing only evidence with `13579bdf` and physically resetting again
restored `4d414c32`. Removing USB power for about five seconds and reconnecting
also produced the success triplet before the first debugger read.

DEMCR was cleared as the last debug operation before accepted physical resets.
An earlier reset with `DEMCR = 01000501` left RAM poisoned and was rejected as
boot evidence because vector catch stopped reset. This is manual bring-up
observation, not proof of every startup failure branch.

### H-FAULT — corrected capture, physical reset and deliberate UDF

**OBSERVED (2026-09-03):** same board (PCB revision reconfirmed by maintainer),
probe serial/firmware and tool versions above; DBGMCU_IDCODE read as `10016451`.
Both variants were built on `ea5cb1d` using Rust 1.95.0, target
`thumbv7em-none-eabihf`, release. Rebuilds from its clean committed tree matched
all four SHA-256 values. No claim is made for the faulty original `f926464` image.

| Configuration | ELF SHA-256 | Flash SHA-256 | Bytes |
|---|---|---|---:|
| UsageFault | `34ff87b7e545887faf08ef22bd00dc0bb0a390dfbccc6ecae8aa3e55193eb44a` | `0fc507f1e8912b7e7990c2e8c028ad9ef6f17ef8a96c96814304ab1cb822108b` | 1200 |
| HardFault escalation | `dbdc8bbfb9cd3ffdbda260573b7affe5fe87db54e46d0e5819034abc725edb16` | `1e7b76d2e82d6e68da46a2a08152c42d3cc0c55db6bc99603c8f06caaa61dcde` | 1212 |

Exact commands used (the files named below are historical local artifacts;
the commands are not instructions to flash an arbitrary current build):

```sh
st-flash --connect-under-reset --reset write /tmp/malleus-c1-p1-4/usage.bin 0x08000000
# For the second experiment the same command used hard.bin.
probe-rs write --chip STM32F767ZITx b32 0x2000000c 0xcafebabe 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef
probe-rs read --chip STM32F767ZITx b32 0x2000000c 20
probe-rs write --chip STM32F767ZITx b32 0xe000edfc 0
# Last debug operation above; maintainer then pressed physical RESET.
probe-rs read --chip STM32F767ZITx b32 0x20000008 21
```

Each flash erased only sector 0, wrote 1200/1212 bytes, and passed verification.
No mass erase or option-byte modification occurred. Before each physical reset,
the read showed `cafebabe` followed by 19 `deadbeef` words; no debugger reset was
accepted as boot evidence. The post-reset reads were:

```text
UsageFault:
20000008: 4d414c32 444f4e45 4d414c46 00000001 00000006 fffffff9 2001ffd8 00000001
20000028: e000ed24 00070000 44415441 44415441 00000000 0800022f 08000478 61000000
20000048: 00010000 00000000 00070008 00000000 00000000
HardFault escalation:
20000008: 4d414c32 444f4e45 4d414c46 00000001 00000003 fffffff9 2001ffd0 00000001
20000028: e000ed24 00030000 00000003 44415441 00000000 08000465 08000482 21000000
20000048: 00010000 40000000 00030000 00000000 00000000
```

The first two words are boot success and capture DONE; the next 19 are
FaultEvidence. Magic `4d414c46`, version 1, exception 6/3, basic MSP frame,
flags 1; CFSR UNDEFINSTR in both; HFSR FORCED only for escalation. MMFAR/BFAR
are invalid and stored as zero. ELF disassembly places UDF at `08000478` and
`08000482`, matching the captured PCs.

The terminal observations used PTY sessions:

```sh
probe-rs debug --chip STM32F767ZITx --no-catch-reset --no-catch-hardfault -c break -c 'info reg pc' -c 'info reg xpsr' -c quit
probe-rs debug --chip STM32F767ZITx --no-catch-reset --no-catch-hardfault -c break -c 'info reg' -c quit
```

Both reported a running core before the requested halt at `080003cc`, a
`yield`/backward-branch terminal loop. The first lowercase `pc` filter did not
match a register name; the halt itself reported PC. xPSR was `41000006`/`41000003`.
In the second, R15/PC `080003cc`, MSP `2001ff64 = 2001ffd0 - 0x6c`.

**FACT H-DIS:** `llvm-objdump -d` on both named ELFs shows no VFP instructions
in capture, core loads from selected r2 at offsets 0..0x1c (PC +0x18), magic
and DONE after snapshot, and prologue `push {r7,lr}` plus `sub sp,#0x64`.
This is source/binary evidence, not a stack-overflow experiment.

## Outstanding dispositions

All items below are **PENDING MAINTAINER DISPOSITION**. Suggested dispositions
are proposals, not risk acceptance or completed regularization.

| ID | Evidence state and limitation | Proposed disposition |
|---|---|---|
| CM-P1 | NOT PERFORMED: direct startup failure-code branches; pre-arm/reentrant fault-entry guard on target. Host cannot execute exception assembly. A controlled debugger-driven entry on a healthy stack could test guard branches, but is not actual nested/corrupt-stack evidence. | Require a separately specified bounded target experiment before accepting these branches; retain pending status now. |
| CM-P2 | NOT PERFORMED: valid MMFAR/BFAR paths, publication interrupted by higher-priority fault/NMI, and unusual SHCSR status combinations. UDF only proves the observed case. | Keep these limitations explicit; maintainer decides whether bootstrap evidence suffices or more targeted experiments are required. NMI vector zero is a known constraint. |
| CM-P3 | PARTIALLY VERIFIED: extended frame and PSP decoded on host only. FPU is deliberately not enabled in P1. | Carry real extended/FPU validation to ARCH-CTX-003 in C1.P2; do not implement it here. Maintainer must approve that disposition. |
| CM-P4 | NOT PERFORMED: actual stacking errors, invalid SP, MSP overflow and recursive failure with damaged stack. Some injections could destroy capture; no claim that every such experiment is impossible. | Carry protected-stack containment to C1.P4; require maintainer disposition of the P1 diagnostic limitation, not a blanket NOT TESTABLE exemption. |
| CM-P5 | NOT PERFORMED: isolated target ordering/stress experiment. Host has no equivalent ARM barrier behavior; no claim is made that physical experimentation is impossible. Architectural semantics and emitted dsb/isb support the invariant; H-FAULT observes only two outcomes. | Consider accepting the architectural/disassembly argument with its residual risk, or require additional stress evidence. Acceptance is pending. |

No new physical reset or extra firmware was required to prepare this record.
No exhaustive hardware validation or completed retrospective regularization is
claimed while these dispositions remain pending.
