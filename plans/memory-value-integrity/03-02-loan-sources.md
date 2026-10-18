# Loan Sources Through Runtime, Temporary, and Call Reads: Memory and Value Integrity

> **Document**: 03-02-loan-sources.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-430` (critical), `DEF-432` (critical), `DEF-433` (critical)
> **Phase**: P2 · **Lenses**: correctness + semantics + memory-safety (AR-31)

## Overview

Three accepted copies lose the loan of the source they read, through three different source
shapes: a runtime-indexed store read back at a constant index, a temporary-derived element in a
view-array literal, and a call-mediated element-view result. Each fix teaches an existing loan
lookup one more source: the runtime store's wildcard position, the provenance owners of a
non-place value (including a lowered temporary), and the binding-loan owners of a call result
(including the direct call form). The binding-loan route covers the non-place copy and direct
forms pinned by ST-7. A call argument that is a **place** carrying a materialized inline temporary
array — the lowering binds the array to a generated owner the source-level value walk cannot name
— is the filed `DEF-451` residual; this phase does not change the lowering. No new rejection
surface: the copies stay accepted; the escape reports the frozen `E10030` (AR-8–AR-11).

## Frozen Rule

- Ch 16 §2 (`spec/16-borrow-rules.md:51-58`): a view binding loans the owner's storage; the loan
  follows every alias of the view.
- Ch 16 BR-1 (`:187`) and §6 (`:233-236`): a use reachable only through the owner's release is
  `E10030`.
- Ch 16 BR-4 (`:190`): a move of the owner requires no live loan.
- The copy itself is legal; the escape reports.

## Root Cause

1. **Runtime store, constant read (`DEF-430`).** A runtime-index element store cannot name the
   element it overwrote, so `replace_element_loans` (`own/place.rs:1111`) keeps the union and only
   adds (`:1119-1127`). That union is not an element position; the later constant-read copy goes
   through `move_element_loans` (`:1192`) / `collect_place_copy_loans` (`:1369`), which translate
   only the **position** records below the read, so the copy loses the runtime write. A naive merge
   of the whole binding loan set would also reject the accepted sibling `ys[0] = a.v; let s =
   ys[1]; fill(s)`, so the runtime store must record a wildcard **`Element(None)`** position and
   the read must match only under the runtime-written path. The non-constant branch of
   `replace_element_loans` records the wildcard in `element_loans` **and** keeps the binding union
   for `E10031`; a recorded `Element(None)` step matches any `Element(_)` step at the same position
   prefix in `collect_place_copy_loans`, `move_element_loans`, and
   `place_recorded_position_owners` (`recorded_position_frame_owners` uses the same match). The
   direct element lend is a separate path and is recorded as a blast-radius probe (AR-8).
2. **Temporary-derived element (`DEF-432`).** `[mk().v; "EE"]` stores a view of the temporary
   `mk()`'s member. `collect_copy_element_loans` (`own/place.rs:1274`) reaches
   `collect_place_copy_loans` (`:1369`); because `mk().v` is not a place, the non-place branch
   (`:1421-1428`) records only `view_loans(value)` and never the provenance owners, so the
   temporary owner is lost. The collector receives only `&Expr`; the lowered value (and its
   `view_owners`/`lowered_root_bindings`, `:1569`/`:1603`) must be threaded from
   `check_let`/`check_assign` so the statement temp (`__cl_owned0` and friends) is nameable.
3. **Call-mediated element view (`DEF-433`).** `pick(xs): str` returns a view of `xs[0]`'s loaned
   storage. The loan is in the binding loans of `s` (or in the call temporary's loans).
   `frame_owned_view_owner` (`own/call_check.rs:585`) first asks
   `recorded_position_frame_owners` (`own/place.rs:975`), which reads only `element_loans` and
   returns `None` for a call result; the fallback then walks the value's provenance, which names
   no owner for a fresh call result. The direct call form has no binding root at all, so the
   binding loans are never consulted; `view_loans(expr)` + `loan_owners` with the frame filter is
   the missing route.

## Fix Approach

1. **`DEF-430`:** record a wildcard runtime position (`Element(None)`) in `element_loans` when a
   runtime store cannot name its element, and match a constant read only when it falls under the
   runtime-written path; do **not** merge the whole binding union onto every read. The wildcard is
   a recorded position: a recorded `Element(None)` step matches any `Element(_)` step at the same
   prefix in `collect_place_copy_loans`, `move_element_loans`, and
   `place_recorded_position_owners` (`recorded_position_frame_owners` shares the match); a read
   whose own path is `Element(None)`/`ElementTemp` keeps the whole-read union.
   `replace_element_loans` records the wildcard path with the union it writes to the binding and
   keeps that binding union for `E10031`. The `k = 0` variant keeps the conservative report (the
   compiler cannot know), and the constant sibling `ys[0] = a.v; let s = ys[1]` stays accepted and
   clean. Expected: the copy's fill reports one `E10030`. Blast radius: if the direct
   runtime-index fill starts rejecting, record the intended delta; if it stays accepted, file a
   residual row (AR-35(e)).
2. **`DEF-432`:** in the non-place branch of `collect_place_copy_loans`, record the lowered
   value's owners (`view_owners` / `lowered_root_bindings`, including a `LoanOwner::Temp`) under
   `base`, as the place branch already does. Thread the lowered value (or its owner set) from
   `check_let`/`check_assign` through `record_copy_element_loans`; the existing per-position drop
   and the statement-scope release still apply. The `__cl_owned0`/temp owner is the impl pin.
3. **`DEF-433`:** in `frame_owned_view_owner`, for a non-place value consult `view_loans(expr)` and
   expand them with `loan_owners` under the frame filter before the provenance fallback; consult
   the binding-loan owners (`binding_loan_owners` / `view_loan_destination_owners`) before the
   provenance walk. Both the `let` copy and the direct call argument take the same fallback. A
   caller-parameter view must stay accepted. The fallback covers the non-place copy and direct
   forms; a place carrying a materialized inline temporary array
   (`pick([mk().v; "EE"], 0)`) is the filed `DEF-451` residual, with no lowering change in this
   phase.

## Specification-First Cases (ST-5–ST-7)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-5 | `DEF-430` | `xs[k] = a.v` with `k = 1`, then `let s = xs[1]; fill(s)`; the `k = 0` variant; the constant-index store control; the constant sibling `ys[0] = a.v; let s = ys[1]; fill(s)`; the direct runtime fill (`d430c`) | One `E10030` at the copy's fill; the `k = 0` variant keeps the conservative report; the constant-index control keeps its report; the sibling stays accepted and clean; the direct form is recorded per AR-8; no TU. The recorded wildcard `Element(None)` matches the constant read `xs[1]` at the same prefix |
| ST-6 | `DEF-432` | `let ys: str[2] = [mk().v; "EE"]; fill(ys[0])`; the direct `fill(mk().v)` twin; an in-scope read | One `E10030` at the copy's fill; the direct twin keeps its report; the in-scope read runs 0; no TU; the impl pin names the `__cl_owned0`/temp owner |
| ST-7 | `DEF-433` | `let s = pick(xs); fill(s)` and `fill(pick(xs), slot)`; the direct `fill(xs[0])` control; a caller-parameter view; an in-scope result | One `E10030` at each affected fill; the direct control keeps its report; the caller-parameter view stays accepted; the in-scope result runs 0; no TU |

The impl pins assert the wildcard runtime position (not a whole-binding union), the provenance
owner (the temp, including `__cl_owned0`), the binding-loan fallback (both forms), the report
code/count/span, the sibling/caller-parameter acceptances, and a re-emission determinism pin.

## Verification

- `make check`; the residue suite plain and with `RDC_ASAN=1`.
- Host ASan/LSan for the three red shapes (before the fix) and the accepted controls (after).
- The batch-M `DEF-373` copy matrix and the `E10031` controls stay green.

## Risks and Lenses

- **Memory-safety** owns the source matrix: runtime/constant indexes, temporary owners, call
  results; the ASan report is the oracle where the plain run varies.
- **Semantics** owns the reading: the copy is legal, the escape reports; a fix that rejects the
  copy itself is wrong.
- **Correctness** owns the blast radius: every caller of `collect_place_copy_loans`,
  `replace_element_loans`, `record_element_view_loans`, `place_recorded_position_owners`,
  `move_element_loans`, `recorded_position_frame_owners`, and `frame_owned_view_owner`.
- **Risk:** the wildcard runtime position is conservative; a fix must not report an element the
  store could not have written (the `k = 0` variant is the counterexample at run time, but the
  compiler cannot know; the frozen rule is conservative and the report is correct). The
  constant-store sibling `ys[1]` must stay accepted and clean; a whole-binding union merge is
  wrong. The wildcard is a recorded-position step, not a read: a read path carrying
  `Element(None)`/`ElementTemp` keeps the whole-read union, so the matcher is one-directional.
- **Risk:** the binding-loan fallback must not shadow the element positions (the batch-M
  single-view controls pin the one-report rule), and the direct call form must not reject a
  caller-parameter view.
- **Risk:** a temporary owner recorded on a copy that outlives the statement is the point of the
  rule; the statement-scope release must still fire once (ASan/LSan).
