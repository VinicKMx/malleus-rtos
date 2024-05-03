# Malleus RTOS documentation

## Start here

| Document | What it answers |
|---|---|
| [Manifesto](design/00-manifesto.md) | What this is, who it is for, and why it should exist |
| [Comparison](design/06-comparison.md) | How it differs from Embassy, RTIC, Tock, Hubris, Zephyr, FreeRTOS |
| [Non-goals](design/12-non-goals.md) | What will never be built, and why |
| [Glossary](design/11-glossary.md) | Terms as this project uses them |

## Design

The architecture, in the order it makes sense to read.

| # | Document |
|---|---|
| 00 | [Manifesto](design/00-manifesto.md) |
| 01 | [Threat model](design/01-threat-model.md) |
| 02 | [Memory model](design/02-memory-model.md) |
| 03 | [Concurrency model](design/03-concurrency-model.md) |
| 04 | [Real-time model](design/04-realtime-model.md) |
| 05 | [Fault model](design/05-fault-model.md) |
| 06 | [Comparison](design/06-comparison.md) |
| 07 | [System manifest](design/07-system-manifest.md) |
| 08 | [IPC](design/08-ipc.md) |
| 09 | [Capabilities](design/09-capabilities.md) |
| 10 | [Observability](design/10-observability.md) |
| 11 | [Glossary](design/11-glossary.md) |
| 12 | [Non-goals](design/12-non-goals.md) |

## Decisions

[Architecture Decision Records](adr/) — twelve decisions, each with what it
costs and what would justify reopening it.

## Reference applications

| Application | Demonstrates |
|---|---|
| [Industrial controller](reference-apps/industrial-controller.md) | The flagship: everything at once, with a live fault demo |
| [Motion controller](reference-apps/motion-controller.md) | 1 kHz PID, encoder, safety monitor |
| [Sensor node](reference-apps/sensor-node.md) | DMA acquisition, storage, MQTT |
| [Modbus gateway](reference-apps/modbus-gateway.md) | RTU ↔ Ethernet, OTA |

## Internals

For contributors and porters.

| Document | For |
|---|---|
| [Working conventions](internals/conventions.md) | The rules any change here must follow |
| [Workspace layout](internals/workspace-layout.md) | Why the repository is arranged this way |
| [Porting guide](internals/porting.md) | Bringing up a new architecture |

## Conventions

Every design document carries a header:

```markdown
> **Status:** accepted · **Checkpoint:** N · **Last reviewed:** YYYY-MM-DD
```

- **Status** — `draft`, `accepted`, or `superseded`
- **Checkpoint** — when the described thing exists. A document describing
  something unbuilt says so.
- **Last reviewed** — documents are re-read at each checkpoint

Where a document describes something that does not work yet, it says so in the
text rather than in the past tense. A documentation set that describes an
aspiration in the present tense is how a project loses the ability to tell its
users the truth.
