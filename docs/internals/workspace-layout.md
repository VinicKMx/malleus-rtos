# Workspace layout

Why the repository is arranged this way. Mostly this is one decision with
consequences: **the root workspace is host-only.**

## The crates

```text
crates/
├── malleusrt/              the kernel — sched, task, time, sync, ipc, fault
├── malleus-arch/           architecture contract + conformance suite
├── malleus-arch-cortex-m/  ARM port                    (Checkpoint 1)
├── malleus-runtime/        task runtime, async executors (Checkpoint 2)
├── malleus-manifest/       manifest schema, parser, validator   ✅ working
├── malleus-analyzer/       response-time analysis               ✅ working
├── malleus-codegen/        generation from the manifest (Checkpoint 3)
└── malleus-trace/          trace and crash-dump wire format (Checkpoint 2)

tools/cargo-malleus/        the developer CLI               ✅ check, analyze
xtask/                      repository automation
boards/                     board support — separate workspace
examples/                   applications — separate workspace
ci/                         scripts CI runs
```

**MalleusRT** is the kernel; **Malleus RTOS** is the platform. Hence one crate
named `malleusrt` and the rest named `malleus-*`.

## The root workspace is host-only

`boards/` and `examples/` are **excluded** from the root workspace. They only
build for bare-metal targets and carry their own workspace.

The reason is that `cargo test` at the root should be a host-only, always-green
operation that needs no cross-toolchain and no hardware. Mixing bare-metal
members into the root workspace makes a plain `cargo test` either fail or
require target flags, and the first thing a new contributor does is run
`cargo test`.

## The kernel compiles for the host, deliberately

`malleusrt` and `malleus-arch` are `#![no_std]` and contain **no**
`#[cfg(target_arch)]`. They compile for `x86_64-unknown-linux-gnu` exactly as
they do for `thumbv7em-none-eabihf`.

That is what makes the 73-test host suite possible: scheduler logic, timing
arithmetic, and fault-disposition rules are all tested with plain `cargo test`,
in milliseconds, with no hardware.

Anything that needs to know what CPU it is running on belongs in an
`malleus-arch-*` crate instead. The `malleus-arch` crate is pure contract, and
keeping it that way is what lets the kernel be tested against a simulated
architecture.

**"Compiles for the host" is not evidence it compiles for a Cortex-M**, and the
gap is exactly where portability bugs live — a `usize` assumption that only
holds at 64 bits, for instance. So CI cross-compiles every bare-metal crate for
both ARM targets on every push, and `cargo xtask cross` does the same locally.

## Lints are scoped on purpose

Workspace-wide: `missing_docs`, `undocumented_unsafe_blocks` (denied),
`missing_safety_doc` (denied), `unwrap_used`, `todo`.

Kernel-only, declared at the top of `malleusrt` and `malleus-arch`:
`arithmetic_side_effects`, `indexing_slicing`.

A panicking add inside a scheduler is a system that stops scheduling. The same
lint on an offline analyser buys nothing and costs readability — and applying a
rule where it does not pay is how a codebase learns to reach for `#[allow]` by
reflex. A codebase that does that will do it in the kernel too. See
[ADR-0010](../adr/0010-unsafe-code-policy.md).

`clippy.toml` allows `unwrap`, `expect`, `panic`, and indexing in tests, because
a failing test is a panic and pretending otherwise just adds ceremony.

## Profiles

```toml
[profile.dev]
opt-level = 1        # -O0 firmware is often too slow to meet its own deadlines
overflow-checks = true

[profile.release]
opt-level = "s"
lto = "fat"
codegen-units = 1
debug = 2            # symbols live in the ELF, not the flash image — free
overflow-checks = true
panic = "abort"
```

Two of these are worth explaining.

**`debug = 2` in release** costs nothing on the device: debug info lives in the
ELF and is not part of the programmed image. It is what makes a crash dump
decodable months later, and stripping it to "save space" saves space that was
never on the device.

**`overflow-checks = true` in release** is unusual. In a control system, a
silently wrapped integer produces a plausible-looking wrong number that
propagates into an actuator. A detected overflow produces a fault the supervisor
can act on. The former is worse, and the cycles are affordable.

## Running the checks

```bash
cargo xtask ci      # everything, in the order CI runs it
cargo xtask fmt
cargo xtask lint
cargo xtask test
cargo xtask cross
```

CI runs the same commands. A pipeline that diverges from the local command
trains everyone to push and hope.
