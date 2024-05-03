# The system manifest

> **Status:** accepted · **Checkpoint:** 0 (schema and validation working)

`malleus.toml` is the single declarative description of a system. Everything
downstream derives from it: the task table, protection regions, typed IPC
endpoints, linker configuration, the schedulability report, the memory report,
and the architecture diagram.

## 1. Why a file and not Rust code

The interesting questions are **whole-system** properties. Is the task set
schedulable? Do the protection regions fit the MPU? Does total stack plus
channels plus kernel exceed RAM? Does the IPC graph have a cycle?

Rust's type system reasons brilliantly about one crate at a time. It has nothing
to say about whether the sum of your stacks fits in 512 KiB.

The manifest is deliberately **data, not code**. It can be diffed, reviewed,
generated, and analysed by tools that are not the compiler — including tools
nobody has written yet. See
[ADR-0002](../adr/0002-static-system-definition.md).

## 2. Complete example

```toml
[system]
name      = "industrial-controller"
board     = "nucleo-f767zi"
tick_rate = "1MHz"

[[task]]
name     = "motor-control"
priority = 7
period   = "1ms"
deadline = "500us"
wcet     = "180us"
stack    = "2KiB"
restart  = "never"
capabilities = ["timer.control", "spi.encoder", "pwm.motor", "ipc.motor-command"]

[[task]]
name     = "telemetry"
priority = 2
period   = "100ms"
wcet     = "8ms"
stack    = "8KiB"
restart  = { on-fault = { budget = 5, window = "60s" } }
capabilities = ["net.mqtt", "ipc.sensor-data"]

[[channel]]
name     = "sensor-data"
from     = "sensor-acquisition"
to       = "telemetry"
capacity = 16
overflow = "drop-oldest"
```

## 3. Schema

### `[system]`

| Key | Required | Meaning |
|---|---|---|
| `name` | yes | Appears in the firmware image and telemetry |
| `board` | yes | Board support crate to build against |
| `tick_rate` | no | Monotonic tick rate. Default `"1MHz"` |

### `[[task]]`

| Key | Required | Meaning |
|---|---|---|
| `name` | yes | Unique; identifies endpoints in generated code and traces |
| `priority` | yes | 1–31. Higher is more urgent. 0 is reserved for idle |
| `stack` | yes | e.g. `"2KiB"` |
| `period` | no | Activation period for periodic tasks |
| `deadline` | no | Relative deadline. Defaults to `period` |
| `wcet` | no | Declared worst-case execution time |
| `capabilities` | no | Resources this task may reach. Anything unlisted is unreachable |
| `isolated` | no | Default `true`. Isolation is opt-out, never opt-in |

### `[[channel]]`

| Key | Required | Meaning |
|---|---|---|
| `name` | yes | Unique |
| `from` / `to` | yes | Endpoint task names |
| `capacity` | yes | Queue depth |
| `overflow` | yes | `block`, `reject`, `drop-oldest`, `drop-newest` |

`overflow` has **no default**. The right answer is entirely application-specific
and the wrong answer is invisible until the system is under load — which is the
day it matters.

## 4. Units

Durations: `ns`, `us`, `ms`, `s`. Sizes: `B`, `KiB`, `MiB`.

**There is no default unit, and a missing one is an error.** A bare number in a
config file has an implied unit that lives only in the reader's head, and the
two most expensive unit confusions in embedded work are milliseconds versus
microseconds and KB versus KiB.

`KB` and `MB` are **rejected**, not silently interpreted. On a part with 512 KiB
of RAM, the difference between 1000 and 1024 compounds into a linker error whose
cause is not obvious.

## 5. Validation

`cargo malleus check` runs 23 classes of check. Every diagnostic states **what is
wrong, where, and what to do about it** — the last part enforced by a test in
`malleus-manifest`, not by good intentions.

```text
error[M0007]: declared worst-case execution time 600us exceeds deadline 500us;
              this task cannot meet its deadline even alone on the CPU
  --> motor-control
  help: reduce the work per activation, split it across activations, or relax the deadline
```

### Diagnostics

| Code | Severity | Condition |
|---|---|---|
| M0001 | error | Duplicate task name |
| M0002 | error | Empty task name |
| M0003 | error | Priority out of range |
| M0004 | error | Priority 0 is reserved for idle |
| M0005 | warning | Multiple deadline-bearing tasks share a priority |
| M0006 | error | Deadline longer than period |
| M0007 | error | WCET exceeds deadline |
| M0008 | warning | Deadline declared without a WCET — verdict becomes `UNKNOWN` |
| M0009 | warning | Period is not an exact tick multiple |
| M0010 | error | Malformed stack size |
| M0011 | error | Stack below the exception-frame minimum |
| M0012 | warning | ARMv7-M MPU alignment padding, with the exact byte cost |
| M0013 | error | Duplicate channel name |
| M0014 | error | Channel endpoint names an undeclared task |
| M0015 | error | Task sends to itself |
| M0016 | error | Zero channel capacity |
| M0017 | error | Invalid overflow policy |
| M0018 | warning | Priority inversion by construction |
| M0019 | warning | Duplicate capability |
| M0020 | error | Capability names an undeclared channel |
| M0021 | error | Capability held by a non-endpoint |
| M0022 | warning | Channel declared but unusable — wasted RAM |
| M0023 | error | Malformed duration |

**M0018 is the one worth highlighting.** A high-priority task blocking on a
channel drained by a lower-priority one is priority inversion built into the
architecture — not a race, not a timing accident, but a structural property
visible in the declaration. It is exactly the kind of thing that is obvious in a
diagram and invisible in code.

## 6. What is generated

| Artefact | Committed? | Purpose |
|---|---|---|
| `tasks.rs` | no | Static task table |
| `ipc.rs` | no | Typed endpoints, visible only to capability holders |
| `regions.rs` | no | Pre-computed protection region sets |
| `memory.x` | no | Linker script fragment |
| `layout.md` | **yes** | Memory report |
| `architecture.md` | **yes** | Task and IPC graph |

Generated *code* is build output. Generated *reports* are committed, so that a
pull request quietly growing a stack by 4 KiB or adding an IPC edge shows up as
a diff a reviewer can see. Making consequences visible in review is worth the
small noise of a checked-in generated file.

**All of it is readable.** `cargo malleus expand` writes generated code to a file
you can open, diff, and step through. In a system where a wrong protection region
means a fault that *does not happen*, the engineer has to be able to check.

## 7. Not yet in the schema

Honest list of what the design calls for and the schema does not yet express:

- **Shared resources** (mutexes), needed to compute blocking terms. Checkpoint 2.
- **Release jitter.** Checkpoint 2.
- **Memory placement hints** (e.g. "put this stack in DTCM"). Checkpoint 3.
- **Executor assignment** — which async executor a service runs on. Checkpoint 2.
- **Rendezvous channels** (zero-capacity, unbuffered). Referenced by the M0016
  diagnostic; not implemented.

Until shared resources exist, the analyser prints
`Blocking terms: not yet modelled` on every report rather than letting a reader
assume they were accounted for.

## See also

- [ADR-0002 — Static system definition](../adr/0002-static-system-definition.md)
- [ADR-0006 — Typed IPC](../adr/0006-typed-ipc.md)
- [Real-time model](04-realtime-model.md)
