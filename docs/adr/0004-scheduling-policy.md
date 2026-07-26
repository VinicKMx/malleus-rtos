# ADR-0004 — Fixed-priority preemptive scheduling, with async executors inside priority levels

> **Status:** Accepted · **Checkpoint:** 0

## Context

Two questions, and they interact.

**First: which scheduling policy?** The realistic candidates are fixed-priority
preemptive, earliest-deadline-first, and rate-monotonic (which is a fixed
priority *assignment*, not a separate policy).

**Second: how do `async` and preemption coexist?** This is the harder question,
and getting it wrong is the most likely way to produce a system that satisfies
neither audience.

`async` gives cheap concurrency: no stack per logical activity, no context
switch to interleave waits. It is excellent for connectivity, where a device
juggles a TLS handshake, an MQTT keepalive, and a Modbus request.

`async` cannot preempt. An `async` task computing for 3 ms blocks every other
future on its executor for 3 ms, however urgent they are. A cooperative
scheduler has no answer to "this must run within 500 µs, now".

An industrial controller needs both, and a design that forces a choice gives the
worst of each: either the control loop is at the mercy of the network stack, or
the network code is written as preemptive tasks with a full stack each and none
of the ergonomics.

## Decision

**Fixed-priority preemptive scheduling for tasks. Multiple `async` executors,
each pinned to one priority level.**

```text
  priority 8–15   preemptive hard real-time tasks
                  ├─ safety-monitor    2 kHz, 200us deadline
                  └─ motor-control     1 kHz, 500us deadline

  priority 6      async executor — control-plane I/O
                  ├─ encoder read
                  └─ ADC/DMA completion

  priority 4      async executor — network and protocol services
                  ├─ Modbus RTU server
                  ├─ MQTT client
                  └─ OTA handler

  priority 2      async executor — telemetry, storage, diagnostics
```

A hard real-time task preempts every executor beneath it. Inside one executor,
futures cooperate.

Supporting decisions:

- **32 priority levels**, so the ready set is one word and the "highest ready
  priority" lookup is a single `CLZ` — O(1) with a small constant, not
  amortised.
- **FIFO within a level.** No round-robin time slicing: it adds a preemption
  point with no analytical benefit and makes response times worse.
- **No ageing, no anti-starvation.** Starving a low-priority task is *correct*
  when a high-priority one is runnable. A heuristic that prevents it makes the
  scheduler's behaviour history-dependent, which destroys analysability.
- **Priorities are assigned by the engineer**, not derived. The analyser reports
  that an assignment misses a deadline; it does not silently fix it.

## Consequences

### What this buys

- Exact response-time analysis applies directly (see
  [real-time model](../design/04-realtime-model.md)).
- Each piece of work uses the model that suits it, and the manifest records
  which — visible in review.
- `async` connectivity code gets Embassy-like ergonomics without the control
  loop inheriting its scheduling.
- Overload degrades predictably from the bottom up.

### What this costs

- **The engineer must choose** which model each piece of work belongs in, and
  choosing badly is possible. A long computation placed in an executor blocks
  its neighbours. The tool can warn about declared budgets; it cannot warn about
  a future that is simply slow.
- **Lower CPU utilisation than EDF** at the theoretical limit.
- **Multiple executors cost RAM** — one stack per priority level that has one.
- **Priority assignment is manual**, and getting it wrong is easy in a system
  with many tasks. Rate-monotonic assignment is a good default; the analyser
  should eventually suggest it. It does not yet.

### What it forecloses

Any scheduling policy whose behaviour depends on execution history. That is the
intended effect.

## Alternatives considered

**Earliest-deadline-first.** Optimal for uniprocessor scheduling; achieves
higher utilisation. Rejected for **overload behaviour**: under transient
overload, EDF can cascade into everything missing, while fixed priority degrades
from the bottom. For a system where the motor loop must survive an overload
caused by the network stack, predictable degradation beats utilisation. EDF is
also considerably harder to reason about during an incident, which is a real
engineering property even though it does not appear in any equation.

**Async only, no preemption** (the Embassy model taken to its conclusion).
Simpler, less RAM, excellent ergonomics. Rejected because a 1 kHz control loop
cannot be guaranteed against a cooperative executor. This is the single decision
that most distinguishes Malleus from Embassy, and if it turns out that priority
executors are sufficient for real control loops, much of Malleus's rationale
weakens.

**Preemptive tasks only, no async.** The classic RTOS model. Rejected because
connectivity code becomes deeply unpleasant — a stack per protocol activity, and
state machines by hand where a future would do.

**One executor with internal priorities.** Attractive, and it avoids the
per-executor stack. Rejected because it cannot preempt: a running future still
blocks a higher-priority one on the same executor. It provides ordering, not
preemption, and the difference is exactly what hard real-time means.

## Revisit when

- Measurement shows the per-executor stack cost is prohibitive on realistic
  parts. That would argue for fewer executor levels, not for a different policy.
- A convincing case emerges for mixed-criticality scheduling (e.g. deferrable
  servers for aperiodic work). This is the most likely genuine extension, and it
  layers on fixed priority rather than replacing it.
- Someone demonstrates that EDF's overload behaviour can be bounded acceptably
  for this domain — the literature on constant-bandwidth servers is relevant and
  we may simply be wrong about the trade.
