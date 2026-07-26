//! Build-time schedulability, stack, and memory analysis.
//!
//! # What this crate is for
//!
//! A real-time system either meets its deadlines or it does not, and that is a
//! property you can compute from the task set *before* the board is powered on.
//! Most embedded projects instead discover it empirically, months later, as an
//! intermittent field failure. This crate closes that gap: it takes the
//! declared timing contracts from `malleus.toml` and answers the question
//! directly.
//!
//! Real output from `cargo malleus analyze` on
//! `examples/industrial-controller`, in microsecond ticks:
//!
//! ```text
//! Schedulability — fixed-priority preemptive, exact response-time analysis
//! System: industrial-controller   tick rate: 1MHz
//!
//!   Task                    Prio     Period   Deadline       WCET  Verdict
//!   safety-monitor             9        500        200         40  PASS  response 40t, slack 160t
//!   motor-control              7       1000        500        180  PASS  response 220t, slack 280t
//!   sensor-acquisition         6       2000       2000        300  PASS  response 560t, slack 1440t
//!   modbus                     4      10000      10000        900  PASS  response 1720t, slack 8280t
//!   telemetry                  2     100000     100000       8000  PASS  response 16920t, slack 83080t
//!
//!   CPU utilisation (periodic tasks): 58.0%
//!
//!   Verdict: PASS
//! ```
//!
//! Note `telemetry`: it needs 8 ms of CPU but does not finish for 16.9 ms,
//! because everything above it preempts it repeatedly. It still meets its
//! 100 ms deadline. That gap between "work" and "response" is the whole reason
//! utilisation alone is not an answer — a 58% loaded system can still miss.
//!
//! # Honesty about what it proves
//!
//! The analysis is exact **given the declared WCETs**. It does not derive them.
//! Deriving a sound WCET on a Cortex-M7 — with its caches, branch predictor,
//! and store buffer — is a research problem, not a build step, and any tool
//! claiming otherwise on that part is overselling.
//!
//! So Malleus does the honest thing: it makes the WCET an explicit, declared,
//! reviewable number, checks the system against it exactly, and *separately*
//! measures the observed maximum on hardware and tells you when reality has
//! exceeded your declaration. A tool that says `UNKNOWN` when it does not know
//! is more useful than one that says `PASS` because it assumed.
//! See `docs/design/04-realtime-model.md`.

mod response_time;

pub use response_time::{Analysis, TaskTiming, Verdict, analyse};
