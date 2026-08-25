# Nucleo-F767ZI bring-up

> **Status:** reset and memory initialization observed on a physical
> Nucleo-F767ZI; terminal fault capture implemented and target-built, with its
> hardware observation still pending. This does not yet make the full Cortex-M7
> port supported; later checkpoint requirements remain unimplemented.

This workspace is deliberately separate from the host-only root workspace. It
targets the STM32F767ZI Cortex-M7 on the Nucleo-F767ZI without introducing a
HAL, runtime framework, allocator, scheduler, or peripheral initialization.

## Build

From the repository root:

```bash
cargo build \
  --manifest-path boards/nucleo-f767zi/Cargo.toml \
  --target thumbv7em-none-eabihf
```

From the board directory, `boards/.cargo/config.toml` supplies the same target:

```bash
cd boards/nucleo-f767zi
cargo build
```

The repository's `rust-toolchain.toml` pins both the compiler and target
component. The resulting ELF is under
`boards/nucleo-f767zi/target/thumbv7em-none-eabihf/debug/`.

## Memory contract and provenance

The linker contract intentionally represents each physically distinct memory
instead of advertising a synthetic contiguous RAM pool:

| Region | Origin | Length | Initial use |
|---|---:|---:|---|
| ITCM | `0x0000_0000` | 16 KiB | Declared, not allocated |
| Flash (AXIM alias) | `0x0800_0000` | 2 MiB | vectors and code |
| DTCM | `0x2000_0000` | 128 KiB | initial stack and data |
| SRAM1 | `0x2002_0000` | 368 KiB | declared, not allocated |
| SRAM2 | `0x2007_C000` | 16 KiB | declared, not allocated |

Sources, all from STMicroelectronics:

