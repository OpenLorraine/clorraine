# Memory and Value Integrity (Batch N) Implementation Plan

> **Feature**: Close the 23 un-parked rows left by the batch-M reviews: the nine remaining
> 🔴 criticals `DEF-428`, `DEF-429`, `DEF-430`, `DEF-431`, `DEF-432`, `DEF-433`, `DEF-437`,
> `DEF-440`, `DEF-441`, and the fourteen 🟠 majors `DEF-410`, `DEF-421`, `DEF-423`, `DEF-424`,
> `DEF-425`, `DEF-426`, `DEF-427`, `DEF-435`, `DEF-436`, `DEF-439`, `DEF-445`, `DEF-446`,
> `DEF-447`, `DEF-448`.
> **Status**: Planning
> **Created**: 2026-10-18
> **Planning baseline**: `f6da78f` (the batch-M close-out commit)
> **Probed**: 21 rows red; `DEF-427` and `DEF-447` verified not reproducible and planned as
> false-positive control resolutions (AR-27, AR-28)
> **CodeOps Artifact Schema**: 1

## Overview

This is batch N, the memory-and-value follow-up to batch M. The rows were filed by the batch-M
reviews after their own corrections: the loan-completeness residuals of the element/store copy
machinery (`DEF-428`–`DEF-433`, `DEF-435`), the raw-deref move-out channels (`DEF-437`,
`DEF-439`–`DEF-441`), the store-lowering and evaluation-order residuals (`DEF-421`, `DEF-423`,
`DEF-424`, `DEF-436`), the per-instance generic judgments (`DEF-410`, `DEF-425`, `DEF-426`), and
the literal-destination residues (`DEF-445`, `DEF-446`, `DEF-448`). Nine rows are the last
un-parked criticals; they are planned first and every one of them has a memory-safety lens.

Every row was rebuilt and probed against the compiler at head `f6da78f` (AR-3; per-row evidence in
`02-current-state.md`). Twenty-one reproduce. Two do not:

- **`DEF-427`** — the filed `len(w.items[i][0])` double index selects a `u8` byte, not the `str`
  element; `len` of a byte is not the element's length. Every view-length spelling
  (`len(w.items[i])`, `len(w.items[i].name)`, concrete, generic, borrow, and loop alias) resolves
  to 2. The row is a false positive whose expected value belongs to the single index.
- **`DEF-447`** — the filed inline read-base owning literal and its nested-field, `String`-field,
  and view-copy variants are LSan-clean in every probe at the head. No reproduction exists. The
  row is a false positive left over from the phase-7 body-scope read-base binding.

Both are planned as **control-resolution** tasks in phase 7: a green control pins the behavior,
the probe evidence closes the row, and no compiler change is made.

Every fix is conformance to the frozen specification. No new language surface, no new diagnostic
code, and no specification or registry edit (AR-33). A row that turns out to need one is parked
with its evidence; the two false positives are closed as controls, not parked. The two
ruling-parked majors (`DEF-167`, `DEF-171`) and every other major or minor stay out (AR-1).

## Minimum-Sufficient Baseline

**Original goal:** eliminate the rows' observable failures — an accepted program that reads freed
storage, double-frees, faults where the frozen guard requires a trap, fails the strict C build,
silently drops a store, or accepts a move the concrete twin rejects — so every accepted spelling
builds strictly and runs with the language's semantics, and every invalid spelling reports the
frozen diagnostic.

**Smallest viable design:** complete the existing machinery the rows name:

- the element/store **loan bookkeeping** in `own/place.rs` and its call sites in `own/expr.rs` and
  `own/statements.rs` (self-assignment, composite copy sources, runtime-union reads, prefix
  fallback, non-place provenance, call-result binding loans, element-store collectors);
- the **raw-deref move-out channels**: the shared materializer, `take_pointee_move_outs`, the
  intrinsic borrow rule, and the guarded value channel (`own/lower.rs`, `own/call.rs`,
  `codegen/function.rs`);
