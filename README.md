<div align="center">

# Malleus RTOS

**A Rust-native RTOS for industrial controllers and connected critical devices.**

Deterministic hard real-time tasks and `async` connectivity in one firmware image —
with hardware-enforced fault isolation, timing contracts checked at build time,
and post-mortem diagnostics you can read from the field.

[![status](https://img.shields.io/badge/status-checkpoint%200%20%C2%B7%20pre--alpha-orange)](#what-is-built)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![rust](https://img.shields.io/badge/rust-1.90%2B-b7410e)](docs/adr/0011-toolchain-and-msrv-policy.md)

[Manifesto](docs/design/00-manifesto.md) ·
[Comparison](docs/design/06-comparison.md) ·
[ADRs](docs/adr/) ·
[Contributing](CONTRIBUTING.md)

</div>

---

> [!WARNING]
> **Malleus RTOS does not run yet.** This is Checkpoint 0: the architecture is
> designed, the contracts are written, and the build-time analysis works. The
> kernel does not boot on hardware. Nothing here is usable in a product, and the
> roadmap says exactly when each piece arrives. If you need a working Rust RTOS
> today, use [Embassy](https://embassy.dev), [RTIC](https://rtic.rs), or
> [Zephyr](https://zephyrproject.org) — all excellent, all shipping.
>
> What *does* work today: `cargo malleus check` and `cargo malleus analyze`.
> They need no hardware and no kernel. Try them below.

---

## The one sentence

> Choose Malleus over Embassy, Zephyr, or FreeRTOS when you need **hard
> real-time control and async connectivity in the same firmware**, with
> **hardware-enforced fault isolation**, **timing contracts verified at build
> time**, and **diagnostics that survive a crash in the field**.

If you do not need all four, one of the existing options is probably a better
fit, and [the comparison document](docs/design/06-comparison.md) says which one
and why. A project that cannot name who should *not* use it has not thought
hard enough about who should.

## The problem

An industrial controller runs a 1 kHz motor loop and an MQTT client in the same
firmware. The MQTT client dereferences a null pointer.

On most embedded systems, one of two things happens. Either the whole device
resets — and the motor stops mid-move — or the fault silently corrupts memory
and the motor does something worse. Either way, the field engineer gets a device
that "just rebooted sometimes" and no evidence about why.

That is not a Rust problem, and Rust does not solve it. Rust removes memory bugs
from *safe code*. It does nothing about a DMA engine writing to the wrong
address, a vendor C driver, a task that overruns its deadline, or a `panic!` in
a library you did not write. In every one of those cases the question is not
"could this happen" — it is **what does the product do when it does**.

## The answer

```
  MQTT client dereferences a null pointer
    │
    ├─ MPU raises a memory fault ......... the isolation boundary held
    ├─ kernel records who, where, what ... attributed to one task
    ├─ only that task is stopped ......... motor loop never misses a tick
    ├─ supervisor applies its policy ..... restart, per the manifest
    ├─ crash dump persisted to flash ..... readable later by `cargo malleus dump`
    └─ 5 failures in 60s → degraded mode . network shed, local control kept
```

Four ideas make that possible, and each is a design commitment rather than a
feature:

**1 · The system is declared, not discovered.** Tasks, priorities, stacks,
deadlines, resources, and channels live in `malleus.toml`. There is no `spawn`.
That is a real constraint, and it buys the only thing that matters here: a
system whose worst case can be computed before it ships.

**2 · Timing contracts are checked by the build.** `cargo malleus analyze` runs
an exact response-time analysis on the declared task set. It tells you a
deadline is missed at compile time, not at 3 a.m. in a plant.

**3 · Isolation is hardware-enforced and capability-gated.** Each task reaches
only the peripherals and channels it declared. A telemetry task cannot touch the
motor PWM register — not by convention, but because the MPU will not let it.

**4 · Failure is a first-class, survivable event.** Faults are attributed,
contained, recorded, and recovered from according to a policy you wrote down.

## Try it now

The build-time analysis is real and runs today. No board required.

```bash
git clone https://github.com/VinicKMx/malleus-rtos
cd malleus-rtos
cargo build -p cargo-malleus

./target/debug/cargo-malleus malleus analyze \
  --manifest examples/industrial-controller/malleus.toml
```

```text
Schedulability — fixed-priority preemptive, exact response-time analysis
System: industrial-controller   tick rate: 1MHz

  Task                    Prio     Period   Deadline       WCET  Verdict
  safety-monitor             9        500        200         40  PASS   response 40t, slack 160t
  motor-control              7       1000        500        180  PASS   response 220t, slack 280t
  sensor-acquisition         6       2000       2000        300  PASS   response 560t, slack 1440t
  modbus                     4      10000      10000        900  PASS   response 1720t, slack 8280t
  telemetry                  2     100000     100000       8000  PASS   response 16920t, slack 83080t

  CPU utilisation (periodic tasks): 58.0%

  Verdict: PASS
```

Look at `telemetry`: it needs 8 ms of CPU but does not finish for 16.9 ms,
because everything above it keeps preempting. It still meets its deadline — but
that gap is why "58% CPU" is not an answer to "will it hold".

Now break it, three ways.

**Give `modbus` a 9 ms budget** in its 10 ms period. Each task's numbers are
individually plausible; the system is not:

```text
  modbus                     4      10000      10000       9000  FAIL   response 12840t, over by 2840t
  telemetry                  2     100000     100000       8000  FAIL   response 109480t, over by 9480t

  CPU utilisation (periodic tasks): 139.0%

  Verdict: FAIL — at least one task provably misses its deadline.
```

Note that `telemetry` fails too, though nobody touched it. That is the point of
analysing the *system*: a change to one task's budget breaks a task three
priority levels away, and no amount of reviewing either task in isolation would
reveal it.

**Set `motor-control`'s `wcet` to `600us`,** longer than its 500 µs deadline.
This never reaches the analyser — the manifest validator rejects it first, with
the more specific diagnosis:

```text
error[M0007]: declared worst-case execution time 600us exceeds deadline 500us;
              this task cannot meet its deadline even alone on the CPU
  --> motor-control
  help: reduce the work per activation, split it across activations, or relax the deadline
```

**Delete a `wcet` line entirely,** and watch the uncertainty spread:

```text
  motor-control              7       1000        500          -  UNKNOWN  no worst-case execution time
                                                                 declared; measure it with
                                                                 `cargo malleus trace`
  sensor-acquisition         6       2000       2000        300  UNKNOWN  a higher-priority task has no
                                                                 declared period or WCET, so the
                                                                 interference on this task cannot
                                                                 be bounded
  ...
  Verdict: UNKNOWN — at least one task lacks a declared WCET.
  This is not a failure. It is the honest answer, and it names what to
  measure next. Malleus will not print PASS for a system it cannot check.
```

One missing number poisons every verdict beneath it, because treating an unknown
as zero would produce a confident and wrong `PASS`. That propagation is the
project's temperament in one output: a tool that says `UNKNOWN` when it does not
know is worth more than one that says `PASS` because it assumed.

`cargo malleus check` covers the manifest itself — priority inversions by
construction, MPU alignment waste, channels nobody can use, capabilities granted
to non-endpoints. Every diagnostic states what is wrong, where, and what to do
about it; that last part is enforced by a test, not by good intentions.

## What is built

| Component | Status |
|---|---|
| Architecture contract (`malleus-arch`) | Defined, with an 18-item conformance suite |
| Manifest schema, parser, validator | **Working** — 23 diagnostic classes |
| Response-time analyser | **Working** — exact, with pinned reference numbers |
| `cargo malleus check` / `analyze` | **Working** |
| MalleusRT kernel | Types and contracts only — no scheduler yet |
| Cortex-M7 port | Not started — Checkpoint 1 |

73 tests, `clippy -D warnings` clean, cross-compiles for `thumbv7em-none-eabihf`
and `thumbv8m.main-none-eabihf`.

## Where to read next

| If you want to know… | Read |
|---|---|
| What this is and why it should exist | [Manifesto](docs/design/00-manifesto.md) |
| How it differs from Embassy, RTIC, Tock, Hubris, Zephyr, FreeRTOS | [Comparison](docs/design/06-comparison.md) |
| Whether the timing claims are real | [Real-time model](docs/design/04-realtime-model.md) |
| What happens when a task fails | [Fault model](docs/design/05-fault-model.md) |
| Why a decision was made the way it was | [ADRs](docs/adr/) |
| What is *deliberately* excluded | [Non-goals](docs/design/12-non-goals.md) |

## Target hardware

Bring-up runs on a **Nucleo-F767ZI** (Cortex-M7, 2 MB flash, 512 KB RAM,
on-board Ethernet) — chosen because it is the board on the bench and because its
ARMv7-M MPU is the *harder* case: power-of-two, naturally-aligned regions. A
region allocator designed against that constraint generalises to ARMv8-M; one
designed against ARMv8-M does not.

**Cortex-M33** (ARMv8-M, 32-byte-granular regions, TrustZone) remains the
strategic target. RISC-V with PMP comes later. See
[ADR-0001](docs/adr/0001-target-architecture.md).

## Contributing

The project is at the stage where **design review is worth more than code**. If
you have shipped an industrial product on FreeRTOS or Zephyr and something in
the [manifesto](docs/design/00-manifesto.md) or the
[ADRs](docs/adr/) looks wrong, that is the most valuable
contribution available right now. Open an issue and say so plainly.

See [CONTRIBUTING.md](CONTRIBUTING.md), [GOVERNANCE.md](GOVERNANCE.md), and
[SECURITY.md](SECURITY.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option — the Rust ecosystem convention, chosen so Malleus can be used in
commercial products without a lawyer being consulted first.
