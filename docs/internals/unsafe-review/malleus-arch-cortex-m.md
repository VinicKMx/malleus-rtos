# Cortex-M bootstrap unsafe review record

**Policy:** [ADR-0010](../../adr/0010-unsafe-code-policy.md) and
[ADR-0013](../../adr/0013-single-maintainer-unsafe-review.md).
**Date:** 2026-11-03. **Reviewing maintainer:** Vinicius Pedrosa;
all dispositions are accepted with the limitations and exit criteria below.
**Reviewed source revision:** `f6f1b2a`; normal capture decoding was corrected
in `ea5cb1d`, introduced in `f926464`; reset introduced in `215816f`.
Opt-in probes were added in `b05f663`, `7d4f602`, `f06a4a9` and `f6f1b2a`.

This is a retrospective checklist, not an assertion of independent review or
complete hardware conformance. Each numbered entry below is one block counted
by `cargo xtask unsafe`. Locators are source symbols plus the operation; line
numbers are deliberately not the identity.

## Inventory reconciliation

OBSERVED: `cargo xtask unsafe` reports 12 constructs in this crate: four
blocks in [startup.rs](../../../crates/malleus-arch-cortex-m/src/startup.rs)
and eight in [fault.rs](../../../crates/malleus-arch-cortex-m/src/fault.rs).
The regex does not count two `unsafe extern "C" fn` declarations or ten unsafe
attributes. Entries CM-01..03 also cover startup's seven attributes and
`__malleus_start`; CM-07/CM-10 cover the three fault attributes and
`__malleus_fault_capture`. The reset macro's assembly and fault-entry
`global_asm!` are covered as boundary owners, although absent from the count.
The original ten counted blocks are CM-01..10. The two additional blocks are
CM-11 (BusFault load assembly, `b05f663`) and CM-12 (data-sentinel write,
`f06a4a9`). CM-10-UDF2 has its own checklist for the new assembly inside the
already-counted CM-10 block; it does not add a thirteenth construct. CM-07 was
also changed to seed CAPT in the reentrant probe (`7d4f602`). No inventory-tool
change is included. All new stimuli are opt-in board diagnostics.

Rows distinguish FACT (source/linker contract), OBSERVED (executed evidence),
and INFERENCE (conclusion with remaining limits). Shared evidence IDs below
are part of each entry's verification record, not private references.

## CM-01 — startup::__malleus_start — DATA_SENTINEL read

**Introduced:** `215816f`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| RAM is initialized before the Rust call; word is aligned, live, loaded from Flash | Reset assembly; board linker .data symbols | Early Rust entry, wrong LMA/range, or misalignment | FACT: copy loop precedes call; linker assertions. OBSERVED H-BOOT: poisoned data restored. |
| Read has a sole bootstrap owner and observes memory | Single-core reset path; no application execution | Concurrent writes or optimizer replacing observation | FACT: volatile raw read, no reference; OBSERVED H-BOOT sentinel value. |

**Negative evidence and limits:** H-BOOT is the negative initial-state experiment. H-PROBE-4 exercises DATA after changing only the initialized sentinel value; it is not a failed .data copy. Other startup failure branches remain limited under CM-P1.

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

**Negative evidence and limits:** H-BOOT rejects stale success evidence. DATA was observed in H-PROBE-4; BSS/BOTH diagnostic branches remain source-inspected only (CM-P1).

## CM-04 — fault::read_scb — volatile u32 read

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Address is one of the aligned privileged SCB constants | Private read_scb callers; ARMv7-M register map | Arbitrary/unmapped access or unprivileged execution | FACT: fixed callers at ED24/28/2c/34/38; OBSERVED H-FAULT status values. |
| Address registers read before validity status; reads do not clear status | Capture ordering; SCB semantics | Mismatched address/status or destructive read | FACT: source/disassembly order; OBSERVED H-FAULT MMFAR/BFAR invalid and zeroed. |

**Negative evidence and limits:** H-FAULT plants UDF and distinguishes configurable/escalated status. Valid BFAR/PRECISERR was observed in H-PROBE-1; valid MMFAR and interleaving NMI remain NOT PERFORMED (CM-P2). No full-MMIO claim is made.

