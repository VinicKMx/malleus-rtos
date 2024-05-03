# Observability

> **Status:** accepted · **Checkpoint:** 0 (design) · 4 (working)

## 1. The premise

**A device that cannot explain itself cannot be supported.**

An industrial controller in a cabinet at a remote site fails. Someone has to
determine why, usually without physical access, often months after the firmware
was written, sometimes from a single reboot counter. The difference between a
product and a prototype is largely whether that is possible.

Observability is therefore not a debugging convenience bolted on at the end. It
is a product requirement with the same standing as the control loop.

## 2. Three artefacts

| | Trace | Crash dump | Health record |
|---|---|---|---|
| **When** | Live, during development or a field session | Once, at fault time | Periodically |
| **Cost** | Bounded per event, perturbs timing slightly | One-off, from a fault handler | Negligible |
| **Answers** | "What is it doing?" | "Why did it die?" | "How is it, generally?" |

All three share one wire format (`malleus-trace`), so the host tooling decodes
them the same way and a dump can carry recent trace events as context.

## 3. `cargo malleus inspect` — live state

```text
Task                State       Priority  CPU     Stack      Deadline
motor-control       Running     7         18%     61%        OK
sensor-acquisition  Waiting     6          8%     42%        OK
storage             Blocked     3          4%     73%        —
telemetry           Waiting     2         11%     55%        MISSED 2
```

Four things are visible at a glance that are normally invisible:

- **`Blocked` versus `Waiting`.** Blocked means another task holds what I need —
  which can indicate priority inversion. Waiting means the outside world has not
  spoken yet — which cannot. Collapsing them into one state throws away the
  distinction that matters during diagnosis.
- **Stack high-water**, so an overflow is predicted rather than discovered.
- **Deadline misses**, counted per task rather than aggregated away.
- **CPU per task**, which is how you find the task that grew.

## 4. `cargo malleus trace` — the timeline

Event categories: scheduling (switch, ready, block, preempt); timing (activation,
deadline met or missed); synchronisation (lock acquire/release, priority
inheritance applied); IPC (send, receive, queue depth, overflow drop);
interrupts (entry, exit, latency sample); faults (raised, supervisor decision,
restart); power (idle entry/exit, sleep depth).

**Three constraints shape the format:**

1. **Bounded cost per event.** Tracing that perturbs timing tells you about a
   system that no longer exists. Every event has a fixed, documented cost,
   published alongside the benchmarks — so you can subtract it.
2. **Lossy under overload, and honest about it.** When the buffer fills, events
   are dropped and **the gap is recorded**. A trace with a silent hole is worse
   than one labelled "127 events lost here", because the silent one looks like
   data.
3. **Self-describing.** A stream carries the architecture, tick rate, and task
   table it was produced with, so a capture from a field device is readable
   without reconstructing the exact build.

## 5. `cargo malleus dump` — post-mortem

Written from the fault handler, persisted to flash, survives reset.

Contents: the fault record (task, kind, address, PC, SP, occurrence count);
every task's state, priority, stack pointer, and high-water mark; the ready set;
recent trace events; reset reason; uptime; firmware version and build identity.

The constraints all follow from *when* it runs — inside a fault handler, on a
system that is already broken:

- No allocation, no locks, no kernel calls.
- Bounded time; the watchdog is still running.
- **Corruption-tolerant.** A partially written dump must still yield its
  readable prefix. An all-or-nothing format fails exactly when the device died
  mid-write, which is the interesting case.

## 6. Health record

Periodic, low-cost, and shipped over whatever transport the device has: uptime,
reset reason and whether it was abnormal, per-task restart counts, deadline-miss
counters, stack high-water marks, minimum free RAM, degraded-mode status, and
firmware version.

This is the artefact that turns "the device seems fine" into a statement with
evidence behind it — and the one that lets a fleet operator notice that stack
usage on one unit has been creeping up for three weeks.

## 7. Error messages are observability too

The same principle applies at build time. From `malleus-manifest`, today:

```text
warning[M0012]: stack 3KiB will occupy 4KiB on an ARMv7-M MPU, wasting 1024
                bytes to alignment
  --> telemetry
  help: round the stack up to `4KiB` to make the cost explicit, or move to an
        ARMv8-M part where regions are 32-byte granular
```

Every diagnostic states what is wrong, where, and what to do about it. The third
part is enforced by a test — a diagnostic without a suggestion is treated as an
incomplete diagnostic, and the test fails.

The aspiration for Checkpoint 3, once stack analysis exists:

```text
error: task `telemetry` requires 5.8 KiB of stack, but 4 KiB were reserved

  largest contributors:
    mqtt::client::connect   1.7 KiB
    tls::handshake          2.1 KiB

  help: stack = "8KiB"
```

## 8. Cost, stated

Observability is not free and the cost is published rather than hidden:

- `metrics` feature: per-task CPU accounting, stack high-water, deadline
  counters. Costs RAM and a few cycles per switch. **Benchmarks are published
  with it both on and off**, so the cost is a number, not a claim.
- `trace` feature: implies `metrics`, plus a per-event cost and a buffer.
- Crash-dump buffer: RAM reserved permanently for an event that may never happen.

A project that does not publish the cost of its instrumentation is asking you to
trust that it is small.

## See also

- [Fault model](05-fault-model.md)
- [Real-time model](04-realtime-model.md)
- `crates/malleus-trace` — the wire format
