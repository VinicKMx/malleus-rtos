# Capabilities

> **Status:** accepted · **Checkpoint:** 0 (design) · 3 (working)

## 1. The idea

A task reaches only what it declared. Everything else is unreachable — not by
convention, but because the hardware will not permit it.

```toml
[[task]]
name = "telemetry"
capabilities = ["net.mqtt", "ipc.sensor-data"]
```

That task gets protection regions covering its own memory, the Ethernet
peripheral, and the `sensor-data` channel storage. It cannot touch the motor PWM
register. If its code tries — through a bug, a bad pointer, or an exploited
vulnerability — the MPU raises a fault and the supervisor handles it.

## 2. Why declared rather than inferred

A tool could infer required resources from the code. It would be less work for
the engineer and it would be the wrong design.

**Inference tells you what the code does. A declaration says what the code is
allowed to do.** The gap between those two is exactly where the interesting bugs
and the interesting attacks live. A capability list that is derived from the code
can never be violated by the code, which makes it documentation rather than a
control.

There is a second reason: a declaration is reviewable. `telemetry` acquiring
`pwm.motor` shows up as a one-line diff in a pull request, and somebody asks why.

## 3. Namespaces

| Prefix | Grants | Example |
|---|---|---|
| `gpio.` | A named pin or port | `gpio.estop` |
| `uart.` | A UART | `uart.rs485` |
| `spi.` | An SPI bus | `spi.encoder` |
| `i2c.` | An I²C bus | `i2c.sensors` |
| `adc.` | An ADC, optionally with DMA | `adc.dma` |
| `pwm.` | A PWM output | `pwm.motor` |
| `timer.` | A hardware timer | `timer.control` |
| `net.` | A network service | `net.mqtt` |
| `flash.` | A flash partition | `flash.config` |
| `ipc.` | A channel endpoint | `ipc.sensor-data` |

Names are defined by the board support crate, except `ipc.*`, which must match a
declared channel. The validator checks that an `ipc.` capability names a real
channel (M0020) and that the holder is actually an endpoint of it (M0021) — a
task cannot be granted a listening position on a conversation it is not part of.

## 4. Enforcement, in three layers

**Compile time.** Generated code places a capability's handle in scope only for
tasks that declared it. An undeclared task cannot *name* the resource, so the
error is "cannot find `motor` in this scope", not a runtime denial. This is the
primary mechanism, and it is free.

**Build time.** The region allocator proves the declared set fits the hardware.
A task requesting more regions than the MPU provides is a build error naming the
task and what it asked for.

**Run time.** The MPU enforces the boundary for everything the compiler cannot
see: `unsafe` code, DMA descriptors, and C libraries. This is the layer that
matters when the first two have been defeated.

The three layers are deliberately redundant. The compile-time check is the one
that makes the model pleasant to use; the runtime check is the one that makes it
true.

## 5. Granularity and its limits

The region budget is the binding constraint. ARMv7-M provides 8 regions, and a
typical isolated task spends most of them on its own memory (see
[memory model](02-memory-model.md)). That leaves room for roughly two to three
peripherals or channels.

The allocator applies two reductions before failing:

1. **Merge adjacent regions** with identical permissions. Free.
2. **Coalesce peripherals** in a contiguous address range into one region. This
   is a real loss of granularity — the task gains access to whatever else lives
   in that range — so it is **reported**, not applied silently. A security
   trade-off made by a tool without telling anyone is not a trade-off; it is a
   hole.

If neither is enough, the build fails with the task named. That is the correct
outcome: a system that does not fit its hardware should not silently become a
system with less isolation than its manifest claims.

## 6. What capabilities do not do

- **They do not protect against a bad manifest.** Whoever writes it can grant
  anything to anyone. The generated architecture documentation exists partly so
  that an over-broad grant is visible in review.
- **They are not dynamic.** No delegation, no revocation, no passing a capability
  to another task at runtime. The set is fixed at build time, which is what makes
  it analysable.
- **They do not span the network.** A capability is a local access-control
  mechanism, not an authentication or authorisation protocol.
- **They mean nothing on a tier-0 board.** Without protection hardware, the
  compile-time layer still helps, and the runtime layer does not exist. The
  analyser reports the tier rather than implying a guarantee it cannot provide.

## 7. Comparison

**Tock** uses capabilities with dynamically loadable processes and a
kernel-mediated syscall interface — more general, and necessary for running
untrusted third-party code.

**Hubris** uses a statically declared task-to-peripheral mapping, very close to
this design. The main difference is that Malleus unifies peripherals and IPC
endpoints under one capability namespace, so a channel grant and a peripheral
grant are reviewed the same way and consume the same region budget.

Neither approach is novel here; the contribution is the combination with
build-time timing analysis over the same declaration.

## See also

- [ADR-0005 — Memory isolation](../adr/0005-memory-isolation.md)
- [Memory model](02-memory-model.md)
- [Threat model](01-threat-model.md)
