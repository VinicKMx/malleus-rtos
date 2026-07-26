# Working conventions

Notes for anyone — human or agent — making changes here.

## What this project is

**Malleus RTOS** is the platform; **MalleusRT** (`crates/malleusrt`) is the
kernel. A Rust RTOS for industrial controllers: hard real-time plus `async`,
MPU-enforced isolation, build-time timing analysis, production diagnostics.

Read [docs/design/00-manifesto.md](../design/00-manifesto.md) first. It states
the thesis and the criteria by which the project should be judged — or
abandoned.

**Status: Checkpoint 0.** The kernel does not boot. `cargo malleus check` and
`cargo malleus analyze` work. Everything else is design.

## Commands

```bash
cargo xtask ci        # everything CI runs, in the same order
cargo xtask test
cargo xtask lint
cargo xtask cross     # bare-metal targets
./ci/check-unsafe.sh

cargo build -p cargo-malleus
./target/debug/cargo-malleus malleus analyze --manifest examples/industrial-controller/malleus.toml
```

Note the argv quirk: the binary is invoked as `cargo malleus ...`, so `main`
strips `"malleus"` from position 1. Running the binary directly requires that
extra word.

## Non-negotiable conventions

**Every `unsafe` block has a `SAFETY:` comment.** Machine-enforced
(`undocumented_unsafe_blocks = "deny"`). A rushed one is worse than none,
because it looks like review happened. `unsafe` is confined to `malleusrt` and
`malleus-arch-*`; `ci/check-unsafe.sh` enforces the boundary.

**Every public operation documents its contract** — complexity, allocation,
blocking, ISR-safety, timeout. This is the product, not a documentation habit.

**Every diagnostic says what to do about it.** What is wrong, *where*, and *what
to do*. A test in `malleus-manifest` fails if a diagnostic has no suggestion.

**Tests state what they are protecting.** `test_analyse_2` tells a future reader
nothing about whether breaking it matters.

**Never claim output you have not run.** Documented terminal output must come
from an actual run. This has already been violated once, and the two-layer
validator/analyser structure was discovered by fixing it.

**Never print `PASS` for something unverified.** The analyser returns `UNKNOWN`
when a WCET is missing and propagates that uncertainty to every task below. This
temperament is the product; do not "improve" it into a confident guess.

## Lint scoping — deliberate, not an oversight

Workspace-wide: `missing_docs`, `undocumented_unsafe_blocks`,
`missing_safety_doc`, `unwrap_used`, `todo`.

**Kernel-only** (declared at the top of `malleusrt` and `malleus-arch`):
`arithmetic_side_effects`, `indexing_slicing`.

A panicking add in a scheduler is a system that stops scheduling. The same lint
on an offline analyser buys nothing and costs readability. Do not "fix the
inconsistency" by applying them everywhere — see
[ADR-0010](../adr/0010-unsafe-code-policy.md).

`clippy.toml` allows `unwrap`/`expect`/`panic`/indexing **in tests**.

## Architecture facts worth knowing

- The kernel is `#![no_std]` with **no** `#[cfg(target_arch)]`, so it compiles
  and tests on the host. Anything CPU-specific belongs in `malleus-arch-*`.
- `boards/` and `examples/` are excluded from the root workspace so that
  `cargo test` stays host-only and always green.
- Priorities: higher number is more urgent. ARM NVIC is the opposite. The
  inversion happens in exactly one place, `Priority::to_hardware`.
- Reference-application response times are **pinned by a test** in
  `malleus-analyzer`. If you change the analyser, that test tells you the docs
  have become fiction.

## Before changing a design decision

Check [docs/adr/](../adr/). If a change contradicts an ADR, either the change
is wrong or the ADR is — say which. ADRs are **immutable once accepted**; a
changed mind produces a superseding ADR, never an edit.

Every ADR has a "revisit when" section. If the stated condition has not
occurred, the decision probably still holds.

## Before adding scope

[docs/design/12-non-goals.md](../design/12-non-goals.md) lists what will not
be built. Adding anything requires answering: why is it core to the thesis, why
can nothing existing be integrated, and **what comes off the roadmap to pay for
it**. The third question is the real one.

## Documentation is a deliverable

Not an afterthought. Design documents carry a status header and a checkpoint. A
document describing something unbuilt says so, in the present tense. A
documentation set that describes aspirations as if they were facts is how a
project loses the ability to tell its users the truth — and for a pre-alpha RTOS
asking people to bet a product on it, that is the whole asset.
