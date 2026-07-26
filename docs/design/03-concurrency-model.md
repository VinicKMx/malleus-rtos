# Concurrency model

> **Status:** accepted · **Checkpoint:** 0 (design) · 2 (working)

## 1. Three kinds of concurrency, deliberately

Most RTOSes offer one model and make everything fit. Malleus offers three,
because an industrial controller genuinely contains three kinds of work.

| Kind | Mechanism | Preempts? | Cost | For |
|---|---|---|---|---|
| **Interrupt** | Hardware ISR | Everything below it | Cheapest | Time-critical hardware response |
| **Task** | Preemptive, fixed priority | Lower priorities | One stack each | Hard real-time control |
| **Future** | `async` on an executor | Nothing | Almost free | Connectivity, protocols, I/O |

The engineer chooses, and the manifest records the choice where a reviewer can
see it.

## 2. Why not just one

**Async only** (the Embassy model): elegant, cheap, and unable to guarantee a
1 kHz control loop. A future that computes for 3 ms blocks its executor for
3 ms, whatever else is waiting. Cooperative scheduling has no answer to "this
must run within 500 µs, now".

**Tasks only** (the classic RTOS model): guarantees the control loop, and makes
connectivity miserable. A stack per protocol activity, and hand-written state
machines where a future would do. A device juggling a TLS handshake, an MQTT
keepalive, and three Modbus requests needs either five stacks or five state
machines.

**So: both, layered.**

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

A hard real-time task preempts every executor beneath it. Inside an executor,
futures cooperate. **`async` is a way to get concurrency within a priority
level, not a replacement for the scheduler.**

## 3. Choosing

Use a **preemptive task** when the work has a deadline that must be met
regardless of what else is running; when it is compute-bound; or when it must be
isolated from a failure elsewhere.

Use a **future** when the work is I/O-bound and spends most of its time waiting;
when there are many similar activities (connections, requests); or when it has
no hard deadline.

Use an **ISR** only for what must respond in microseconds — and keep it short,
handing off to a task or waking a future.

**The rule of thumb:** if you would be upset that it was late, it is a task. If
you would merely be disappointed, it is a future.

## 4. Synchronisation

Every primitive documents its contract. The block is extracted by
`cargo malleus analyze` and cross-checked against call sites: calling a blocking
operation from a task declared hard real-time, without a bounded timeout, is a
build error rather than a code-review comment somebody might miss.

```text
Mutex::lock()
  complexity:   O(waiters)
  allocates:    no
  blocks:       yes
  isr-safe:     no
  timeout:      yes
  inheritance:  priority
```

### Priority inversion — two options, both offered

**Priority inheritance.** A task holding a mutex is temporarily raised to the
priority of the highest-priority waiter. Cheap when uncontended, requires no
declaration, bounds inversion to the length of the critical section. Does not
prevent deadlock, and does not bound chained blocking as tightly as ceiling.

**Immediate priority ceiling.** A task acquiring a mutex is raised at once to the
declared ceiling — the highest priority of any task that may acquire it. Bounds
each task to at most one block per activation, prevents deadlock among
ceiling-protected mutexes outright, and makes response-time analysis tractable.
Costs a declaration, which the analyser verifies against the actual user set.

Both are offered because they are genuinely different trade-offs. An RTOS that
picks one silently is making a systems decision on the engineer's behalf.

### Timeouts

There is **no unbounded wait by default**:

```rust
pub enum Timeout { None, Ticks(u64), Until(Instant), Forever }
```

`Timeout::Forever` exists and is deliberately verbose, so a reviewer sees it.
Most field hangs in embedded systems are an unbounded wait that nobody noticed
was unbounded.

### Wake reasons

```rust
pub enum WakeReason { Signalled, Expired, Cancelled, HolderFaulted }
```

`HolderFaulted` is the interesting one: the task holding the resource failed, so
its state is unknown and the waiter must decide. This is `std`'s lock poisoning,
for the same reason.

`Signalled` and `Expired` are returned rather than folded into a `Result`
because both are normal outcomes a control loop must distinguish, and `Result`
invites treating one as exceptional.

## 5. ISR safety

Enforced two ways: a compile-time marker on the ISR-safe API subset, and a
runtime check (`CriticalSection::in_interrupt()`) returning
`Error::NotFromInterrupt`. The runtime check backs up the compile-time one at
the C FFI boundary, where the type system cannot reach.

**ISR-safe:** `ReadySet` operations, `now()`, non-blocking sends, notifications,
waking a task.

**Not ISR-safe:** anything that can block — `Mutex::lock`, blocking sends,
`await`.

## 6. Critical sections

The usual way an RTOS destroys its own interrupt latency is blanket interrupt
disabling. Malleus offers two forms:

- `enter()` masks everything. Reserved for the few operations touching state
  shared with the fault handler. Every use is individually justified in
  `docs/internals/critical-sections.md` with a measured worst case.
- `enter_below(priority)` masks only at or below a priority. **Everything else
  uses this**, so a hard real-time ISR keeps its latency while lower-priority
  kernel work proceeds.

`BENCH-005` publishes the longest observed critical section. It bounds the
interrupt latency the kernel imposes, and a project unwilling to publish it is
hiding something.

## 7. What this model does not do

- **No SMP.** See [non-goals](12-non-goals.md).
- **No task migration** between priority levels at runtime — that would break
  the analysis. Priority inheritance is the bounded, analysable exception.
- **No `Send`/`Sync` across protection boundaries.** Isolated tasks cannot share
  a pointer; they exchange messages.
- **No work stealing** between executors. Each is pinned to its priority.

## See also

- [ADR-0004 — Scheduling policy](../adr/0004-scheduling-policy.md)
- [Real-time model](04-realtime-model.md)
- [IPC](08-ipc.md)
