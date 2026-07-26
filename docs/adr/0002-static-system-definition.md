# ADR-0002 — The task set is static and declared in a manifest

> **Status:** Accepted · **Checkpoint:** 0

## Context

Every RTOS must decide whether tasks can be created at runtime. FreeRTOS,
Zephyr, and most others say yes. RTIC and Hubris say no.

The decision looks like a convenience question. It is not — it determines
whether the system can be analysed at all.

If the task set can change at runtime, then total stack usage, total RAM, CPU
utilisation, and schedulability are all *unknowable* at build time. A tool could
only ever report the properties of one possible configuration among many, and
the configuration that actually fails in the field is by definition the one
nobody analysed.

Malleus's central claim is that these properties are computed before the device
ships. That claim is incompatible with dynamic task creation.

## Decision

**The task set is closed at build time. There is no `spawn`.**

Tasks, priorities, stacks, periods, deadlines, WCETs, capabilities, and channels
are declared in `malleus.toml`, validated by `malleus-manifest`, and generated
into `static` tables by `malleus-codegen`.

Corollaries:

- Task identity is a `const TaskId` in generated code. There is **no** runtime
  "look up a task by name" API — a name lookup that can fail is an error class a
  static system does not need to have.
- `MAX_TASKS = 64` is a hard ceiling bounding every kernel table so they live in
  `.bss` with a known size.
- Channels, their capacities, and their maximum message sizes are equally
  static, so channel storage is a `static` array.

## Consequences

### What this buys

- **Schedulability is computable.** This is the whole point.
- **Memory is computable.** Total RAM is known at link time; running out is a
  build error, not a field failure.
- **The IPC graph is a build artefact.** It can be drawn, diffed in review, and
  checked for cycles.
- **Capability checks are compile-time.** A task cannot name a resource it did
  not declare, so the check is a compile error rather than a runtime denial.
- **No allocation failure path.** There is no "task creation failed" to handle,
  because there is no task creation.

### What this costs

This is a **real restriction and it will cost users.** Stating the cases
honestly:

- **Worker pools sized at runtime.** Not possible. You declare N workers.
- **Plugin architectures.** Not possible without dynamic loading, which is also
  a non-goal.
- **Per-connection tasks.** A server spawning a task per client cannot work. The
  Malleus answer is an `async` executor handling many connections inside one
  task — which is the better design on a microcontroller anyway, but it *is* a
  different design, and telling someone their architecture must change is a real
  cost.
- **Porting friction.** Code from FreeRTOS or Zephyr using `xTaskCreate` needs
  restructuring, not just an API swap.

### What it forecloses

Dynamic application loading, permanently. Untrusted third-party code loaded at
runtime — Tock's use case — is outside what Malleus can offer.

## Alternatives considered

**Dynamic creation with static analysis of "the common case".** Analyse the
tasks declared at build time, permit `spawn` beyond that. Rejected as the worst
of both: the analysis reports `PASS` for a configuration the device may never
run, which is more dangerous than no analysis, because it carries authority it
has not earned.

**Static tasks with dynamic priorities.** Keeps the task set fixed but lets
priorities change. Rejected: response-time analysis assumes fixed priorities.
Priority *inheritance* is a bounded, analysable exception and is supported;
arbitrary runtime priority changes are not.

**A pool of pre-allocated task slots, claimed and released at runtime.** Bounds
memory but not timing — the analyser would have to assume every slot is active
at worst case, which is the same as declaring them all statically, but with more
machinery and a worse error message.

**Declare tasks in Rust rather than TOML.** Attractive: one language, type
checking, no new syntax. Rejected because the interesting questions are
*whole-system* properties. Rust's type system reasons well about one crate at a
time; it has nothing to say about whether the sum of your stacks fits in 512 KiB.
A manifest is data — diffable, reviewable, and analysable by tools that are not
the compiler, including tools nobody has written yet.

## Revisit when

- A credible industrial use case appears that genuinely cannot be expressed with
  a static task set plus `async` concurrency. "It would be more convenient" does
  not qualify; "this class of product is impossible" does.
- Someone demonstrates a sound analysis technique for a bounded-dynamic task
  set — this exists in the literature for restricted models, and if one fits
  this domain the restriction could be relaxed without giving up the claim.
