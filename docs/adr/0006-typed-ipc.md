# ADR-0006 — IPC is typed and generated from the manifest

> **Status:** Accepted · **Checkpoint:** 0 (design) · 3 (working)

## Context

Once tasks are isolated ([ADR-0005](0005-memory-isolation.md)), they cannot
share a pointer. Everything crossing a protection boundary goes through the
kernel. That makes the message channel the load-bearing abstraction of the whole
system rather than a convenience.

The traditional RTOS API is:

```c
xQueueSend(queue_handle, &buffer, timeout);
```

Every interesting property is unchecked. Is the sender allowed to talk to this
receiver? Is `buffer` the type the receiver expects? Is it the right size? What
happens when the queue is full? Passing the wrong handle is undefined behaviour.

These are exactly the errors that survive code review and appear in the field,
because nothing in the toolchain is looking for them.

## Decision

**Channels are declared in the manifest and generated into typed endpoints.**

```toml
[[channel]]
name     = "motor-command"
from     = "modbus"
to       = "motor-control"
capacity = 4
overflow = "reject"
```

```rust
// Generated. `motor` exists only inside tasks holding `ipc.motor-command`.
motor.send(SetSpeed { rpm: 1_500 }, Timeout::Ticks(10)).await?;
```

Checked at **compile time**: who may talk to whom; which message types are legal;
maximum encoded size; the overflow policy; whether the caller is an endpoint at
all. An undeclared task cannot even *name* the endpoint, so the check is a
compile error, not a runtime denial.

Checked at **runtime** only what cannot be known statically: whether the receiver
is currently alive (`Error::PeerUnavailable`).

Supporting decisions:

- **`overflow` has no default.** Every channel declares one. The right answer is
  entirely application-specific and the wrong answer is invisible until the
  system is under load — which is the day it matters. Options are `block`,
  `reject`, `drop-oldest`, `drop-newest`, each documented with when it is right.
- **The validator flags priority inversion by construction** (M0018): a
  high-priority task blocking on a channel drained by a lower-priority one.
- **Unused channels are flagged** (M0022) — they cost RAM and clutter the graph.
- **Capabilities must match endpoints** (M0021): a task cannot hold
  `ipc.motor-command` unless it is actually an endpoint of that channel.
- **`static_bytes()` is reported per channel**, because a generously sized
  channel is one of the easiest ways to run out of RAM, and a build-time table
  is the cheapest place to find the waste.

## Consequences

### What this buys

- Whole classes of IPC bug become compile errors.
- The communication topology is a build artefact: drawable, diffable, reviewable.
- The RAM cost of every channel is known and reported.
- Overflow behaviour is a documented decision rather than an emergent property.
- Versioning becomes tractable — the generator can check message compatibility
  across firmware versions.

### What this costs

- **Adding a channel means editing the manifest,** not just writing code. This
  is friction, and it is deliberate: a new communication path between protection
  domains *should* be a visible change.
- **The generator is now load-bearing.** A bug in it is a bug in every system.
- **Generated code must be inspectable** or engineers cannot verify it — hence
  `cargo malleus expand`, and the rule in `malleus-codegen` that everything
  generated is readable.
- **Dynamic topologies are impossible.** A task cannot open a channel to a peer
  chosen at runtime.
- **Serialisation cost.** Crossing a protection boundary means encoding, and for
  large messages that copy is measurable.

### What it forecloses

Anonymous or discovered communication — service registries, dynamic
publish/subscribe between tasks. The IPC graph is fixed at build time. External
pub/sub (MQTT to a broker) is unaffected; this is about intra-device messaging.

## Alternatives considered

**Raw byte-buffer send/receive with a handle.** Simple, familiar, flexible.
Rejected: it is the status quo whose failure modes motivated this decision.

**Shared memory with a lock.** Fastest, no copying. Rejected because it
contradicts isolation — a shared region is a hole in the boundary. It remains
available as an explicit, declared opt-in for bulk data paths where the copy is
genuinely unaffordable, and being explicit is the point.

**A proc-macro DSL in Rust instead of TOML.** Type-checked, one language.
Rejected for the same reason as [ADR-0002](0002-static-system-definition.md):
the analyser, the diagram generator, and the memory report all need to read the
topology *as data*, and a macro-embedded DSL is only readable by the compiler.

**An IDL like Protocol Buffers or Cap'n Proto.** Mature, versioned,
cross-language. Rejected as too heavy for intra-device messages on a
microcontroller, where a message is often 8 bytes and the schema machinery would
outweigh the payload. Worth revisiting for the *host*-facing protocol, which is
a different problem.

## Revisit when

- The C FFI work (Checkpoint 5) forces a decision about how a C task
  participates in typed IPC. The likely answer is generated C headers with
  runtime checks at that one boundary, but it may argue for a different core
  design.
- Message versioning across OTA updates turns out to need more than the
  generator can express, at which point a real IDL becomes worth its weight.
