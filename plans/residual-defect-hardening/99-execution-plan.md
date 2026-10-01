# Execution Plan: Residual Defect Hardening

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-02 00:06
> **Progress**: 38/38 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the four residual filed defects DEF-46 – DEF-49 in four implementation phases (flat array
literal types and the deferred row store, the multi-extent return copy, the loop-condition
owned-call release, substitution-aware ownership) plus a close-out phase, following the
specification-first ordering. Governing designs:
[03-01](03-01-flat-array-literal-types.md), [03-02](03-02-multi-extent-return-copy.md),
[03-03](03-03-loop-condition-owned-call-release.md),
[03-04](03-04-substitution-aware-ownership.md). Specification cases:
[07](07-testing-strategy.md). Decisions: `00-ambiguity-register.md`.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Flat Array Literal Types and the Deferred Row Store | 8 |
| 2 | Multi-Extent Return Copy | 6 |
| 3 | Loop-Condition Owned-Call Release | 7 |
| 4 | Substitution-Aware Ownership | 12 |
| 5 | Close-out | 5 |

**Total: 38 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan skill's quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`. Every implementation task
>    runs `make check` before it is promoted; the close-out runs `make verify` (AR #6).
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Flat Array Literal Types and the Deferred Row Store

> **Phase baseline tree**: `0fd5f8f8959648e0841b3d68337ffd13ee1cc2b3` (recorded 2026-10-01 14:48;
> Phase 1 starts on the plan-set commit)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/src/types/mod.rs`,
> `compiler/src/codegen/guards.rs`, `compiler/src/types/` unit tests,
> `compiler/tests/residual_defect_hardening_{spec,impl}_test.rs`,
> `plans/residual-defect-hardening/*`

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-flat-array-literal-types.md) §Proposed Changes 1–3 · AR #3, #9
**Objective**: Pin the flat literal type and the deferred row store from ST-1 – ST-5.

- [x] 1.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-1 – ST-5 and ST-17 — `compiler/tests/residual_defect_hardening_spec_test.rs` ✅ (completed: 2026-10-01 14:51)
- [x] 1.1.2 Run the specification tests — verify ST-1, ST-2, ST-3, ST-4, and ST-17 fail before implementation (invalid C / type mismatch) and record ST-5 as the pre-passing regression pin (red phase) ✅ (completed: 2026-10-01 14:51)

> **Red-phase record (2026-10-01 14:51):** 8 fail / 3 pass. Failing: ST-1 (then, else, short-circuit — invalid C from the nested `i32 lit[2]` declarator and the brace-assignment row stores), ST-2 (invalid C: `i32 lit[2]` with brace row stores), ST-3 (both arms — `E10200 cannot convert 'i32[2][2]' to 'i32[2][2]'` from the nested-vs-flat mismatch), ST-4 (both arms — invalid C on the owned rows). Passing pre-implementation: the two ST-5 pins (slice-element literal runs; longer row rejected `E10215`) and ST-17. Note: ST-17 was expected red in the task text but already passes — the `let`-form fill with a conditional row is stored correctly by the statement core today, so it stays as a regression pin for the helper routing (documented per the spec-first pre-passing rule).

### Step 1.2: Implementation

**Reference**: [03-01](03-01-flat-array-literal-types.md) §`Ty::array` folding, §`array_row_expr`
**Objective**: Make the no-context literal flat and give every deferred row shape an expression store.

- [x] 1.2.1 Fold a fixed-array element into the extents in `Ty::array` and route `substitute` through it (slice elements and slice outer extents unchanged) — `compiler/src/types/mod.rs` ✅ (completed: 2026-10-01 14:54)
- [x] 1.2.2 Add `array_row_expr` (conditional with a common-typed arm tail, place with the place-aware move zero, call, closure-call, fallback) and route non-literal rows and fills through it — `compiler/src/codegen/guards.rs` ✅ (completed: 2026-10-01 14:54)
- [x] 1.2.3 Run the specification tests — verify ST-1 – ST-5 and ST-17 pass (green phase) ✅ (completed: 2026-10-01 14:54)

> **Green-phase record (2026-10-01 14:54):** `residual_defect_hardening_spec_test` 11/11 pass. Workspace run: 356 passed, 1 known failure — `types::table::tests::duplicability_stops_recursing_at_the_depth_cap` builds its shape through `Ty::array`, which now folds it flat, so the case no longer reaches the recursion cap; task 1.3.2 rebuilds that shape with an explicit `TyKind::Array`.

### Step 1.3: Implementation Tests & Hardening

**Objective**: Pin the emitted shapes and the no-regression edges.

- [x] 1.3.1 Write implementation tests: the flat declarator (`i32 lit[2][2]`), the per-row expressions (mixed-arm conditional, fill form, adjusted-parameter row), and byte-identical re-emission — `compiler/tests/residual_defect_hardening_impl_test.rs` ✅ (completed: 2026-10-01 14:56)
- [x] 1.3.2 Unit tests: a nested fixed element folds; `substitute(T[2], T = i32[3])` is flat; a slice element and a slice outer extent do not; update the depth-cap and nested-array tests to build their shape with an explicit `TyKind::Array` — `compiler/src/types/` test block ✅ (completed: 2026-10-01 14:56)
- [x] 1.3.3 Full verification — `make check` ✅ (completed: 2026-10-01 14:57)

**Deliverables**:
- [x] The no-context literal has the flat fixed-array type and the C declarator carries every extent
- [x] A deferred literal with a conditional, place, or call row compiles to valid C
- [x] ST-1 – ST-5 and ST-17 pass; all verification passing

> **Phase 1 review record (2026-10-01 16:05):** independent review found RV-1 (🔴 critical) and
> RV-2 (🟠 major) plus RV-3/RV-4 (🟡 minor); the user approved the recommended fixes (option
> "Fix both + minors, then re-review"). Fixed in the follow-up commit: **RV-1** — the deferred
> literal no longer walks its elements twice, so a materialized take-call row runs once and
> empties its source once; **RV-2** — a row call expression no longer appends a discarded comma
> value (the conditional arm wrapper supplies the common value), so call rows build under
> `-Werror`; **RV-3** — the unreachable scalar fallback for rows was removed and the helper doc
> aligned; **RV-4** — three specification cases (call row, call arm, take-call row) and two
> emitted-shape count pins added. Repros confirmed before/after: the take-call row segfaulted and
> now exits 7; the call row failed `-Werror=unused-value` and now builds and exits 7.
> `make check`: 36 suites, 0 failures; spec 14/14; impl 9/9.
>
> **Re-review (2026-10-01 16:15):** scoped to the fix diff — **No findings**; RV-1 .. RV-4
> confirmed resolved (each new test fails against the pre-fix compiler; the take-call row runs
> once, call rows build under `-Werror`, no guard is dropped from any element shape, the
> re-materialized temporary drops before the element stores). Phase 1 closed. Non-finding noted
> for separate triage: a fixed-array conditional used as a short-circuited index base is still
> materialized eagerly (pre-existing on the phase base, outside this plan's four defects).

**Verify**: `make check`

---

## Phase 2: Multi-Extent Return Copy

> **Phase baseline tree**: `b61ef344bac725bc4427780f3269ef3efaf6cd49` (recorded 2026-10-01 16:15)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/src/codegen/function.rs`,
> `compiler/src/codegen/cleanup.rs`, `compiler/tests/residual_defect_hardening_{spec,impl}_test.rs`,
> `plans/residual-defect-hardening/*`

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-multi-extent-return-copy.md) §Proposed Changes · AR #12
**Objective**: Pin the multi-extent owned and duplicable return from ST-6 – ST-7.

- [x] 2.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-6 – ST-7 — `compiler/tests/residual_defect_hardening_spec_test.rs` ✅ (completed: 2026-10-01 16:16)
- [x] 2.1.2 Run the specification tests — verify both fail before implementation (invalid C) (red phase) ✅ (completed: 2026-10-01 16:16)

> **Red-phase record (2026-10-01 16:16):** both ST-6 and ST-7 fail on the unmodified compiler at the host C build: `error: assignment to expression with array type` (the per-extent `__cl_ret[0] = a[0];` spelling).

### Step 2.2: Implementation

**Reference**: [03-02](03-02-multi-extent-return-copy.md) §Return arm
**Objective**: Delegate the place branch to the fixed-array store core.

- [x] 2.2.1 Promote `emit_fixed_array_assign_to` to sibling visibility and delegate the return place branch — `compiler/src/codegen/function.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-10-01 16:16)
- [x] 2.2.2 Run the specification tests — verify ST-6 – ST-7 pass (green phase) ✅ (completed: 2026-10-01 16:16)

> **Green-phase record (2026-10-01 16:16):** `residual_defect_hardening_spec_test` 16/16; the returned place now emits one `memcpy(__cl_ret, a, (sizeof(__cl_ret[0]) * 2));` and, for an owned source, a single element-wise `cl_zero` walk.

### Step 2.3: Implementation Tests & Hardening

- [x] 2.3.1 Write implementation tests for the return `memcpy` shape and the source emptying — `compiler/tests/residual_defect_hardening_impl_test.rs` ✅ (completed: 2026-10-01 16:18)
- [x] 2.3.2 Full verification — `make check` ✅ (completed: 2026-10-01 16:18)

> **Full-verification record (2026-10-01 16:18):** `make check` 36 suites, 0 failures. Two pre-existing implementation tests pinned the old per-extent return spelling and were updated to the new byte-copy contract as part of this task (`call_value_integrity_impl_test.rs` returned-parameter case; `runtime_value_integrity_impl_test.rs` conditional-return case); these are implementation-shape tests, not the immutable specification oracle.

**Deliverables**:
- [x] Every extent of a returned fixed-array place is copied and the owned source is emptied once
- [x] ST-6 – ST-7 pass; all verification passing

> **Phase 2 review record (2026-10-01 16:24):** independent review — no 🔴/🟠 findings. One 🟡
> minor (RV-5: stale "element by element" in the return-arm doc) fixed in the follow-up commit.
> The reviewer independently re-ran `make check` (36 suites), the four relevant suites, and
> ad-hoc return probes (non-square extents, struct base, generic, lambda, nested conditional,
> early return, forwarding) — all correct. Non-finding noted for separate triage: a user local
> named `__cl_ret` collides with the hidden ABI parameter (pre-existing on the phase base,
> outside this plan's four defects).

**Verify**: `make check`

---

## Phase 3: Loop-Condition Owned-Call Release

> **Phase baseline tree**: `5d697a6f76b983238b0d03842b3a614abbac454b` (recorded 2026-10-01 16:26)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/src/own/loops.rs`,
> `compiler/src/ir/guards.rs`, `compiler/tests/residual_defect_hardening_{spec,impl}_test.rs`,
> `plans/residual-defect-hardening/*`

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-loop-condition-owned-call-release.md) §Proposed Changes · AR #5, #10
**Objective**: Pin the condition evaluation and leak-freedom from ST-8 – ST-9.

- [x] 3.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-8 – ST-9 — `compiler/tests/residual_defect_hardening_spec_test.rs` ✅ (completed: 2026-10-01 16:27)
- [x] 3.1.2 Run the specification tests and record the pre-fix AddressSanitizer probe of ST-8 (leak-only red phase); the exit codes alone cannot show the failure ✅ (completed: 2026-10-01 16:27)

> **Red-phase record (2026-10-01 16:27):** the three new cases pass pre-implementation (leak-only
> pin): the condition call is printed inline in the header
> (`while ((cl_spec_rdh_st8_grow().count > 0) && (i < 3))`). Pre-fix host ASan probe
> (`clang -fsanitize=address`, `ASAN_OPTIONS=detect_leaks=1`): **96 bytes leaked in 8
> allocations** (three container builds plus their buffers).

### Step 3.2: Implementation

**Reference**: [03-03](03-03-loop-condition-owned-call-release.md) §Proposed Changes 1–2
**Objective**: Materialize owned call results in the three loop conditions and select the guarded path.

- [x] 3.2.1 Add the condition walk that materializes owned call results (`owns_drop`; the substitution-aware phase widens it with `maybe_owns`; skip a call already stored by a `Materialize`) and call it at the three condition lowering points — `compiler/src/own/loops.rs` ✅ (completed: 2026-10-01 16:30)
- [x] 3.2.2 Make `value_needs_guards` report a `Materialize` as needing guards — `compiler/src/ir/guards.rs` ✅ (completed: 2026-10-01 16:30)
- [x] 3.2.3 Run the specification tests — verify ST-8 – ST-9 pass (green phase) and the ST-8 probe is clean ✅ (completed: 2026-10-01 16:30)

> **Green-phase record (2026-10-01 16:30):** the three cases pass; the emitted `while` is
> `while (1)` with the previous-value drop and the re-store in the body, the `do while` takes the
> flag-first `for (;;)` form, and the `for` keeps its step in the header. Post-fix host ASan probe
> of ST-8: exits `3`, **no leak report**.

### Step 3.3: Implementation Tests & Hardening

- [x] 3.3.1 Write implementation tests: the guarded flag-first form for `while`, `do`, and `for`; a non-owning condition keeps the inline spelling; the guarded fixed-array condition emits one temp and one release; a `for` step holding a materialize drops to the flat form — `compiler/tests/residual_defect_hardening_impl_test.rs` ✅ (completed: 2026-10-01 16:30)
- [x] 3.3.2 Full verification — `make check` ✅ (completed: 2026-10-01 16:30)

> **Full-verification record (2026-10-01 16:30):** `make check` 36 suites, 0 failures.

**Deliverables**:
- [x] An owned call result in any loop condition is evaluated per iteration, dropped before a re-store, and released after the loop
- [x] ST-8 – ST-9 pass; the pre-fix leak is recorded; all verification passing (ST-18 lands in Phase 4)

> **Phase 3 review record (2026-10-01 17:00):** independent review found RV-7 (🟠) and RV-8 (🟠)
> plus RV-9/RV-10 (🟡). User ruling: fix RV-8, record the RV-7 limit, defer RV-9, fix RV-10.
> Fixed in the follow-up commit: **RV-8** — `materialize_read_place` runs the owned-call walk
> before its eager store, so a call trapped inside a materialized read gets its own temporary
> (re-store drop + final release); the repro `while ([grow().count][0] > 0 && i < 3)` now exits 3
> with a clean host ASan probe (pre-fix: 96 bytes / 8 allocations). **RV-10** — the fixed-array
> condition test now uses an owned `String[2]` and pins the expression-form re-store drop and the
> single final release. **RV-7** — recorded as a pre-existing limit and filed as roadmap
> `DEF-51`: a drop-needing element (growable `Array<T>` with drop-needing `T`, or a struct
> containing one) has no expression-form drop, so a conditionally evaluated materialized operand
> skips the previous-value drop; consistent with the guard skip already recorded in
> `call-value-integrity`. **RV-9** — deferred to the existing DEF-45 transfer-temporary class; it
> leaks identically outside loops and is not this walk's root. Re-review of the fix diff follows.
>
> **RV-11 fix (2026-10-01 17:44):** the scoped re-review found a critical regression from the RV-8
> fix: an owned conditional array element was copied without moving its chosen arm (the eager walk
> newly wrapped the conditional arms), so the element and the arm released the same storage and
> the program double-freed. Fixed in the element store path by routing an owned conditional
> through the existing per-arm move store (`emit_conditional_move_assign`), mirroring the
> owned-scalar assignment rule; the fill form stays unchanged because a non-duplicable fill value
> is rejected when the literal is checked. Local verification per the user's ruling (no third
> review pass): the statement and loop double-free repros are ASan-clean and compute their
> values; three specification cases and one emitted-shape implementation test added;
> `make check` 36 suites, spec 23/23, impl 19/19.

**Verify**: `make check`

---

## Phase 4: Substitution-Aware Ownership

> **Phase baseline tree**: `15f5ea4c3cd93d6fa8d05a3e7ecd08c94786e273` (recorded 2026-10-01 17:47)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/src/ir/mono.rs`,
> `compiler/src/ir.rs`, `compiler/src/own.rs`,
> `compiler/src/own/{lower,expr,statements,loops,instance_check,call,call_check,place,flow}.rs`,
> `compiler/src/codegen/function.rs`, `compiler/tests/residual_defect_hardening_{spec,impl}_test.rs`,
> `stdlib/std.mem.clo` (user-approved expansion, 2026-10-01), `plans/residual-defect-hardening/*`

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-substitution-aware-ownership.md) §The site table · AR #17, #11
**Objective**: Pin one case per duty kind and per suppression site from ST-10 – ST-16.

- [x] 4.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-10 – ST-16 and ST-18 – ST-22 (ST-14 rewritten for site `own/lower.rs:619`) — `compiler/tests/residual_defect_hardening_spec_test.rs` ✅ (completed: 2026-10-01 17:49)
- [x] 4.1.2 Run the specification tests and record the pre-fix AddressSanitizer probes of ST-10 (the filed take-parameter leak) and ST-19 (the take-argument double-free) plus the ST-12 observation; the exit codes alone cannot show the failure ✅ (completed: 2026-10-01 17:49)

> **Red-phase record (2026-10-01 17:49):** 8 failed / 29 passed. Failing pre-implementation:
> ST-14, ST-15, ST-16, ST-19, and ST-20 emit invalid C (a fixed-array call or an instantiated
> call rendered in a value position); ST-21 double-frees (`(c ? a : b)[i]` with owned rows);
> ST-12 and ST-22 with the `String` instance are accepted but must report `E10020`. Leak-only
> pins (pass pre-implementation, ASan evidence): ST-10 leaks 11 bytes / 1 allocation (the take
> parameter never released); ST-18 leaks 22 bytes / 2 allocations (the loop-condition call never
> released). Pre-passing regression pins: ST-11, ST-12 (`i32`), ST-13, ST-22 (`i32`). Note:
> ST-14 and ST-15 are realized with a `func(): T[2]` fixed-array call because a `T[]` slice is a
> view and cannot carry the owning duty; ST-20 is realized with the numeric index instantiation
> the compound store requires; both keep the case's expectation and site unchanged.

### Step 4.2: Implementation

**Reference**: [03-04](03-04-substitution-aware-ownership.md) §Proposed Changes 1–5
**Objective**: Deposit the five duty kinds and discharge them per instance.

- [x] 4.2.1 Add `maybe_owns` and register maybe-owning bindings and temporaries for drop and release, keeping the `mode == Mode::Take` parameter gate — `compiler/src/own.rs` ✅ (completed: 2026-10-01 20:42)
- [x] 4.2.2 Emit the move shape for param-typed place stores and `take` argument places, add the codegen non-owning branch (plain copy, no zero), and empty a param-typed take place through the argument move-out — `compiler/src/own/{statements,lower,call_check,call}.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-01 20:42)
- [x] 4.2.3 Add the `RequiresDuplicable` record (with the per-function carrier into `Monomorphization`) and the moved-if-owning marker in `PlaceState` with its `read_place` detection, clear-on-write, and join merge — `compiler/src/ir/mono.rs`, `compiler/src/own.rs`, `compiler/src/own/{expr,place,flow}.rs` ✅ (completed: 2026-10-01 20:42)
- [x] 4.2.4 Discharge `RequiresDuplicable` in the per-instance audit (`E10020` once per function and binding) — `compiler/src/own/instance_check.rs` ✅ (completed: 2026-10-01 20:42)
- [x] 4.2.5 Remove the five materialization skips (guard operand, two-use operand, read place, `take`/owned argument, for-in iterable) and widen the loop-condition walk's predicate with `maybe_owns` — `compiler/src/own/lower.rs`, `compiler/src/own/loops.rs` ✅ (completed: 2026-10-01 20:42)
- [x] 4.2.6 Add the loan-owner predicate site for maybe-owning temporaries and run the bounded reachability probe; record the result (an explicit acceptance ruling is required first if reachable) — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-01 20:42)
- [x] 4.2.7 Run the specification tests — verify ST-10 – ST-16 and ST-18 – ST-22 pass (green phase) and the ST-10, ST-18, and ST-19 probes are clean ✅ (completed: 2026-10-01 20:42)

> **Green-phase record (2026-10-01 20:42):** `residual_defect_hardening_spec_test` 37/37. Host ASan
> probes clean: ST-10 exits 7 (pre-fix 11 bytes / 1 allocation leaked), ST-18 exits 3 (pre-fix
> 22 bytes / 2 allocations), ST-19 exits 0 (pre-fix: missing `cl_sink__String` instance, invalid
> C). Four runtime discoveries were ruled on during implementation: **(1)** the maybe-owning
> parameter release double-freed with the stdlib's raw-copy `push<T>`; the user approved expanding
> the modification set with `stdlib/std.mem.clo`, and `push` now reserves through the same
> `cl_buffer_reserve` FFI `reserve` used and writes the element through a checked slice store (the
> previewed `reserve()` call would have needed nested generic instances); **(2)** a generic body
> calling another user generic was never recorded (`cl_sink__String` emitted undeclared); the user
> approved adding transitive instance collection to `compiler/src/own/generics.rs`, which now
> re-walks each recorded instance with its arguments substituted to a fixpoint; **(3)** the ST-14,
> ST-15, and ST-21 programs as written collide with the recorded v1 element-move (`E10026`) and
> copy-element-loop (`E10040`) limits; the user approved realizing them as borrow-based shapes
> that exercise the same sites (`borrow mk()[i]` into a sink; `for (borrow e in mk())`; `take`
> rows with `borrow (c ? a : b)[i]`); **(4)** the loan-owner reachability probe found a
> use-after-free (`let view: T[] = slice(mk(), 0, 1); return view[0];` segfaulted) and the user
> approved shipping the widening, which reports `E10030` for every instance of the generic (the
> recorded acceptance consequence).

### Step 4.3: Implementation Tests & Hardening

- [x] 4.3.1 Write implementation tests: the maybe-owning drop, the non-owning no-op move branch, the emptied `take` place, and `E10020` reported once per function and binding — `compiler/tests/residual_defect_hardening_impl_test.rs` ✅ (completed: 2026-10-01 20:48)
- [x] 4.3.2 Unit tests: `maybe_owns`; the parameter-mode gate; the `RequiresDuplicable` discharge; the loan-owner probe; a scalar instance emits no release — `compiler/src/own/` test blocks ✅ (completed: 2026-10-01 20:48)
- [x] 4.3.3 Full verification — `make check` ✅ (completed: 2026-10-01 20:48)

> **Hardening record (2026-10-01 20:48):** implementation tests pin the per-instance release and
> move shapes (`cl_keep__String` releases, `cl_keep__i32` no-ops; `dup__String` empties,
> `dup__i32` copies; the owning caller empties its take place, the duplicable one keeps it), the
> single `E10020` report, the nested-instance declaration, and the loan-owner `E10030` rejection.
> Unit tests cover `maybe_owns_ty`, the discharge (once for an owning instance, accepted for a
> duplicable one), and the borrow parameter gate. `make check` 36 suites, 0 failures; spec 37/37;
> impl 25/25; lib 364/364. Two substitution gaps surfaced by the full run and were fixed: a
> dot-call result type in a non-generic caller and an indexed element of one are substituted
> before a materialized temporary is declared (`materialized_ty`), so the emitted C never names a
> bare type parameter.

**Deliverables**:
- [x] An owning instance releases and moves correctly (including `take` argument places); a `borrow` parameter is never dropped; a non-owning instance keeps its values and behavior unchanged
- [x] A body that reuses a moved-if-owning source reports `E10020` once per function and binding, at the body's use
- [x] Every site in the 03-04 table is covered by its mapped case or the task 4.2.6 probe
- [x] ST-10 – ST-16 and ST-18 – ST-22 pass; all verification passing

> **Phase 4 review record (2026-10-01 23:35):** independent review found nine 🔴 and two 🟠
> (RV-13 .. RV-23) plus a test gap (RV-24); the user approved fixing RV-13..RV-22 and adding
> tests, and filing RV-23 as a pre-existing residual. Fixed in the follow-up commit:
> projection-aware maybe-moved markers with a borrow-argument use hook (RV-13, RV-16, RV-17,
> RV-20; the duty record now carries its declared type so base-overridden bindings are judged);
> take-argument element place extraction (RV-14); the return of a projected place empties
> through the arm walk (RV-15); the take-conditional gate widens to maybe-owning (RV-18); the
> materializer binds an ownerless maybe-owning non-take argument (RV-21); `unread_locals` ignores
> a drop that no-ops at the instance (RV-22); and the host and hosted-amiga runtime shims
> zero-fill newly reserved buffer capacity, so the stdlib `push` writes into the canonical empty
> state (RV-19; the modification set now includes
> `compiler/runtime/shim_{host,hosted_amiga}.c`). Ten specification cases and one implementation
> case pin the shapes; strict-LSan and `MALLOC_PERTURB_` probes are clean; `make check` 36 suites,
> 0 failures; spec 47/47; impl 26/26.
>
> **RV-23 (filed):** a generic call inside a lifted lambda in a generic body is not re-walked with
> the enclosing instance, so the nested instance is never emitted. Filed as roadmap `DEF-52`
> (pre-existing; likely predates this plan).
>
> **Phase 4 re-review (2026-10-01 23:55):** scoped to the fix diff. RV-25 (the element write
> cleared unrelated consumed paths — a regression from the RV-13 fix), RV-26 (an ancestor use of a
> consumed path was missed), and RV-27 (concrete conditional array arms unmarked) were fixed: the
> element write retains unrelated paths, the matcher also conflicts when the consumed path is a
> descendant of the use, and the concrete array-literal arm marks whole-binding arms while
> projected rows keep their row store. Three pre-existing defects adjacent to the fixed code were
> filed: RV-28 → `DEF-53` (ownerless element take double-frees), RV-29 → `DEF-54` (a generic-struct
> borrow argument uses the unsubstituted type and emits invalid C), RV-30 → `DEF-55` (a returned
> field of a fresh call result leaks its siblings). RV-15/17/18/19/20/21/22 verified correct;
> RV-13/14/16 completed by the three fixes. `make check` 36 suites, 0 failures; spec 50/50;
> impl 26/26.

**Verify**: `make check`

---

## Phase 5: Close-out

> **Phase baseline tree**: `262b5ba9fb73766bebe0036f9681fd5b8956c2a9` (recorded 2026-10-01 23:58)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `AGENTS.md`, `plans/00-roadmap.md`,
> `plans/residual-defect-hardening/*`, `build/` (gitignored probes)

### Step 5.1: Acceptance, Probes, and Synchronization

- [x] 5.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg) ✅ (completed: 2026-10-01 23:58)
- [x] 5.1.2 Host AddressSanitizer probes for the four defects: the ST-4 owned deferred literal (DEF-46), ST-6 (DEF-47), ST-8/ST-9/ST-18 (DEF-48), ST-10/ST-11/ST-14/ST-15/ST-16/ST-19/ST-21 (DEF-49), with leak detection on — `build/closeout-probes/` (AR #6) ✅ (completed: 2026-10-01 23:58)
- [x] 5.1.3 Update documentation: the `AGENTS.md` test map (both new suites) and the roadmap rows for DEF-46 – DEF-49 (Done with this plan linked) — `AGENTS.md`, `plans/00-roadmap.md` (AR #7, #10) ✅ (completed: 2026-10-01 23:58)
- [x] 5.1.4 Check every acceptance criterion in [01-requirements.md](01-requirements.md); set the plan status Complete and the progress header to 38/38; final `make check` ✅ (completed: 2026-10-01 23:59)
- [x] 5.1.5 Phase review evidence and plan summary — commit per the active commit mode ✅ (completed: 2026-10-01 23:59)

> **Close-out record (2026-10-02 00:05):** `make verify` passed every leg (`check` 36 suites,
> hosted-amiga corpus, FS-UAE capture, the six runtime faults). Thirteen host ASan close-out
> probes built under `build/closeout-probes/` with leak detection on, all clean at their expected
> exit codes (ST-4, ST-6, ST-8, ST-9 do/for, ST-18, ST-10, ST-11, ST-14, ST-15, ST-16, ST-19,
> ST-21), re-runnable with `build/closeout-probes/run-residual-hardening.sh`; the pre-fix leaks
> were recorded in the phase records. `spec/` has zero changes and no diagnostic code was added
> (R7, R8 verified by diff). `AGENTS.md` maps both new suites; the roadmap DEF-46 – DEF-49 rows
> are Done with this plan linked, and DEF-50 – DEF-55 record the filed residuals.

**Deliverables**:
- [x] `make verify` all legs passed
- [x] ASan probes clean with leak detection on
- [x] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (flat literal types + row store)
    ↓  (the flat type is a precondition for the multi-extent shapes)
Phase 2 (return copy)
    ↓
Phase 3 (loop-condition release)
    ↓  (the condition walk's ownership predicate and its param-typed case, ST-18)
Phase 4 (substitution-aware ownership)
    ↓
Phase 5 (close-out)
```

Phase 4 runs last so the generic-ownership diff is reviewed on its own. It consumes two earlier
artifacts: the flat type from `substitute` (Phase 1) and the condition walk whose ownership
predicate Phase 4 widens with `maybe_owns` (the generic-loop case ST-18 lands in Phase 4's
red/green set). Phases 2 and 3 may swap order if execution finds it useful; the spec-first
ordering within each phase is fixed.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per task, `make verify` at close-out)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security hardened — N/A (no input surface changes); recorded in the register
6. ✅ Documentation updated (test map and roadmap)
7. ✅ Code reviewed (strict profile; the phase review runs per phase)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
