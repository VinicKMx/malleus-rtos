# Real-time model

> **Status:** accepted · **Checkpoint:** 0 (analysis working, kernel pending)

## 1. What class of real-time

Malleus targets **hard real-time for a declared subset of tasks**, with firm and
soft real-time work coexisting in the same image.

This mixed posture is the point. An industrial controller is not uniformly hard
real-time — the motor loop is, and the MQTT client emphatically is not. A system
that forces one discipline on both either over-constrains the connectivity or
under-protects the control loop.

| Class | Meaning | Example |
|---|---|---|
| **Hard** | A missed deadline is a system failure | `safety-monitor`, `motor-control` |
| **Firm** | A late result is worthless but not dangerous | `sensor-acquisition` |
| **Soft** | Late is degraded, not wrong | `telemetry`, `modbus` |

The class is not a label the engineer writes. It follows from what is declared:
a task with a `deadline` and a `wcet` is analysed as hard real-time and its
verdict must be `PASS`; a task with a period and no deadline is treated as
best-effort and reported as such.

## 2. Scheduling policy

**Fixed-priority preemptive, FIFO within a priority level.** The
highest-priority runnable task runs, always.

There is no fair-share, no ageing, no anti-starvation heuristic. Starving a
low-priority task is the *correct* behaviour when a high-priority one is
runnable, and hiding it behind a heuristic destroys analysability — you can no
longer compute a worst case, because the scheduler's behaviour depends on
history.

Priorities are assigned by the engineer and verified by the analyser. The tool
will tell you your assignment misses a deadline. It will not silently fix it at
runtime. See [ADR-0004](../adr/0004-scheduling-policy.md).

**Why not EDF?** Earliest-deadline-first achieves higher utilisation and is
optimal for uniprocessor scheduling. It is rejected because its overload
behaviour is catastrophic rather than graceful — under transient overload EDF
can cause a cascade in which *everything* misses, whereas fixed priority
degrades predictably from the bottom up. For a system where the motor loop must
survive an overload caused by the network stack, predictable degradation is
worth more than utilisation. It is also far harder to reason about during a
3 a.m. incident, which is a real engineering property.

## 3. The analysis

`cargo malleus analyze` performs **exact response-time analysis**. For task *i*:

```
R = C_i + B_i + Σ  ⌈R / T_j⌉ · C_j
             j∈hp(i)
```

- `C` — worst-case execution time
- `B` — worst-case blocking from lower-priority tasks holding shared resources
- `T` — period
- `hp(i)` — tasks of higher priority

Solved by iteration from `R = C_i + B_i`. The recurrence increases
monotonically, so it either converges (schedulable) or crosses the deadline
(provably not) — both outcomes are conclusive, neither is an approximation.

This is Joseph & Pandya's response-time analysis (1986) with Sha's blocking
term. It is **exact** for this scheduling model, unlike the Liu–Layland
utilisation bound, which is merely sufficient and rejects task sets that are in
fact schedulable.

### Assumptions, stated plainly

1. Tasks are independent except through declared shared resources.
2. Deadlines are no greater than periods — enforced by the validator (M0006).
3. Context-switch cost is folded into each declared WCET.
4. Release jitter is zero. **Not yet modelled.** A task with real release jitter
   should currently declare it inside its WCET; explicit jitter support is a
   planned extension.
5. Blocking terms are **not yet computed** — shared-resource declarations arrive
   with the mutex work in Checkpoint 2. Until then the tool prints
   `Blocking terms: not yet modelled` on every report rather than letting a
   reader assume they were accounted for.

Assumptions 4 and 5 are limitations of today's implementation, not of the
model. They are printed by the tool, not buried here.

### Worked example

From `examples/industrial-controller`, in microsecond ticks:

```text
  Task                    Prio     Period   Deadline       WCET  Verdict
  safety-monitor             9        500        200         40  PASS   response 40t
  motor-control              7       1000        500        180  PASS   response 220t
  sensor-acquisition         6       2000       2000        300  PASS   response 560t
  modbus                     4      10000      10000        900  PASS   response 1720t
  telemetry                  2     100000     100000       8000  PASS   response 16920t

  CPU utilisation (periodic tasks): 58.0%
```

`motor-control` needs 180 µs of CPU but does not finish for 220 µs, because
`safety-monitor` preempts it. `telemetry` needs 8 ms and takes 16.9 ms, because
everything above it preempts it repeatedly.

That gap between *work* and *response* is why utilisation alone answers nothing.
A 58%-loaded system can still miss; a 90%-loaded one can be fine. Only the
per-task response time settles it.

## 4. The WCET problem

**This is the project's biggest open risk, and it deserves a direct statement
rather than a footnote.**

The analysis is exact *given the declared WCETs*. It does not derive them.
Deriving a sound WCET on a Cortex-M7 — with caches, branch prediction, a store
buffer, and flash wait states — is a research problem, not a build step. Any
tool claiming to do it automatically on that part is overselling.

