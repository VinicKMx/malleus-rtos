# ADR-0011 — Pinned toolchain, declared MSRV, reviewed bumps

> **Status:** Accepted · **Checkpoint:** 0

## Context

Two distinct questions that are often conflated.

**Which compiler do we develop and test with?** If contributors and CI use
different versions, "works on my machine" becomes a category of bug, and
published benchmark numbers become incomparable — a context-switch measurement
from one compiler version is not the same measurement as from another.

**Which is the oldest compiler users may have?** Industrial users often have
pinned, vendor-blessed, or air-gapped toolchains. A project that requires the
newest stable release each month is unusable to a large part of the target
audience — and that audience is precisely the one Malleus is for.

## Decision

**Pin the development toolchain. Declare a separate, older MSRV. Bump either
only as a reviewed change.**

```toml
# rust-toolchain.toml — what we develop and test with
[toolchain]
channel = "1.95.0"
components = ["rustfmt", "clippy", "llvm-tools"]
targets = ["thumbv7em-none-eabihf", "thumbv8m.main-none-eabihf"]
```

```toml
# Cargo.toml — the oldest we support
rust-version = "1.90"
edition = "2024"
```

### Rules

1. **Stable Rust only.** No nightly features in any crate that ships. Nightly
   may appear in optional developer tooling, never on the path to a firmware
   image.
2. **MSRV is roughly six months behind stable**, giving vendors and enterprise
   toolchains time to catch up.
3. **An MSRV bump is a minor version bump** before 1.0 and a **major** one
   after, announced in the changelog with the reason. "A dependency needed it"
   is a reason; "a nicer syntax became available" is not.
4. **CI verifies both**: the pinned toolchain and the declared MSRV. An MSRV
   that is not tested is a guess.
5. **Edition 2024**, adopted because the MSRV already exceeds its requirement.
6. **Toolchain bumps are reviewed changes** with a changelog entry, because a
   compiler change can move every published benchmark number and silently
   invalidate the comparison.
7. **`clippy -D warnings` in CI.** A warning nobody must fix is a warning
   everybody ignores.

## Consequences

### What this buys

- One compiler produces every benchmark number, so the numbers are comparable
  across releases and a regression is real rather than an artefact.
- Contributors and CI agree.
- Industrial users with pinned toolchains can adopt.
- No nightly dependency, so a firmware image never depends on an unstable
  feature that might change.

### What this costs

- **New language features arrive late.** Async traits, const generics
  improvements, and similar are unavailable for months after stabilisation.
- **Some dependencies will require a newer compiler**, forcing a choice between
  the dependency and the MSRV. That choice must be made deliberately each time.
- **Testing two toolchains** doubles part of the CI matrix.
- **Nightly-only tooling is off the table** for anything on the shipping path —
  which occasionally means writing something by hand that a nightly feature
  would have given us.

### What it forecloses

Nightly-dependent design. Some elegant approaches to `async` in `no_std` have
historically needed nightly; those are unavailable until they stabilise. This is
a real constraint on the Checkpoint 2 executor work.

## Alternatives considered

**Track latest stable, no MSRV policy.** Simplest, best language features.
Rejected: it excludes exactly the industrial users the project targets.

**Nightly for the kernel.** Would allow more expressive `unsafe` and better
`async` machinery. Rejected outright — a production RTOS on a moving compiler is
not a production RTOS.

**A very old MSRV, e.g. 2 years.** Maximum compatibility. Rejected as too
costly: it would rule out edition 2024, let-chains, and a large amount of
`const fn` capability that the kernel design actively uses.

**No pinned toolchain, MSRV only.** Rejected because benchmark reproducibility
requires a pinned compiler. Real-time numbers that cannot be compared across
releases are not much use.

## Revisit when

- A dependency essential to a checkpoint requires a newer MSRV. Decide
  deliberately, record it here.
- Six months of evidence suggests the MSRV lag is either too aggressive (users
  cannot adopt) or too conservative (nobody is actually held back). Either is
  useful data.
- Async trait support in stable Rust reaches the point where the Checkpoint 2
  executor could be materially simpler.
