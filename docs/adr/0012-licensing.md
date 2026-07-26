# ADR-0012 — Dual-license under MIT OR Apache-2.0

> **Status:** Accepted · **Checkpoint:** 0

## Context

The license decides who can adopt the project, and for an RTOS aimed at
commercial industrial products, it decides it before any technical merit is
considered. A license that requires a legal review is a license that stops
evaluation at the first meeting.

Three things must be true:

1. A company can put Malleus in a commercial product without publishing their
   firmware.
2. The project can be combined with the rest of the Rust ecosystem, which is
   almost uniformly MIT OR Apache-2.0.
3. Contributors and users have patent protection.

## Decision

**Dual-licensed under MIT OR Apache-2.0, at the user's option.** This is the
Rust ecosystem convention, and following it is worth more than any marginal
improvement a different choice might offer.

Contributions are accepted under the same terms, stated in `CONTRIBUTING.md`.
There is **no CLA** — contributors keep their copyright.

## Consequences

### What this buys

- **Commercial adoption without a legal conversation.** A firmware engineer can
  evaluate Malleus on a Tuesday without opening a ticket with legal.
- **Frictionless combination** with `embedded-hal`, `defmt`, `smoltcp`, and
  essentially everything else in embedded Rust.
- **Patent protection** via Apache-2.0's grant, with MIT available for users
  whose policies prefer the simpler license.
- **No CLA**, which removes the most common reason people decline to contribute
  to a project owned by one person or company.

### What this costs

- **A company can fork Malleus, improve it, and never contribute back.** This is
  a real cost and it is accepted deliberately: for infrastructure software, the
  adoption a permissive license enables is worth more than the contributions a
  copyleft license might compel. GPL'd RTOS projects exist and have not, on the
  whole, captured this market.
- **No leverage over commercial forks.** If a vendor ships a proprietary
  derivative, nothing can be done about it.
- **Without a CLA, relicensing later is effectively impossible** — it would
  require every contributor's agreement. This is intentional: it means the
  license cannot be changed out from under contributors, which is itself a
  reason to contribute.

### What it forecloses

Any future move to a copyleft or source-available license. Permanently, in
practice. That is the intended effect.

## Alternatives considered

**GPL-3.0.** Ensures improvements return to the community. Rejected: it would
eliminate most of the target audience. Firmware in a commercial industrial
controller under GPL-3.0 is a non-starter at nearly every company that would
otherwise be a user.

**MPL-2.0.** File-level copyleft; a reasonable middle ground that permits
commercial use while requiring modifications to specific files be shared.
Rejected as friction without proportionate benefit: it is unusual in the Rust
ecosystem, and "unusual license" alone costs evaluations. The value of matching
ecosystem convention is underrated.

**Apache-2.0 only.** Patent protection, well understood. Rejected because some
organisations have policies favouring MIT, and dual licensing costs nothing to
offer.

**MIT only.** Simplest. Rejected for the absence of a patent grant, which
matters in industrial and automotive contexts where patent portfolios are real.

**Business Source License or similar source-available terms.** Preserves a
commercial option. Rejected: it is not open source, it would prevent inclusion in
the ecosystem, and it would contradict the stated goal of community adoption.

## Revisit when

Realistically, never — this decision is one-way by design. The only scenario
worth naming is a future foundation or governance transfer, which would raise
questions about trademark and project stewardship, not about the code license.
Trademark on the name "Malleus RTOS" is a separate question and is not addressed
here.
