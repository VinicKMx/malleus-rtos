//! Exact response-time analysis for fixed-priority preemptive scheduling.
//!
//! # The method
//!
//! For a task *i*, the worst-case response time *R* is the fixed point of
//!
//! ```text
//! R = C_i + B_i + Σ  ⌈R / T_j⌉ · C_j
//!              j∈hp(i)
//! ```
//!
//! where *C* is worst-case execution time, *B* is worst-case blocking from
//! lower-priority tasks holding shared resources, *T* is period, and *hp(i)* is
//! the set of higher-priority tasks. The recurrence is solved by iteration from
//! `R = C_i + B_i`; it is monotonically increasing, so it either converges or
//! exceeds the deadline, and both outcomes are conclusive.
//!
//! This is Joseph and Pandya's response-time analysis (1986) with Sha's
//! blocking term, and it is exact for this scheduling model — not a sufficient
//! condition like the Liu–Layland utilisation bound, which rejects systems that
//! are in fact schedulable. See `docs/design/04-realtime-model.md` for why
//! Malleus uses the exact test and what it assumes.
//!
//! ## Assumptions, stated plainly
//!
//! - Tasks are independent except through declared shared resources.
//! - Deadlines are no greater than periods (the manifest validator enforces).
//! - Context-switch cost is folded into each task's declared WCET.
//! - Release jitter is zero. Jitter support is a planned extension; until it
//!   exists, a task with real release jitter should declare it inside its WCET.

/// One task's timing contract, as the analyser sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTiming {
    /// Task name, for reporting.
    pub name: String,
    /// Priority; higher is more urgent.
    pub priority: u8,
    /// Activation period in ticks. `None` for aperiodic tasks, which are
    /// excluded from the analysis and reported as such.
    pub period: Option<u64>,
    /// Relative deadline in ticks.
    pub deadline: Option<u64>,
    /// Declared worst-case execution time in ticks.
    pub wcet: Option<u64>,
    /// Worst-case blocking in ticks, from lower-priority tasks holding
    /// resources this task needs. Computed from the manifest's shared-resource
    /// declarations; zero when the task shares nothing.
    pub blocking: u64,
}

/// The outcome for one task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The worst-case response time fits inside the deadline.
    Pass {
        /// Computed worst-case response time, in ticks.
        response: u64,
        /// Ticks of margin remaining.
        slack: u64,
    },
    /// The task provably misses its deadline.
    Fail {
        /// Computed worst-case response time, in ticks.
        response: u64,
        /// Ticks by which the deadline is overrun.
        overrun: u64,
    },
    /// Not enough was declared to decide.
    ///
    /// A first-class outcome, not an error. `UNKNOWN` tells the engineer
    /// exactly what to measure next; a fabricated `PASS` tells them nothing and
    /// costs them a field failure.
    Unknown {
        /// What is missing.
        reason: &'static str,
    },
}

impl Verdict {
    /// Whether this verdict permits shipping.
    #[must_use]
    pub const fn is_pass(&self) -> bool {
        matches!(self, Self::Pass { .. })
    }
}

/// The complete analysis of a task set.
#[derive(Debug, Clone)]
pub struct Analysis {
    /// Per-task verdicts, in the order given.
    pub verdicts: Vec<(String, Verdict)>,
    /// Total CPU utilisation of the periodic task set, in parts per million.
    ///
    /// Parts per million rather than a float, so the number is exactly
    /// reproducible across platforms — a build report that differs between two
    /// machines is a build report nobody trusts.
    pub utilisation_ppm: u64,
}

impl Analysis {
    /// Whether every task provably meets its deadline.
    #[must_use]
    pub fn is_schedulable(&self) -> bool {
        self.verdicts.iter().all(|(_, v)| v.is_pass())
    }

    /// Whether any task could not be decided.
    #[must_use]
    pub fn has_unknowns(&self) -> bool {
        self.verdicts
            .iter()
            .any(|(_, v)| matches!(v, Verdict::Unknown { .. }))
    }
}

/// Maximum iterations of the response-time recurrence before giving up.
///
/// The recurrence converges quickly for schedulable task sets. A high iteration
/// count means the system is at or past saturation, where the response time
/// diverges. Bounding this keeps the build from hanging on a pathological
/// input — the tool must always terminate with an answer, even if the answer is
/// "this does not converge".
const MAX_ITERATIONS: u32 = 1_000;

