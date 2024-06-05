# Contributing to Malleus RTOS

## The most valuable contribution right now is not code

The project is at Checkpoint 0. The architecture is written down and the kernel
does not exist. That means **design review is worth more than patches.**

If you have shipped an industrial product on FreeRTOS, Zephyr, or bare metal,
and something in the [manifesto](docs/design/00-manifesto.md), the
[ADRs](docs/adr/), or the [comparison](docs/design/06-comparison.md) looks wrong
— that is the contribution the project most needs. Open an issue and say so
plainly. "I tried this approach and here is how it failed" is worth a week of
speculation.

Particularly wanted:

- **Where the static task set breaks down.** If your product genuinely cannot be
  expressed without runtime task creation, that is important evidence about
  [ADR-0002](docs/adr/0002-static-system-definition.md).
- **Whether declared WCETs are realistic.** This is the project's biggest risk.
  If you think engineers will guess and the `PASS` will be meaningless, say so.
- **Whether 8 MPU regions are enough** for a real industrial task set.
- **Corrections to the comparison.** If you maintain Embassy, RTIC, Tock,
  Hubris, Zephyr, or FreeRTOS and we have misrepresented your project, please
  tell us. Getting that document wrong damages us more than it damages you.

## Before writing code

**Open an issue first** for anything beyond a typo or an obvious bug fix. Not
bureaucracy — the design is still moving, and it would be unkind to let someone
spend a weekend on something that conflicts with a decision recorded in an ADR.

## Setting up

```bash
git clone https://github.com/VinicKMx/malleus-rtos
cd malleus-rtos
cargo xtask ci     # exactly what CI runs, nothing more, nothing hidden
```

The toolchain is pinned in `rust-toolchain.toml`; `rustup` will fetch it.

## What CI requires

`cargo xtask ci` runs, in order:

| Step | Requirement |
|---|---|
| `fmt` | `cargo fmt --check` clean |
| `lint` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `test` | Full host suite passing |
| `cross` | Bare-metal crates build for `thumbv7em-none-eabihf` and `thumbv8m.main-none-eabihf` |
| `unsafe` | Every `unsafe` block documented |

Warnings are errors. A warning nobody must fix is a warning everybody ignores.

## Code standards

### Every `unsafe` block is documented

Non-negotiable, and machine-enforced via `undocumented_unsafe_blocks = "deny"`.

```rust
// SAFETY: `regions` was validated by the build-time allocator against this
// MPU's region count and alignment constraints, and the caller holds the
// scheduler lock, so no concurrent installation can occur.
unsafe { Memory::install(regions)? };
```

A rushed `SAFETY:` comment is worse than none, because it looks like review
happened. See [ADR-0010](docs/adr/0010-unsafe-code-policy.md).

### Every public operation documents its contract

```rust
/// # Contract
///
/// - O(1), no allocation.
/// - ISR-safe.
/// - Cannot block.
```

This is the product, not a documentation convention. An API that hides a
potentially unbounded operation behind a pleasant signature is not usable for
hard real-time work.

### Every diagnostic says what to do about it

```text
error[M0007]: declared worst-case execution time 600us exceeds deadline 500us
  --> motor-control
  help: reduce the work per activation, split it across activations, or relax the deadline
```

What is wrong, **where**, and **what to do**. The third part is enforced by a
test in `malleus-manifest`.

### Tests state what they are protecting

```rust
#[test]
fn uncertainty_propagates_downward_through_priorities() {
    // If the high-priority task's cost is unknown, nothing below it can be
    // declared safe — silently treating the unknown as zero would produce a
    // confident and wrong PASS.
```

A test name that describes the mechanism (`test_analyse_2`) tells a future
reader nothing about whether breaking it matters.

### Comments explain why, not what

The code says what it does. Comments are for the reasoning that is not
recoverable from reading it — the constraint, the alternative rejected, the bug
this shape prevents.

## Commits and pull requests

- Conventional-style prefixes: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`,
  `chore:`.
- Explain **why** in the body. The diff shows what.
- One logical change per PR.
- If it touches `unsafe` in `malleusrt` or an arch crate, it needs **two
  reviewers**.
- If it contradicts an ADR, either it is wrong or the ADR is. Say which in the
  description — do not leave the contradiction for a reviewer to discover.

## Adding a board

1. Implement `malleus_arch::Arch` for the architecture, if new.
2. Declare the isolation tier honestly. A board without protection hardware is
   tier 0 and must say so.
3. Supply the true NVIC priority-bit count. The kernel never guesses.
4. Pass every `Required` conformance item **on hardware**, not in simulation.
5. Document what is untested.

A board is not "supported" because it compiles. See
`malleus_arch::conformance`.

## Licensing

Contributions are accepted under **MIT OR Apache-2.0**, matching the project.
**There is no CLA** — you keep your copyright. This also means the license
cannot be changed out from under you later, which is deliberate. See
[ADR-0012](docs/adr/0012-licensing.md).

## Conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Short version: argue with ideas as
hard as you like; be decent to people.

## Security

Do not open a public issue for a vulnerability. See
[SECURITY.md](SECURITY.md).
