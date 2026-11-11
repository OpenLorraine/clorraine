# Findings and Evidence: Template Placeholder Residuals

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-11 22:02
> **Status**: Phase 1 implemented; phase review and Phase 2 follow

## 1. Phase 1 — Type Diagnostics

### 1.1 Red Evidence (phase-start `b6f12d8`)

The new suite (`compiler/tests/template_placeholder_residuals_spec_test.rs`)
ran against the phase-start compiler. `cargo test -p clc --test
template_placeholder_residuals_spec_test`:

- 7 red failures: `st1_null_placeholder_is_rejected`,
  `st2_module_null_placeholder_is_rejected`,
  `st5_mixed_conditional_placeholder_is_rejected`,
  `st6_mixed_conditional_store_is_rejected`,
  `st9_mixed_view_conditional_is_rejected`,
  `st25_generic_null_body_is_rejected_at_the_declaration`,
  `st26_null_arm_with_a_handle_sibling_is_rejected`.
- 7 behavior guards passed: `st3`, `st4`, `st7`, `st8`, `st10`, `st24`,
  `st28`.
- The base accepted the red programs (ST-5/6/9/26 crash when the null arm is
  selected; ST-1/2/25 report the internal `cl_text_unknown` C failure).

An early ST-2 draft printed the module view, which reports E10038 (borrowing a
global); the test was reduced to the template initializer so the E10500 is the
only judged error. This is a test-oracle correction, not an implementation
change.

### 1.2 Implementation

- `compiler/src/sema/expr.rs::check_template`: a null-shaped placeholder
  reports E10500 (`type 'null' is not formattable`) with the frozen help; the
  opaque gate still excludes null-shaped types.
- `compiler/src/types/mod.rs::conditional_result`: a `settle_conditional`
  helper carries the nullable flag of each non-null-shaped arm, and a
  null-shaped arm makes a handle/`Box` result nullable (`String?`); a view/
  `raw`/`cstring`/slice result keeps the empty state; a still-open parameter
  defers to the instance duty.
- `compiler/src/sema/ops.rs`: unchanged; the written-expected
  `report_null_conditional_arm` path keeps its E10012 rule.
- The first draft added an own-stage skip of null-shaped arms and a
  type-stage sibling E10012; both were removed after the preflight/grounding
  pass found that they would diverge from #26 ST-37 and the
  `def_semantic_parity` inferred-binding case. The final rule types the
  conditional as `String?` and lets the existing E10010 use rule report.

### 1.3 Green Evidence

`cargo test -p clc --test template_placeholder_residuals_spec_test`:
16 passed.

`cargo test -p clc --test template_placeholder_residuals_impl_test`:
2 passed (IT-1 null E10500 surface, IT-2 mixed E10010 and null-arm E10010).

`cargo test -p clc --test def_semantic_parity_integrity_spec_test`:
18 passed, including `should_judge_the_null_arm_contexts` (the inferred
`let y = c ? h : null;` binding narrows and runs).

`cargo test -p clc --test checker_conformance_impl_test`:
118 passed, including the opaque conditional null arm (one E10012).

`template_placeholder_integrity_spec_test`: 41 passed (including ST-34 and
ST-37). `def_zero_null_spec_test`: 44 passed. `make check` green
(fuzz-smoke 28/0).

### 1.4 Phase Review

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | no 🔴/🟠; 4 🟡 | RV-001/002/003/004 fixed; RV-005 bookkeeping |
| Semantics | 1 🟠, 1 🔴 (fix review), 2 🟡 | SR-001 fixed; SR-004 fixed; SR-002/SR-003 doc fixes |

**SR-001 (MAJOR) — a narrowed conditional operand was rejected.** After the
conditional type became nullable, an index base or `len` operand
(`(c ? a : y)[0]`, `len(c ? a : y)`) went through the direct
`check_nonnull_expr` path, which reported the whole conditional instead of
judging its arms. The direct path now judges a conditional's nullable and bare
`cstring` arms, matching `check_value_into`; ST-29 pins `97` and `1`.

**SR-004 (CRITICAL, fix review) — a bare cstring arm was skipped.** The first
arm-aware version judged only `nullable` arms, so a non-nullable `cstring` arm
(a bare cstring is empty-capable) passed and `len` could read NULL. The branch
now includes `TyKind::CString` arms; ST-30 pins E10010 at the bare cstring.
Both arm orders, `len`, index, and dot-`len` were probed; no C is emitted. The
cstring fix re-check is clean, and the both-bare-cstring conditional still
reports once at the conditional.

**RV-005 — bookkeeping.** 1.3.2–1.3.5 and the progress counter are updated.

**RV-001 — impl spans.** IT-1/IT-2 now assert the full columns (`4:16`,
`8:27`, `6:23`).

**RV-002 — stale E10012 docs.** The spec/impl module docs, 01 R8/scope,
00-index, 02, and 03-01 now say the handle null arm is `String?` with E10010;
the annotated non-null destination keeps E10012.

**RV-003 — unused test helpers.** The unused `assert_trap`/`assert_leak_free`
helpers and the unused `tag` parameter were removed; Phase 2 adds the leak
helper when it has a caller.

**RV-004 — bookkeeping.** 1.3.2 is complete; 1.3.5 closes after this review.

**SR-002 — stale AR-2 sentence.** AR-2 now says a null-shaped arm sets the flag
only for a handle/`Box` result.

**SR-003 — invariant 8.** It now says the changes add no new runtime helper or
allocation; an accepted nullable view may render the existing guarded widening.

**Pre-existing (not folded in).** A generic `flag ? null : x` with
`T = str`/`String` still reports E10012+E10010 at the null arm; a scalar
`flag ? null : 5` and `$"{null + 1}"` still emit invalid C. They are recorded
for the `text(null)` follow-up at closeout.

### 1.5 Green after fixes

`template_placeholder_residuals_spec_test` 16/16 (including ST-29/ST-30);
`..._impl_test` 2/2; `def_semantic_parity_integrity_spec_test` 18/18;
`checker_conformance_impl_test` 118/118; #26 41/41; `def_zero_null_spec_test`
44/44; `make check` green.

## 2. Phase 2 — Runtime and Ownership Residuals

_Pending._

## 3. Acceptance

_Pending._
