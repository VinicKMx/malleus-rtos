# Comparison with existing systems

> **Status:** accepted · **Checkpoint:** 0

An honest comparison, written by the people building the competitor. Treat it
accordingly, and please open an issue if anything here misrepresents a project —
particularly if you maintain one of them. Getting this wrong damages us more
than it damages them.

**All six systems below are good.** Several are better-engineered than Malleus
will be for years. The question is never "which is best" but "which is aimed at
your problem".

## The short version

| | Embassy | RTIC | Tock | Hubris | Zephyr | FreeRTOS | **Malleus** |
|---|---|---|---|---|---|---|---|
| Language | Rust | Rust | Rust | Rust | C (+Rust) | C | Rust |
| Hard real-time | good | **excellent** | limited | good | good | good | goal |
| `async` | **excellent** | via extern | no | no | no | no | goal |
| Memory isolation | no | no | **yes** | **yes** | optional | optional | goal |
| Build-time timing analysis | no | no | no | no | no | no | **working** |
| Static system declaration | partial | **yes** | no | **yes** | partial | no | **working** |
| Fault containment + restart | no | no | **yes** | **yes** | limited | no | goal |
| Production diagnostics | `defmt` | `defmt` | limited | **Humility** | MCUmgr | limited | goal |
| Driver ecosystem | **large** | shared | own | own | **huge** | **huge** | reuse Rust |
| **Production-ready today** | **yes** | **yes** | **yes** | **yes** | **yes** | **yes** | **no** |

That last row is the one that matters right now. Everything below is about where
Malleus intends to land, not where it is.

---

## Embassy

**What it is:** an `async`-first embedded framework with a heapless executor,
statically allocated tasks, integrated timers, and multi-priority executors.

**What it does better than Malleus will:** the `async` programming model, the
driver ecosystem, HAL coverage, and — for years to come — maturity. Embassy is
the reference for how `async` embedded Rust should feel.

**Where Malleus differs:**

Embassy's multi-priority executors already give you preemption between priority
levels. What it does not give you is a *contract*. There is no place to declare
that `motor-control` runs every 1 ms with a 500 µs deadline, and therefore no
build step that can tell you the declaration is violated. There is no memory
isolation between an `async` MQTT client and the control loop — a bad DMA
descriptor in one reaches the other. And when a task panics, the system panics.

**Choose Embassy when:** you want the best `async` embedded Rust experience,
your timing requirements are met by construction rather than needing proof, and
you can accept that a fault anywhere takes down everything.

**We depend on Embassy's world:** Malleus implements `embedded-hal-async` so
that drivers written for Embassy work here unmodified. This is deliberate. See
[ADR-0009](../adr/0009-ecosystem-interoperability.md).

## RTIC

**What it is:** a concurrency framework mapping tasks directly onto hardware
interrupt priorities, with compile-time-verified resource sharing (Stack
Resource Policy) and essentially zero scheduling overhead.

**What it does better than Malleus will:** raw efficiency and latency. RTIC's
model is not "a fast RTOS" — it is the absence of one. Nothing Malleus does will
beat a hardware interrupt priority controller doing the scheduling directly.
RTIC's compile-time deadlock and race freedom is genuinely excellent.

**Where Malleus differs:**

RTIC's model gives you one stack, no isolation, and no blocking. That is a
feature for its target and a limit outside it: a task cannot block on a network
socket, so complex connected applications get awkward. RTIC also assigns
resources at compile time but does not analyse *timing* — it will not tell you a
deadline is missed.

**Choose RTIC when:** you have a bounded set of reactive tasks, no need for
isolation, and you want the lowest possible overhead. For a pure motor
controller with no connectivity, RTIC is very likely the right answer and
Malleus is over-engineering.

## Tock

**What it is:** an embedded OS with a Rust kernel and MPU-isolated processes,
which can be written in any language, communicating with the kernel through a
system-call interface.

**What it does better than Malleus will for years:** it is the reference for
memory isolation on microcontrollers, with a mature capsule model and real
deployments. Its security model has had far more scrutiny than ours has.

**Where Malleus differs:**

Tock's isolation boundary is the *process*, and processes are relatively
heavyweight and dynamically loadable — a deliberate choice serving its goal of
running mutually distrusting third-party applications. Malleus's boundary is the
*task*, statically declared, which is lighter and analysable but cannot host
untrusted code loaded after the fact.

The scheduling models differ accordingly. Tock's is designed around
kernel-mediated process scheduling; Malleus is built around a 1 kHz control loop
that must never be late. Neither is better; they are aimed at different failures.

**Choose Tock when:** you need to run applications you do not control, or you
need a security-focused OS with a track record.

## Hubris

**What it is:** a memory-protected, message-passing kernel from Oxide Computer,
with all tasks statically defined at build time and a first-class debugger
(Humility).

**Hubris is the closest relative to Malleus, and the most instructive.**

