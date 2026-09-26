# Preflight Report: Ownership and Drop Model Completion (PLAN-11)

> **Status**: ✅ PREFLIGHT PASSED WITH NOTES — 15/15 findings resolved and fixed; iteration-2 bounded in-session re-check passed (a fresh-session full re-scan is recommended before execution)
> **Iteration**: 2 (bounded re-scan after fixes) — 15 resolved, 0 carried forward
> **Audit target**: `plans/ownership-drop-completion/` — 9 files, committed at `a1032e4`
> **Context documents** (read to test the target, not audited): `compiler/src/**`, `spec/**`, `stdlib/std.mem.clo`, `compiler/runtime/shim_host.c`
> **Codebase grounded**: ~30 source files examined; references verified by direct reads, and by emitting C with `target/debug/clorraine` inside the SDK container and compiling it with host `cc -std=c99 -fsyntax-only`
> **Last Updated**: 2026-09-26 14:30

⚠️ **SAME-SESSION REVIEW**: this artifact was created earlier in the current session.
Same-agent bias risk is elevated. Mitigation: one independent `general` subagent
re-verified the three highest-risk claims against the code and the emitted C, and
surfaced several additional findings. A fully blind challenger was not used.

## Codebase Context Summary

**Tech Stack:** Rust workspace (std-only) with `clc` (emit-only) and `clorraine`
(driver); the compiler emits C99 for `host`, `hosted-amiga`, and `freestanding`.
**Architecture:** staged pipeline `lex → parse → load → resolve → sema → own →
ir → codegen`, one module per stage; `spec/` is the frozen oracle.
**Key files examined:** `compiler/src/types/{mod,table}.rs`, `compiler/src/borrow.rs`,
`compiler/src/own/{expr,lower,call,place}.rs`, `compiler/src/sema/{mod,items,expr,tests}.rs`,
`compiler/src/codegen/{drop,types,function,calls,cleanup,program,expr}.rs`,
`compiler/src/codegen.rs`, `compiler/tests/compile_fail_spec_test.rs`,
`spec/08-arrays-strings.md`, `spec/14-diagnostics.md`, `spec/decisions.md`,
`spec/appendix-stdlib.md`, `spec/tests/staged/README.md`, `spec/check.sh`,
`stdlib/std.mem.clo`, `compiler/runtime/shim_host.c`.

**Reference Verification:** the plan's `file:line` claims for the *current* code all
verify (types/mod.rs:56-62; borrow.rs:151-186,195; table.rs:367-385; own/expr.rs:151-164;
own/lower.rs:158-175; codegen/drop.rs:32-104; sema/mod.rs:240,266-289; sema/items.rs:35-44;
spec/08:289; staged/README.md:11; check.sh:92-94,163-166; shim_host.c:119). The verified
gaps are in the plan's *proposed* codegen and spec edits, not in its reading of the code.

### Summary by Dimension

| # | Dimension | Findings | Highest Severity |
|---|-----------|----------|-----------------|
| 1 | Ambiguities | 1 | 🟠 |
| 2 | Implicit Assumptions | 2 | 🔴 |
| 3 | Logical Contradictions | 1 | 🟠 |
| 4 | Completeness Gaps | 4 | 🔴 |
| 5 | Dependency Issues | 0 | — |
| 6 | Feasibility Concerns | 3 | 🔴 |
| 7 | Testability | 2 | 🟠 |
| 8 | Security Blind Spots | 0 | — |
| 9 | Edge Cases | 2 | 🟠 |
| 10 | Scope Creep Indicators | 0 | — |
| 11 | Ordering & Sequencing | 1 | 🟠 |
| 12 | Consistency | 2 | 🟡 |
| 13 | Codebase Alignment | 6 | 🔴 |

### Summary by Severity

| Severity | Count | Status |
|----------|-------|--------|
| 🔴 CRITICAL | 1 | all resolved and fixed |
| 🟠 MAJOR | 6 | all resolved and fixed |
| 🟡 MINOR | 6 | all resolved and fixed |
| 🔵 OBSERVATION | 2 | all resolved and fixed |