## CM-05 — fault::write_shcsr — volatile u32 write

**Introduced:** `f926464`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Aligned privileged SHCSR write with deliberate handler enables | configure_probe_fault and architectural SCB | Wrong address or enable mask | FACT: constant ED24 and read/modify/write; OBSERVED H-FAULT UsageFault versus escalation. |
| Status fields preserved by configuration sequence | SCB register semantics and configure_probe_fault | Unintended status/configuration modification | OBSERVED H-FAULT SHCSR 00070008/00030000; H-PROBE-1 adds 00070002. Other status combinations remain untested (CM-P2). |

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
**Probe change:** `7d4f602`; the opt-in branch writes CAPT instead of ARMD to the same aligned owned word. Selection is checked by the board build script, not by direct crate consumption.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| State u32 is aligned and writable; reset initializes it before RAM setup | Linker .uninit; Reset assembly | Capture crosses Rust boundary before memory is valid | FACT: first Reset writes RST0; arm_capture after RAM/sentinel reads; H-FAULT reaches DONE. |
| Single-core transfer to exception assembly is exclusive | Startup/capture state machine | Reentrant or pre-arm capture enters Rust | FACT: assembly accepts ARMD only; OBSERVED H-PROBE-2/3 reject RST0 and seeded CAPT; H-PROBE-5 rejects actual HardFault reentry on healthy MSP. |

**Negative evidence and limits:** H-PROBE-2/3 exercise both guard branches with valid RAM; CAPT in stage 3 is seeded, not a real nested fault. H-PROBE-5 separately demonstrates a nested HardFault with a healthy stack. A real fault during Reset or a damaged MSP is not demonstrated.

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
| Snapshot precedes magic MALF and state DONE; debugger reads after halt | Volatile writes in capture; halted debugger | Stale/torn snapshot interpreted as complete | OBSERVED H-FAULT poison replaced; FACT: disassembly writes magic/state last in normal images; OBSERVED H-PROBE-5 interruption leaves magic zero and state CAPT. |
| Entry gets IPSR, untouched EXC_RETURN and corresponding stacked SP | Fault-entry global_asm and unsafe function caller contract | ABI clobber, wrong MSP/PSP or insufficient handler MSP | H-FAULT validates basic MSP only; H-DIS shows 108-byte handler use, CM-P3/CM-P4 remain. |

**Negative evidence and limits:** H-FAULT poison and physical reset distinguish stale capture. H-PROBE-5 adds actual higher-priority fault interruption on a healthy MSP; NMI/interleavings and MSP exhaustion remain outside observed coverage (CM-P2/CM-P4).

## CM-11 — fault::run_bootstrap_probe — reserved-address load assembly

**Introduced:** `b05f663`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| A fixed board-reserved address is read, never written or exposed as a Rust reference | RM0410 memory map; probe assembly author; board selection | Wrong mapped address produces peripheral effects or a Rust invalid-memory access | FACT: RM0410 Rev 5 section 2.2.2 figure 2 reserves 20080000..3fffffff; emitted load uses r0=30000000. OBSERVED H-PROBE-1: precise BusFault and BFAR30000000. |
| Installed fault vectors terminate; fallback UDF distinguishes a missing bus error; asm has no return/stack access | Board vectors, configure_probe_fault, asm noreturn/nostack | Fallthrough, write-buffer ambiguity, or false positive from fallback | OBSERVED H-PROBE-1: exception5, CFSR8200, PC08000484=ldr rather than fallback UDF08000486; terminal080003cc. |
| Exactly one board diagnostic stimulus is selected | Board build.rs and invoking maintainer | Combining diagnostics invalidates the scenario assumptions | OBSERVED: fifteen pairs of incompatible board features rejected; per-variant builds/Clippy pass. Direct architecture-crate combinations are not guarded (configuration debt). |

**Negative evidence:** H-PROBE-1 is physical read-fault injection. It does not
prove behavior at other reserved addresses, imprecise stores, MPU or MMFAR.

