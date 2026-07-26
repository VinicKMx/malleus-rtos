# Non-goals

> **Status:** accepted · **Checkpoint:** 0

What Malleus RTOS will **not** build, and why. This document exists because the
most common way an ambitious systems project dies is not a wrong technical
decision — it is doing twelve things adequately instead of four things well.

Each entry names what we do instead. "We won't build X" without an alternative
is not a plan; it is a gap.

---

## Never — these contradict the design

### An SMP scheduler

Multi-core scheduling with shared memory brings cache coherency, memory
ordering, and lock contention into a system whose entire value proposition is
analysability. Response-time analysis for multiprocessors is dramatically harder
and, for many models, an open research question.

**Instead:** on multi-core parts, run an independent Malleus instance per core
with explicit message passing between them. Asymmetric multiprocessing is
analysable; symmetric is not, at the level of rigour this project claims.

### Dynamic application loading

Loading code at runtime makes the task set dynamic, which makes memory and
timing unknowable at build time — see
[ADR-0002](../adr/0002-static-system-definition.md). It is fundamentally
incompatible with the central claim.

**Instead:** OTA firmware update with A/B partitions and rollback. The whole
image changes atomically, and the new image is analysed before it ships.

### A POSIX compatibility layer

POSIX assumes processes, `fork`, a filesystem, signals, and dynamic allocation.
Implementing it faithfully means implementing an operating system Malleus is
deliberately not.

**Instead:** a C ABI for calling into and out of Rust, and optional CMSIS-RTOS2
compatibility — which is what embedded C code actually uses.

### AI in the scheduler

Occasionally proposed, so worth stating: a scheduler whose behaviour depends on
a learned model has no worst case you can compute. That is the opposite of this
project.

**Instead:** exact response-time analysis, with numbers you can check by hand.

---

## Not ours — solved better elsewhere

These follow from [ADR-0009](../adr/0009-ecosystem-interoperability.md).
Building any of them would mean spending the project's scarcest resource
recreating something that already exists and works.

| We will not build | We integrate |
|---|---|
| A TCP/IP stack | `smoltcp`, `embassy-net` |
| A cryptography library | RustCrypto crates |
| A bootloader | MCUboot |
| A filesystem | `embedded-storage` implementations, `littlefs` bindings |
| A logging format | `defmt` |
| Debug probe software | `probe-rs` |
| A firmware image format | MCUboot's |

Writing your own crypto is the classic mistake. Writing your own TLS is the same
mistake with more steps. Writing your own bootloader is how devices get bricked
in the field.

**One arguable exception:** the trace and crash-dump wire format
(`malleus-trace`). It is built because it must be encodable from a fault handler
with no allocation, no locks, and bounded time — a constraint existing formats do
not target. If someone identifies a format that meets it, we should adopt it,
and that invitation is recorded in the crate.

---

## Not yet — plausible later, wrong now

### Dozens of microcontrollers

Supporting many parts before any port is proven produces many half-ports and an
architecture contract shaped by nothing in particular.

**Order:** Cortex-M7 → Cortex-M33 → RISC-V with PMP. The second port is what
proves the architecture contract is a real abstraction; the third proves it
across vendors. Only then does breadth make sense.

### Formal certification

IEC 61508, ISO 26262, and IEC 62304 require an enormous, sustained evidence
effort, and certifying an unstable design certifies the wrong thing.

**Instead, from day one:** document every `unsafe` block, keep requirements
traceable, generate test artefacts, maintain a requirement–test matrix, and use
reproducible builds. Qualified toolchains such as Ferrocene make a future path
plausible — but a qualified compiler certifies the compiler, not this kernel,
and its qualification has a specific scope. The project would need to produce its
own evidence. See [ADR-0010](../adr/0010-unsafe-code-policy.md).

### A GUI configuration tool

Kconfig-style GUIs are widely disliked and expensive to maintain.

**Instead:** a hand-editable TOML manifest that diffs cleanly in review, with
excellent error messages. If a GUI is ever built, it should generate that file,
not replace it.

### An on-device package manager

Requires dynamic loading. Same objection.

### A `no_std` async ecosystem of our own

Embassy has one and it is good.

**Instead:** implement `embedded-hal-async` so their drivers work here
unmodified. See [ADR-0009](../adr/0009-ecosystem-interoperability.md).

---

## The general rule

> **Build only what is core to the thesis. Integrate everything else.
> Contribute upstream where it falls short.**

The thesis is: deterministic hard real-time plus `async` connectivity, with
hardware-enforced isolation, build-time analysis, and production diagnostics.

Four things are genuinely ours because nothing else provides them:

1. The kernel and its scheduling model
2. The manifest, analyser, and code generator
3. The capability and isolation model
4. The fault, supervision, and diagnostics machinery

Everything else is somebody else's better-solved problem.

## Proposing a change

If you believe something here belongs in the project, open an issue arguing:

1. Why it is **core to the thesis** rather than adjacent to it.
2. Why no existing project can be integrated instead.
3. What is **removed** from the roadmap to pay for it.

That third question is the one that matters. Scope is not free, and a proposal
without a subtraction is a proposal to do everything more slowly.