- the **store lowering**: the `Deref`-rooted place, whole-element target pinning, RHS-before-clear
  ordering, and the shared materializer on the element/raw-target RHS (`own/place.rs`,
  `own/expr.rs`, `codegen/function.rs`);
- the **per-instance judgments**: conditional move sources that mention a type parameter, element
  moves through a borrowed parameter with a projection path, and the division guard's resolved
  operand type (`own/expr.rs`, `own/place.rs`, `own/instance_check.rs`, `codegen/guards.rs`);
- the **literal destinations**: conditional arms in the read/store predicates and the element-wise
  store form for conditional literals (`own/lower.rs`, `codegen/function.rs`).

No new pass, no new abstraction, no new dependency (AR-34).

**Excluded machinery:** a new diagnostics family (every rejection reuses the frozen registry); a
new place kind (the frozen closed list governs, AR-17); a new IR channel beyond the existing
`value_move_outs` and `Materialize` records; a spec edit.

**Approved complexity:** None — every fix extends an existing hook (AR-34).

## Document Index

| #    | Document | Description |
| ---- | -------- | ----------- |
| AR   | [Ambiguity Register](00-ambiguity-register.md) | Zero-Ambiguity Gate decisions (audit trail) |
| 00   | [Index](00-index.md) | This document — overview and navigation |
| 01   | [Requirements](01-requirements.md) | Scope, the 23 rows, and acceptance criteria |
| 02   | [Current State](02-current-state.md) | The head-`f6da78f` machinery and each row's probe gap |
| 03-01 | [Loan transfer on stored copies](03-01-loan-store-transfers.md) | P1 design (`DEF-428`, `429`, `431`, `435`) |
| 03-02 | [Loan sources through runtime, temporary, and call reads](03-02-loan-sources.md) | P2 design (`DEF-430`, `432`, `433`) |
| 03-03 | [Raw-deref move-outs](03-03-deref-move-outs.md) | P3 design (`DEF-437`, `439`, `440`, `441`) |
| 03-04 | [Store lowering and evaluation order](03-04-store-lowering-order.md) | P4 design (`DEF-421`, `423`, `424`, `436`) |
| 03-05 | [Per-instance generic judgments](03-05-per-instance-generic.md) | P5 design (`DEF-410`, `425`, `426`) |
| 03-06 | [Literal destinations and conditional arrays](03-06-literal-destinations.md) | P6 design (`DEF-445`, `446`, `448`) |
| 03-07 | [Control resolution](03-07-control-resolution.md) | P7 design (`DEF-427`, `447` — false positives) |
| 05   | [Findings Ledger](05-findings.md) | Phase-review verdicts, fix rounds, and filed rows |
| 07   | [Testing Strategy](07-testing-strategy.md) | ST cases and verification |
| 99   | [Execution Plan](99-execution-plan.md) | Phases, sessions, and task checklist |

## Quick Reference

### What each phase closes

| Phase | Rows | Surface |
|-------|------|---------|
| 1 | `DEF-428`, `429`, `431`, `435` | Element and store loan transfer on stored copies: exact self-assignment, composite source into element/projected destinations, the deeper-projection prefix fallback, and the element-store nested fill |
| 2 | `DEF-430`, `432`, `433` | Loan sources through a runtime-index store then constant read, a temporary-derived element in a view-array literal, and a call-mediated element-view result |
| 3 | `DEF-437`, `439`, `440`, `441` | Raw-deref move-outs: a projection of a raw pointee, a guarded take whose raw pointer aliases the conditional target, a guarded borrowing intrinsic, and a guarded take of a member over a raw pointer |
| 4 | `DEF-421`, `423`, `424`, `436` | Store lowering and order: the `Deref`-rooted place, whole-element target pinning, RHS-before-clear with the constant-index self-copy, and the shared materializer on the element/raw-target RHS |
| 5 | `DEF-410`, `425`, `426` | Per-instance generic judgments: a conditional move of a by-value parameter, an element move through a borrowed parameter, and the generic compound division guard |
| 6 | `DEF-445`, `446`, `448` | Literal destinations: the conditional-of-literals read base, the take conditional literal in an eager ternary arm, and the take of a conditional array literal of places |
| 7 | `DEF-427`, `447` | Control resolution: two false positives closed with green controls and probe evidence; no code change |