/// Analyse a task set.
///
/// # Contract
///
/// Pure, deterministic, and terminating. Given the same task set it returns the
/// same result on every platform, which is what makes the report suitable as a
/// reviewable build artefact.
#[must_use]
pub fn analyse(tasks: &[TaskTiming]) -> Analysis {
    let mut verdicts = Vec::with_capacity(tasks.len());
    let mut utilisation_ppm: u64 = 0;

    for task in tasks {
        match (task.period, task.wcet) {
            // A zero period is unbounded demand, not absent demand. Skipping it
            // reported a comfortable figure for a task set that cannot run at
            // all; saturating makes the impossibility visible in the number.
            // The manifest validator rejects this upstream (M0026), so reaching
            // it means the analyser was driven directly as a library.
            (Some(0), Some(_)) => utilisation_ppm = u64::MAX,
            (Some(period), Some(wcet)) => {
                utilisation_ppm =
                    utilisation_ppm.saturating_add(wcet.saturating_mul(1_000_000) / period);
            }
            _ => {}
        }
        verdicts.push((task.name.clone(), verdict_for(task, tasks)));
    }

    Analysis {
        verdicts,
        utilisation_ppm,
    }
}

fn verdict_for(task: &TaskTiming, all: &[TaskTiming]) -> Verdict {
    let Some(wcet) = task.wcet else {
        return Verdict::Unknown {
            reason: "no worst-case execution time declared; measure it with `cargo malleus trace` \
                     and add `wcet` to the manifest",
        };
    };
    let Some(deadline) = task.deadline.or(task.period) else {
        return Verdict::Unknown {
            reason: "task is aperiodic and declares no deadline, so there is no timing contract \
                     to check",
        };
    };
    if task.period.is_none() {
        return Verdict::Unknown {
            reason: "task declares a deadline but no period; without a minimum inter-arrival \
                     time its interference on others is unbounded",
        };
    }

    // Higher-priority tasks that can preempt this one. A higher-priority task
    // without a declared WCET makes this task's interference unknowable, so the
    // uncertainty propagates rather than being quietly dropped.
    let mut interferers = Vec::new();
    for other in all {
        if other.name == task.name || other.priority <= task.priority {
            continue;
        }
        match (other.period, other.wcet) {
            (Some(0), _) => {
                return Verdict::Unknown {
                    reason: "a higher-priority task declares a zero period, so it demands the \
                             CPU without bound and the interference on this task cannot be \
                             computed",
                };
            }
            (Some(period), Some(wcet)) => interferers.push((period, wcet)),
            _ => {
                return Verdict::Unknown {
                    reason: "a higher-priority task has no declared period or WCET, so the \
                             interference on this task cannot be bounded",
                };
            }
        }
    }

    let base = wcet.saturating_add(task.blocking);
    let mut response = base;

    for _ in 0..MAX_ITERATIONS {
        let mut next = base;
        for &(period, other_wcet) in &interferers {
            // ⌈response / period⌉ activations of the interferer fit in the
            // window, each costing its full WCET.
            let activations = response.div_ceil(period);
            next = next.saturating_add(activations.saturating_mul(other_wcet));
        }

        if next == response {
            return if response <= deadline {
                Verdict::Pass {
                    response,
                    slack: deadline - response,
                }
            } else {
                Verdict::Fail {
                    response,
                    overrun: response - deadline,
                }
            };
        }
        if next > deadline {
            // The recurrence is monotonically increasing, so once it passes the
            // deadline it can never come back. Stopping here is exact, not an
            // approximation.
            return Verdict::Fail {
                response: next,
                overrun: next - deadline,
            };
        }
        response = next;
    }

    // Falling out of the loop means the recurrence neither converged nor passed
    // the deadline within the bound. Reporting `Fail` here published whatever
    // value the iteration happened to stop on as though it were a computed
    // worst case. No worst-case response time was established, so the honest
    // answer is that it is unknown.
    Verdict::Unknown {
        reason: "the response-time recurrence did not converge within the iteration bound, \
                 so no worst-case response time was established for this task",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The iteration bound exists so the tool terminates. It used to terminate
    /// by publishing whatever value the loop stopped on as a computed `Fail`,
    /// which is a fabricated worst case. Not converging is a statement about
    /// the analysis, not about the task.
    #[test]
    fn non_convergence_is_unknown_not_a_fabricated_failure() {
        // A 100%-utilisation interferer (T = C = 1) makes the recurrence grow
        // by exactly one tick per iteration, so a deadline far above the
        // iteration bound exhausts the loop without ever crossing it.
        let tasks = vec![
            task("interferer", 9, 1, 1, 1),
            task("victim", 1, 100_000, 100_000, 1),
        ];
        let analysis = analyse(&tasks);
        let (_, verdict) = analysis
            .verdicts
            .iter()
            .find(|(n, _)| n == "victim")
            .expect("victim is analysed");
        match verdict {
            Verdict::Unknown { reason } => {
                assert!(
                    reason.contains("converge"),
                    "reason must name non-convergence, got: {reason}"
                );
            }
            other => panic!("expected Unknown for a non-converging recurrence, got {other:?}"),
        }
    }

    /// The reason string is the whole value of an `UNKNOWN`. Blaming a missing
    /// declaration for a task that declared both period and WCET sends the
    /// engineer to fix something that is not wrong.
    #[test]
    fn a_zero_period_interferer_is_named_accurately() {
        let mut interferer = task("interferer", 9, 0, 100, 10);
        interferer.period = Some(0);
        let tasks = vec![interferer, task("victim", 1, 1000, 1000, 10)];
        let analysis = analyse(&tasks);
        let (_, verdict) = analysis
            .verdicts
            .iter()
            .find(|(n, _)| n == "victim")
            .expect("victim is analysed");
        match verdict {
            Verdict::Unknown { reason } => {
                assert!(
                    reason.contains("zero period"),
                    "reason must name the zero period, got: {reason}"
                );
                assert!(
                    !reason.contains("no declared period or WCET"),
                    "must not claim a missing declaration that is present: {reason}"
                );
            }
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    /// A zero period is unbounded demand. Dropping it from the sum reported a
    /// comfortable utilisation for a task set that cannot run.
    #[test]
    fn a_zero_period_task_does_not_vanish_from_utilisation() {
        let tasks = vec![
            task("greedy", 5, 0, 100, 50),
            task("other", 3, 1000, 1000, 100),
        ];
        let analysis = analyse(&tasks);
        assert_eq!(
            analysis.utilisation_ppm,
            u64::MAX,
            "unbounded demand must not be reported as 10% CPU"
        );
    }

    fn task(name: &str, priority: u8, period: u64, deadline: u64, wcet: u64) -> TaskTiming {
        TaskTiming {
            name: name.to_owned(),
            priority,
            period: Some(period),
            deadline: Some(deadline),
            wcet: Some(wcet),
            blocking: 0,
        }
    }

    #[test]
    fn a_single_task_responds_in_its_own_wcet() {
        let set = [task("solo", 5, 1000, 1000, 300)];
        let analysis = analyse(&set);
        assert_eq!(
            analysis.verdicts[0].1,
            Verdict::Pass {
                response: 300,
                slack: 700
            }
        );
        assert!(analysis.is_schedulable());
        assert_eq!(analysis.utilisation_ppm, 300_000, "300/1000 = 30%");
    }

    #[test]
    fn a_higher_priority_task_interferes_exactly_once_per_period() {
        // `high` runs 100 every 500. `low` runs 200 every 1000.
        // Worst case for `low`: 200 + ⌈R/500⌉·100. R = 200 → ⌈200/500⌉=1 → 300.
        // R = 300 → still 1 → 300. Converged at 400? Recompute: base 200,
        // iteration 1 gives 200 + 1*100 = 300; iteration 2: ⌈300/500⌉ = 1 →
        // 300. Fixed point 300.
        let set = [
            task("high", 8, 500, 500, 100),
            task("low", 3, 1000, 1000, 200),
        ];
        let analysis = analyse(&set);
        assert_eq!(
            analysis.verdicts[0].1,
            Verdict::Pass {
                response: 100,
                slack: 400
            }
        );
        assert_eq!(
            analysis.verdicts[1].1,
            Verdict::Pass {
                response: 300,
                slack: 700
            }
        );
        assert!(analysis.is_schedulable());
    }

    #[test]
    fn interference_compounds_when_the_interferer_is_fast() {
        // `fast` runs 100 every 200; `slow` runs 300 every 1000.
        // base 300 → ⌈300/200⌉=2 → 300+200 = 500 → ⌈500/200⌉=3 → 300+300 = 600
        // → ⌈600/200⌉=3 → 600. Fixed point 600.
        let set = [
            task("fast", 9, 200, 200, 100),
            task("slow", 2, 1000, 1000, 300),
        ];
        let analysis = analyse(&set);
        assert_eq!(
            analysis.verdicts[1].1,
            Verdict::Pass {
                response: 600,
                slack: 400
            }
        );
    }

    #[test]
    fn an_overloaded_set_provably_fails() {
        // 600/1000 + 600/1000 = 120% utilisation. No schedule exists.
        let set = [task("a", 9, 1000, 1000, 600), task("b", 5, 1000, 1000, 600)];
        let analysis = analyse(&set);
        assert!(!analysis.is_schedulable());
        assert!(matches!(analysis.verdicts[1].1, Verdict::Fail { .. }));
        assert!(analysis.utilisation_ppm > 1_000_000);
    }

    #[test]
    fn blocking_is_added_to_the_response_time() {
        let mut set = [task("solo", 5, 1000, 1000, 300)];
        set[0].blocking = 150;
        let analysis = analyse(&set);
        assert_eq!(
            analysis.verdicts[0].1,
            Verdict::Pass {
                response: 450,
                slack: 550
            }
        );
    }

    #[test]
    fn blocking_alone_can_break_a_deadline() {
        // Priority inversion is exactly this: the task's own work fits, and it
        // still misses, because it waited on something below it.
        let mut set = [task("solo", 5, 1000, 400, 300)];
        set[0].blocking = 200;
        let analysis = analyse(&set);
        assert_eq!(
            analysis.verdicts[0].1,
            Verdict::Fail {
                response: 500,
                overrun: 100
            }
        );
    }

    #[test]
    fn a_missing_wcet_yields_unknown_not_pass() {
        let set = [TaskTiming {
            name: "mystery".to_owned(),
            priority: 5,
            period: Some(1000),
            deadline: Some(1000),
            wcet: None,
            blocking: 0,
        }];
        let analysis = analyse(&set);
        assert!(matches!(analysis.verdicts[0].1, Verdict::Unknown { .. }));
        assert!(
            !analysis.is_schedulable(),
            "UNKNOWN must never count as schedulable"
        );
        assert!(analysis.has_unknowns());
    }

    #[test]
    fn uncertainty_propagates_downward_through_priorities() {
        // If the high-priority task's cost is unknown, nothing below it can be
        // declared safe — silently treating the unknown as zero would produce a
        // confident and wrong PASS.
        let set = [
            TaskTiming {
                name: "unknown-high".to_owned(),
                priority: 9,
                period: Some(500),
                deadline: Some(500),
                wcet: None,
                blocking: 0,
            },
            task("low", 3, 1000, 1000, 100),
        ];
        let analysis = analyse(&set);
        assert!(
            matches!(analysis.verdicts[1].1, Verdict::Unknown { .. }),
            "interference from an undeclared task must poison the verdict below it"
        );
    }

    #[test]
    fn deadline_defaults_to_period_when_unstated() {
        let set = [TaskTiming {
            name: "implicit".to_owned(),
            priority: 5,
            period: Some(1000),
            deadline: None,
            wcet: Some(300),
            blocking: 0,
        }];
        assert_eq!(
            analyse(&set).verdicts[0].1,
            Verdict::Pass {
                response: 300,
                slack: 700
            }
        );
    }

    #[test]
    fn analysis_terminates_on_a_saturated_set() {
        // Exactly 100% utilisation with two tasks: the low-priority one's
        // recurrence does not converge below its deadline. This must terminate.
        let set = [task("a", 9, 1000, 1000, 500), task("b", 5, 1000, 1000, 500)];
        let analysis = analyse(&set);
        assert_eq!(analysis.utilisation_ppm, 1_000_000);
        // Whatever the verdict, the call returned — that is what is under test.
        assert_eq!(analysis.verdicts.len(), 2);
    }

    #[test]
    fn the_reference_industrial_controller_is_schedulable() {
        // The flagship demo from docs/reference-apps/industrial-controller.md,
        // in microsecond ticks. If this ever stops passing, either the demo's
        // numbers or the analyser has drifted, and both matter.
        let set = [
            task("safety-monitor", 9, 500, 200, 40),
            task("motor-control", 7, 1_000, 500, 180),
            task("sensor-acquisition", 6, 2_000, 2_000, 300),
            task("modbus", 4, 10_000, 10_000, 900),
            task("telemetry", 2, 100_000, 100_000, 8_000),
        ];
        let analysis = analyse(&set);
        assert!(
            analysis.is_schedulable(),
            "reference application must be schedulable"
        );
        assert!(!analysis.has_unknowns());
        assert_eq!(analysis.utilisation_ppm, 580_000, "58.0% CPU");

        // Exact response times, pinned. These numbers are published in the
        // crate documentation and in the reference-application write-up; if the
        // analyser drifts, those documents become fiction and this test says so.
        let expected = [
            ("safety-monitor", 40u64),
            ("motor-control", 220),
            ("sensor-acquisition", 560),
            ("modbus", 1_720),
            ("telemetry", 16_920),
        ];
        for ((name, verdict), (expected_name, expected_response)) in
            analysis.verdicts.iter().zip(expected)
        {
            assert_eq!(name, expected_name);
            match verdict {
                Verdict::Pass { response, .. } => {
                    assert_eq!(*response, expected_response, "response time for {name}");
                }
                other => panic!("{name} should pass, got {other:?}"),
            }
        }
    }
}