---

## 🔴 CRITICAL

### PF-001: A fixed-array return is not representable in the emitted C 🔴 CRITICAL

**Dimension:** Completeness Gaps / Feasibility / Codebase Alignment
**Location:** `01-requirements.md` R3, AC-1; `03-01-array-classification.md` §Code Examples:98-102; `00-index.md` §Usage F1(a); `07-testing-strategy.md` ST-1; `02-current-state.md` Risk 1
**Codebase Evidence:** `c_type` returns the **element** type for a fixed array (`codegen/types.rs:36-42`); function returns use `c_type` (`codegen.rs:401-407` user functions; `function.rs:197-201` lambdas; `program.rs:267-270` closures); `emit_closing_return` has no fixed-array arm and falls to `return 0;` (`cleanup.rs:322-333`). Independently reproduced: a generic `id<u8[2]>` emits `static u8 cl_id__u8(u8 v[2])` and `cc` rejects the array return/assignment.
**The Problem:** After the plan's `is_elided_view` fix, `func pair(): String[2]` type-checks, but C cannot return an array. The emitter would declare `static ClString cl_..._pair(void)` and return an array literal, which does not compile. R3 and ST-1 (a required acceptance test in Phase 2.3.1) cannot pass. The plan treats R3 as a classification fix; its own risk table only says codegen work "if needed", and 99-execution-plan.md assigns no codegen task or design. Fixing it is a material codegen/ABI change (out-parameter or generated wrapper), which interacts with AR-15 ("no ABI change").

**Options:**

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| A | Design + task a by-value fixed-array return (hidden out-parameter or generated struct wrapper) | satisfies the frozen `spec/08 §6` and F2 | large; touches `c_type`/`declarator`/signatures/calls/cleanup; expands the plan materially |
| B | Descope R3/ST-1: keep `E10036` for fixed-array returns in v1 and record a named deferral | keeps PLAN-11 bounded | contradicts the frozen spec example; needs an explicit spec ruling and a new plan |

**Recommendation:** Option A — the frozen spec (`spec/08-arrays-strings.md:158-171`, F2) requires a by-value fixed-array return, so it is a real implementation gap, not an optional addition. If you judge it too large for PLAN-11, Option B is valid only with an explicit spec ruling and a named deferral (owner + revisit trigger).

**User Decision:** Resolved — User accepted recommendation: Option A (design + task a by-value fixed-array return).

## 🟠 MAJOR

### PF-002: Multi-dimensional fixed-array drop uses a flat product index 🟠 MAJOR

**Dimension:** Feasibility / Codebase Alignment
**Location:** `03-02-array-drop-glue.md` §Fixed Array:47-55; `07` ST-9; `99` 2.3.1
**Codebase Evidence:** `declarator` emits `ClString grid[2][2]` for `String[2][2]` (`codegen/types.rs:109-117`); confirmed by emitting `u8 d[3][2]` → `u8 d[3][2]` (`target/probe_nested.c:544`). The plan emits `arr[i]` for `i in 0..product(extents)`.
**The Problem:** `grid[0]` is a row (`ClString[2]`), not an element, so `FreeString(grid[0])`/`grid[0] = NULL` are type errors, and `grid[2]`/`grid[3]` are out of bounds. Reproduced by compiling the plan's shape: `cc` errors "assignment to expression with array type" and "incompatible pointer type". ST-9 (`String[2][2]`) would fail.
**Options:** (A) recurse per extent (nested loops/recursive emission) so each dimension indexes correctly; (B) flatten with correct pointer arithmetic on a cast element pointer. **Recommendation:** Option A — it matches existing per-dimension semantics and needs no cast.

**User Decision:** Resolved — User accepted recommendation: Option A (recurse per extent).

### PF-003: ST-9 depends on nested-array-literal codegen that is already broken 🟠 MAJOR