### Key Decisions

| Decision | Outcome | AR |
|----------|---------|-----|
| Batch scope | The 23 rows named by the dispatch: 9 criticals + 14 majors; parked majors and the rest of the backlog stay out | AR-1 |
| Probe status | 21 red; `DEF-427` and `DEF-447` not reproducible and closed as false positives | AR-3, AR-27, AR-28 |
| Phase grouping | Seven machinery phases: P1 (4 rows), P2 (3), P3 (4), P4 (4), P5 (3), P6 (3), P7 controls (2); execution is severity-first (P1–P3 carry all nine criticals; P4–P6 are the majors; P7 is fix-free) | AR-4 |
| Handling mode | Red → green spec cases per row; two control resolutions with no code change | AR-5 |
| `DEF-428` | Preserve element loans on an exact whole-binding self-assignment | AR-6 |
| `DEF-429` | Union the composite source positions at an element/projected destination | AR-7 |
| `DEF-430` | Include the runtime store's binding-union loans in a constant-read copy | AR-8 |
| `DEF-431` | Add the collector's prefix fallback to `move_element_loans` | AR-9 |
| `DEF-432` | Record the provenance owners (including a temp) of a non-place element | AR-10 |
| `DEF-433` | Make the cross-frame fallback consult binding-loan owners | AR-11 |
| `DEF-435` | Route the element-store value through the composite-aware collector | AR-12 |
| `DEF-437` | Pin a `Deref` anywhere in the projection chain before the move | AR-13 |
| `DEF-439` | Consult `value_move_outs` in `arm_consumes_target` | AR-14 |
| `DEF-440` | Return the `Deref` unchanged for a borrowing intrinsic | AR-15 |
| `DEF-441` | Record the pointee move-out for a projection rooted at a `Deref` | AR-16 |
| `DEF-421` | Emit the `Deref`-rooted member store; the frozen place list accepts it | AR-17 |
| `DEF-423` | Pin the whole-element base/index before the RHS | AR-18 |
| `DEF-424` | Evaluate the self-read before the clear; skip the identical constant-index self-copy | AR-19 |
| `DEF-436` | Run the shared materializer on the element/raw-target RHS | AR-20 |
| `DEF-410` | Report one `E10034` per owning instance (concrete-twin parity) | AR-21 |
| `DEF-425` | Report one `E10034` per owning instance through the borrowed parameter | AR-22 |
| `DEF-426` | Feed the resolved per-instance operand type into the division guard | AR-23 |
| `DEF-445` | Recurse into conditional arms in the read/store predicates | AR-24 |
| `DEF-446` | Route the ternary-arm literal through the guarded literal store | AR-25 |
| `DEF-448` | Route the conditional array literal through the element-wise store form | AR-26 |
| Test placement | Additions to the origin suites; no new suite pair; only `_impl_test.rs` byte pins may be re-pinned | AR-30 |
| Lenses | P1/P2 correctness + semantics + memory; P3/P4/P5/P6 all four; P7 correctness + memory | AR-31 |

## Specialist Agents

| Role | Kind | Use | AR Ref |
| ---- | ---- | --- | ------ |
| `correctness-reviewer` | reviewer (project brief) | Every phase | AR-31 |
| `semantics-reviewer` | reviewer (project brief) | P1–P6 (the loan, place, evaluation-order, and per-instance rules) | AR-31 |
| `memory-safety-auditor` | reviewer (read-only) | Every phase (P1–P6 own ASan/LSan diff cases; P7 re-reads the false-positive evidence) | AR-31 |
| `emission-integrity-auditor` | reviewer (read-only) | P3, P4, P5, P6 (the move-out channel, store shapes, guard text, and literal forms) | AR-31 |

_Detection evidence: the four agents exist under `.opencode/agents/` (committed project briefs)
and authored the batch-M evidence for these rows; no new specialist type is needed._

