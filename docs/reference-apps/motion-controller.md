# Reference application — Motion controller

> **Status:** specified · **Delivered:** Checkpoint 2 · **Board:** Nucleo-F767ZI

The simplest application that is genuinely hard real-time. No connectivity, no
isolation story — just a control loop that must not be late, and the machinery
to prove it was not.

## What it does

A single-axis position controller: read an SPI absolute encoder, run a PID loop,
drive a PWM output, at 1 kHz with a 500 µs deadline. A 2 kHz safety monitor
watches an E-stop input and has independent authority over the PWM.

## Task set

```toml
[[task]]
name = "safety-monitor"   priority = 9   period = "500us"   deadline = "200us"
wcet = "40us"             stack = "1KiB"   restart = "never"
capabilities = ["gpio.estop", "pwm.motor"]

[[task]]
name = "motor-control"    priority = 7   period = "1ms"     deadline = "500us"
wcet = "180us"            stack = "2KiB"   restart = "never"
capabilities = ["timer.control", "spi.encoder", "pwm.motor"]

[[task]]
name = "supervisor"       priority = 1
stack = "1KiB"
capabilities = ["gpio.status"]
```

Both real-time tasks are `restart = "never"`. A motor loop that restarts
mid-move is worse than one that stops: the controller's internal state — the
integral term, the last position, the commanded direction — is gone, and
resuming from a clean slate while the axis is moving is precisely the wrong
thing to do.

## What it proves

- **Jitter.** One million activations at 1 kHz, with the distribution published,
  not just the mean (`BENCH-004`).
- **The deadline is real**, not aspirational — measured, not asserted.
- **Declared WCET versus observed maximum.** The number that catches an
  optimistic engineer, and the mitigation for the project's largest risk.
- **Stack high-water** against the 2 KiB reservation.
- **Tickless idle** actually sleeps between activations.

## Instrumentation

A GPIO toggled at the top of `motor-control`, on a scope. The trace either has
visible jitter or it does not — a form of evidence that requires no trust in the
project's own measurements.

## What it is not

Not a certified safety function. Not multi-axis. No field-bus determinism. The
`safety-monitor` task demonstrates independent authority over an actuator; it is
not SIL-rated and must not be presented as such.