## CM-12 — startup::__malleus_start — data-failure probe sentinel write

**Introduced:** `f06a4a9`. Shared date, reviewing maintainer and revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| Reset completed RAM initialization; the aligned u32 is live and exclusively owned | Reset copy/zero loops, linker, startup sole writer | Uninitialized/misaligned write or competing access | FACT: raw volatile write occurs only after Reset enters Rust; linker unchanged. OBSERVED H-PROBE-4: DATA_SENTINEL0, BSS0 and DATA diagnostic. |
| Only the value is corrupted, never RAM validity, layout or the unsafe caller contract | Feature-selected probe and startup diagnostic | Test bypasses memory initialization or is mistaken for a real copy failure | FACT: write zero before comparison; OBSERVED H-PROBE-4 boot44415441, subsequent normal fault capture and PC0800047c. |
| Normal image is unaffected | Feature default off; board selection | Diagnostic contaminates ordinary boot | OBSERVED: normal Flash byte-identical to ea5cb1d; stripped ELF identical, integral ELF differs only in debug. |

**Negative evidence:** H-PROBE-4 deliberately changes a valid sentinel's value.
It proves DATA reporting, not an actual .data copy failure, BSS failure or
execution with invalid Rust RAM.

## CM-10-UDF2 — fault::__malleus_fault_capture — publication interruption assembly

**Introduced:** `f6f1b2a`, inside the existing CM-10 unsafe block (not an
additional counted block). Shared date/reviewing maintainer/revision above.

| Invariant | Owner | Failure condition | Verification and result |
|---|---|---|---|
| The raw snapshot stores occur before the injected UDF; magic/DONE are not published | Compiler output for this ELF; volatile writes; capture ownership | Compiler moves/elides the stores, or incomplete evidence appears complete | OBSERVED H-PROBE-5 and published disassembly below: stores0800035e..080003a6 precede UDF080003a8. Raw fields replaced poison, magic0, stateCAPT. nomem alone is not an ordering proof. |
| The first fault is UsageFault and UDF #2 escalates to the installed HardFault guard on initialized, healthy MSP | Exclusive board feature, configure_probe_fault, PRIMASK, hardware exception entry | Reenter Rust capture, lockup or corruption from a damaged/invalid stack | OBSERVED H-PROBE-5 live HFSR40000000, IPSR3, LRfffffff1, guardPC0800028c. This tests a healthy stack only. |
| The injected instruction is terminal and does not access software memory/stack itself | asm noreturn/nomem/nostack and architectural UDF behavior | Fallthrough or instruction operands modify the record | FACT: emitted udf #2 followed by compiler trap, no operands; OBSERVED terminal guard and invalid raw record. Hardware exception stacking is separate from asm operands. |

**Negative evidence:** H-PROBE-5 interrupts publication with an actual
higher-priority fault. It does not test NMI, every publication interleaving,
stack overflow or all compiler/optimization configurations. Recheck the binary
order when the source, toolchain or build flags change.

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

## H-PROBE — physical diagnostic experiments, 2026-11-02

**OBSERVED:** five independent physical resets confirmed by the maintainer as
stages 1..5. All source and images are from committed `f6f1b2a`. Every result
below matched its expectation recorded before the experiment; no expectation
was changed after observing hardware.

Setup: Nucleo-F767ZI MB1137 Rev B, integrated ST-LINK/V2-1 serial
`0667FF495177514867234631`, enumerated again for this round. Board/IDCODE
`10016451` and probe firmware `V2J39S27` identification are retained from the
same-board bring-up above. Tools: probe-rs 0.32.0, st-flash 1.8.0; Rust 1.95.0,
release, `thumbv7em-none-eabihf`. No MPU/FPU or application runtime was enabled.

### Image identity and build checks

