# Reference application — Modbus gateway

> **Status:** specified · **Delivered:** Checkpoint 2 · **Board:** Nucleo-F767ZI

Protocol handling, a hostile input surface, and firmware update. The application
that exercises the parts of a product that are not the interesting engineering
and are most of the work.

## What it does

Bridge Modbus RTU over RS-485 to MQTT over Ethernet. Serve a register map to a
plant PLC, publish changes to a broker, accept writes from either side, and
update its own firmware over the network with rollback.

## Task set

```toml
[[task]]
name = "modbus-rtu"    priority = 6   period = "5ms"   deadline = "5ms"
wcet = "400us"         stack = "4KiB"
restart = { on-fault = { budget = 10, window = "60s" } }
capabilities = ["uart.rs485", "ipc.registers"]

[[task]]
name = "register-map"  priority = 5
stack = "2KiB"         restart = "never"
capabilities = ["ipc.registers", "ipc.updates"]

[[task]]
name = "mqtt-bridge"   priority = 3   period = "50ms"
wcet = "6ms"           stack = "8KiB"
restart = { on-fault = { budget = 5, window = "60s" } }
capabilities = ["net.mqtt", "ipc.updates"]

[[task]]
name = "ota"           priority = 2
stack = "4KiB"         restart = "never"
capabilities = ["net.http", "flash.image-b"]
```

`register-map` is the shared state and is `restart = "never"` — it is the one
piece whose loss would leave both protocol sides disagreeing about reality.
Both protocol tasks are restartable, because both parse input from outside the
device and outside input is where the faults come from.

## What it proves

- **Fault containment at the protocol boundary.** Feed the RTU parser malformed
  frames until it faults; `mqtt-bridge` and the register map keep working. This
  is the containment argument applied to the most realistic attack surface an
  industrial device has.
- **Timing under a hostile input rate.** RTU has inter-frame timing requirements;
  the analysis must hold while the bus is saturated.
- **OTA with rollback.** Update to a deliberately broken image, confirm the
  device detects it and rolls back — using MCUboot, not our own bootloader.
- **Capability separation on flash.** `ota` can write partition B and nothing
  else. It cannot corrupt the running image or the configuration.

## What it is not

Not a complete Modbus implementation — a documented subset of function codes.
No Modbus TCP, no gateway addressing modes, no security (Modbus RTU has none,
and nothing here changes that; the point is that a compromise of the parser is
contained).
