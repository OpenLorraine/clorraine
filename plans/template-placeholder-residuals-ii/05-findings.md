# Findings and Evidence: Template Placeholder Residuals II

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Created**: 2026-11-12 03:17
> **Status**: Phase 1 implemented and green; Phase 1 review pending

## 1. Phase 1 — Static Diagnoses

### 1.1 Red Evidence (baseline `bb3956c` + PLAN-33 `c5e67cb`)

The Phase 1 cases in `compiler/tests/template_placeholder_residuals_ii_spec_test.rs`
ran against the phase-start compiler (the `text` arity fix is already in the
tree because PLAN-33 executes first). 14 failed:

- `st1_text_null_is_rejected`, `st2_text_struct_is_rejected`,
  `st3_generic_text_struct_is_rejected`: the explicit converter accepted a
  null/non-formattable/opaque argument and emitted a nonexistent converter.
- `st5_nullable_text_is_rejected`: the nullable `text` argument was accepted.
- `st7_opaque_text_projection_is_rejected`: `text(x.n)` emitted
  `cl_text_unknown`.
- `st9_null_plus_one_is_rejected`: accepted and emitted invalid pointer
  arithmetic.
- `st11_generic_null_arm_at_str_runs`,
  `st12_generic_null_arm_at_string_is_e10010_only`,
  `st13_generic_null_arm_scalar_is_e10012_only`: the generic null arm reported
  the wrong E10012+E10010 pair (and the `str` guard was rejected).
- `st14_inferred_scalar_null_arm_is_rejected`,
  `st15_inferred_struct_null_arm_is_rejected`: accepted and emitted
  `(flag ? NULL : ...)`.
- `st28_generic_nullable_text_is_rejected`: the generic nullable `text`
  instance was accepted.
- `st33_null_plus_null_is_rejected`: accepted and emitted `(NULL + NULL)`.
- `st36_nested_value_operand_condition_reports_null` (added by the review fix):
  the base accepted a conditional nested in a value operand of a condition.

The 14 guards passed: `st4`, `st6`, `st8`, `st10`, `st16`, `st17`, `st26`,
`st27`, `st29`, `st30`, `st31`, `st34`, `st35`, `st37`. (`st33` was first written as
`return c`, which reports through the store; it now discards the both-null
value so the base accepts and emits `(NULL + NULL)`.)

### 1.2 Implementation

- `compiler/src/sema/call.rs`: the compiler-known `text` branch reports E10500
  for a null-shaped, concrete non-formattable, or opaque-projection argument;
  the arity fix from PLAN-33 stays. A module-level value named `text` is
  excluded from the branch.
- `compiler/src/own/expr.rs`: a formattable `text` argument goes through
  `check_value_into` with a `str` destination (E10010 unless narrowed); a
  generic argument records the existing deferred placeholder duty.
- `compiler/src/types/mod.rs`: adds `NullArmSettlement`, `settle_null_arm`,
  and `accepts_bare_null` (views/`raw`/`cstring`/slice only) with unit tests.
- `compiler/src/sema/ops.rs`: the binary null rule now covers a literal null
  and a both-null pair, keeps `==`/`!=` with a null-capable or open sibling,
  and leaves `&&`/`||` to E10218. `check_conditional` reports E10012 at a
  null arm when the settled result cannot hold null, except a non-bool
  conditional in a condition context (E10218 owns that report).
  `judge_conditional_argument` skips a site that already reported.
- `compiler/src/own/instance_check.rs`: the instance conversion duty owns the
  E10012 for an inferred generic null arm; the placeholder duty owns the
  E10010 for a nullable settlement. Both use the shared helper.
- `compiler/src/sema/mod.rs`, `stmt.rs`, `expr.rs`: an `in_condition` context
  marks if/while/do/for/`assert`/`!`/`&&`/`||` operands so a non-bool
  conditional keeps the frozen E10218 only.

### 1.3 Green Evidence

- `template_placeholder_residuals_ii_spec_test`: 28 passed.
- `template_placeholder_residuals_ii_impl_test`: 8 passed (IT-1..IT-6,
  IT-14, IT-15; IT-13 is the `types::tests` predicate unit test).
- `template_placeholder_integrity_spec_test`: 41 passed (ST-37 intact).
- `template_placeholder_residuals_spec_test`: 30 passed.
- `def_zero_null_spec_test`: 44 passed.
- `def_semantic_parity_integrity_spec_test`: 18 passed (including the
  context cases st23a–st23g).
- `checker_conformance_impl_test`: 118 passed.
- `make check`: green (fuzz-smoke 28/0).

### 1.4 Phase Review

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | no critical/major after the fix | RV-001/RV-002 fixed; RV-003–RV-005/RV-007 fixed |
| Semantics | 1 major, fixed | SR-001 fixed; three pre-existing shapes recorded |
| Emission integrity | PASS | no unintended diff; 14 host + 42 Amiga TUs strict; `bench-shape` green |

**RV-001 (MAJOR) — a nested value operand inherited the condition
exemption.** `in_condition` spanned the whole condition subtree, so
`if (-(flag ? null : 5) > 0)` and `if (xs[flag ? null : 1] > 0)` skipped the
null report and emitted invalid C. `condition_root` is now one-shot:
`check_expr_inner` takes it, a value operator drops it, and only `&&`/`||`/`!`
give their operands their own condition context. `ST-36` pins the nested
value operand.

