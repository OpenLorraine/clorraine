# Execution Plan: Call Value Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-01 09:11
> **Progress**: 32/39 tasks (82%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the sixteen filed defects DEF-30 – DEF-45 in four implementation phases (owned conditional
stores, fixed-array value positions, func pointers and callable resolution, guards and cleanup)
plus a close-out phase, following the specification-first ordering. Governing designs:
[03-01](03-01-scalar-conditional-ownership.md), [03-02](03-02-fixed-array-value-positions.md),
[03-03](03-03-func-pointers-and-callable-resolution.md), [03-04](03-04-guards-and-cleanup.md).
Specification cases: [07](07-testing-strategy.md). Decisions: `00-ambiguity-register.md`.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Owned Conditional Stores | 8 |
| 2 | Fixed-Array Value Positions | 9 |
| 3 | Func Pointers and Callable Resolution | 9 |
| 4 | Guards and Cleanup | 8 |
| 5 | Close-out | 5 |

**Total: 39 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
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
>    runs `make check` in the pinned container before it is promoted; `make check` per task is the
>    binding cadence (AR #17).
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

## Phase 1: Owned Conditional Stores

> **Phase baseline tree**: `d606000124794de3146191c65e34afc28c44a856` (recorded 2026-09-30 15:34
> from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/tests/call_value_integrity_spec_test.rs`,
> `compiler/src/own/{statements,expr,lower}.rs`, `compiler/src/own.rs`, `compiler/src/codegen/{function,calls,expr,cleanup}.rs`,
> `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs`, `compiler/tests/call_value_integrity_impl_test.rs`,
> `plans/call-value-integrity/*`, `plans/00-roadmap.md`

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-scalar-conditional-ownership.md) §Proposed Changes 1–7 · AR #2, #3, #15, #19
**Objective**: Pin the owned conditional and deferred-take behavior from ST-1 – ST-8 and ST-32 – ST-37.

- [x] 1.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-1 – ST-8 and ST-32 – ST-37 — `compiler/tests/call_value_integrity_spec_test.rs` ✅ (completed: 2026-09-30 15:38)
- [x] 1.1.2 Run the specification tests — verify at least one case per Phase-1 defect fails before implementation (red phase); record ST-4/ST-7/ST-8 as pre-passing regression pins ✅ (completed: 2026-09-30 15:38)

**Red phase (2026-09-30 15:38):** 12 passed / 17 failed on the unmodified compiler
(`cargo test --test call_value_integrity_spec_test`; every failure is a run-stage double-free).
DEF-34 red cases: ST-1 (both directions), ST-2 (place arm), ST-3 (place arm), ST-32 (both),
ST-33 (both), ST-34 (place arm), ST-35 (both), ST-36 (both), ST-37 (place arm and the
struct-field place arm). Pre-passing regression pins: ST-4, ST-5, ST-6, ST-7, ST-8
(call-arm direction), and the call-arm directions of ST-2/ST-3/ST-34/ST-37. DEF-45 is
leak-only and invisible without AddressSanitizer; its red evidence is the close-out ASan probe
plus the emitted-shape implementation pin.

### Step 1.2: Implementation

**Reference**: [03-01](03-01-scalar-conditional-ownership.md) §Proposed Changes 1–3 · AR #2, #19
**Objective**: Emit the branch store for owned conditionals (handles and owned aggregates; places, calls, and mixed arms).

- [x] 1.2.1 Owned conditional branch store: destination predicate (non-array `needs_drop`, aggregates included), old-target/save and self-naming-arm rules, nested arms, arm sources, and emission — `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-09-30 19:41)
- [x] 1.2.2 Transfer temporary release registration (release-on-discard) and adoption move-out at binding, assignment, `take` argument (non-releasing argument temporary), `return`, and struct-literal field — `compiler/src/own/lower.rs`, `compiler/src/own.rs`, `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-09-30 19:41)
- [x] 1.2.3 Update the two existing pins that assert the old transfer behavior — `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs` (AR #15) ✅ (completed: 2026-09-30 19:41)
- [x] 1.2.4 Run the specification tests — verify they PASS (green phase); a failing case means the implementation is wrong ✅ (completed: 2026-09-30 19:41)

**Task 1.2.3 note (mechanical correction):** both pins were re-run and still describe the
post-fix behavior, so no assertion edit was needed. The eager (non-deferred) take-call path is
deliberately unchanged: its temporary never joins the release set, so
`should_not_release_a_take_call_result_temporary` and
`a_take_call_result_temporary_is_not_released` stay accurate. Only the deferred conditional
transfer temporary changed (release-on-discard + adoption zeroing); its coverage lands in task
1.3.1. AR #15's update authorization therefore remains unused.

**Green phase (2026-09-30 19:41):** `cargo test --test call_value_integrity_spec_test` —
29 passed / 0 failed; full `make check` also passed (33 suites, 0 failures).

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests · AR #15
**Objective**: Pin the emitted shapes and run the phase verification.

- [x] 1.3.1 Write implementation tests (branch-store shape, arm zeroing, transfer registration/release, argument temporary) — `compiler/tests/call_value_integrity_impl_test.rs` ✅ (completed: 2026-09-30 19:43)
- [x] 1.3.2 Full verification — `make check` ✅ (completed: 2026-09-30 19:43)

**Implementation tests (2026-09-30 19:43):** `cargo test --test call_value_integrity_impl_test`
— 11 passed / 0 failed; full `make check` — 34 suites, 0 failures, 0 warnings.

**Deliverables**:
- [x] Roadmap rows `DEF-30` – `DEF-45` set to Executing with the plan link (AR #18)
- [x] ST-1 – ST-8 and ST-32 – ST-37 pass; at least one case per Phase-1 defect failed first (red
  phase recorded); ST-4/ST-7/ST-8 recorded as regression pins
- [x] The two updated pins pass; the closed plans' specification suites are unchanged
- [x] `make check` passes with no warnings

**Verify**: `make check`

### Phase 1 Review (post-phase quality step)

**Reviewer:** correctness-reviewer (strict profile), phase diff against baseline tree
`d606000124794de3146191c65e34afc28c44a856`; scope mode strict; lenses
correctness / maintainability / standards.

| Finding | Severity | User ruling | Fix |
| ------- | -------- | ----------- | --- |
| RV-1: a composite conditional arm (`ok ? Box { name: sink(s), tag: 2 } : b`) copied a transfer temporary and the statement release then freed it (ASan heap-use-after-free) | 🔴 critical | Fix now (2026-09-30) | `emit_arm_source_moves` empties every storage source a rendered arm copied, recursing into struct-literal fields and nested conditionals |
| RV-2: a `take` conditional argument was materialized eagerly, so a short-circuit operand ran it on a skipped path | 🟠 major | Fix now | A deferred operand stores into a non-releasing argument temporary through the expression-form owned conditional store; the enclosing call's move-out empties it after the call |
| RV-3: the struct-literal store form emitted invalid C for a fixed-array field | 🟠 major | Fix now | The store form is gated off while any field is a fixed array; the aggregate-initializer form stays. The residual pre-existing double-free for that mixed shape belongs to the fixed-array field work in `03-02` |
| RV-4: the struct-literal store form double-freed the old target when a field read it | 🟠 major | Fix now | The literal is built in a fresh local first, then the old target is released and the local moved in |
| RV-5: an arm whose `take` call consumed the destination left the saved value to be released | 🟠 major | Fix now | The saved value is emptied when an arm's deferred move-outs name the destination |

**Fix verification (2026-09-30):** the five host AddressSanitizer probes (RV-1, RV-2 both `ok`
directions, RV-4, RV-5 shapes) run clean under
`cc -std=c99 -O0 -g -fsanitize=address` with `ASAN_OPTIONS=detect_leaks=1`; RV-3's shape
compiles again (its residual double-free is the pre-existing mixed-shape issue owned by Phase 2).
`cargo test --test call_value_integrity_spec_test` 29/29, `call_value_integrity_impl_test`
16/16 (five new emitted-shape pins), `make check` 34 suites, 0 failures, 0 warnings.

**Re-review (one permitted pass, scoped to the fix diff `235582d..aab9d08`, 2026-10-01):** the
five fixes hold for the reported shapes; the re-review surfaced three new major findings, which
the user ruled to fix:

| Finding | Severity | Fix |
| ------- | -------- | --- |
| F1: emptying a member/index arm unconditionally emitted `mk().name = NULL;` for a field of a fresh call result (invalid C) and re-ran the call | 🟠 major | The source move is gated on a storage root (a place or temporary, or a projection over one); a field of a fresh call result is copied without being emptied |
| F2: the composite-arm zero pass re-evaluated a condition that read the destination, so it emptied the wrong arm (ASan double-free) | 🟠 major | A struct-literal arm is built in a fresh local through per-field stores, so a conditional field's condition is evaluated once by the branch store |
| F3: the save-empty test inspected only the direct arm, so a `take` call inside a nested condition consumed the destination before the saved value was released (ASan double-free) | 🟠 major | The whole arm subtree is scanned for deferred move-outs naming the destination, and the saved value is emptied when the outer or a nested condition consumes it |

**F1–F3 verification (2026-10-01):** host AddressSanitizer probes for the three shapes plus the
five original probes run clean (`ASAN_OPTIONS=detect_leaks=1`); the call-field shape compiles
again; `call_value_integrity_impl_test` 19/19 (four new pins); `make check` 34 suites, 0
failures, 0 warnings. No third review per the one-re-review cap.

---

## Phase 2: Fixed-Array Value Positions

> **Phase baseline tree**: `3237bc667633feeaff72952c4fbd84b7385f1900` (recorded 2026-10-01 01:35
> from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/tests/call_value_integrity_spec_test.rs`,
> `compiler/src/own/{expr,lower}.rs`, `compiler/src/own.rs`, `compiler/src/codegen/{expr,function}.rs`,
> `compiler/tests/call_value_integrity_impl_test.rs`, `plans/call-value-integrity/*`

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-fixed-array-value-positions.md) §Proposed Changes 1–4 · AR #7, #11, #13
**Objective**: Pin the fixed-array store, argument, projection, and struct-field behavior from ST-9 – ST-17 and ST-41.

- [x] 2.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-9 – ST-17 and ST-41 — `compiler/tests/call_value_integrity_spec_test.rs` ✅ (completed: 2026-10-01 01:38)
- [x] 2.1.2 Run the specification tests — verify at least one case per Phase-2 defect fails before implementation (red phase); record pre-passing cases as regression pins ✅ (completed: 2026-10-01 01:38)

**Red phase (2026-10-01 01:38):** 29 passed / 15 failed on the unmodified compiler
(`cargo test --test call_value_integrity_spec_test`). Every failure is a host-C build rejection of
the emitted translation unit: the five element-store cases (runtime and constant index, literal
and conditional arms; DEF-31) emit `assignment to expression with array type` or an
array-literal syntax error; the three conditional/plain call-argument cases (DEF-32) emit a call
missing its destination argument; the two conditional literal-argument cases (DEF-33) emit the
array-literal syntax error; the projection case (DEF-35) emits the same; the four struct-literal
cases (DEF-38) emit an integer-from-pointer initializer with missing braces. No Phase-2 case
pre-passes, so there are no new regression pins; the 29 Phase-1 cases pass.

### Step 2.2: Implementation

**Reference**: [03-02](03-02-fixed-array-value-positions.md) §Proposed Changes 1–4
**Objective**: Give every fixed-array value position a valid C place.

- [x] 2.2.1 Element stores: refactor the fixed-array assign core to a rendered target and route constant- and runtime-index element stores through it; return the full element type from the projected type — `compiler/src/own/expr.rs`, `compiler/src/own.rs`, `compiler/src/codegen/function.rs` (DEF-31) ✅ (completed: 2026-10-01 01:45)
- [x] 2.2.2 Materialize duplicable fixed-array values and struct literals with a place-less field at place-requiring positions (gate the skip) — `compiler/src/own/lower.rs` (AR #11; DEF-32, DEF-33, DEF-38) ✅ (completed: 2026-10-01 01:45)
- [x] 2.2.3 Materialize fixed-array literal sources at projections (value and argument forms) — `compiler/src/own/lower.rs`, `compiler/src/codegen/expr.rs` (DEF-35) ✅ (completed: 2026-10-01 01:45)
- [x] 2.2.4 Struct-literal initialization: materialize when any field lacks an initializer form; zero-init then store every field in source order (single rule) — `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs` (AR #7; DEF-38) ✅ (completed: 2026-10-01 01:45)
- [x] 2.2.5 Run the specification tests — verify they PASS (green phase) ✅ (completed: 2026-10-01 01:45)

**Green phase (2026-10-01 01:45):** `cargo test --test call_value_integrity_spec_test` — 44
passed / 0 failed (15 Phase-2 cases green). Full `make check` passed (34 suites, 0 failures, 0
warnings); the Phase-1 impl pin that reserved the aggregate-initializer form for a fixed-array
field was updated to the new store form, as the fixed-array store machinery now reaches struct
fields.

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted shapes and run the phase verification.

- [x] 2.3.1 Write implementation tests (element-store byte copy, materialized argument/projection, struct field store) — `compiler/tests/call_value_integrity_impl_test.rs` ✅ (completed: 2026-10-01 01:46)
- [x] 2.3.2 Full verification — `make check` ✅ (completed: 2026-10-01 01:46)

**Implementation tests (2026-10-01 01:46):** `cargo test --test
call_value_integrity_impl_test` — 28 passed / 0 failed (9 new emitted-shape pins: constant- and
runtime-index row byte copies, a runtime-index literal store, a runtime-index conditional branch
store, a conditional call-argument local, a conditional literal-argument local, a projection
temporary, a struct-literal argument local, and a returned struct-literal local). The Phase-1 pin
that reserved the aggregate-initializer form for a fixed-array field was replaced by the store-form
pin. Full `make check` — 34 suites, 0 failures, 0 warnings.

**Deliverables**:
- [x] ST-9 – ST-17 and ST-41 pass; at least one case per Phase-2 defect failed first (red phase
  recorded); pre-passing cases recorded as regression pins
- [x] `make check` passes with no warnings

**Verify**: `make check`

### Phase 2 Review (post-phase quality step)

**Reviewer:** correctness-reviewer (strict profile), phase diff against baseline tree
`3237bc667633feeaff72952c4fbd84b7385f1900`; scope mode strict; lenses
correctness / maintainability / standards.

| Finding | Severity | User ruling | Fix |
| ------- | -------- | ----------- | --- |
| RV2-1: a run-time-index store of an owned value copied it without moving the source, so the source cleanup and the element double-freed (fixed arrays and the scalar sibling) | 🔴 critical | Fix now (2026-10-01) | `emit_indexed_store` moves an owned place RHS through `consume_place`; the scalar owned branch evaluates the value into a fresh local first, releases the replaced element, then moves the local in |
| RV2-2: an array-literal projection materialized eagerly inside `&&`/`\|\|`/`?:`, running element side effects on a skipped path | 🟠 major | Fix now | A guarded projection defers the literal through a `Materialize` value; the guard emitter fills it (statement or comma-expression form, with each element's guards embedded where the element runs) |
| RV2-3: a `take` argument skipped the place gate, so a fixed-array literal/call or a struct literal with a fixed-array field emitted invalid C | 🟠 major | Fix now | The take branch materializes a place-less value into a non-releasing argument temporary (deferred for literal and call operands), exactly like an owned conditional |
| RV2-4: a conditional whose arms are struct literals with a fixed-array field still rendered an aggregate initializer (owned and duplicable paths) | 🟠 major | Fix now | A struct-literal arm always stores per field; a duplicable struct conditional stores per arm through `emit_struct_conditional_assign`; a returned conditional uses the same branches |
| RV2-5: a nested row value in an element literal stored as a scalar assignment | 🟡 minor | Fix now | `emit_array_assign_to` routes a nested non-literal row through the fixed-array store core |
| RV2-6: the `projects_from_place` doc comment was attached to `value_needs_place` | 🟡 minor | Fix now | The comment is back on its function and the new predicate has its own |
| RV2-7: `projected_type` duplicated `Ty::iterated_element` | 🟡 minor | Fix now | The element arm calls `ty.iterated_element()` |

**RV2 verification (2026-10-01 02:56):** while verifying RV2-4, the review probes exposed that
`consume_place` inside a conditionally evaluated literal moved both arms' field sources eagerly,
leaving the skipped arm's value in an unreleased temporary; the lowering now leaves those sources
in place and each adopting store empties the source it moves. The closed plan's
`should_not_release_a_literal_adoption_temporary` pin was updated in place (AR #15) to assert the
emptied source instead of an untouched temporary. Host AddressSanitizer probes for all seven
findings (fixed and scalar owned stores, the guarded projection and its arm form, take literal,
take call, take struct sentinel/conditional/duplicable, conditional struct arms, nested row
literal) run clean with `ASAN_OPTIONS=detect_leaks=1`; `call_value_integrity_spec_test` 44/44,
`call_value_integrity_impl_test` 33/33 (five new emitted-shape pins), `make check` 34 suites, 0
failures, 0 warnings. One re-review of the fix diff follows (single permitted pass).

**Re-review (single permitted pass, fix diff `f3a44ad..2c6cd5c`, 2026-10-01):** RV2-1, RV2-2
(top-level literal projection), RV2-3, RV2-5, RV2-6, and RV2-7 verified fixed with no new
critical or major regressions; adoption zeroing and deferred arm sources are clean under
AddressSanitizer. RV2-4 was confirmed for bindings, struct-literal arguments, and returns, but two
same-class remainders surfaced (both reproduce on the pre-fix compiler, so they are gaps in the
fix, not regressions):

| Finding | Severity | User ruling | Disposition |
| ------- | -------- | ----------- | ----------- |
| RR2-1: a duplicable struct conditional as the argument itself (`eat(ok ? Box { arr: a } : Box { arr: b })`, non-`take` and `take`) rendered a C ternary of aggregate initializers | 🟠 major | Fix now (2026-10-01) | Fixed: `value_needs_place` reports a struct conditional whose arm stores fields, so the argument materializes and the per-arm/per-field store runs; `materialize_take_place` defers only an owned conditional in a guard; `emit_array_assign_to` stores a struct-literal element field by field |
| RR2-2: a nested non-literal fixed-array row inside a deferred array literal (`ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1`) emits a brace list and a row ternary inside the guard comma expression | 🟠 major | File as a follow-up | Filed as `DEF-46` in the roadmap (Backlog) with the repro and suggested fix (an expression-form store for a nested row, or pre-materialization outside the comma expression) |

**RR2 verification (2026-10-01 07:10):** host AddressSanitizer probes for the argument and `take`
conditional-struct forms and the struct-element array literal run clean
(`ASAN_OPTIONS=detect_leaks=1`); the full fix-probe set is clean; `call_value_integrity_spec_test`
44/44; `call_value_integrity_impl_test` 36/36 (three new emitted-shape pins); `make check` 34
suites, 0 failures, 0 warnings. The review budget is exhausted (phase review + one fix-diff
re-review); RR2-1 ships probe-verified without a third pass, and RR2-2 is filed rather than
fixed.

**Recorded, not fixed (out of phase):** the `DEF-36` array-parameter zero walk (Phase 4) still
mis-zeroes a by-reference array parameter when a Phase-2 return path reaches it; a *conditional*
fixed-array index base inside a guarded operand still materializes eagerly (the pre-existing
variant of RV2-2, distinct from the array-literal variant fixed here).

---

## Phase 3: Func Pointers and Callable Resolution

> **Phase baseline tree**: `4a1b54d4c5e8ba8be33ab508ae77b184f616c4a6` (recorded 2026-10-01 07:23
> from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/tests/call_value_integrity_spec_test.rs`,
> `compiler/src/codegen.rs`, `compiler/src/codegen/{types,program,function,expr,calls,control,drop,guards}.rs`,
> `compiler/src/own/{call,lower,generics}.rs`, `compiler/src/types/{mod,table}.rs`,
> `compiler/src/sema/{expr,call,mod}.rs`, `compiler/src/abi.rs`,
> `compiler/tests/call_value_integrity_impl_test.rs`, `plans/call-value-integrity/*`

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-func-pointers-and-callable-resolution.md) §Proposed Changes 1–5 · AR #4, #12, #14
**Objective**: Pin the declarator, resolution, widening, and borrow behavior from ST-18 – ST-25, ST-39, and ST-40.

- [x] 3.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-18 – ST-25, ST-39, and ST-40 — `compiler/tests/call_value_integrity_spec_test.rs` ✅ (completed: 2026-10-01 07:28)
- [x] 3.1.2 Run the specification tests — verify at least one case per Phase-3 defect fails before implementation (red phase); record pre-passing cases as regression pins ✅ (completed: 2026-10-01 07:28)

**Red phase (2026-10-01 07:28):** 44 passed / 11 failed on the unmodified compiler
(`cargo test --test call_value_integrity_spec_test`); 11 new cases (ST-19 split into the callable
bound and the plain field form). Every Phase-3 defect has at least one red case: the four DEF-30
cases emit a lambda pointer without the hidden destination and an undeclared result variable (ST-18,
ST-19a — the closure call passes the destination the field lacks — ST-19b, ST-39); the DEF-43 case
emits an unnamed function-pointer return declarator (`expected identifier before ')'`); the DEF-40
case builds but calls the same-named module function (exit 104 instead of 5); the DEF-41 case emits
an implicit declaration of the global name; the DEF-42 case passes `ClString` where the func value
expects `ClStr`; the three DEF-44 cases assign a by-value pointer from a borrow pointer (ST-24,
ST-25) or accept the mode mismatch that must report `E10210` (ST-40). No Phase-3 case pre-passes,
so there are no new regression pins; the 44 Phase-1/2 cases pass.

### Step 3.2: Implementation

**Reference**: [03-03](03-03-func-pointers-and-callable-resolution.md) §Proposed Changes 1–5
**Objective**: Use the shared declarator everywhere and resolve func-value callees.

- [x] 3.2.1 Declarator reuse: one shared signature builder (optional environment prefix, hidden destination), return position, closure struct field, lifted-lambda signature with `result_var`, and destination-aware closure call sites — `compiler/src/codegen.rs`, `compiler/src/codegen/types.rs`, `compiler/src/codegen/program.rs`, `compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs` (DEF-30, DEF-43) ✅ (completed: 2026-10-01 07:41)
- [x] 3.2.2 Capture and module-global resolution for calls by name (lowering and emission) — `compiler/src/own/call.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs` (DEF-40, DEF-41) ✅ (completed: 2026-10-01 07:41)
- [x] 3.2.3 Func-value expected parameter types and widening, with the resolved callee type passed into the argument renderer — `compiler/src/codegen/calls.rs`, `compiler/src/codegen/function.rs` (AR #12; DEF-42) ✅ (completed: 2026-10-01 07:41)
- [x] 3.2.4 Function-type parameter modes: `FuncParam { ty, mode }` in `FuncTy`; `FuncSig::ty`, `resolve_ty_with`, and the callable-bound path zip modes; mode-aware signature; exact-signature equality (mode mismatch = `E10210`) and display; all `TyKind::Func` consumers; call-site pointer passing from the resolved callee type — `compiler/src/types/mod.rs`, `compiler/src/types/table.rs`, `compiler/src/sema/{expr,call,mod}.rs`, `compiler/src/own/{lower,generics,call}.rs`, `compiler/src/abi.rs`, `compiler/src/codegen/{types,calls,function}.rs` (AR #4, #14; DEF-44) ✅ (completed: 2026-10-01 07:41)
- [x] 3.2.5 Run the specification tests — verify they PASS (green phase) ✅ (completed: 2026-10-01 07:41)

**Green phase (2026-10-01 07:41):** `cargo test --test call_value_integrity_spec_test` — 55
passed / 0 failed (11 Phase-3 cases green). The shared declarator (`function_declarator`) now builds
the named function, closure field, lifted lambda, and value-position spellings from one place, so
`func(): func(i32): i32` nests (`i32 (*make(void))(i32)`) and a fixed-array return appends
`__cl_ret` everywhere; a fixed-array closure call passes the destination after the environment and
its own arguments. Calls by name resolve to a capture's environment field, to a func-typed local,
and to a module-level func-typed global, and the func value's own signature drives `str` widening
and `borrow` address passing. `FuncTy` carries `FuncParam { ty, mode }`, so the exact-signature
comparison now reports `E10210` for a mode mismatch and the display renders the mode. The closed
plan's `runtime_value_integrity_impl_test.rs` pin for the unnamed hidden destination was updated in
place to the named form (AR #15).

### Step 3.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted shapes and run the phase verification.

- [x] 3.3.1 Write implementation tests (declarator shapes, capture/global spelling, widening, borrow pointer) — `compiler/tests/call_value_integrity_impl_test.rs` ✅ (completed: 2026-10-01 07:41)
- [x] 3.3.2 Full verification — `make check` ✅ (completed: 2026-10-01 07:41)

**Implementation tests (2026-10-01 07:41):** `cargo test --test
call_value_integrity_impl_test` — 42 passed / 0 failed (6 new emitted-shape pins: the nested
return declarator and its call, the shared closure-field/lifted-lambda destination signature and
the closure call destination, a fixed-array func-value declaration and destination call, the
capture and global call spellings, the func-value `String` widening, and the `borrow` pointer
through a binding and a field). Full `make check` — 34 suites, 0 failures, 0 warnings.

**Deliverables**:
- [x] ST-18 – ST-25, ST-39, and ST-40 pass; at least one case per Phase-3 defect failed first (red
  phase recorded); pre-passing cases recorded as regression pins
- [x] `make check` passes with no warnings

**Verify**: `make check`

**Phase 3 Review (post-phase quality step, 2026-10-01 07:51):** the correctness reviewer
(correctness-reviewer, strict scope, compiler-and-language lens) reviewed the phase diff
`4a1b54d4..HEAD`; the green phase and the six defect fixes were confirmed, and five findings were
raised (probes under the gitignored `build/rv3-review/`):

| Finding | Severity | Statement | Status |
| ------- | -------- | --------- | ------ |
| RV3-1 | 🔴 critical | A func type spells a `borrow` parameter as a pointer for every type, while a named function pointer-lowers only a non-duplicable struct, so a matching assignment emits incompatible C types (debug build failure; release silent wrong ABI) | Awaiting ruling |
| RV3-2 | 🟠 major | A fixed-array closure call inside a conditionally evaluated operand goes through the expression-guard path, which lacks the destination arm, so it emits a destination-less lifted-lambda call | Awaiting ruling |
| RV3-3 | 🟠 major | `callee_result_type` never resolves a module-level func-typed global, so a discarded fixed-array call through one omits the hidden destination | Awaiting ruling |
| RV3-4 | 🟠 major | `callee_result_type` never resolves a closure-typed callable parameter, so a discarded fixed-array call through one omits the hidden destination | Awaiting ruling |
| RV3-5 | 🟡 minor | A callable field call drops the field's parameter modes (`dot_call` populates `modes`, `check_dot_call` ignores them), so a marker/take mismatch on `ops.run(...)` is not diagnosed as it is on a named call | Report-only |

**RV3 fix verification (2026-10-01 08:06):** user ruling "Fix all five". RV3-1: `is_pointer_param`
moved to `compiler/src/codegen/types.rs` and applied in `func_pointer_params`, with the type table
threaded through `c_type`/`declarator`/`function_declarator`/`pointer_to_array_type`/
`pointer_to_array_declarator`/`index_text`, and `call_arguments` applying the same rule at the call
site; the func type now spells exactly like every function it can hold (`dupborrow` compiles and
exits 1, `dupborrow2` exits 2, `scalarborrow` compiles and exits 0). RV3-2: a fixed-array
`ClosureCall` arm in `guard_expressions` (`guardarr` exits 7). RV3-3: `callee_result_type` now
resolves through `callee_func_type` including globals (`discardglobal` exits 0). RV3-4: the
`closure_param_type` fallback in `callee_result_type` (`discardclosure2` exits 0). RV3-5: the type
stage passes field modes to `check_args_impl` and `check_call_expr` validates markers against the
field's modes (`markerfield` and `markernamed` each report one `E10039`). Five impl pins added
(47/47), `make check` 34 suites, 0 failures, 0 warnings.

**RV3 re-review (single permitted pass on the fix diff `a2d5499..worktree`, 2026-10-01 08:18):**
RV3-1, RV3-3, and RV3-4 verified fixed; RV3-2's named arm verified but related guard shapes remain
as left/pre-existing findings; RV3-5 verified for markers, with the added move enforcement not
honored by the lowering. The reviewer raised a fix-introduced regression and four left/pre-existing
findings (classified against a baseline binary; all four reproduce on `a2d5499`):

| Finding | Severity | Class | Statement | Status |
| ------- | -------- | ----- | --------- | ------ |
| RR3-1 | 🟠 major | Fix-introduced regression | When a closure-typed generic parameter shares a name with a module function, `callee_result_type` takes the closure's return type while `render_call` calls the module function, so a discarded fixed-array result emits destination-less C (baseline compiled) | Fixed: `render_call` tests the closure parameter before the named lookup |
| RR3-2 | 🟠 major | Left/pre-existing | The field-call fix marks a non-duplicable `take` argument as moved in the checker, but lowering still skips field modes, so no move-out is emitted and both sides free the value (double free; the design records `take`-in-a-func-type as having no ownership semantics) | Fixed as recorded: the move enforcement is reverted (marker validation stays); the `take`-in-a-func-type program remains the recorded Won't Have and still aborts, exactly as on the reviewed baseline |
| RR3-3 | 🟠 major | Left/pre-existing | A direct call of a capturing lambda literal is typed `Unknown`, so the guard, discarded, and argument positions emit a destination-less fixed-array call | Fixed: a `TyKind::Closure` arm in the type stage's `check_call` returns the closure's own return type |
| RR3-4 | 🟠 major | Left/pre-existing | A fixed-array call through a callable-bound parameter in a conditionally evaluated operand decides the destination from the recorded `Unknown` type instead of the resolved callee signature | Fixed: a callable-bound `TyKind::Param` arm in `check_call` types the call from the collected bound, so the guard materializes and passes the destination |
| RR3-5 | 🟡 minor | Left/pre-existing | A capturing closure's fixed-array parameter spells the field parameter with `c_type` but the lifted function with `declarator`, so the closure value is initialized with an incompatible pointer | Fixed: closure field parameters use the declarator spelling, matching `func_pointer_params` and the lifted lambda |

**RR3 fix verification (2026-10-01 08:31):** probes for all five findings plus the earlier
regression probes run clean: `rr3-nameshadow`/`2`/`3` (0/15/15), `rr3-guardcap` (0),
`rr3-condcap` (5), `rr3-capdiscard` (0), `rr3-caparg` (20), `rr3-capreturn`/`capcall` (15),
`rr3-condplain` (5), `rr3-guardparam` (7), `rr3-guardglobal` (7), `rr3-closurearr` (17),
`rr3-fieldborrow`/`globalborrow` (0). Four new impl pins (51/51); `make check` 34 suites, 0
failures, 0 warnings. The re-review budget is exhausted: these fixes ship probe-verified without
another independent pass. The `take`-in-a-func-type double free stays the recorded Won't Have.

---

## Phase 4: Guards and Cleanup

> **Phase baseline tree**: `63433c43fbf0f6f3ae185a9744ce604ae0bd8672` (recorded 2026-10-01 08:53
> from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/tests/call_value_integrity_spec_test.rs`,
> `compiler/src/codegen/{function,control,drop,guards,cleanup}.rs`,
> `compiler/tests/call_value_integrity_impl_test.rs`, `plans/call-value-integrity/*`
> (`cleanup.rs` added during 4.2.1: the return and guarded move-out paths zero a
> C-adjusted array parameter through the same place-aware walk, a necessary
> correction of the same defect surface)

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-guards-and-cleanup.md) §Proposed Changes 1–3 · AR #8, #9, #10
**Objective**: Pin the parameter move, do-while condition, and growable guard behavior from ST-26 – ST-31 and ST-38.

- [x] 4.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-26 – ST-31 and ST-38 — `compiler/tests/call_value_integrity_spec_test.rs` ✅ (completed: 2026-10-01 08:58)
- [x] 4.1.2 Run the specification tests — verify at least one case per Phase-4 defect fails before implementation (red phase); record pre-passing cases as regression pins ✅ (completed: 2026-10-01 08:58)

**Red phase (2026-10-01 08:58):** 58 passed / 4 failed on the unmodified compiler
(`cargo test --test call_value_integrity_spec_test`); 7 new cases. DEF-36 is red: the parameter
move rejects at the host C build (`sizeof` on an array parameter at `cl_zero(&a)`). DEF-37 is red
three ways: ST-27 and ST-38 are killed by SIGSEGV after the guarded condition is skipped on the
`continue`/fall-through edge, and ST-28 exits 1 instead of 3 because the guard temp is stale. The
three DEF-39 cases (ST-29, ST-30, ST-31) pre-pass and are regression pins: the growable guard leak
is only visible under AddressSanitizer, so DEF-39's red-phase evidence is the emitted-shape
implementation pin in 4.3.1 plus the close-out ASan probes. ST-38's body gains `i = i + 1;` before
`continue;` so the loop can terminate once the condition re-evaluates (the semantics under test are
unchanged).

### Step 4.2: Implementation

**Reference**: [03-04](03-04-guards-and-cleanup.md) §Proposed Changes 1–3
**Objective**: Correct the parameter zero walk, the do-while condition entry, and the embedded growable drop.

- [x] 4.2.1 Array-parameter copy and zero walk on both the source and the target of a move; `is_pointer_param` and the parameter ABI stay unchanged; `cl_zero` stays for true locals — `compiler/src/codegen/function.rs`, `compiler/src/codegen/guards.rs`, `compiler/src/codegen/cleanup.rs` (the return and guarded move-out paths reuse the same place-aware zero) (AR #10; DEF-36) ✅ (completed: 2026-10-01 09:06)
- [x] 4.2.2 Goto-free flag-first do-while when the condition carries guards (fall-through, `continue`, and embedded-only guards all reach the guards/test; unguarded keeps today's emission) — `compiler/src/codegen/control.rs` (AR #8; DEF-37) ✅ (completed: 2026-10-01 09:04)
- [x] 4.2.3 Growable-array drop expression form for element types that need no drop; the limitation for drop-needing elements is recorded in Won't Have and pinned — `compiler/src/codegen/drop.rs`, `compiler/src/codegen/guards.rs` (AR #9; DEF-39) ✅ (completed: 2026-10-01 09:11)
- [x] 4.2.4 Run the specification tests — verify they PASS (green phase) ✅ (completed: 2026-10-01 09:11)

**Green phase (2026-10-01 09:11):** all 62 specification cases pass
(`cargo test --test call_value_integrity_spec_test`); the guard expression now drops each
`Array<i32>` element's buffer and resets its count before re-materializing `growable()`
(`__cl_tmp0[0].data ? (FreeBuffer(__cl_tmp0[0].data), __cl_tmp0[0].data = NULL, __cl_tmp0[0].count = 0) : 0`).

### Step 4.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted shapes and run the phase verification.

- [ ] 4.3.1 Write implementation tests (parameter copy/zero on both sides, flag-first do-while with `continue`, limited growable drop expression) — `compiler/tests/call_value_integrity_impl_test.rs`
- [ ] 4.3.2 Full verification — `make check`

**Deliverables**:
- [ ] ST-26 – ST-31 and ST-38 pass; at least one case per Phase-4 defect failed first (red phase
  recorded); pre-passing cases recorded as regression pins
- [ ] `make check` passes with no warnings

**Verify**: `make check`

---

## Phase 5: Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — docs and evidence phase)

### Step 5.1: Acceptance and Documentation

**Reference**: [01](01-requirements.md) §Acceptance Criteria · AR #16, #17, #18
**Objective**: Full acceptance, memory-safety probes, documentation, and the plan summary.

- [ ] 5.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg)
- [ ] 5.1.2 Host AddressSanitizer probes for the memory-safety defects DEF-34, DEF-36, DEF-37, DEF-39, and DEF-45: build with `clang -std=c99 -O0 -g -fsanitize=address` and run with `ASAN_OPTIONS=detect_leaks=1` (host side; the pinned container has no ASan runtime), using the probe programs in `07-testing-strategy.md` (AR #16)
- [ ] 5.1.3 Update documentation: `AGENTS.md` test map (both new suites) and the roadmap rows for `DEF-30` – `DEF-45` (done + plan link) — `AGENTS.md`, `plans/00-roadmap.md` (AR #18)
- [ ] 5.1.4 Check every acceptance criterion in `01-requirements.md`; set this plan's status Complete and the progress header to 39/39; final `make check`
- [ ] 5.1.5 Phase review evidence and plan summary — commit per the active commit mode

**Deliverables**:
- [ ] `make verify` all legs passed
- [ ] ASan probes clean with leak detection on
- [ ] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (owned conditional stores)
    ↓
Phase 2 (fixed-array values) — independent of Phase 1 internals; ordered after for a clean baseline
    ↓
Phase 3 (func pointers) — independent of 1–2; ordered after for context economy
    ↓
Phase 4 (guards and cleanup) — independent of 1–3
    ↓
Phase 5 (close-out)
```

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security hardened — input validation, injection prevention, auth, rate limiting, data protection (N/A for an off-line compiler; the mandatory security check is the ASan ownership verification, AR #16)
6. ✅ Documentation updated
7. ✅ Code reviewed (strict-profile phase reviews per the exec-plan skill)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