From `boards/nucleo-f767zi`, each selected feature below was built using
`cargo build --release --target thumbv7em-none-eabihf --features FEATURE` and
checked using `cargo clippy --target thumbv7em-none-eabihf --features FEATURE
-- -D warnings`. All passed. Default and hardfault-escalation-probe also passed.
All fifteen pairs of the six optional board features were rejected with
`select at most one diagnostic probe feature`. Board all-features is therefore
an intentionally invalid selection; the root workspace all-features CI passed
with 91 host tests. `cargo xtask unsafe` reports architecture 15 / Cortex-M 12.

| Stage / feature | ELF SHA-256 | Flash SHA-256 | Bytes |
|---|---|---|---:|
| 1: busfault-read-probe | `9e889a9dbede1529c9996f17a00c96b23536f1f232d189c8de38bad05e8efc08` | `dcfda80d33f6abd3f8a2bbdf82173969af43efbdf59e9b0aaf15e701a600b56d` | 1216 |
| 2: prearm-fault-probe | `39fc7bed7d2b2db6376df68afdfe36944642757f73319a70675bf094d649813c` | `5800c9fed376e5a97b83308e15c9f409391d60cd8e535634e59c0d5aa12c1ab4` | 1184 |
| 3: reentrant-fault-probe | `14ebcc259b33a5138b893017de528644337d13f5cd71778b2bd9fdb85614731e` | `2fb60868596aa119daaee09d267e94fd71b58debaf5ca5f7b54be8c8c023831a` | 1200 |
| 4: startup-data-failure-probe | `f94601124e480738fdc7d0b9d3721182f51d4f080af5e872162c983a3432aa86` | `0f99392166d01f4012a2e9ce515abb3460aa48abfe7036db3dfc470f85c83b23` | 1204 |
| 5: interrupted-capture-probe | `a0a986b0082430d6f1d5e11c09bc8af9690cd22d12c6b49dc94e996af637cd53` | `36abdeca769bac9276ed2696548d0e3c3c4bb3eb58b392558f6d3551d2b2c35a` | 1248 |

Builds before and after committing the probes matched all these full hashes.
Before each flash, both ELF and bin were hashed again and matched the table.
The default Flash image remains byte-identical to `ea5cb1d`, SHA-256
`0fc507f1e8912b7e7990c2e8c028ad9ef6f17ef8a96c96814304ab1cb822108b`.
Default ELF is now
`80db40df02ca768f978e3cdaabab9da7e7c9ba75e49f3b48368aac222b4916dd`;
`llvm-objcopy --strip-debug` followed by `cmp` matched the old stripped ELF.
Only debug data changed in the full normal ELF, not its executable image.

### Exact flash and observation procedure

The observations below identify each image by its published SHA-256. The
following procedure reproduces the image preparation and flash from a clean
clone; it does not depend on the original machine's local artifact directory.
Run one complete experiment at a time. From `boards/nucleo-f767zi`, select the
feature for the corresponding table row and build/extract it before moving to
the next feature (which overwrites the board ELF):

```sh
# Set probe_variant to one feature from the table, for example:
probe_variant=busfault-read-probe
cargo build --release --target thumbv7em-none-eabihf --features "$probe_variant"
llvm-objcopy -O binary target/thumbv7em-none-eabihf/release/nucleo-f767zi "/tmp/${probe_variant}.bin"
sha256sum target/thumbv7em-none-eabihf/release/nucleo-f767zi "/tmp/${probe_variant}.bin"
# Verify the expected image hash above before flashing.
st-flash --connect-under-reset --reset write "/tmp/${probe_variant}.bin" 0x08000000
```

The historical observations used the same hashed image contents and flash
flags/address. File paths are not evidence of image identity. A different
absolute source path can change ELF debug data; compare the Flash image hash
and inspect the emitted instructions when reproducing elsewhere.

All five writes erased only the initial sector and passed read-back
verification. No mass erase or option-byte modification was used. After each
flash, the exact same preparation and readback commands were run:

```sh
probe-rs write --chip STM32F767ZITx b32 0x20000008 0x13579bdf 0xcafebabe 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef 0xdeadbeef
probe-rs read --chip STM32F767ZITx b32 0x20000008 21
probe-rs write --chip STM32F767ZITx b32 0xe000edfc 0
```

