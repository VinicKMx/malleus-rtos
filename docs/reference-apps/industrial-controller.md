# Reference application — Industrial controller

> **Status:** specified · **Delivered:** Checkpoint 3 · **Board:** Nucleo-F767ZI

The flagship. Everything the project claims, running at once, on one board, in
one firmware image — culminating in a fault demonstration that either works or
tells you the thesis was wrong.

Its manifest exists and validates today:
[`examples/industrial-controller/malleus.toml`](../../examples/industrial-controller/malleus.toml).

## What it does

A motor controller with industrial connectivity:

- **1 kHz motor control loop** — PID over an SPI encoder, PWM output
- **2 kHz safety monitor** — E-stop input, independent authority over the PWM
- **500 Hz sensor acquisition** — ADC via DMA
- **Modbus RTU** over RS-485 — the plant PLC talks to it
- **MQTT over Ethernet** — telemetry to a broker
- **Local storage** — configuration and a crash-dump partition
- **OTA update** with A/B partitions and rollback

## The task set

```toml
[[task]]
name = "safety-monitor"      priority = 9   period = "500us"    deadline = "200us"
wcet = "40us"                stack = "1KiB"
capabilities = ["gpio.estop", "pwm.motor"]

[[task]]
name = "motor-control"       priority = 7   period = "1ms"      deadline = "500us"
wcet = "180us"               stack = "2KiB"
capabilities = ["timer.control", "spi.encoder", "pwm.motor", "ipc.motor-command"]

[[task]]
name = "sensor-acquisition"  priority = 6   period = "2ms"      deadline = "2ms"
wcet = "300us"               stack = "2KiB"
capabilities = ["adc.dma", "ipc.sensor-data"]

[[task]]
name = "modbus"              priority = 4   period = "10ms"     deadline = "10ms"
wcet = "900us"               stack = "4KiB"
capabilities = ["uart.rs485", "ipc.motor-command"]

[[task]]
name = "telemetry"           priority = 2   period = "100ms"    deadline = "100ms"
wcet = "8ms"                 stack = "8KiB"
capabilities = ["net.mqtt", "ipc.sensor-data"]
```

### Three design decisions worth noticing

**`safety-monitor` holds no IPC capability.** It acts directly on the hardware it
is responsible for and depends on no other task. An emergency stop that has to
wait in a queue is not an emergency stop. *(The validator caught this during
authoring: an earlier draft granted it `ipc.motor-command`, and M0021 rejected it
because it is not an endpoint of that channel.)*

**`sensor-data` uses `drop-oldest`.** Sampled state — a fresh reading supersedes
a stale one, and a telemetry backlog must never stall acquisition.

**`motor-command` uses `reject`.** Commands, not samples. Dropping the oldest
would silently reorder intent. A rejected command is reported to the Modbus
master, which can retry; a reordered one is a machine doing the wrong thing.

## The verified analysis

Real output, today:

```text
  Task                    Prio     Period   Deadline       WCET  Verdict
  safety-monitor             9        500        200         40  PASS   response 40t, slack 160t
  motor-control              7       1000        500        180  PASS   response 220t, slack 280t
  sensor-acquisition         6       2000       2000        300  PASS   response 560t, slack 1440t
  modbus                     4      10000      10000        900  PASS   response 1720t, slack 8280t
  telemetry                  2     100000     100000       8000  PASS   response 16920t, slack 83080t

  CPU utilisation (periodic tasks): 58.0%          Verdict: PASS
```

These exact numbers are pinned by a test in `malleus-analyzer`. If the analyser
drifts, this document becomes fiction and the test fails.

## The demonstration

The whole point of the project, reduced to something that works on stage or does
not:

```text
1. System running: motor at 1 kHz, Modbus polling, MQTT publishing.
   A scope on a GPIO toggled by motor-control shows a clean 1 kHz square wave.

2. Trigger a null dereference inside the MQTT client.

3. MPU raises a memory fault.
   Kernel attributes it: task `telemetry`, address 0x0000_0000, write, PC.

4. Only `telemetry` is stopped.
   → The scope trace does not move. Not one missed tick.
   → Modbus keeps answering the PLC.

5. Supervisor applies `on-fault { budget = 5, window = "60s" }` and restarts it.
   MQTT reconnects. Telemetry resumes.

6. `cargo malleus dump` on the host prints the crash: task, address, PC, stack,
   every other task's state at the moment of the fault, and the last events
   before it.

7. Force the fault five times within a minute.
   → Budget exhausted. Network subsystem shed.
   → Local control continues. System marked degraded.
   → Degraded state visible on the local indicator and in the Modbus register map.
```

**If step 4 fails — if the motor loop misses a tick — the project has failed at
its central thesis** and should say so rather than continuing on momentum. This
is written here so it cannot be quietly renegotiated later.

## Why this demonstration and not a benchmark

A context-switch number impresses people who already know what they are looking
at. This demonstration is legible to anyone who has ever had a device reboot in
the field, which is everyone in the target audience.

It also cannot be faked. Either the scope trace stays clean or it does not.

## Hardware

| | |
|---|---|
| Board | Nucleo-F767ZI (STM32F767ZI, Cortex-M7 @ 216 MHz, 2 MB flash, 512 KB RAM) |
| Ethernet | On-board LAN8742A PHY |
| Motor | Small BLDC or stepper on a driver board |
| Encoder | SPI absolute encoder |
| RS-485 | Transceiver on a UART |
| E-stop | Momentary switch on a GPIO |
| Instrumentation | Oscilloscope on the control-loop GPIO |

Cost is a few hundred dollars, deliberately: a demonstration nobody can
reproduce is a claim, not evidence. The full bill of materials and wiring will
ship with the application.

## What it does *not* demonstrate

- **Certified safety.** The `safety-monitor` task is not a certified safety
  function and must not be treated as one.
- **Production hardening.** It is a demonstration, not a product.
- **Tier-2 isolation.** The M7 is ARMv7-M. TrustZone is not involved.
- **Multi-axis coordination**, field-bus determinism (EtherCAT and similar), or
  functional-safety architecture.

## See also

- [Fault model](../design/05-fault-model.md)
- [Real-time model](../design/04-realtime-model.md)
