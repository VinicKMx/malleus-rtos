# Manifesto

> **Status:** accepted · **Checkpoint:** 0

This document says what Malleus RTOS is, who it is for, and — as precisely as
possible — who it is *not* for. Everything else in `docs/` elaborates on it. If
a design decision anywhere in the project contradicts this document, one of the
two is wrong and the contradiction is a bug worth filing.

## 1. The thesis

**"Written in Rust" is not a reason to exist.**

The Rust embedded ecosystem is not empty. Embassy has a mature `async` executor
with no heap, integrated timers, and multiple priority levels. RTIC gives
near-zero-cost concurrency built directly on hardware priorities. Tock isolates
processes with an MPU. Hubris ships a memory-protected, message-passing kernel
with a real debugger. Zephyr has official — if limited — Rust support and a
decade of production hardening behind it.

A new project entering that field owes everyone an answer to "why does this
exist", and "it's Rust" is not one. Here is ours.

**The gap is not in any single capability. It is in the combination.**

Take a real industrial controller: a 1 kHz motor loop, DMA sensor acquisition,
Modbus RTU over RS-485, MQTT over Ethernet, local storage, and OTA updates. One
board, one firmware image. Today you must choose:

- **Embassy or RTIC** give you excellent Rust concurrency and real-time
  behaviour — but no memory isolation between the network stack and the control
  loop, and no build-time answer to "will this meet its deadlines".
- **Tock** gives you process isolation — but its scheduling model is built for
  a different problem, and hard real-time control loops are not its target.
- **Hubris** gives you isolation, message passing, and superb tooling — but it
  is designed for and around Oxide's own hardware, and its scheduling is not
  aimed at 1 kHz control.
- **Zephyr or FreeRTOS** give you drivers, protocols, and production maturity —
  but a C codebase, weak-to-absent fault containment in typical configurations,
  and no static timing analysis in the build.

Every one of those is a good system. None of them is aimed at *this* target.

**So Malleus aims at exactly this:**

> A Rust-native RTOS for industrial controllers and connected critical devices,
> combining deterministic hard real-time tasks, `async` application code,
> hardware-enforced fault isolation, build-time resource and timing analysis,
> and production diagnostics.

## 2. The one sentence

Every RTOS should be able to complete this sentence. Ours:

> **Choose Malleus over Embassy, Zephyr, or FreeRTOS when you need hard
> real-time control and async connectivity in the same firmware, with
> hardware-enforced fault isolation, timing contracts verified at build time,
> and diagnostics that survive a crash in the field.**

If you need two of those four, use something that exists today. That is not
modesty — it is the criterion by which this project should be judged, and by
which it should be abandoned if it stops meeting it.

## 3. Who this is for

**Primary:** industrial automation, robotics, industrial IoT, measurement
instruments, connected controllers needing OTA, diagnostics, and high
availability.

These share a shape that motivates every decision here:

1. A control loop that must not miss its deadline, ever.
2. Connectivity that is complex, third-party, and *will* fail.
3. A device in a place where nobody can press reset.
4. A field failure that costs a truck roll, or worse.
5. A product lifetime measured in years, on hardware that cannot change.

**Not the target:** consumer wearables optimised purely for power; single-loop
devices with no connectivity (use RTIC); Linux-class systems (use Linux);
applications needing SMP, dynamic loading, or a filesystem — see
[non-goals](12-non-goals.md).

## 4. What we believe

### 4.1 A system you cannot analyse is a system you are guessing about

Whether a task set meets its deadlines is computable from the declaration. Most
projects instead discover it empirically, months later, as an intermittent field
failure. Malleus computes it in the build.

The price is that the task set must be **static**. No `spawn`, no dynamic
priorities, no runtime task creation. This is a genuine restriction and it will
cost some users. We are taking it deliberately, because a system that can change
shape at runtime cannot have its worst case computed at build time — and for
this class of device, knowing the worst case is worth more than the flexibility.

### 4.2 Determinism must be in the API, not the marketing

Every public operation documents its complexity, whether it allocates, whether
it can block, whether it is ISR-safe, and whether it supports a timeout:

```text
Mutex::lock()
  complexity:   O(waiters)
  allocates:    no
  blocks:       yes
  isr-safe:     no
  timeout:      yes
  inheritance:  priority
```

An API that hides a potentially unbounded operation behind a pleasant signature
is not usable for hard real-time work, however elegant it reads.

