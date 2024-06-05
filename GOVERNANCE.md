# Governance

## Current state: one maintainer

**Malleus RTOS currently has a single maintainer**, Vinicius Pedrosa, who makes
all decisions.

Saying this plainly matters more than a governance document describing a
structure that does not exist. A project with an elaborate constitution and one
contributor is not well-governed; it is pretending. The bus factor is one, and
that is a documented risk tracked internally.

What follows is how decisions are made now, and how that changes as the project
grows. Each stage has an explicit trigger, so the transition is not left to
whenever the maintainer feels like sharing.

## How decisions are made

### Ordinary changes

Bug fixes, documentation, tests, board support: pull request, review, merge.

### Design changes

Anything altering the architecture, the public API, or a decision recorded in an
[ADR](docs/adr/) requires **a new ADR**. Existing ADRs are immutable; a changed
mind produces a superseding record, not an edit. Editing history to look
consistent destroys the record's only value.

An ADR must state:

1. What is decided
2. What it **costs** — an ADR listing only advantages is a sales pitch
3. What was rejected, fairly
4. **What would justify reopening it**

That last requirement is unusual and deliberate. A decision without a stated
revisit condition becomes permanent by default, long after the forces that
produced it have changed.

### Disagreements

Argue in the issue. If it does not resolve, the maintainer decides and records
the reasoning in an ADR. Once there are multiple maintainers, unresolved
disagreements go to a maintainer vote.

## Growing up

| Stage | Trigger | Structure |
|---|---|---|
| **1 — Now** | — | One maintainer. All decisions. |
| **2** | 3+ recurring contributors | Reviewers with merge rights in defined areas. Maintainer retains release and ADR authority. |
| **3** | Checkpoint 3 complete | 3 maintainers with release rights. ADRs need two approvals. Simple majority on disagreements. |
| **4 — 1.0** | Checkpoint 6 | Formal RFC process, security team, release managers, published LTS policy. |

**Stage 3 is a hard requirement for 1.0.** A 1.0 release promising stability from
a single-maintainer project is promising something one person's circumstances
can withdraw. Three maintainers with release rights is a stated 1.0 goal, not an
aspiration.

## Becoming a maintainer

There is no application. Maintainers are invited, based on:

- sustained, quality contributions over months
- **good judgement about what not to build** — the scarcer skill
- reviews that improve other people's work
- reliability: saying what you will do, then doing it

The second point deserves emphasis. This project's main risk is scope, not
capability. A contributor who argues convincingly that something belongs in
[non-goals](docs/design/12-non-goals.md) is demonstrating exactly the judgement
maintainership requires.

## Adding scope

Any proposal to expand what the project builds must answer:

1. Why is it **core to the thesis** rather than adjacent to it?
2. Why can no existing project be integrated instead?
3. **What is removed from the roadmap to pay for it?**

The third question is the real one. Scope is not free, and a proposal without a
subtraction is a proposal to do everything more slowly.

## Releases

Until 1.0: released when there is something worth releasing, with a changelog.
No schedule promised, because a schedule this project cannot keep is worse than
no schedule.

After 1.0: a published cadence, LTS branches, and a deprecation policy, per the
Checkpoint 6 goals.

## If the maintainer disappears

A real risk with a bus factor of one, and worth planning for while it is
hypothetical.

The project is MIT OR Apache-2.0 with **no CLA**, so anyone can fork and
continue. The documentation is written to make that possible: every decision has
an ADR, every design has a document, and the reasoning is recorded rather than
resident in one person's head.

That is not a substitute for succession planning. It is the minimum that makes
the work salvageable, and it is one of the reasons the documentation is treated
as a first-class deliverable rather than something to do later.

## Trademark

The name "Malleus RTOS" and any associated marks are not currently registered.
If the project reaches a scale where this matters, it will be addressed in a
dedicated ADR — trademark and code licensing are separate questions and should
not be conflated.