## Exclusions

- **The parked majors**: `DEF-167` (a call-shaped comparison emits a call) and `DEF-171` (an
  accepted `embed` outside a constant context) stay parked for their user rulings (AR-1).
- **The other majors and minors** stay out; the batch is exactly the 23 rows listed above.
- **Fix-free controls**: `DEF-427` and `DEF-447` are closed as false positives with green control
  cases and probe evidence; no compiler change is planned (AR-27, AR-28). At close-out both move
  to Done with the control evidence.
- Any new diagnostic code, registry row, specification chapter, or golden (AR-33).
- A new test-suite pair: every case has an origin home (AR-30).
- A new place kind, a conditional-base store route, or a new IR move-out channel (AR-17, AR-14).
- The batch-M suites, fixtures, and rulings are the regression oracle and are not re-litigated.

## Related Files

- `compiler/src/own/place.rs` — `mark_moved_if_owning_at` (:541), `note_borrowed_element_move`
  (:629), `recorded_position_frame_owners` (:975), `set_binding_loans` (:1096),
  `replace_element_loans` (:1111), `move_element_loans` (:1192), `record_copy_element_loans`
  (:1236), `collect_copy_element_loans` (:1274), `collect_place_copy_loans` (:1369),
  `provenance_loan_owners` (:1435), `place_target` (:1931)
- `compiler/src/own/statements.rs` — `bind_value` (:793), `prepare_binding_effect` (:816),
  `apply_binding_effect` (:881), `emit_store` (:422), `prepare_store` (:445),
  `emit_prepared_store` (:667)
- `compiler/src/own/expr.rs` — `consume_owned_place` (:48), `conditional_move_sources` (:175),
  `check_assign` (:990, the raw-deref target :1003), the projected-store loan recording
  (:1288-1307), `emit_assigned` (:2280), `indexed_member_target` (:2361), `indexed_target`
  (:2490), `emit_indexed_store` (:2557), `emit_indexed_update` (:2639), `raw_deref_target`
  (:2765)
- `compiler/src/own/lower.rs` — `materialize_owning_deref_at` (:3699),
  `bind_intrinsic_argument` (:4171), `literal_read_needs_stores` (:4839),
  `literal_field_read_needs_stores` (:4849), `array_element_read_needs_stores` (:4864),
  `materialize_owned_argument` (:5238)
- `compiler/src/own/call.rs` — `take_pointee_move_outs` (:804); `compiler/src/own/call_check.rs`
  — `frame_owned_view_owner` (:585)
- `compiler/src/own/instance_check.rs` — `check_not_lend_requirements` (:210),
  `check_element_move_requirements` (:448)
- `compiler/src/codegen/function.rs` — `emit_fixed_array_assign_to` (:1533),
  `emit_fixed_array_conditional_assign` (:1723), `struct_literal_needs_stores` (:2071),
  `arm_consumes_target` (:2400), the `Statement::Move` emission (:659)
- `compiler/src/codegen/guards.rs` — `emit_division_guard` (:1435),
  `division_guard_condition` (:1444)
- `compiler/src/ir.rs` — `Materialize` (`move_outs`, `value_move_outs`), `MoveOutValue`
- Origin suites: `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (P1–P4,
  P6), `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs` (P5: `DEF-410`, `425`),
  `compiler/tests/array_value_integrity_{spec,impl}_test.rs` (P7: `DEF-427`), and the frozen
  spec suites as the regression oracle
- Recorded probe sources under the gitignored `build/batchN-plan/probes/` tree (the plan's
  executor records them there; the planning scratch is `/tmp/opencode/batchN/probes/`)

## Next Steps

| Step | Owner |
|------|-------|
| Run the preflight audit over this plan set | reviewer (preflight) |
| Execute phase 1 (spec cases first, then red, then the fix) | executor |
| Keep `05-findings.md` and `99-execution-plan.md` current after every task | executor |
| Close out with `make verify`, the roadmap sync, and the specialist check | executor |
