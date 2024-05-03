# Fault model

> **Status:** accepted · **Checkpoint:** 0 (design) · 3 (demonstrable)

## 1. The premise

**Rust does not remove the need for a fault model.**

It removes a large class of memory bugs from safe code. It does nothing about:

| Source | Why Rust cannot help |
|---|---|
| `unsafe` blocks | Every kernel has them, including this one |
| DMA engines | Hardware writes to addresses; it does not consult the borrow checker |
| Vendor C libraries | Outside the type system entirely |
| Misconfigured peripherals | A wrong register value makes hardware write where it likes |
| Deadline overruns | Not a memory bug at all |
| Deadlocks | Safe Rust permits them |
| `panic!` | Safe, well-defined, and still stops the task |

So the question is never "can a task fail". It is **what the product does when
one does**. That is a systems question, and hardware protection is part of the
answer whether the language is Rust or C.

## 2. What a fault is

```rust
pub enum FaultKind {
    MemoryAccess { address: Option<usize>, write: bool },
    StackOverflow,
    IllegalOperation,
    Arithmetic,
    Panic { location: Option<&'static Location<'static>> },
    DeadlineMiss { overrun_ticks: u64 },
    WatchdogTimeout,
    KernelInvariant { invariant: &'static str },
}
```

**`DeadlineMiss` is in this list deliberately.** In a control system a missed
deadline is a failure of the same seriousness as a bad pointer; the only
difference is that the consequences arrive later. Most RTOSes treat it as a
statistic, if they notice at all. Malleus treats it as a fault with a
supervision policy attached.

Fault records are fixed-size, `Copy`, contain no pointers into the failed task's
memory, and are safe to write to flash from inside the fault handler. The system
is already broken when this code runs; it must not allocate, lock, or call back
into the kernel.

## 3. Containment

### 3.1 Isolation tiers

The tier in use is always **reported**, never assumed:

| Tier | Hardware | Guarantee |
|---|---|---|
| **0** | none | Rust type safety only. `unsafe` or a bad DMA descriptor reaches anything. |
| **1** | ARMv7-M MPU (M4/M7) | Task-granular isolation, power-of-two aligned regions. Costs padding. |
| **2** | ARMv8-M MPU (M33) / RISC-V PMP | Task-granular, 32-byte granular regions. Little padding. |

A board declaring tier 0 makes `cargo malleus analyze` emit an explicit warning,
and **any task requesting `restart = "on-fault"` is rejected at build time**.
Restarting a task you cannot contain is a false promise: if it may have
scribbled on a neighbour, restarting it cleans up nothing and hides the damage.

### 3.2 Did the fault escape?

```rust
FaultKind::may_have_escaped(isolated: bool) -> bool
```

- Memory faults, stack overflows, illegal operations: **contained iff isolated**.
- Arithmetic, panic, deadline miss, watchdog, kernel invariant: **always
  contained** — they cannot corrupt a neighbour by construction.

This single predicate governs whether restart is safe or the supervisor must
escalate. It is unit-tested, because it is the load-bearing decision of the
whole model.

### 3.3 Stack overflow

Detected **before** it corrupts adjacent memory, not after. A guard region sits
below every task stack; the first access into it faults with the task correctly
attributed.

This is `ARCH-FAULT-002` in the conformance suite and is not optional for a port
claiming tier 1 or above. The common alternative — a canary value checked at
context-switch time — detects the overflow *after* the damage, at an unknown
remove from the cause, and is not good enough here.

## 4. Supervision

### 4.1 Policy, declared per task

```toml
restart = "never"                                       # RestartPolicy::Never
restart = { on-fault = { budget = 5, window = "60s" } } # RestartPolicy::OnFault
restart = "escalate-to-reset"                           # RestartPolicy::EscalateToReset
```

| Policy | When it is right |
|---|---|
| `never` | Partial operation is more dangerous than absence. A motor loop that restarts mid-move is worse than one that stops. |
| `on-fault` | The task is restartable and the system is better with it than without. Network clients, protocol servers, telemetry. |
| `escalate-to-reset` | The task's absence means the system cannot be trusted at all. A safety supervisor. |

