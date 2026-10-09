# Findings Ledger: Borrow/Generic Integrity

Independent per-phase review record. One compact row per ruling batch; the full
evidence lives in the reviewers' reports and in the dispositions below.

| Finding | Phase | Severity | Area | Ruling |
|---------|-------|----------|------|--------|
| SR-001, MS-001 | 1 | CRITICAL | `own/instance_check.rs` nullable exemption too broad | fixed |
| RV-001, SR-004 | 1 | MAJOR | `own/expr.rs` mixed-conditional duplicate `E10034` | fixed |
| SR-002, RV-002 | 1 | MAJOR (pre-existing) | generic whole-binding self-assign through a lend slot | deferred — filed DEF-100 |
| SR-003 | 1 | MINOR | out-slot self-assign on non-droppable slot types | accepted — root-based rule as confirmed; the projected clause depends on it |
| RV-003 | 1 | MINOR | out-slot shape impl test is a base-green guard | accepted — the loop-count test is the red pin |
| RV-004 | 1 | MINOR (pre-existing) | all-borrow conditional reports two diagnostics at one site | accepted — base-identical; folded into the AR-13 refinement reasoning |
| RV-005 | 1 | MINOR (pre-existing) | field store into a non-nullable borrow parameter | accepted — field stores keep their existing rules by design |
| MS-002 | 1 | MINOR (pre-existing) | concrete `borrow T?` return double-frees | deferred — filed DEF-101 |
| PF-005 residual | 1 | MINOR (pre-existing) | runtime-indexed out-slot element self-assign | deferred — filed DEF-102 |

## Phase 1 dispositions (2026-10-09)

**SR-001 / MS-001 (CRITICAL) — fixed.** The nullable exemption in
`check_not_lend_requirements` was unconditional, so a generic `let`, `return`,
or `take` move of a nullable-substituted lend was accepted; the return shape
newly double-freed (the concrete `borrow T?` return has a pre-existing release
bug, filed DEF-101). Fix: the duty now records whether the move destination is
a writable parameter slot (`RequiresNotLend.stores_into_slot`), and the audit
exempts a nullable substitution only for those duties. Probes confirm the
generic `let`/`return`/`take` forms are rejected again with one `E10034` while
the nullable out-slot store stays accepted and ASan-clean. Pinned by two new
specification cases (owned local, returned lend).

**RV-001 / SR-004 (MAJOR) — fixed.** `lend_moves_judged` skipped the
target-side judgment only when every right-side move source was an already
judged lend move, so a conditional mixing a local arm with a lend arm produced
two `E10034` at one site (base produced one). Fix: the skip applies when any
source is judged. Pinned by the mixed-conditional clause of the both-lends
specification case.

**SR-002 / RV-002 (MAJOR, pre-existing) — deferred, DEF-100.** The exact
self-assign through a parameter-typed lend root (`copySelf<T>(borrow slot: T)`)
is unguarded at base and head; at `T = String` it double-frees. Filing it as a
defect keeps the phase's confirmed scope; expanding it would need the self
assigned rule extended to parameter-typed roots with a per-instance duty.

**SR-003 (MINOR) — accepted.** The confirmed rule rejects an exact self-assign
on an out-slot root regardless of the slot's element or pointee type; the
`T[]?` projected clause depends on that root-based shape. Narrowing it to
droppable slot types is recorded as a possible future refinement, not a phase
change.

**MS-002 (MINOR, pre-existing) — deferred, DEF-101.** The concrete
`borrow String?` return emits a non-emptying return and double-frees; base and
head emission is byte-identical apart from `#line`. The generic form stays
rejected (SR-001 fix), so this residual is unreachable through the new rules.

**PF-005 residual — deferred, DEF-102.** The runtime-indexed element
self-assign (`xs[i] = xs[i]`) stays unguarded; the exact constant-projection
form is covered.
