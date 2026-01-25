# Nucleo-F767ZI target foundation

> **Status:** linkable target foundation only. This image has not been shown to
> boot and must not be treated as hardware validation.

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

The exact board and silicon revision remain required evidence before hardware
execution is claimed. The revision does not change the memory addresses used by
this link-only foundation, but it can change which errata constrain reset,
cache, FPU, and exception work in later commits.

## Current entry contract

The vector table occupies the first 512 bytes of Flash. It contains only the
initial DTCM stack pointer and the `Reset` placeholder; all other entries are
zero. Flashing this image is not a validation step. C1.P1.2 will replace the
placeholder with bounded `.data` copy and `.bss` zeroing before any Rust state
is used.
