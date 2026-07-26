# Inter-task communication

> **Status:** accepted · **Checkpoint:** 0 (design) · 3 (working)

## 1. Why IPC is load-bearing

Once tasks are isolated, they cannot share a pointer. Everything crossing a
protection boundary goes through the kernel. That promotes the message channel
from a convenience to **the** abstraction the system is built on.

Get it wrong and either isolation is bypassed (shared memory everywhere) or the
system is unusable (every interaction is a syscall with a hand-rolled protocol).

## 2. Typed endpoints, generated

The traditional API:

```c
xQueueSend(queue_handle, &buffer, timeout);  // nothing is checked
```

The Malleus equivalent:

```rust
motor.send(SetSpeed { rpm: 1_500 }, Timeout::Ticks(10)).await?;
```

`motor` exists only inside tasks holding `ipc.motor-command`. An undeclared task
cannot name it — the error is "cannot find `motor` in this scope", at compile
time.

**Checked at compile time:** who may talk to whom; which message types are legal
on the channel; maximum encoded size; the overflow policy; whether the caller is
an endpoint.

**Checked at run time:** only what cannot be known statically — whether the
receiver is alive (`Error::PeerUnavailable`).

## 3. Overflow: no default, ever

```rust
pub enum Overflow { Block, Reject, DropOldest, DropNewest }
```

| Policy | Correct when | Wrong when |
|---|---|---|
| `Block` | Every message matters and the sender can wait | The sender is hard real-time — the analyser requires a bounded timeout and rejects `Forever` |
| `Reject` | The sender has something better to do and can retry | Losing the message silently matters and nobody checks the return |
| `DropOldest` | Sampled state — fresh data supersedes stale | Commands: dropping the oldest silently reorders intent |
| `DropNewest` | Logging — the first messages after an event are the informative ones | Anything where the latest value is the true one |

Two properties are exposed for analysis: `can_block()` feeds the sender's
worst-case blocking time, and `is_lossy()` marks the edge on the generated
architecture diagram so a lossy path is visible rather than buried in config.

The linter warns when `DropOldest` is used on a channel whose message type is
named like a command.

## 4. Priority inversion by construction

```text
warning[M0018]: `control` (priority 9) blocks on a channel drained by
                `logger` (priority 2)
  --> cmd
  help: this is priority inversion by construction: the urgent sender waits for
        the less urgent receiver. Use `drop-oldest` if the data is sampled
        state, or raise the receiver's priority
```

This is a *structural* property, visible in the declaration. Not a race, not a
timing accident — a property of the architecture that is obvious in a diagram
and invisible in code. Catching it in a linter is one of the clearest arguments
for declaring the topology as data.

## 5. Cost, reported

```rust
ChannelDescriptor::static_bytes() == capacity × max_message_bytes
```

Reported per channel in the memory report. A generously sized channel is one of
the easiest ways to run out of RAM on a microcontroller, and a build-time table
is the cheapest place to find the waste — far cheaper than a debugger session
six months later.

Unused channels are flagged (M0022): declared, costing RAM, cluttering the
graph, and reachable by nobody.

## 6. Restart semantics

When a task restarts, its channels are drained and its peers are notified.

**The notification is the part that is easy to skip and expensive to skip.** A
peer continuing as though nothing happened — holding a sequence number, a
session, a half-acknowledged command — is a rich source of second-order bugs
that appear only after a restart in the field, which is to say only in
production.

A waiter blocked on a channel whose peer faulted receives
`WakeReason::HolderFaulted` and must decide what to do. This is `std`'s lock
poisoning, for the same reason.

## 7. Bulk data

A hardware boundary means either a copy or an explicitly declared shared region.
There is no third option.

For most industrial messages — a setpoint, a sensor reading, a command — the copy
is a few bytes and irrelevant. For genuinely large transfers (a firmware image,
an oscilloscope capture), a shared region declared by both endpoints is
available. It **weakens the boundary**, so it is explicit in the manifest,
visible on the architecture diagram, and a subject of review.

## 8. What this does not provide

- **No dynamic topology.** A task cannot open a channel to a peer chosen at
  runtime.
- **No service discovery or registry.** The graph is fixed at build time.
- **No multicast.** One sender, one receiver. Fan-out is multiple channels, which
  makes the RAM cost of fan-out visible — arguably a feature.
- **No cross-device IPC.** This is intra-device only. Talking to another device
  is MQTT, Modbus, or whatever protocol you chose.
- **No zero-copy between isolated tasks**, except via a declared shared region.

## See also

- [ADR-0006 — Typed IPC](../adr/0006-typed-ipc.md)
- [Capabilities](09-capabilities.md)
- [System manifest](07-system-manifest.md)
