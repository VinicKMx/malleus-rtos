//! Task runtime and per-priority `async` executors.
//!
//! # Status: not yet implemented — Checkpoint 2
//!
//! # The idea this crate exists to express
//!
//! `async` does not replace the scheduler. It is a way to get cheap concurrency
//! *inside* one priority level, and it is very good at that: no extra stack per
//! logical activity, no context switch to interleave two waits. What it cannot
//! do is preempt. An `async` task that computes for 3 ms blocks every other
//! task on its executor for 3 ms, no matter how urgent they are.
//!
//! Systems that need both — and industrial controllers always do — get the
//! worst of it when forced to choose. So Malleus runs several executors, each
//! pinned to a preemptive priority level:
//!
//! ```text
//!   priority 8-15   preemptive hard real-time tasks
//!                   ├─ motor-control    1 kHz, 500us deadline
//!                   └─ safety-monitor   2 kHz, 200us deadline
//!
//!   priority 6      async executor: control-plane I/O
//!                   ├─ encoder read
//!                   └─ ADC/DMA completion
//!
//!   priority 4      async executor: network and protocol services
//!                   ├─ Modbus RTU server
//!                   ├─ MQTT client
//!                   └─ OTA update handler
//!
//!   priority 2      async executor: telemetry, storage, diagnostics
//! ```
//!
//! A hard real-time task preempts every executor beneath it. Inside one
//! executor, futures cooperate. The engineer chooses which model each piece of
//! work belongs in, and the manifest records that choice where a reviewer can
//! see it.
//!
//! # What Checkpoint 2 must deliver here
//!
//! - A `no_std`, non-allocating executor with statically allocated task slots
//! - A waker implementation that costs a bounded, documented number of cycles
//! - Timers integrated with the kernel's monotonic source and tickless idle
//! - Cancellation with defined semantics on drop, including for in-flight DMA
//! - `embedded-hal-async` implementations so existing drivers work unmodified
//!
//! On that last point: the goal is that a driver written for Embassy runs here
//! without being rewritten. Malleus competes on isolation, analysis, and
//! diagnostics — not by making the community port its drivers twice.
//! See `docs/adr/0009-ecosystem-interoperability.md`.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