### 4.3 Rust is necessary and not sufficient

Rust eliminates a large class of bugs in safe code. It does nothing about:

- `unsafe` blocks — every kernel has them, including this one;
- DMA engines, which ignore the type system entirely;
- vendor C libraries and proprietary binary drivers;
- a peripheral misconfigured into writing wherever it likes;
- a task that overruns its deadline, deadlocks, or panics.

So the question is never "can a task fail". It is **what the product does when
one does**. That is a systems question, and hardware memory protection is part
of the answer whether the language is Rust or not.

### 4.4 Failure is normal; the response to failure is the product

A fault should be **attributed** (which task), **contained** (only that task),
**recorded** (readable after the fact), and **recovered from** (per a policy
someone wrote down). A device that reboots without saying why is a device nobody
can debug remotely — and remote debuggability is most of what separates a
product from a prototype.

### 4.5 The first hour decides adoption

Time from empty board to running, inspectable firmware is a *technical
requirement*, not marketing. If it takes an afternoon of linker scripts, most
evaluators never reach the parts that make the project worth choosing.

Every error message must say what is wrong, **where**, and **what to do about
it**. This is enforced by a test in `malleus-manifest`, not by good intentions.

### 4.6 Inspectability beats magic

Generated code is written to a file you can open, diff, and step through.
Memory layout, protection regions, and the IPC graph are printable. In a system
where a wrong protection region means a fault that *does not happen* — an
isolation failure that leaves no trace — the engineer has to be able to check.
A tool that asks to be trusted, in that position, has not earned it.

### 4.7 Compatibility beats purity

A driver written for Embassy should run on Malleus without being rewritten. We
implement `embedded-hal`, `embedded-hal-async`, `embedded-io`,
`embedded-storage`; we use `defmt` and `probe-rs`; we integrate MCUboot rather
than writing a bootloader; we use `smoltcp` rather than writing a TCP stack.

Malleus competes on isolation, analysis, and diagnostics. Not by asking the
community to port its drivers twice.

### 4.8 Honesty about maturity is a feature

The README opens with a warning that the kernel does not boot. The analyser
prints `UNKNOWN` rather than guessing. Unimplemented CLI commands name the
checkpoint that delivers them.

This costs us nothing we want. Someone who adopts a pre-alpha RTOS believing it
is production-ready becomes an angry ex-user and a cautionary tale. Someone who
adopts it knowing exactly what it is becomes a contributor.

## 5. What we will not do

Summarised from [non-goals](12-non-goals.md), which gives the reasoning:

- No SMP scheduler
- No filesystem, TCP/IP stack, crypto, or bootloader of our own
- No dynamic application loading
- No POSIX compatibility layer
- No GUI configuration tool
- No on-device package manager
- No dozens of MCUs in the first years — two architectures, done properly
- No AI in the scheduler

Each of these consumes effort without proving the central thesis. Several are
things a mature RTOS eventually needs; all of them are things we would do badly
right now, and doing them badly is worse than integrating someone else's.

## 6. How to judge this project

Fair criteria, stated in advance so they cannot be moved later:

**By Checkpoint 1** — does the kernel boot on two boards, with published
worst-case benchmarks from CI, and every `unsafe` block documented?

**By Checkpoint 3** — can a live demo show a task faulting, the MPU containing
it, the motor loop never missing a tick, the supervisor restarting only the
failed task, and a host tool reading the crash dump?

**By Checkpoint 4** — are two external organisations using it in prototypes, and
does one reference product run continuously?

**Throughout** — is the documentation honest about what does not work yet?

If the answer to the Checkpoint 3 question is no, the project has failed at its
central thesis and should say so rather than continuing on momentum.

## 7. On the name

*Malleus* is Latin for hammer. It is also the first bone in the ossicular chain
of the middle ear — the one that receives the vibration and passes it on,
faithfully and without delay.

**MalleusRT** is the kernel proper. **Malleus RTOS** is the whole platform:
kernel, runtime, analyser, tooling, board support. The distinction matters,
because most of what makes this platform worth using lives *outside* the kernel,
and a name that blurs the two invites the usual mistake of judging an RTOS
solely by its scheduler.

---

*This is a living document. It will be revised as reality argues with it, and
each revision recorded in the git history. If it stops describing what the
project actually does, that is a defect — please open an issue.*