Every preparation read showed `13579bdf cafebabe` followed by nineteen
`deadbeef` words. DEMCR was cleared as the last debug operation before each
physical RESET. There was no debug polling or read until the maintainer
confirmed that stage's reset. The st-flash commanded reset did not count as
physical-boot evidence. This separates a fresh result from residual RAM or a
reset held by vector catch.

After each confirmation the following commands were executed, without reset;
the debug command used a PTY:

```sh
probe-rs read --chip STM32F767ZITx b32 0x20000000 23
probe-rs read --chip STM32F767ZITx b32 0xe000ed24 6
probe-rs debug --chip STM32F767ZITx --no-catch-reset --no-catch-hardfault -c break -c 'info reg' -c quit
```

The RAM reads contain DATA, BSS, boot evidence, capture state, then the nineteen
FaultEvidence words. The six SCB words are SHCSR, CFSR, HFSR, DFSR, MMFAR, BFAR.
The debugger reported a running core before each requested halt; PC/IPSR below
are from that halt, not a debugger-generated exception or reset.

All register/address constants in the following comparison are hexadecimal.

| Stage | Expected before the experiment | Observed |
|---|---|---|
| H-PROBE-1 | Precise BusFault, BFARVALID, PC at ldr08000484 rather than fallbackUDF08000486 | MATCH: exception5, flags11, CFSR8200, BFAR30000000, HFSR0, SHCSR70002, PC08000484; terminal080003cc/IPSR5 |
| H-PROBE-2 | RST0, bootMAL2, all evidence words still poisoned, assembly guard terminal | MATCH: state52535430, boot4d414c32, nineteen deadbeef; terminal0800028c/IPSR6 |
| H-PROBE-3 | Seeded CAPT, bootMAL2, poisoned evidence, assembly guard terminal | MATCH: state43415054, boot4d414c32, nineteen deadbeef; terminal0800028c/IPSR6 |
| H-PROBE-4 | Initialized DATA sentinel changed to zero, bootDATA, normal UsageFault snapshot | MATCH: DATA0/BSS0, boot44415441, DONE/MALF, exception6, flags1, PC0800047c, CFSR10000, HFSR0; terminal080003cc/IPSR6 |
| H-PROBE-5 | Publication interrupted after raw aggregate, before magic/DONE, by HardFault | MATCH: CAPT, magic0, raw exception6/PC080004a8, live HFSR40000000; guard terminal0800028c/IPSR3, LRfffffff1 |

### H-PROBE-1 — busfault-read-probe: raw reads

```text
20000000: 44415441 00000000 4d414c32 444f4e45 4d414c46 00000001 00000005 fffffff9
20000020: 2001ffd0 00000011 30000000 00070000 44415441 44415441 00000000 08000465
20000040: 08000484 61000000 00008200 00000000 00070002 00000000 30000000
e000ed24: 00070002 00008200 00000000 00000001 30000000 30000000
```

This is a precise read fault: captured BFAR is valid and equals the load
address. Live MMFAR also read `30000000`, but MMARVALID is absent and the
snapshot correctly stores MMFAR as zero. That live value is not MMFAR-valid
evidence and does not cover DACCVIOL.

### H-PROBE-2 — prearm-fault-probe: raw reads

```text
20000000: 44415441 00000000 4d414c32 52535430 deadbeef deadbeef deadbeef deadbeef
20000020: deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef
20000040: deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef
e000ed24: 00070008 00010000 00000000 00000001 00000000 00000000
```

**What this proves:** the pre-arming guard branch rejects RST0 after Rust RAM
is valid. **What it does not prove:** a real fault during the Reset copy/zero
loops or before valid RAM. The injected UDF occurs after initialization.

### H-PROBE-3 — reentrant-fault-probe: raw reads

```text
20000000: 44415441 00000000 4d414c32 43415054 deadbeef deadbeef deadbeef deadbeef
20000020: deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef
20000040: deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef deadbeef
e000ed24: 00070008 00010000 00000000 00000001 00000000 00000000
```

