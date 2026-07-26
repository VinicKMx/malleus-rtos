# Reference applications

Four applications, each proving something specific. They exist to keep the
design honest: a kernel designed without a real application is a kernel shaped
by the designer's assumptions, and those assumptions are usually wrong in ways
that only an application reveals.

| Application | Proves | Checkpoint |
|---|---|---|
| [Motion controller](motion-controller.md) | Hard real-time determinism | 2 |
| [Sensor node](sensor-node.md) | DMA, storage, connectivity | 2 |
| [Modbus gateway](modbus-gateway.md) | Protocol handling, OTA | 2 |
| [Industrial controller](industrial-controller.md) | **All of it, plus fault containment** | 3 |

## Rules these applications follow

**They are complete.** Not snippets. A reference application that omits error
handling, configuration, and the update path is not demonstrating what it claims
to demonstrate — those are where the difficulty actually lives.

**They run on hardware anyone can buy.** A demonstration nobody can reproduce is
a claim, not evidence.

**Their numbers are pinned by tests.** The response times in
[industrial-controller](industrial-controller.md) are asserted in
`malleus-analyzer`. If the analyser drifts, the document becomes fiction and CI
says so.

**They are honest about what they are not.** None of them is a certified safety
function, and each says so.

## What they are for, beyond demonstration

1. **Design pressure.** Every one of these has already changed the design —
   the industrial controller's manifest exposed a capability-model error during
   authoring, which the validator caught.
2. **Regression tests.** They are built and analysed in CI.
3. **Documentation that cannot go stale**, because it is compiled.
4. **The honest answer to "show me".** Which is the only question that matters
   to an evaluator.
