# Nucleo-F767ZI bring-up

> **Status:** reset and memory initialization observed on a physical
> Nucleo-F767ZI. This does not yet make the full Cortex-M7 port supported; later
> checkpoint requirements remain unimplemented.

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

The vector table occupies the first 512 bytes of Flash. It contains only the
initial DTCM stack pointer and the `Reset` entry; all other entries are zero.
`Reset` executes entirely in assembly while it copies the word-aligned `.data`
image from Flash and clears the word-aligned `.bss` range. Only then does it
enter Rust. `.uninit` is deliberately preserved.

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
