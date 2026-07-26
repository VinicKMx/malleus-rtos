# Changelog

Notable changes to Malleus RTOS. Format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versioning follows
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While the version is `0.0.x`, **anything may change without notice.** There is
no compatibility promise until 1.0, and pretending otherwise would be a promise
this project cannot keep.

## [Unreleased]

### Added — Checkpoint 0

**Architecture and design**

- Manifesto, threat model, memory model, concurrency model, real-time model,
  fault model, IPC design, capability model, observability design, glossary,
  and non-goals
- Comparison with Embassy, RTIC, Tock, Hubris, Zephyr, and FreeRTOS, including
  an explicit section on how this project could fail
- 12 Architecture Decision Records
- Four reference application specifications

**Working code**

- `malleus-manifest` — schema, parser, and validator with 23 diagnostic
  classes. Every diagnostic states what is wrong, where, and what to do about
  it; the last part is enforced by a test.
- `malleus-analyzer` — exact response-time analysis for fixed-priority
  preemptive scheduling. Reports `PASS`, `FAIL`, or `UNKNOWN`, and propagates
  uncertainty rather than assuming zero.
- `cargo malleus check` and `cargo malleus analyze`
- `malleus-arch` — the architecture contract, with an 18-item conformance suite
- `malleusrt` — kernel types and contracts: O(1) priority ready set, drift-free
  periodic activation, fault kinds and dispositions, restart policies

**Infrastructure**

- Host test suite (73 tests), `clippy -D warnings` clean
- Cross-compilation for `thumbv7em-none-eabihf` and `thumbv8m.main-none-eabihf`
- CI: format, lint, test, docs, cross-compile, MSRV, reference-manifest
  validation, and an `unsafe` audit
- `cargo xtask ci` — the same commands CI runs
- Contribution, governance, and security policies

### Not yet implemented

Stated here because a changelog listing only additions implies the rest exists:

- **The kernel does not boot.** No scheduler, no context switch, no port.
- No `async` runtime, no drivers, no code generation, no tracing, no crash
  dumps, no OTA.
- Blocking terms are not modelled in the analyser — the tool prints
  `Blocking terms: not yet modelled` on every report rather than letting a
  reader assume they were accounted for.
- Release jitter is not modelled.

[Unreleased]: https://github.com/VinicKMx/malleus-rtos/commits/main