**What this proves:** the guard rejects CAPT deliberately seeded by firmware.
**What it does not prove:** a real nested fault or a damaged stack. The active
exception is the first UsageFault. H-PROBE-5 is the separate real nested-fault
experiment on a healthy stack.

### H-PROBE-4 — startup-data-failure-probe: raw reads

```text
20000000: 00000000 00000000 44415441 444f4e45 4d414c46 00000001 00000006 fffffff9
20000020: 2001ffd8 00000001 e000ed24 00070000 44415441 44415441 00000000 0800022f
20000040: 0800047c 81000000 00010000 00000000 00070008 00000000 00000000
e000ed24: 00070008 00010000 00000000 00000001 00000000 00000000
```

**What this proves:** the DATA diagnostic detects a changed initialized
sentinel. **What it does not prove:** a real .data copy failure, execution before
Rust RAM validity, or the BSS/BOTH diagnostic branches.

### H-PROBE-5 — interrupted-capture-probe: raw reads

```text
20000000: 44415441 00000000 4d414c32 43415054 00000000 00000001 00000006 fffffff9
20000020: 2001ffd8 00000001 e000ed24 00070000 44415441 44415441 00000000 0800022f
20000040: 080004a8 61000000 00010000 00000000 00070008 00000000 00000000
e000ed24: 00070008 00010000 40000000 00000001 00000000 00000000
```

These are intentionally **invalid raw aggregate words**, not a published
FaultEvidence: magic is zero, capture remains CAPT and DONE was never written.
The raw HFSR is zero from the first UsageFault; live HFSR is FORCED after the
second UDF escalated. That difference is expected, not a substituted status.
Current IPSR3 and LRfffffff1 demonstrate HardFault entry from Handler mode.
MSP `2001ff38` is consistent with original selected SP `2001ffd8`, a 128-byte
capture prologue and a 32-byte nested hardware frame. This probe's stack use
is different from the normal capture's 108 bytes; neither proves overflow
containment. No NMI or damaged-stack behavior was tested.

### Binary ordering evidence for H-PROBE-5

**OBSERVED:** `llvm-objdump -d` of the exact ELF SHA-256
`a0a986b0082430d6f1d5e11c09bc8af9690cd22d12c6b49dc94e996af637cd53`
shows r0 set to `20000010`, nineteen snapshot stores at offsets 0x00..0x48, then
`udf #2`. The final store is at `080003a6`, UDF at `080003a8`. No magic/DONE
publication follows on this path. The instruction order is measured from the
binary; it is not inferred from the asm `nomem` option. The raw readback above
also shows that the aggregate replaced poison before the guard halted.

```text
 8000356: f240 0010    	movw	r0, #0x10
 800035a: f2c2 0000    	movt	r0, #0x2000
 800035e: 6001         	str	r1, [r0]
 8000360: 991c         	ldr	r1, [sp, #0x70]
 8000362: 6041         	str	r1, [r0, #0x4]
 8000364: 991b         	ldr	r1, [sp, #0x6c]
 8000366: 6081         	str	r1, [r0, #0x8]
 8000368: 991a         	ldr	r1, [sp, #0x68]
 800036a: 60c1         	str	r1, [r0, #0xc]
 800036c: 9919         	ldr	r1, [sp, #0x64]
 800036e: 6101         	str	r1, [r0, #0x10]
 8000370: 9918         	ldr	r1, [sp, #0x60]
 8000372: 6141         	str	r1, [r0, #0x14]
 8000374: 9917         	ldr	r1, [sp, #0x5c]
 8000376: 6181         	str	r1, [r0, #0x18]
 8000378: 9916         	ldr	r1, [sp, #0x58]
 800037a: 61c1         	str	r1, [r0, #0x1c]
 800037c: 9915         	ldr	r1, [sp, #0x54]
 800037e: 6201         	str	r1, [r0, #0x20]
 8000380: 9914         	ldr	r1, [sp, #0x50]
 8000382: 6241         	str	r1, [r0, #0x24]
 8000384: 9913         	ldr	r1, [sp, #0x4c]
 8000386: 6281         	str	r1, [r0, #0x28]
 8000388: 9912         	ldr	r1, [sp, #0x48]
 800038a: 62c1         	str	r1, [r0, #0x2c]
 800038c: 9911         	ldr	r1, [sp, #0x44]
 800038e: 6301         	str	r1, [r0, #0x30]
 8000390: 9910         	ldr	r1, [sp, #0x40]
 8000392: 6341         	str	r1, [r0, #0x34]
 8000394: 990f         	ldr	r1, [sp, #0x3c]
 8000396: 6381         	str	r1, [r0, #0x38]
 8000398: 990e         	ldr	r1, [sp, #0x38]
 800039a: 63c1         	str	r1, [r0, #0x3c]
 800039c: 990d         	ldr	r1, [sp, #0x34]
 800039e: 6401         	str	r1, [r0, #0x40]
 80003a0: 990c         	ldr	r1, [sp, #0x30]
 80003a2: 6441         	str	r1, [r0, #0x44]
 80003a4: 990b         	ldr	r1, [sp, #0x2c]
 80003a6: 6481         	str	r1, [r0, #0x48]
 80003a8: de02         	udf	#0x2
```