**RV-002 (MAJOR) — a module-level `text` value got a spurious E10010.**
`is_builtin_text_call` used only lexical locals. It now also checks the module
global table and imported globals, matching the type stage. `ST-37` pins a
module-level `text: func(String?): String` call.

**RV-003/RV-005/RV-007 — bookkeeping/docs.** The red count is 14 (including
`ST-36`), the guard count is 14, the spec suite is 29; `check_condition_value`
sits below `name_scope_depth`; the test docs no longer reference plan phases.

**RV-004 — one bare-null predicate.** `null_capable_destination` delegates to
`crate::types::accepts_bare_null`.

**SR-001 (MAJOR) — a null binding against an open parameter regressed.** The
param allowance in the comparison carve-out accepted `let z = null; z == x`
with `x: T`; the base rejected it. The allowance now applies only when the
null side is the literal `null` (`x != null` with `x: T` stays legal).
`ST-38` pins the rejected binding form.

**Pre-existing shapes (not caused by this phase; recorded for the closeout
follow-up):**
- An aliased or qualified `text` spelling (`import { text as t }`,
  `std.fmt.text(7)`) bypasses the intrinsic gate and emits an undeclared
  function.
- A generic conditional used directly as a condition is reported E10012 at
  its null arm where the concrete twin reports E10218; the instance
  conditional record carries no condition-context flag.
- `let z = c ? null : null;` stays `Deferred` and is accepted; the frozen
  spec does not decide the both-null conditional.
- A literal `null == x` with `x: T` stays accepted; the binding form is
  rejected by SR-001's rule.

## 2. Phase 2 — Lowering and Runtime Residuals

### 2.1 Red Evidence (Phase 1 head `9199fd7`)

The Phase 2 cases were added before the changes. Seven failed:

- `st18_for_step_owning_assignment_is_leak_free`,
  `st19_for_step_continue_is_leak_free`: ASan reported the old `s` value
  leaked on each iteration (49 bytes in the 3-iteration case).
- `st20_for_init_template_runs_once`,
  `st21_for_init_owning_call_runs_once`, `st32_for_init_increment_runs`: the
  init effect did not run (the counter printed `0`).
- `st24_null_view_panic_is_clean`, `st25_zero_state_string_panic_is_clean`:
  UBSan reported the null-pointer `fwrite` argument.
- `st39_for_step_aggregate_order`: the compact step header emitted the
  declaration-order initializer and printed `12`.

`st22_same_order_aggregate_evaluates_in_order` and
`st23_differing_order_aggregate_uses_stores` were green guards (`12`/`21`).

### 2.2 Implementation

- `compiler/src/own/loops.rs`: `for_step_is_structured` rejects an `Assign`
  whose target owns or may own, or whose literal needs fieldwise stores, so the
  flat loop runs the ordinary pre-drop/store path; `ForInit::Expr` lowers each
  expression through `check_expr_statement` with a release floor and drain.
- `compiler/runtime/shim_host.c`, `shim_hosted_amiga.c`: `cl_panic` skips a
  zero-length message `fwrite`, keeping the host `stderr` and Amiga `stdout`
  streams.
- `compiler/src/codegen/function.rs`, `guards.rs`: the initializer-order
  contract comments state the C99 caveat and the supported-toolchain
  assumption; the same/differing order behavior is pinned by ST-22/ST-23.

### 2.3 Green Evidence

`template_placeholder_residuals_ii_spec_test`: 39 passed;
`..._impl_test`: 14 passed (IT-1–IT-12, IT-14, IT-15).

### 2.4 Phase Review

| Review | Verdict | Findings |
|--------|---------|----------|
| Correctness | no critical/major | RV-001/RV-002 fixed |
| Semantics | no critical/major | SR-001 doc fixed; SR-P1 fixed; SR-P2/SR-P3 pre-existing |
| Memory safety | clean | one test-hygiene minor fixed; two pre-existing shapes recorded |
| Emission integrity | PASS | no unintended diff; 40 host + 68 Amiga + 54 amiga-13 + 12 freestanding TUs strict |

**RV-001 (MINOR) — the IT-7 pre-drop assertion could match the declaration's
drop.** It now searches after the step block label (`__cl_b3:`), so the
assertion pins the step's own release.

**RV-002 (MINOR) — the aggregate contract comments named the wrong test
results.** They now name the same-order `st22` (`12`) and differing-order
`st23` (`21`) functions, and no longer imply that `21` pins the C initializer
order.

**SR-001 (MINOR) — vbcc was described as build-only.** `spec/17-codegen.md`
§13 compiles and runs the vbcc leg on `vamos`; the plan and comments now say
so.

**SR-P1 (pre-existing, fixed in this phase) — a for-step aggregate assignment
reordered its fields.** The compact header rendered a reversed `P { b: ...,
a: ... }` in declaration order (`12`). `for_step_is_structured` now also
rejects a literal that needs fieldwise stores, so the flat store path runs and
prints `21`; `st39` pins it.

**Memory-safety minor (fixed).** The new leak/panic helpers now run the child
with a 30-second bound and set `handle_segv=0` for ASan.

**Pre-existing shapes recorded for the closeout follow-up:**
- `panic(cstring null)` calls `cl_cstring_len(NULL)` before the guarded
  `cl_panic` and segfaults.
- A for-step assignment of a non-owning fixed array from a call or member
  emits an invalid array assignment/call.
- An aliased or qualified `text` spelling and a both-null conditional remain
  the Phase 1 follow-ups.

## 3. Acceptance

_Pending._
