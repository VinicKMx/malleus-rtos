# Threat model

> **Status:** accepted · **Checkpoint:** 0

A threat model that claims to defend against everything defends against nothing.
This one states what Malleus protects, from whom, and — importantly — what it
does not protect against at all.

## 1. What we are protecting

An industrial controller in the field: on a plant floor, in a cabinet, at a
remote site. It controls physical equipment and is connected to a network.

**Assets, in order of importance:**

1. **Physical safety.** The equipment must not do something dangerous.
2. **Availability.** The device must keep controlling, including through partial
   failure.
3. **Integrity of the control path.** Commands and sensor readings must not be
   silently altered.
4. **Firmware integrity.** Only authentic firmware runs.
5. **Confidentiality of credentials.** Keys and certificates on the device.
6. **Confidentiality of process data.** Usually the least critical, and usually
   the one vendors focus on.

## 2. Adversaries and exposure

| Adversary | Access | In scope |
|---|---|---|
| **Remote network attacker** | Network only | Yes — primary |
| **Malicious or broken peer** | Speaks Modbus/MQTT to the device | Yes — primary |
| **Buggy third-party code** | Runs as a task on the device | Yes — primary |
| **Local network attacker** | Same LAN, can spoof and replay | Yes |
| **Insider with credentials** | Legitimate management access | Partially |
| **Attacker with physical access** | JTAG, flash, board rework | **Largely out of scope** |
| **Nation-state, side channels, glitching** | Laboratory attack | **Out of scope** |

**The "buggy third-party code" row is the one Malleus is unusual in taking
seriously.** In practice, the most likely thing to compromise an industrial
controller is not an attacker — it is an MQTT library with a buffer overflow, a
vendor driver that scribbles, or a protocol parser fed a malformed frame. Malleus
treats a component failure and a component compromise as the same containment
problem, because from the kernel's position they are.

## 3. What Malleus defends

### 3.1 Fault and compromise containment — primary

A compromised or broken task cannot:

- read or write another task's memory (MPU, `ARCH-MPU-001`)
- reach a peripheral it did not declare (`ARCH-MPU-003`)
- program a DMA transfer outside its own regions (`ARCH-MPU-003`)
- send on a channel it does not hold a capability for (compile-time)
- prevent a higher-priority task from running (preemption)
- corrupt the kernel (privilege separation)

**This is the core defence.** An MQTT client with a remote code execution
vulnerability is contained to the MQTT task. The motor loop keeps running.

### 3.2 Availability under attack

A task that hangs, floods, or overruns is bounded by:

- fixed-priority preemption — it cannot starve anything above it
- bounded channels with declared overflow policies — it cannot exhaust memory
- per-task watchdogs — a hang is detected and supervised
- restart budgets — a crash loop degrades rather than spinning forever

### 3.3 Least privilege by construction

Capabilities are declared per task and enforced by hardware. A telemetry task
that never declared `pwm.motor` cannot reach the motor PWM register, whatever
its code does.

### 3.4 Firmware authenticity

Via **MCUboot** (Checkpoint 4). We do not build this ourselves — see
[non-goals](12-non-goals.md).

### 3.5 Forensics

Crash dumps and traces let an operator determine *what* happened after a field
incident, which is a security property as much as a debugging one: an
exploitation attempt that crashes a task leaves evidence.

## 4. What Malleus does **not** defend against

Stated plainly. A threat model's honesty is measured here.

### 4.1 Physical attackers

An attacker with the board can attach a debugger, read flash, glitch the CPU,
and desolder parts. Malleus offers **nothing** against this beyond what the
silicon vendor provides (readout protection, secure boot fuses, TrustZone on
ARMv8-M).

If your threat model includes physical access, you need hardware
countermeasures, and Malleus is not one.

### 4.2 Side-channel attacks

No constant-time guarantees, no power-analysis resistance, no cache-timing
mitigations. Cryptographic operations should use libraries that provide these;
the kernel does not.

### 4.3 A compromised kernel

The kernel is trusted. A vulnerability in it compromises everything, because
there is no containment above it. This is why the kernel is deliberately small,
why every `unsafe` block is documented, and why kernel changes need two
reviewers ([ADR-0010](../adr/0010-unsafe-code-policy.md)).

### 4.4 Supply-chain attacks

A malicious dependency, a compromised build machine, or a backdoored toolchain
defeats everything here. Reproducible builds and dependency review help; they do
not solve it.

### 4.5 A hostile system integrator

Someone who writes the manifest can grant any task any capability. The model
protects tasks from each other according to a policy; it does not protect
against a bad policy. The generated architecture documentation exists partly so
that a bad policy is *visible* in review.

### 4.6 Protocol-level attacks

Modbus RTU has no authentication. If your network is hostile and you speak
plain Modbus, Malleus contains the damage to the Modbus task — it does not make
Modbus secure, because nothing can.

## 5. Assumptions

1. The manifest is written by someone trusted.
2. The toolchain and dependencies are not compromised.
3. The hardware behaves as documented. (Errata are a real and recurring
   exception.)
4. Secure boot, where used, is correctly provisioned.
5. Physical access is controlled by other means.
6. The kernel is correct — mitigated by size and review, not eliminated.

## 6. Residual risks

| Risk | Why it remains | Mitigation |
|---|---|---|
| Kernel vulnerability | No containment above the kernel | Small kernel, documented `unsafe`, two-reviewer policy, fuzzing |
| MPU misconfiguration | A wrong region means a fault that *does not happen* — silent | Build-time region allocation; `ARCH-MPU-*` tested on hardware; layout report reviewed |
| Capability over-granting | Human decision | Generated architecture docs make the grant visible in review |
| DMA outside regions | Depends on correct MPU/peripheral setup | `ARCH-MPU-003` tested explicitly on hardware |
| Physical attack | Out of scope | Vendor silicon features |

The MPU misconfiguration row deserves emphasis. Every other failure here
announces itself. An isolation boundary that was never programmed correctly
produces a system that appears to work perfectly — until the day the containment
is needed and is not there. That is why the region layout is a committed,
reviewed artefact and why the conformance tests run on hardware rather than in
simulation.

## 7. Reporting a vulnerability

See [SECURITY.md](../../SECURITY.md).

## 8. Review

This document is reviewed at each checkpoint and whenever the isolation or
capability model changes. If you believe something here is wrong or missing,
please open an issue — a threat model nobody argues with has probably not been
read.