This result is specific to the recorded binary/configuration. It does not make
`nomem` a memory barrier or prove ordering for other compiler output.

### Residual configuration and experiment limits

The exclusive-feature gate is in the **board build.rs only**. Direct consumers
of `malleus-arch-cortex-m` can combine bootstrap features; this round does not
fix that configuration debt. These observations apply to the validated board
selections only.

MMARVALID/DACCVIOL was not exercised: no documented F767 MPU-free route was
established for this experiment, and MPU configuration is outside P1. A
non-executable-region IACCVIOL does not establish valid MMFAR. NMI was not
injected because its vector is zero; that is a distinct uncontrolled lockup
path. The tested HardFault interruption is not a substitute for NMI or all
possible SHCSR/status/publication interleavings. These limits remain explicit
in CM-P2. The probe and synthetic-state limitations are not silently promoted
to broader safety claims.

## Maintainer dispositions — 2026-11-03

All dispositions below were explicitly **ACCEPTED** by the maintainer on
2026-11-03. Acceptance records the retained risks and future exit criteria; it
is not inferred from compilation or broader claims than these experiments.

| ID | Evidence, retained limitation and disposition |
|---|---|
| CM-P1 | ACCEPTED for P1 with limits: observed guard/DATA coverage is sufficient. A real fault during Reset and the BSS/BOTH diagnostic branches remain explicit residual risks; the latter share the DATA path and differ in their diagnostic code. H-PROBE-2/3 do not become real early-reset or damaged-stack experiments by this acceptance. |
| CM-P2 | ACCEPTED for P1 with residual risk: H-PROBE-1 covers BFARVALID/PRECISERR and H-PROBE-5 covers HardFault interruption. Mandatory C1.P4 exit criteria are to install the NMI vector and observe evidence-publication interruption by NMI, and to physically verify MMARVALID/DACCVIOL in the MPU phase. Arbitrary SHCSR combinations and other interleavings remain explicit residual risks. |
| CM-P3 | ACCEPTED by maintainer as a P1 diagnostic limitation. Extended/FPU and physical PSP validation are mandatory ARCH-CTX-003 exit criteria in C1.P2. Host decoding is not physical evidence and the port is not supported. |
| CM-P4 | ACCEPTED by maintainer as a P1 diagnostic limitation. Real stacking errors, invalid/overflowed MSP and recursion with a damaged stack are mandatory C1.P4 protected-stack/containment exit criteria. The present healthy-stack experiments do not satisfy them. |
| CM-P5 | ACCEPTED by maintainer: architectural argument plus emitted dsb/isb. Residual risk remains that no isolated ordering/stress or barrier-removal experiment demonstrates necessity on this hardware run. |

The retrospective limitations are regularized under these accepted dispositions.
This completes the bootstrap review scope, not a supported hardware port or
physical validation of the inherited C1.P2/C1.P4 requirements.