Malleus does four things instead:

1. **Makes the WCET explicit.** It is a declared, reviewable number in a file
   under version control, with a name attached to the commit.
2. **Refuses to guess.** No WCET means `UNKNOWN`, not `PASS`. The uncertainty
   propagates: a task whose higher-priority neighbour has no declared WCET is
   also `UNKNOWN`, because treating the unknown as zero would produce a
   confident and wrong answer.
3. **Measures the observed maximum on hardware** and reports when reality
   exceeds the declaration — the number that catches an optimistic engineer.
4. **Publishes the margin,** so a task running at 95% of its declared budget is
   visible before it becomes a field failure.

This is a weaker guarantee than formal WCET analysis and a much stronger one
than the usual practice of not thinking about it. If the risk in
[the comparison](06-comparison.md#where-malleus-could-fail) materialises — if
engineers routinely declare optimistic numbers — the `PASS` becomes theatre and
this design has failed. We would rather find that out at Checkpoint 2 than at
year four.

## 5. Determinism in the API

Every public operation documents four properties. This is a hard rule, checked
in review:

```text
Channel::send()
  complexity:   O(1)
  allocates:    no
  blocks:       yes (policy-dependent)
  isr-safe:     no
  timeout:      yes

ReadySet::highest()
  complexity:   O(1) — one CLZ instruction, constant regardless of task count
  allocates:    no
  blocks:       no
  isr-safe:     yes
  timeout:      n/a
```

The workspace lint configuration is the machine-checked half of this promise:
`arithmetic_side_effects` and `indexing_slicing` are **denied** in `malleusrt`
and `malleus-arch`, because a panicking add inside a scheduler is a system that
stops scheduling. They are deliberately *not* applied to host tooling, where the
same rule buys nothing and costs readability — applying a rule where it does not
pay is how a codebase learns to reach for `#[allow]` by reflex.

## 6. Time

- **Monotonic**, 64-bit, in ticks of a board-declared rate. Never runs backwards,
  never jumps, unaffected by wall-clock changes, continues across tickless idle.
- **Default 1 MHz**, giving microsecond resolution. A 64-bit tick counter at
  1 MHz does not wrap for 584,000 years.
- **Ticks, not nanoseconds**, because converting on a Cortex-M without a
  hardware divider costs more than the abstraction saves.
- **Periods must be exact tick multiples.** A fractional ratio produces
  systematic jitter that looks like a hardware fault and is not. The validator
  warns (M0009).
- **Periodic activation does not drift.** `Period::advance` moves an absolute
  reference rather than sleeping for an interval, so execution time does not
  accumulate into phase error. Missed activations are counted and reported, not
  silently skipped — and the catch-up is arithmetic, never a loop, so a badly
  overrunning task cannot take unbounded time inside the kernel at exactly the
  moment you can least afford it.

See [ADR-0008](../adr/0008-time-and-tickless-idle.md).

## 7. Interrupt latency

Interrupt latency is the floor under every real-time guarantee, and the usual
way an RTOS destroys its own is blanket interrupt disabling in kernel critical
sections.

Malleus exposes two kinds:

- `CriticalSection::enter()` masks everything. Reserved for the handful of
  operations touching state shared with the fault handler. Every use is
  individually justified with a measured worst-case duration.
- `CriticalSection::enter_below(priority)` masks only at or below a given
  priority. **Everything else in the kernel uses this**, so a hard real-time ISR
  keeps its latency while lower-priority kernel work proceeds.

`BENCH-005` publishes the longest observed critical section inside the kernel.
It is a headline number, not a footnote: it bounds the interrupt latency the
kernel imposes, and a project unwilling to publish it is hiding something.

## 8. Published benchmarks

From the first release, CI publishes on real hardware:

| ID | Measurement |
|---|---|
| BENCH-001 | Interrupt latency — event to first handler instruction |
| BENCH-002 | Context switch — last instruction out to first instruction in |
| BENCH-003 | Wake-up latency — ISR signals a task to that task running |
| BENCH-004 | Periodic jitter over one million activations at 1 kHz |
| BENCH-005 | Longest critical section inside the kernel |

**Every one reports minimum, mean, p99.9, and observed maximum.**

A mean is close to useless in real-time work. The number that decides whether a
product ships is the worst one observed, and a project that publishes only
averages is either not measuring or not telling.

## References

- Joseph, M. & Pandya, P. (1986). *Finding Response Times in a Real-Time
  System.* The Computer Journal 29(5).
- Sha, L., Rajkumar, R. & Lehoczky, J. (1990). *Priority Inheritance Protocols.*
  IEEE Transactions on Computers 39(9).
- Audsley, N. et al. (1993). *Applying New Scheduling Theory to Static Priority
  Pre-emptive Scheduling.* Software Engineering Journal 8(5).
- Buttazzo, G. (2011). *Hard Real-Time Computing Systems*, 3rd ed.
