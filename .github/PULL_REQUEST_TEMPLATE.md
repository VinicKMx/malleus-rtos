## What and why

<!-- The diff shows what changed. Explain why it should change. -->

## Checklist

- [ ] `cargo xtask ci` passes locally
- [ ] Any new `unsafe` block has a `SAFETY:` comment stating its invariants
- [ ] Any new public operation documents its contract (complexity, allocation,
      blocking, ISR-safety, timeout)
- [ ] Any new diagnostic states what is wrong, **where**, and **what to do**
- [ ] Tests explain what they are protecting, not just what they call

## Does this contradict an ADR?

<!--
If it does, say which one and why. Either the change is wrong or the ADR is —
decide which, and do not leave the contradiction for a reviewer to discover.
If it supersedes an ADR, this PR should include the new one.
-->

- [ ] No ADR is affected
- [ ] An ADR is affected, and it is addressed above
