# Reference application — Sensor node

> **Status:** specified · **Delivered:** Checkpoint 2 · **Board:** Nucleo-F767ZI

Acquisition, storage, and connectivity. The application that exercises DMA — the
mechanism that most clearly shows why type safety alone is not enough.

## What it does

Sample four analogue channels at 1 kHz via ADC with DMA, filter them, store a
rolling window to flash, and publish aggregates over MQTT. Survive a power cut
without losing committed data.

## Task set

```toml
[[task]]
name = "acquisition"   priority = 7   period = "1ms"   deadline = "800us"
wcet = "120us"         stack = "2KiB"   restart = "never"
capabilities = ["adc.dma", "ipc.samples"]

[[task]]
name = "storage"       priority = 4   period = "100ms"
wcet = "12ms"          stack = "4KiB"
capabilities = ["flash.data", "ipc.samples", "ipc.aggregates"]

[[task]]
name = "telemetry"     priority = 2   period = "1s"
wcet = "40ms"          stack = "8KiB"
restart = { on-fault = { budget = 5, window = "60s" } }
capabilities = ["net.mqtt", "ipc.aggregates"]
```

## What it proves

- **DMA within capability bounds.** The acquisition task's descriptors are
  validated against its own regions; a transfer targeting `telemetry`'s memory
  faults (`ARCH-MPU-003`). This is the concrete demonstration that Rust's type
  system does not reach the DMA engine and the MPU does.
- **Cache maintenance on the Cortex-M7.** Forgetting to clean or invalidate
  around a DMA transfer produces corruption that appears only at speed and only
  sometimes. The API makes it mandatory rather than a documented obligation.
- **Backpressure.** `samples` uses `block`; `aggregates` uses `drop-oldest`. The
  storage path must not lose samples; the telemetry path must not stall storage.
- **Power-cut recovery.** Pull the plug mid-write, confirm committed data
  survives and the partial write is detected rather than read as valid.
- **Restart with in-flight state.** `telemetry` restarting while an MQTT publish
  is outstanding — the case that makes restart semantics genuinely subtle.

## What it is not

No sensor calibration, no time synchronisation, no store-and-forward with
guaranteed delivery. A real product needs all three; this demonstrates the
mechanisms they would be built on.