Statically declared tasks, memory protection, synchronous IPC, task supervision
and restart, no dynamic allocation in the kernel, and excellent tooling — that
list is largely the Malleus design, and Hubris got there first. Anyone
evaluating Malleus should look hard at Hubris.

**Where Malleus differs:**

1. **Target.** Hubris was built for Oxide's server hardware and is shaped by
   that. Malleus targets third-party industrial controllers, which means broad
   board support and off-the-shelf-part support are requirements rather than
   incidental.
2. **Timing analysis.** Hubris does not do build-time schedulability analysis.
   This is Malleus's primary differentiator and the thing most worth stealing
   from us.
3. **`async`.** Hubris IPC is synchronous by design. Malleus runs `async`
   executors inside priority levels, because industrial connectivity —
   MQTT, TLS, Modbus, HTTP — is far more pleasant to write that way.
4. **Positioning.** Hubris is explicit that it is not a general-purpose RTOS
   seeking wide adoption. Malleus is trying to be exactly that, which is a
   harder problem and may well be the wrong bet.

**Choose Hubris when:** you want a proven memory-protected message-passing
kernel and can work within its hardware assumptions. Genuinely: if Hubris fits
your hardware, it is a more sensible choice than Malleus for the next two years.

## Zephyr

**What it is:** a Linux Foundation RTOS with enormous device support, a full
networking stack, device management via MCUmgr, LTS releases, and commercial
backing. Rust support exists but is limited.

**What it does better than Malleus ever will:** breadth. Hundreds of boards,
thousands of drivers, a complete networking and Bluetooth stack, established
certification paths, and a large employed engineering community. Malleus will
never match this and should not try.

**Where Malleus differs:**

Zephyr's kernel is C. Its memory-protection support exists but is optional and
not the default posture. It has no build-time schedulability analysis. Its
configuration system (Kconfig plus devicetree) is powerful and widely considered
difficult to learn.

**Choose Zephyr when:** you need broad hardware support, a complete protocol
stack, LTS guarantees, or a certification story today. For most commercial
industrial products shipping in the next two years, this is the responsible
choice, and Malleus is the interesting bet you make on a side project first.

**We borrow from Zephyr:** MCUmgr shows what device management must cover, and
`west` shows that build, flash, debug, and test belong in one integrated tool.

## FreeRTOS

**What it is:** the most widely deployed embedded RTOS. Small, portable, broadly
understood, with LTS releases and AWS backing.

**What it does better:** ubiquity. Every embedded engineer has used it, every
vendor supports it, and it runs on essentially everything.

**Where Malleus differs:**

FreeRTOS is a scheduler with synchronisation primitives. It offers no memory
isolation in typical configurations, no fault containment, no build-time
analysis, and no integrated diagnostics. Its API is C and unchecked — passing a
bad handle is undefined behaviour.

**Choose FreeRTOS when:** your vendor's SDK is built on it, your team knows it,
and your requirements do not include fault containment or timing proof. That is
a very large fraction of embedded projects and there is nothing wrong with it.

**Malleus offers a migration path rather than a rewrite demand:** C ABI, header
generation, and optional CMSIS-RTOS2 compatibility, so a legacy protocol library
can run as an isolated task while new services are written in Rust. Planned for
Checkpoint 5.

---

## Where Malleus could fail

Stated plainly, because a comparison that only lists advantages is advertising.

1. **The combination may not be worth its cost.** Static declaration plus
   isolation plus declared WCETs is a real burden on the engineer. If teams find
   Embassy plus careful review sufficient, Malleus is solving a problem people
   do not feel.
2. **WCET declaration may prove impractical.** The analysis is only as good as
   its inputs, and on a cached Cortex-M7 a sound WCET is hard. If engineers
   guess, the `PASS` is theatre. We mitigate by measuring on hardware and
   flagging when reality exceeds the declaration — but this is the project's
   biggest open risk.
3. **The ecosystem may not follow.** If `embedded-hal-async` compatibility turns
   out to be partial in practice, the driver reuse argument collapses and
   Malleus needs its own driver ecosystem, which it cannot afford.
4. **Hubris may simply be enough.** If Oxide broadens Hubris's hardware support,
   much of Malleus's rationale evaporates. That would be a good outcome for the
   world and a bad one for this project.

These are tracked as risks, and each checkpoint has an exit criterion designed
to expose the corresponding risk early rather than at year four.

## Sources

- Embassy — <https://embassy.dev>
- RTIC — <https://rtic.rs>
- Tock — <https://www.tockos.org/documentation/design/>
- Hubris — <https://github.com/oxidecomputer/hubris>
- Zephyr MCUmgr — <https://docs.zephyrproject.org/latest/services/device_mgmt/mcumgr.html>
- Zephyr host tools — <https://docs.zephyrproject.org/latest/develop/flash_debug/host-tools.html>
- FreeRTOS — <https://www.freertos.org/>
- Embedded Rust interoperability — <https://docs.rust-embedded.org/embedonomicon/soc-support.html>
