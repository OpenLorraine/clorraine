# Loan Sources Through Runtime, Temporary, and Call Reads: Memory and Value Integrity

> **Document**: 03-02-loan-sources.md
> **Parent**: [Index](00-index.md)
> **Rows**: `DEF-430` (critical), `DEF-432` (critical), `DEF-433` (critical)
> **Phase**: P2 · **Lenses**: correctness + semantics + memory-safety (AR-31)

## Overview

Three accepted copies lose the loan of the source they read, through three different source
shapes: a runtime-indexed store read back at a constant index, a temporary-derived element in a
view-array literal, and a call-mediated element-view result. Each fix teaches an existing loan
lookup one more source: the runtime store's binding union, the provenance owners of a non-place
value (including a temporary), and the binding-loan owners of a call result. No new rejection
surface: the copies stay accepted; the escape reports the frozen `E10030` (AR-8–AR-11).

## Frozen Rule

- Ch 16 §2 (`spec/16-borrow-rules.md:51-58`): a view binding loans the owner's storage; the loan
  follows every alias of the view.
- Ch 16 BR-1 (`:187`) and §6 (`:233`): a use reachable only through the owner's release is
  `E10030`.
- Ch 16 BR-4 (`:190`): a move of the owner requires no live loan.
- The copy itself is legal; the escape reports.

## Root Cause

1. **Runtime store, constant read (`DEF-430`).** A runtime-index element store cannot name the
   element it overwrote, so `replace_element_loans` (`own/place.rs:1111`) keeps the union and only
   adds. The later constant-read copy goes through `move_element_loans` (`:1192`) /
   `collect_place_copy_loans` (`:1369`), which translate only the **position** records below the
   read; the runtime union is not in that path, so the copy loses it. The direct element lend is a
   separate path and is recorded as a blast-radius probe (AR-8).
2. **Temporary-derived element (`DEF-432`).** `[mk().v; "EE"]` stores a view of the temporary
   `mk()`'s member. `collect_copy_element_loans` (`own/place.rs:1274`) reaches
   `collect_place_copy_loans` (`:1369`); because `mk().v` is not a place, the non-place branch
   (`:1424-1432`) records only `view_loans(value)` and never calls
   `provenance_loan_owners` (`:1435`), so the temporary owner is lost.
3. **Call-mediated element view (`DEF-433`).** `pick(xs): str` returns a view of `xs[0]`'s loaned
   storage. The loan is in the binding loans of `s` (or in the call temporary's loans).
   `frame_owned_view_owner` (`own/call_check.rs:585`) first asks
   `recorded_position_frame_owners` (`own/place.rs:975`), which reads only `element_loans` and
   returns `None` for a call result; the fallback then walks the value's provenance, which names
   no owner for a fresh call result. The binding-loan owners are never consulted.

## Fix Approach

1. **`DEF-430`:** when the source binding has a runtime-index record (the union), include its
   loans in the constant-read transfer: in `collect_place_copy_loans` / `move_element_loans`,
   merge the runtime record at the destination's `base` path. Do not turn the runtime store into a
   per-element record (the index is unknown). Expected: the copy's fill reports one `E10030`.
   Blast radius: if the direct runtime-index fill starts rejecting, record the intended delta; if
   it stays accepted, file a residual row (AR-35(e)).
2. **`DEF-432`:** in the non-place branch of `collect_place_copy_loans`, record
   `provenance_loan_owners(value)` (or the lowered temporary's owner) under `base`, as the place
   branch already does. The temporary owner is a `LoanOwner::Temp`, so the existing
   per-position drop and the statement-scope release still apply.
3. **`DEF-433`:** in `frame_owned_view_owner`, consult the binding-loan owners
   (`binding_loan_owners` / `view_loan_destination_owners`) before the provenance fallback:
   a binding with binding loans is authoritative for the storage its views read; only a binding
   with neither element nor binding loans falls through to provenance. Both the `let` copy and the
   direct call argument take the same fallback.

## Specification-First Cases (ST-5–ST-7)

All go to `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (AR-30).

| ST | Row | Case | Expected |
|----|-----|------|----------|
| ST-5 | `DEF-430` | `xs[k] = a.v` with `k = 1`, then `let s = xs[1]; fill(s)`; the `k = 0` variant; the constant-index store control; the direct runtime fill (`d430c`) | One `E10030` at the copy's fill; no TU; the constant-index control keeps its report; the direct form is recorded per AR-8 |
| ST-6 | `DEF-432` | `let ys: str[2] = [mk().v; "EE"]; fill(ys[0])`; the direct `fill(mk().v)` twin; an in-scope read | One `E10030` at the copy's fill; the direct twin keeps its report; the in-scope read runs 0; no TU |
| ST-7 | `DEF-433` | `let s = pick(xs); fill(s)` and `fill(pick(xs), slot)`; the direct `fill(xs[0])` control; an in-scope result | One `E10030` at each affected fill; the direct control keeps its report; the in-scope result runs 0; no TU |

The impl pins assert the union record (the runtime path), the provenance owner (the temp), the
binding-loan fallback, the report code/count/span, and a re-emission determinism pin.

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
  `move_element_loans`, `recorded_position_frame_owners`, and `frame_owned_view_owner`.
- **Risk:** the runtime union is conservative; a fix must not report an element the store could
  not have written (the `k = 0` variant is the counterexample at run time, but the compiler cannot
  know; the frozen rule is conservative and the report is correct).
- **Risk:** the binding-loan fallback must not shadow the element positions (the batch-M
  single-view controls pin the one-report rule).
- **Risk:** a temporary owner recorded on a copy that outlives the statement is the point of the
  rule; the statement-scope release must still fire once (ASan/LSan).
