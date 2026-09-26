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
