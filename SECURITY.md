# Security policy

## Current status

> **Malleus RTOS is pre-alpha and is not used in production anywhere.** There is
> no kernel yet. Nothing here is deployed, so there is no fleet at risk.

That said, the policy exists from day one, because a project that adds one after
its first vulnerability report handles that report badly.

## Reporting a vulnerability

**Do not open a public issue.**

Report privately via **[GitHub Security Advisories](https://github.com/VinicKMx/malleus-rtos/security/advisories/new)**,
or by email to **vinicius.eduardo.pedrosa@gmail.com** with `[MALLEUS SECURITY]`
in the subject.

Please include: what you found, how to reproduce it, what an attacker could
achieve, affected versions or commits, and how you would like to be credited.

### What to expect

| | Target |
|---|---|
| Acknowledgement | 72 hours |
| Initial assessment | 7 days |
| Fix or mitigation plan | 30 days for high severity |
| Public disclosure | Coordinated, default 90 days |

These are targets for a single-maintainer project, stated honestly rather than
copied from a corporate template. If a deadline will be missed, you will be told
why rather than left waiting.

## Scope

### In scope

- **Isolation bypass** — a task reaching memory or peripherals outside its
  declared capabilities. This is the most serious class: it defeats the
  project's central security mechanism.
- **Privilege escalation** — unprivileged task code gaining kernel privilege.
- **Kernel memory corruption** from any input a task can control.
- **Capability enforcement failure** at compile time, build time, or run time.
- **Denial of service** — one task preventing a higher-priority task from
  meeting its deadline.
- **Unsound `unsafe`** in `malleusrt` or an `malleus-arch-*` crate.
- **Analyser unsoundness** — a `PASS` verdict for a task set that provably
  misses. The analyser is a safety tool and a wrong `PASS` is a security
  problem, not merely a bug.
- **Crash dumps leaking secrets.**
- **Build-tool code execution** from a malicious `malleus.toml`.

### Out of scope

Stated so that a reporter does not spend effort on something we have already
declined to defend, per the [threat model](docs/design/01-threat-model.md):

- **Physical attacks** — JTAG, flash extraction, glitching, board rework.
- **Side channels** — timing, power analysis, cache attacks.
- **Supply-chain compromise** of the toolchain or dependencies.
- **A hostile manifest author.** Whoever writes the manifest can grant any
  capability to any task. The model enforces a policy; it does not evaluate
  whether the policy is sensible.
- **Protocol-level weaknesses in protocols that have none.** Modbus RTU has no
  authentication. Malleus contains a compromise of the Modbus task; it does not
  make Modbus secure, and nothing can.
- **Vulnerabilities in integrated third-party crates** — report those upstream.
  Tell us too, so we can pin or patch.

## Disclosure

Coordinated. We will agree a date with you, credit you as you prefer, publish a
GitHub Security Advisory with a CVE where warranted, and describe the issue and
its fix plainly in the changelog.

**We will not** quietly patch and hope nobody notices. For an RTOS, users need to
know whether they are affected, and a silent fix denies them that.

## Security-relevant design

If you want to attack this project's model, these are the interesting documents:

- [Threat model](docs/design/01-threat-model.md) — adversaries, assets, residual risks
- [ADR-0005 — Memory isolation](docs/adr/0005-memory-isolation.md)
- [Capabilities](docs/design/09-capabilities.md)
- [ADR-0010 — Unsafe code policy](docs/adr/0010-unsafe-code-policy.md)

The residual risk most worth probing is **MPU misconfiguration**: a wrong
protection region means a fault that *does not happen*. Every other failure in
this system announces itself; an isolation boundary that was never programmed
correctly produces a system that appears to work perfectly, until the day
containment is needed and is not there.

## No bounty

There is no bug bounty. This is an unfunded personal project. Reports are
genuinely valued and will be credited; there is no money.
