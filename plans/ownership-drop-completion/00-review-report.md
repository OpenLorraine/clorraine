# Phase Review Report: Ownership and Drop Model Completion

> **Document**: 00-review-report.md
> **Parent**: [Index](00-index.md)
> **Profile**: strict defaults (no structured `codeops/codeops.json`): independent
> correctness review for every non-trivial executed phase, base lenses
> correctness + maintainability + standards.

## Phase 1 — Array classification, nullability, and element moves

**Reviewed revision**: `7426a11`
**Phase diff**: source changes in `compiler/src/types/mod.rs`,
`compiler/src/types/table.rs`, `compiler/src/types/tests.rs`,
`compiler/src/borrow.rs`, `compiler/src/own/expr.rs`,
`compiler/src/own/lower.rs`, `compiler/tests/ownership_drop_spec_test.rs`.

### Findings and rulings

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-001 | 🟠 MAJOR | Routing `is_view`/`is_elided_view` through `is_slice` removed the fixed-array-return diagnostics (`E10035`/`E10036`), so a program the base revision rejected (`func pair(): String[2]`) is now accepted and emits invalid C. The by-value return lowering was deferred to Phase 2, leaving a window on `main`. | **Accepted — pull the fixed-array-return lowering forward** (user ruling). The `03-04` lowering lands now, as Step 1.4, with its specification tests first. |
| RV-002 | 🟡 MINOR | `is_fixed_array` is vacuously true for an empty extent list while `is_slice` is false; the asymmetry is untested. | Report-only. Addressed with the documented empty-extent assertion in the `types` unit test. |

| RV-006 | 🟠 MAJOR | (re-review) A `return inner();` where `inner` returns a fixed array was accepted and emitted `__cl_ret = inner();` — the destination was not passed and C arrays are not assignable. | **Accepted — fix now** (user ruling): the returned call writes into `__cl_ret` via `call_with_destination`. Fixed. |
| RV-007 | 🟠 MAJOR | (re-review) A discarded fixed-array-returning call `inner();` emitted no destination, failing to compile. | **Accepted — fix now** (user ruling): a statement-local destination is materialized and released. Fixed. |
| RV-008 | 🟡 MINOR | Nested fixed-array return is unlowered (one-dimension `extents.first()`); the plain nested-literal gap is scheduled for Phase 2 (03-05). | Report-only; the Phase 2 nested-literal emission covers the shared root cause. |
| RV-009 | 🟡 MINOR | The owning-array run oracle catches a double free but not a leak; the positive ownership-transfer claim is not observable. | Report-only. |

### Verification asks (confirmed by the reviewer)

| Ask | Result |
|-----|--------|
| The updated `types::table` unit test is an implementation test, not a spec oracle | ✅ `#[cfg(test)]` in `types/table.rs` |
| The slice-duplicability correction matches the specification | ✅ `spec/03-ownership.md:20-21`, AR-22 |
| New unit tests are not tautological | ✅ consumer-level and predicate-level coverage |
| Spec-test expectations derive from `spec/` | ✅ E10020/E10011/lend behavior |
| `make check` re-run independently | ✅ PASS |

### Review evidence

- Verify at review: `make check` — `spec check: OK (27 files, 103 compile-fail tests)`; 324 library unit tests pass; all integration suites pass.
- Re-review after the RV-001 fix: one scoped pass on the fix diff (Step 1.4), per the single-re-review budget.

## Phase 2 — Nested-array literals

**Reviewed revision**: `3095145`
**Phase diff**: `30988fc..3095145` — `compiler/src/codegen/function.rs`
(`emit_array_assign_to`), `compiler/tests/ownership_drop_spec_test.rs`,
`compiler/tests/ownership_drop_impl_test.rs`.

### Findings and rulings

| ID | Severity | Finding | Ruling |
|----|----------|---------|--------|
| RV-010 | 🟠 MAJOR | An array literal with an explicit prefix **and** a fill silently dropped the fill: `emit_array_assign_to` returned as soon as `elements` was non-empty, so indices `elements.len()..extent` were never written. The nested form (`u8[3][2] = [[1,2]; [7,8]]`) changed from a loud invalid-C failure before the phase into valid C with wrong values (exit `0`, not `8`); the 1-D form (`u8[5] = [1,2; 9]`) was a pre-existing silent defect masked by the `0` fill. | **Accepted — fix both forms now** (user ruling): the fill loop runs from `elements.len()` to the extent for the scalar and nested fills. Added the build-and-run case `a_prefix_and_fill_literal_fills_the_remaining_elements` (1-D and nested, exit `17`). |
| F4 | 🔵 OBSERVATION | A move-only nested fill (`String[3][2] = [; [a, b]]`) now compiles and aliases the moved temporaries (currently leaks; would double-free once Phase 3 adds element drop). | Report-only. AR-10 resolved to reject a move-only fill with `E10215`; Phase 4's guard reads `result_element` (the base element), so it covers the nested form. No action now. |
| F3 | 🔵 OBSERVATION | A multi-extent fixed-array return of a **place** copies one extent (`__cl_ret[i] = d[i];`, a C array assignment); a non-literal array element likewise emits array assignment. | Report-only; pre-existing and outside the nested-literal scope (the fixed-array-return component is one-dimension, RV-008). |

### Verification asks (confirmed by the reviewer)

| Ask | Result |
|-----|--------|
| The derived element type matches the flat multi-extent encoding | ✅ `Ty::array(element, extents[1..])`; no off-by-one |
| The recursion consumes one extent per level and terminates | ✅ |
| The 1-D path and the `__cl_ret` out-parameter path are unchanged | ✅ |
| The new tests fail on the pre-change code for the right reason | ✅ `cc` rejects the old `d[0] = { 1, 2 };` |
| `make check` re-run independently | ✅ PASS |

### Review evidence

- Verify at review: `make check` — `spec check: OK (27 files, 103 compile-fail tests)`; all suites pass.
- One scoped re-review follows on the RV-010 fix diff, per the single-re-review budget.