The budget matters as much as the policy. A task that fails five times in a
minute is not having bad luck — it has a bug, or its environment has changed.
Restarting it forever converts a diagnosable fault into an invisible reboot loop.

### 4.2 What restart actually means

A restarted task gets a wiped stack, drained channels, and reset kernel state.
**Its peers are notified.**

That last part is easy to skip and expensive to skip. A peer that continues as
though nothing happened — holding a sequence number, a session, a
half-acknowledged command — is a rich source of second-order bugs that appear
only after a restart in the field. Waiters on a resource held by a faulted task
receive `WakeReason::HolderFaulted`, which forces a decision at the call site,
exactly as a poisoned lock does in `std`.

### 4.3 Degraded mode

```text
telemetry fails
  → stop telemetry, record fault, restart it, keep motor-control running

telemetry fails 5 times in 60s
  → stop it, shed the network subsystem, keep local control,
    mark the system degraded, report it in telemetry and on the local indicator
```

Degraded mode is a **declared, visible state**, not an emergent one. An operator
can see it, telemetry reports it, and the health record carries it. A device
that has quietly lost a subsystem and looks healthy is worse than one that has
lost it loudly.

## 5. Crash dumps

Written from the fault handler, persisted to flash, survives reset, read later
by `cargo malleus dump`.

Contents: the fault record; every task's state, priority, stack pointer and
high-water mark; the ready set; recent trace events; the reset reason; uptime;
and the firmware version and build identity.

Constraints, all forced by *when* this code runs:

- No allocation, no locks, no kernel calls.
- Bounded execution time — the watchdog is still running.
- Self-describing, so a dump from the field is readable without reconstructing
  the exact build.
- Corruption-tolerant: a partially written dump must still yield its readable
  prefix. A dump format that is all-or-nothing fails exactly when the device
  died mid-write, which is the interesting case.

## 6. Reset reason

```rust
pub enum ResetReason {
    PowerOn, External, Watchdog, FaultEscalation, SoftwareUpdate, Unknown,
}
```

Read at boot, reported in the health record, `is_abnormal()` flags
`Watchdog | FaultEscalation | Unknown`.

**A device that cannot say why it restarted cannot be debugged remotely**, and
remote debuggability is most of what separates a product from a prototype.
`Unknown` is counted as abnormal deliberately: a port that has not implemented
reset-reason decoding should be visibly incomplete, not quietly reassuring.

## 7. The Checkpoint 3 demonstration

The project's central claim, reduced to something that either works on stage or
does not:

```text
Nucleo-F767ZI running the industrial controller:
  1 kHz motor loop · DMA acquisition · Modbus RTU · Ethernet · MQTT

  1. Trigger a null dereference in the MQTT client
  2. MPU faults the task
  3. Kernel attributes it, records it, stops only that task
  4. Motor loop does not miss a single tick — shown live on a scope
  5. Supervisor restarts MQTT per its declared policy
  6. `cargo malleus dump` on the host shows the crash: task, address, PC, stack
  7. Force it 5 times → network shed, local control retained, degraded mode
     reported
```

If that demonstration does not work by Checkpoint 3, **the project has failed at
its central thesis and should say so** rather than continuing on momentum. This
is stated here so it cannot be quietly renegotiated later.

## 8. What this model does not do

- **It does not prevent bugs.** It contains them.
- **It does not make a task correct after restart.** If the fault came from
  persistent state, the restart repeats it — hence the budget.
- **It does not survive kernel corruption.** A fault inside the kernel escalates
  to reset. The kernel is small and heavily reviewed precisely because it is
  the part with no containment above it.
- **It does not replace a hardware watchdog.** The independent watchdog stays
  enabled and catches everything above, including a kernel that stops running.
- **It offers nothing on tier-0 boards** beyond attribution and recording, and
  it says so rather than implying otherwise.

## See also

- [ADR-0005 — Memory isolation](../adr/0005-memory-isolation.md)
- [ADR-0007 — Fault model and supervision](../adr/0007-fault-model.md)
- [Threat model](01-threat-model.md)
- [Observability](10-observability.md)