**Dimension:** Completeness / Feasibility / Testability
**Location:** `07` ST-9; `03-02` §Code Examples:145-149; `99` 2.3.1
**Codebase Evidence:** `emit_array_assign` emits `target[index] = <value>` (`function.rs:521-549`) and a nested literal's value is `{ ... }` (`codegen/expr.rs:556-565`). Reproduced: `u8 d[3][2] = [[1,2],[3,4],[5,6]]` emits `d[0] = { 1u, 2u }; ...` and `cc -std=c99 -fsyntax-only` fails "expected expression before '{'". The frozen `spec/08:58` shows the same form.
**The Problem:** The plan's R4 only adds `consume_place` to array-literal elements; no task touches `emit_array_assign`/`array_literal`. ST-9 stays uncompilable C. Because a multi-dimensional `String[2][2]` can only be constructed with a nested literal (a move-only fill is rejected by R7), PF-002's runtime test is blocked too.
**Options:** (A) add a task to emit a compound literal (`(T[N]){ ... }`) for a nested array-literal value and add a build-and-run test; (B) restructure ST-9 to a construction the emitter already supports and record the nested-literal defect as a separate out-of-scope issue. **Recommendation:** Option A is needed if multi-dim drop (R5/AR-11) is to be runtime-tested; otherwise Option B leaves R5 untested for N>1.

**User Decision:** Resolved — User accepted recommendation: Option A (fix nested-literal emission and add a build-and-run test).

### PF-004: `is_nullable_capable` still allows `T[N]?`, emitting invalid C 🟠 MAJOR

**Dimension:** Completeness / Edge Cases / Codebase Alignment
**Location:** `01-requirements.md` R1/R2; `03-01` §Consumer Changes:59-65 (the consumer table omits nullability)
**Codebase Evidence:** `is_nullable_capable` matches `TyKind::Array { .. }` for both shapes (`borrow.rs:203-214`); `own/place.rs:530`, `own/program.rs:274`. Reproduced: `let a: String[2]? = null;` is accepted and emits `a = NULL;` on a C array, which `cc` rejects.
**The Problem:** Once a fixed array is an owned aggregate (not a view), `?` is not meaningful for it; the plan's "one predicate used by every consumer" misses `is_nullable_capable`, so a nullable fixed array still type-checks and emits invalid C. (Partly pre-existing, but the plan asserts the shape is handled everywhere.)
**Options:** (A) route `is_nullable_capable` through `is_slice` (fixed arrays not nullable; slices nullable); (B) leave it and record as a known pre-existing issue. **Recommendation:** Option A — it is a one-line routing change consistent with R1/R2 and removes an invalid-C path the plan claims to close.

**User Decision:** Resolved — User accepted recommendation: Option A (route `is_nullable_capable` through `is_slice`).

### PF-005: Removing the `E90000` trigger breaks an existing unit test 🟠 MAJOR

**Dimension:** Ordering & Sequencing / Completeness
**Location:** `03-02` §Removing the `E90000` Trigger; `99` 2.2.3 and 4.1.3
**Codebase Evidence:** `compiler/src/sema/tests.rs:541-564` `the_array_element_rule_fires_only_for_a_known_move_only_element` asserts `&["E90000", "E90000"]`.
**The Problem:** `Sema::check_sequence_element` is the only producer of `E90000`; deleting it (task 2.2.3) makes this unit test assert two errors and find none, so `make check` goes red immediately, and the plan lists no task to update or remove the test.
**Options:** (A) add a task in Phase 2.2.4 to update the test (remove it, or invert it to assert that `Array<move-only>` is accepted); (B) leave the code path in place. **Recommendation:** Option A — the un-stage (AR-2/AR-3) requires deleting the trigger, so the test must move with it.

**User Decision:** Resolved — User accepted recommendation: Option A (add a task to update the `sema/tests.rs` unit test).

### PF-006: `is_lend_candidate` change regresses a view return from a fixed-array parameter 🟠 MAJOR

**Dimension:** Logical Contradictions / Codebase Alignment
**Location:** `01-requirements.md` R2; `03-01` §Consumer Changes:63; `00-index.md` key decisions
**Codebase Evidence:** `is_lend_candidate` currently includes `TyKind::Array { .. }` (`borrow.rs:175-186`); a function `head(arr: u8[2]): u8[] { return slice(arr, 0, 1); }` compiles today (reproduced: `target/probe_elide.clo` emits `target/probe_elide.c`). `slice` on a fixed array is explicitly supported (`codegen/calls.rs:53-67`).
**The Problem:** A fixed-array parameter is borrowed storage a returned view may point into, so it is a legitimate elision candidate. The plan removes fixed arrays from `is_lend_candidate`, which drops the candidate count to zero and turns this currently-valid program into `E10036`. The plan claims the classification change "reaches `place.rs`/`statements.rs` without further edits" and has no test for this.
**Options:** (A) keep fixed arrays as `is_lend_candidate` (do not route that predicate through `is_slice`) and add a regression test; (B) remove them and add a test that the program is now `E10036` and a spec rationale. **Recommendation:** Option A — being owned does not make a fixed array non-lendable; only its borrowability matters. Restate the rationale in 03-01.

**User Decision:** Resolved — User accepted recommendation: Option A (keep fixed arrays as `is_lend_candidate`; add a regression test).

### PF-007: Un-staging `Array<move-only>` leaves `set<T>` violating its spec contract 🟠 MAJOR

**Dimension:** Completeness / Edge Cases / Codebase Alignment
**Location:** `01-requirements.md` R6; `03-02`; `02-current-state.md` Gap 2
**Codebase Evidence:** `spec/appendix-stdlib.md:62` — `xs.set(i, take v)` "replace element `i`, **dropping the old one**"; `stdlib/std.mem.clo:97-102` implements `set` as `one[0] = v;` with no drop of the old element.
**The Problem:** The plan audits only the drop ladder. Once `Array<move-only>` is accepted (R6), `set<String>` becomes legal and overwrites the old element without releasing it → a leaked handle. The spec requires the old element to drop.
**Options:** (A) add a task to make `set` drop the replaced element (or route it through the element drop glue) and test it; (B) record it as a known leak with a named follow-up. **Recommendation:** Option A — R6 makes this path reachable, so the contract must hold; the impact is small and local to `std.mem`.

**User Decision:** Resolved — User accepted recommendation: Option A (make `set` drop the replaced element; add a test).

## 🟡 MINOR

### PF-008: New `E10215` message conflicts with the single registry template 🟡 MINOR

**Dimension:** Consistency
**Location:** `03-03-view-field-enforcement.md` §The Fill Guard and §Specification Text; `00-ambiguity-register.md` AR-20
**Codebase Evidence:** `spec/14-diagnostics.md:99` has one E10215 row with the message `array fill requires a fixed extent`; `compiler/tests/compile_fail_spec_test.rs:182-214` matches rendered messages against that single template.
**The Problem:** The plan emits a second different message (`array fill cannot repeat a move-only element`) for the same code while AR-20/AC-4 require the spec text to stay consistent. The plan does not say whether the registry keeps one message, gains an alternative, or the code reuses the existing text.
**Options:** (A) reuse the existing message for both triggers and document the second trigger in the description; (B) define two accepted message templates in spec/14. **Recommendation:** Option A — the frozen registry has one message per code and the harness expects that.

**User Decision:** Resolved — User accepted recommendation: Option A (reuse the existing E10215 message text).

### PF-009: The new `emits_drop` arm routes fixed-array assignment through a C-invalid scratch local 🟡 MINOR

**Dimension:** Codebase Alignment
**Location:** `03-02` §`emits_drop`:109-116 and §Integration Points:135
**Codebase Evidence:** `emit_assign` emits `declarator(&ty,&save); save = target;` when the RHS reads the target (`function.rs:483-518`); for an array `declarator` yields `ClString __cl_save[2]`, and `__cl_save[2] = target;` is invalid C (reproduced: `target/probe_arrayassign.clo` → `cc` array-assignment errors). `materialize_operand` already avoids assigning fixed arrays in C (`own/lower.rs:491-492,499`).
**The Problem:** The plan adds `emits_drop(fixed array) = emits_drop(element)` "so the assignment scratch local is used", but the scratch local is exactly a C array it cannot assign. Whole fixed-array assignment is not scoped by the plan at all.
**Options:** (A) exclude fixed arrays from the scratch path and scope/reject whole fixed-array assignment; (B) revisit whether the `emits_drop` fixed-array arm is needed at all. **Recommendation:** Option A.

**User Decision:** Resolved — User accepted recommendation: Option A (exclude fixed arrays from the scratch path; scope/reject whole fixed-array assignment).

### PF-010: Hard-coded `__cl_elems` / `__cl_i` bypass `unique_local` 🟡 MINOR

**Dimension:** Codebase Alignment
**Location:** `03-02` §Growable `Array<T>`:86-99
**Codebase Evidence:** `codegen.rs:375-385` provides `unique_local`; `sanitize_identifier` permits `__cl_…` user names (`codegen.rs:531-545`). Sibling drop loops do not collide (each is in its own `if (…) { }` block — refuted as a redeclaration problem), but a user local named `__cl_elems` would be shadowed.
**Recommendation:** Use `self.unique_local("__cl_elems")`/`("__cl_i")` for the loop locals.

**User Decision:** Resolved — User accepted recommendation: use `unique_local` for the loop locals.

### PF-011: "One predicate used by every stage" is overstated 🟡 MINOR

**Dimension:** Consistency / Codebase Alignment
**Location:** `00-index.md`:21-28,41; AR-5 resolution note
**Codebase Evidence:** codegen/ABI still re-decide the shape inline: `render.rs:32`, `calls.rs:57,156-165`, `guards.rs:331-341`, `cleanup.rs:329`, `lower.rs:499,601-606`, `program.rs:840`, `abi.rs:58`.
**The Problem:** Only `borrow.rs`/`table.rs` are routed; the plan claims universality, which hides the sites where the flat/multi-dim assumption is re-made.
**Recommendation:** soften the claim and note (or route) the remaining sites.

**User Decision:** Resolved — User accepted recommendation: soften the claim and note the remaining sites.

### PF-012: Multi-dimensional runtime bounds guard uses a flat product (pre-existing) 🟡 MINOR

**Dimension:** Edge Cases
**Location:** `03-02`; `07` ST-9
**Codebase Evidence:** the runtime bounds guard flattens `grid[i][0]` to `i >= 4` (`codegen/guards.rs:331-337`), while the constant check and C use row semantics (`grid[2]` → `E10216`, reproduced). Same flat-index misunderstanding as PF-002.
**Recommendation:** record as a known pre-existing issue (or fix), so multi-dim tests are not surprised by it.

**User Decision:** Resolved — User accepted recommendation: record as a known pre-existing issue and fix it in this plan.

### PF-013: The Phase 3.3.1 multi-dimensional fill edge test has the wrong premise 🟡 MINOR

**Dimension:** Testability
**Location:** `99` 3.3.1 ("a multi-dimensional fill"); `07`
**Codebase Evidence:** for `String[2][2] = [; a]` the context element is `String[2]`, so `E10200` fires before the new `E10215` guard can; for `u8[2][2] = [; 0]` likewise `E10200` (reproduced: `probe_fillgrid.clo`, `probe_filled.clo`).
**Recommendation:** replace the edge test with a 1-D move-only fill (the case the guard actually covers).

**User Decision:** Resolved — User accepted recommendation: replace the edge test with a 1-D move-only fill.

## 🔵 OBSERVATION

### PF-014: `emits_drop` location is mis-attributed 🔵 OBSERVATION

`03-02` §Integration Points:135 says `emits_drop` gates a scratch local in `codegen/cleanup.rs`; it lives in `codegen/drop.rs:70-95` and gates `emit_assign` in `codegen/function.rs:486`.
**Recommendation:** correct the reference.

**User Decision:** Resolved — User accepted recommendation: correct the reference.

### PF-015: ST-1/ST-9 are specification behaviors placed as implementation tests 🔵 OBSERVATION

`07-testing-strategy.md:79` assigns ST-1/ST-9 (spec-derived drop behavior) to `ownership_drop_impl_test.rs`. The CodeOps split expects spec oracles for spec-derived behavior.
**Recommendation:** place spec-derived cases in the `*_spec_test.rs` oracle, keeping only runtime oracle scaffolding in `*_impl_test.rs`.

**User Decision:** Resolved — User accepted recommendation: place spec-derived cases in the spec oracle.

---

## Verified correct (no finding)

- `Ty::is_slice`/`is_fixed_array` matches the representation and the established convention (`types/mod.rs:56-62`; consistent with `calls.rs:156`, `render.rs:32`, `cleanup.rs:329`).
- Slice duplicability (AR-22) grounded in `spec/03-ownership.md:21`; recursion at `table.rs:382`.
- Growable drop order (elements then buffer) matches `spec/appendix-stdlib.md:66-67` / O2; `cl_buffer_ptr` exists (`shim_host.c:119`).
- `is_handle_or_aggregate` adding fixed arrays reaches the X4 check (`own/call_check.rs:429`).
- The `E10205` slice change is consistent with `spec/08:289`; the `cstring` field exception is handled correctly.
- `push<T>` for move-only `T` is sound (generic `take` parameters are not auto-dropped).

---

## Iteration 2 (bounded re-scan, in-session)

All 15 findings were resolved by the user (bulk acceptance) and the plan documents
were edited. Fix map:

| Finding | Fix location |
|---------|--------------|
| PF-001 | new `03-04-fixed-array-return.md`; `01` R3, `02` Gap 5, `99` Phase 2 |
| PF-002 | `03-02` §Fixed Array (per-dimension recursion) |
| PF-003 | new `03-05-nested-array-literals.md`; `99` Phase 2; `07` ST-9 |
| PF-004 | `01` R2, `03-01` consumer table, `02` Gap 7, `07` ST-16, AR-23 |
| PF-005 | `99` 3.2.5, `03-02`, `02` risk |
| PF-006 | `01` R2, `03-01` consumer table, `07` ST-15, AR-24 |
| PF-007 | `01` R10, `03-02` §The replaced element in `set<T>`, `07` ST-17, `99` 3.2.3, AR-25 |
| PF-008 | `03-03` fill guard + §Specification Text |
| PF-009 | `03-02` §`emits_drop` (no fixed-array arm); `02` known issues; AR-28 |
| PF-010 | `03-02` (unique_local) |
| PF-011 | `00-index`, `03-01` §Proposed Changes |
| PF-012 | `02` §Known Pre-existing Issues |
| PF-013 | `99` 4.3.1, `07` |
| PF-014 | `03-02` §Integration Points |
| PF-015 | `07` §Implementation Tests note |

Register extended with AR-23…AR-29 (29 total). Execution plan revised to 5
phases / 45 tasks with the new work ordered ahead of its dependents
(fixed-array return and nested literals before the multi-dimensional drop test).

Bounded re-scan result: each fix is present and consistent with its section; no
new contradiction or broken reference was found in the changed sections; the
register row count is 29 and the phase task counts sum to 45. Residual notes:

- Iteration 2 was run in the same session that authored the fixes, so it carries
  same-agent bias. A fresh-session full 13-dimension re-scan is recommended
  before `exec-plan`.
- The plan now cites both `AR-*` and `PF-*` identifiers; the `AR-*` register is
  the normative decision log and the `PF-*` report is the review trail.

**Verdict:** ✅ PASSED WITH NOTES — no unresolved critical/major finding; all
minor and observation findings were explicitly accepted and applied.
