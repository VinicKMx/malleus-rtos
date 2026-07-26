# ADR-0003 — The kernel does not allocate

> **Status:** Accepted · **Checkpoint:** 0

## Context

Dynamic allocation on a microcontroller has three problems, and only the first
is widely acknowledged.

1. **Allocation can fail,** and the failure path is the least tested code in any
   embedded system.
2. **Allocation time is unbounded.** A general-purpose allocator's worst case
   depends on fragmentation, which depends on history. That makes it
   unanalysable, which makes any operation that allocates unanalysable.
3. **Fragmentation is a time bomb.** A device with a heap runs fine for weeks and
   then fails, and the failure depends on the exact sequence of allocations
   since boot. It is not reproducible on a bench.

The third is why "just add more RAM" does not help.

## Decision

**`malleusrt` has no allocator, does not depend on `alloc`, and never
allocates.**

Every object the kernel manages — task control blocks, stacks, channel storage,
timer entries, wait queues — is placed by `malleus-codegen` into statically sized
storage derived from the manifest.

This is enforced mechanically, not by discipline: `malleusrt` is `#![no_std]`
without an `alloc` dependency, so an allocating call does not compile.

**Application code may allocate.** A task can pull in `alloc` with a
bump or pool allocator if its problem calls for it. That is the application's
decision and the application's risk — and it is confined to that task, whose
memory is bounded by its protection regions. The kernel's guarantee is about the
kernel.

## Consequences

### What this buys

- **No allocation failure path exists** in the kernel, so it cannot be untested.
- **Every kernel operation has a bounded, analysable execution time.**
- **RAM usage is known at link time.** Running out is a build error.
- **No fragmentation**, so no class of failure that appears only after weeks of
  uptime.
- The memory report from `cargo malleus analyze` is *complete*, not an estimate.

### What this costs

- **Everything must be sized in advance,** and sizing is genuinely hard. An
  over-sized channel wastes RAM; an under-sized one drops messages. The tool
  reports both, but the engineer still has to decide.
- **Some data structures are awkward.** A variable-length message becomes a
  fixed-size buffer with a length field. That is more code and more waste.
- **Worst-case sizing dominates.** A channel that is usually empty and
  occasionally holds 64 messages costs 64 slots permanently.
- **Porting friction** for code that assumes a heap.

### What it forecloses

Nothing at the application level. It does mean the kernel cannot offer APIs
whose natural shape requires allocation — for example, a `Vec`-returning
introspection call. Those become fixed-capacity out-parameters, which is less
pleasant and more honest.

## Alternatives considered

**A fixed-size pool allocator inside the kernel.** Bounded time, no
fragmentation between pools. Rejected because it does not remove the failure
path — pool exhaustion is still a runtime error — and because every use of it
would be a place where static sizing was avoidable but avoided. If the pool must
be sized in advance anyway, a `static` array is the same thing with better error
messages and no allocator.

**Allocation permitted only at initialisation.** A common embedded compromise:
allocate everything at boot and never free. Rejected as strictly worse than
static allocation — the same memory is consumed, but the linker cannot see it,
so the build cannot tell you it does not fit. You trade a build error for a boot
failure.

**A global allocator with a documented worst case.** Some allocators (TLSF)
offer O(1) bounded-time allocation. Rejected because the bound covers time, not
fragmentation, and fragmentation is the failure that matters. It also would not
remove the failure path.

## Revisit when

Honestly: probably never for the kernel. This is one of the decisions least
likely to be reversed, because every argument for a heap is an argument about
convenience and every argument against is about a failure mode that appears
after shipping.

The one signal worth watching: if the *application-level* boundary proves too
restrictive — if real industrial applications routinely need allocation in
places the current design pushes into the kernel — that would indicate the
kernel/application split is drawn in the wrong place, which is a different
problem than this decision.