- [DS11532 Rev 9](https://www.st.com/resource/en/datasheet/stm32f767zi.pdf), the
  STM32F767ZI datasheet and density/package selection;
- [RM0410 Rev 5](https://www.st.com/resource/en/reference_manual/rm0410-stm32f7-series.pdf),
  sections 2.2 through 2.4 for the memory organization and aliases;
- [ES0334 Rev 9](https://www.st.com/resource/en/errata_sheet/es0334-stm32f76xxx-and-stm32f77xxx-device-errata-stmicroelectronics.pdf),
  which covers silicon revisions A and Z/1;
- [UM1974](https://www.st.com/resource/en/user_manual/um1974-stm32-nucleo144-boards-mb1137-stmicroelectronics.pdf),
  for the MB1137 Nucleo-144 board family.

The exact PCB revision must accompany hardware evidence. The silicon can also be
identified mechanically through `DBGMCU_IDCODE`; its revision does not change
the memory addresses above, but it can change which errata constrain reset,
cache, FPU, and exception work in later commits.

## Reset contract

The vector table occupies the first 512 bytes of Flash. It contains the initial
DTCM stack pointer, `Reset`, and the four architectural fault entries described
below; unused entries remain zero. `Reset` executes entirely in assembly while
it copies the word-aligned `.data` image from Flash and clears the word-aligned
`.bss` range. Only then does it enter Rust. `.uninit` is deliberately
preserved.

The board image contains non-zero `.data` and zero-valued `.bss` sentinels.
After checking both, it writes one of these values to the exported
`MALLEUS_BOOT_EVIDENCE` word in `.uninit` and stops:

| Value | Meaning |
|---:|---|
| `0x4d41_4c32` | both `.data` and `.bss` are valid (`MAL2`) |
| `0x4441_5441` | `.data` initialization failed (`DATA`) |
| `0x4253_5321` | `.bss` initialization failed (`BSS!`) |
| `0x424f_5448` | both checks failed (`BOTH`) |

Reading the success value from an ELF or after debugger-assisted RAM setup is
not sufficient evidence. Hardware validation must flash the image, perform a
physical reset without debugger initialization, and read the evidence word.
It must then poison both sentinel locations, reset again, and observe that the
startup path restores them and reports success.

## C1.P1.2 hardware evidence

On 2026-08-21, the release image built with Rust 1.95.0 was observed on an
MB1137 Rev B Nucleo-F767ZI through its integrated ST-LINK/V2-1. The target
reported `DBGMCU_IDCODE = 0x10016451` (`DEV_ID = 0x451`, `REV_ID = 0x1001`).
The 688-byte Flash image had SHA-256
`cae011749dbba61305b4b40247c904c901ba601dc54446b3872090b3ae84c00d`.

The validated sequence was:

1. Flash and read-back verification completed through ST-LINK.
2. A physical reset restored a deliberately corrupted `.data` word from
   `0xdeadbeef` to `0x44415441`, cleared a `.bss` word from `0xa5a5a5a5` to
   zero, and replaced `0xcafebabe` with the success evidence `0x4d414c32`.
3. A second physical reset independently replaced evidence value `0x13579bdf`
   with `0x4d414c32`.
4. A complete USB power cycle produced
   `44415441 00000000 4d414c32` before the debugger read memory.

The debugger's reset vector catch was explicitly disabled before physical-reset
observations. A halted reset is not accepted as boot evidence.

## Exception and fault-capture contract

The architectural encodings, frame layouts, and SCB register semantics follow
[PM0253 Rev 6](https://www.st.com/resource/en/programming_manual/DM00237416.pdf),
especially its exception model and Cortex-M7 peripheral chapters.

`HardFault`, `MemManage`, `BusFault`, and `UsageFault` share one assembly entry.
It masks configurable interrupts, preserves `EXC_RETURN`, selects the
pre-exception MSP or PSP, and crosses into Rust only after reset has armed fault
capture. A fault before arming or a recursive entry stops in assembly instead
of trusting Rust memory state again.

The Rust boundary accepts only the six ARMv7-M `EXC_RETURN` encodings used by
basic and extended frames. It does not dereference the frame if CFSR reports an
unstacking, stacking, or lazy-state preservation error. A valid extended frame
has its 18-word floating-point prefix skipped before reading the eight-word
core frame. Capture is terminal: it neither returns to faulting code nor
attempts task attribution, supervision, recovery, or persistent storage.

`MALLEUS_FAULT_CAPTURE_STATE` exposes the capture lifecycle in `.uninit`:

| Value | Meaning |
|---:|---|
| `0x5253_5430` | reset/memory initialization in progress (`RST0`) |
| `0x4152_4d44` | Rust memory is valid and capture is armed (`ARMD`) |
| `0x4341_5054` | the common entry owns capture (`CAPT`) |
| `0x444f_4e45` | the complete snapshot is published (`DONE`) |

`MALLEUS_FAULT_EVIDENCE` is a fixed 19-word snapshot. The first word is written
last, so only `0x4d41_4c46` (`MALF`) authorizes interpretation of the remaining
words:

| Word(s) | Field |
|---:|---|
| 0 | magic (`MALF`) |
| 1 | format version (`1`) |
| 2–5 | exception number, `EXC_RETURN`, selected stack pointer, frame flags |
| 6–13 | stacked `r0`–`r3`, `r12`, `lr`, `pc`, and `xPSR` |
| 14–18 | CFSR, HFSR, SHCSR, MMFAR, and BFAR |

Frame flags are bit 0 for a valid core frame, bit 1 for an extended frame, bit
2 for PSP selection, bit 3 for a valid MMFAR, and bit 4 for a valid BFAR. An
address without its validity flag is stored as zero. The snapshot can contain
sensitive register values and is intended only for controlled bootstrap
diagnostics.

The default release image enables configurable faults and executes `UDF`, so
the expected terminal exception is `UsageFault`:

```bash
cargo build --release \
  --manifest-path boards/nucleo-f767zi/Cargo.toml \
  --target thumbv7em-none-eabihf
```

The alternate image leaves UsageFault disabled before the same instruction,
exercising escalation to `HardFault`:

```bash
cargo build --release \
  --manifest-path boards/nucleo-f767zi/Cargo.toml \
  --target thumbv7em-none-eabihf \
  --features hardfault-escalation-probe
```

Both probes are intentionally terminal and must not be used as an application
runtime. Hardware evidence must record the exact ELF or Flash-image hash,
board/PCB and silicon revisions, probe, feature set, state word, all 19 evidence
words, and confirmation that a reset does not return to the planted fault.

## C1.P1.3 verification status

On 2026-08-25, both release configurations cross-built with Rust 1.95.0.
Inspection of the default ELF confirmed the four populated architectural fault
vectors, the shared entry, a 76-byte evidence object, and an 84-byte total
`.uninit` allocation. Host tests cover every accepted `EXC_RETURN` layout, all
six frame-rejection status bits, and the fixed evidence offsets.

Physical observation of the configurable UsageFault and escalated HardFault is
not yet recorded because no debug probe was connected during this validation.
Consequently, C1.P1.3 and the C1.P1 phase are not represented as closed.
