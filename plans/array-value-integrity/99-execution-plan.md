# Execution Plan: Array Value Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-29 02:08
> **Progress**: 9/24 tasks (38%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the remaining fixed-array value defects: fixed-array copy/move assignment
(DEF-22, new), `len` of fixed multi-extent arrays (AR-6), multi-extent element
loops (DEF-15), projections over array-returning calls (DEF-21), and `len` of
an owned `String` (DEF-19). Each feature phase follows the mandatory
specification-first ordering; the close-out phase runs the AddressSanitizer
probes, the full acceptance gate, and the documentation/roadmap updates.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Fixed-array values and `len` | 6 |
| 2 | Multi-extent element loops | 6 |
| 3 | Projections and element moves | 7 |
| 4 | Close-out: probes, acceptance, documentation | 5 |

**Total: 24 tasks across 4 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of
> truth** for progress. Every task line appears exactly once in this document.
> The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the
>    Last Updated stamp after EVERY task — never batch updates. Only `[x]`
>    counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]`
>    task is resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append
>    `Blocked: <short reason>` on the same line. The plan lifecycle is `Ready`,
>    `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Fixed-array values and `len`

> **Phase baseline tree**: `d7da76d10baad0eea910e3a8dd230223f1ca17ae`
> **Expected modification set** (strict scope): `compiler/tests/array_value_integrity_spec_test.rs` (new), `compiler/src/codegen/function.rs`, `compiler/src/codegen/calls.rs`, `compiler/src/codegen/drop.rs` (doc comment), `compiler/tests/array_value_integrity_impl_test.rs` (new), `plans/array-value-integrity/99-execution-plan.md`, `plans/array-value-integrity/07-testing-strategy.md`, `plans/array-value-integrity/03-01-multi-extent-element-loops.md`, `plans/00-roadmap.md` (`DEF-25`)
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)
> **Phase 1 review**: independent reviewer (generic-subagent fallback; named reviewer agents are manual-only) on the diff vs `d7da76d1` — RV-001 🔴 missing argument guards (fixed in `676c2ee`), RV-002 🟠 conditional fixed-array value (filed `DEF-25`), RV-003/RV-004 🟡 applied; single re-review on the fix: `FIX ACCEPTED`, RR-001 wording applied, RR-002 filed `DEF-26`, RR-003 filed `DEF-27` — no blocking issue remains (2026-09-29 01:07)

### Step 1.1: Specification Tests

**Reference**: [03-01 §Implementation Details](03-01-multi-extent-element-loops.md) · [03-02 §New Functions](03-02-array-value-projections.md) · AR-2, AR-4, AR-6, AR-7
**Objective**: Pin fixed-array copy/move, self-assignment, and `len` behavior before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from the 07 ST cases — `compiler/tests/array_value_integrity_spec_test.rs` (ST-1 – ST-9, ST-22, ST-25, ST-26) ✅ (completed: 2026-09-28 18:10)
- [x] 1.1.2 Run the spec suite and record the red phase — the split recorded in `07-testing-strategy.md` (ST-1: wrong value; ST-2 – ST-5, ST-7, ST-8: the C compiler rejects the emitted C; ST-22, ST-25, ST-26: the `len` value is wrong; ST-6, ST-9: pass as guards; the release shape is pinned red by the Phase 1 implementation tests) ✅ (completed: 2026-09-28 18:12) — red evidence (Phase 1 batch): ST-6 (`E10020` message) and ST-9 (call-return binding) pass; ST-2 – ST-5, ST-7, ST-8 reject in C; ST-22, ST-25, ST-26 return the wrong `len` value; ST-1 returns the wrong `6` (its program reads an element so the array stays live in the emitted C; verified against the pre-fix build). The later-phase cases join the suite in tasks 2.1.1/3.1.1 so every phase's `make check` runs a fully green suite; deviation from the recorded split: ST-14 (a Phase 2 case) fails on the message type (`non-duplicable 'String'` pre-fix, the row type arrives with the fix) — the case is unchanged, and the `07` split table records it

**Deliverables**:
- [x] Spec suite with the ST-1 – ST-9, ST-22, ST-25, ST-26 cases and the shared scratch/run helpers
- [x] Red-phase evidence recorded (suite output)

**Verify**: `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test"`

### Step 1.2: Implementation

**Reference**: [03-01 §Implementation Details](03-01-multi-extent-element-loops.md) · [03-02 §New Functions](03-02-array-value-projections.md) · AR-2, AR-4, AR-6, AR-7
**Objective**: Byte-copy array stores, the array save path, the call-into-temporary assignment path, and the `len` arms (the outer-extent rule and the `String` byte count).

- [x] 1.2.1 Implement the fixed-array store path (`emit_assign`, `Statement::Move`, the save path, the call-into-temporary assignment path, and the same-place no-op) and the `len` arms (the outermost-extent rule for fixed multi-extent arrays and the owned-`String` byte count) — `compiler/src/codegen/function.rs`, `compiler/src/codegen/calls.rs` ✅ (completed: 2026-09-28 18:13)
- [x] 1.2.2 Run the spec suite and record the green phase — ST-1 – ST-9, ST-22, ST-25, ST-26 pass ✅ (completed: 2026-09-28 18:15) — green evidence: the Phase 1 suite is fully green, 12/12 cases pass (ST-1 – ST-9, ST-22, ST-25, ST-26)

**Deliverables**:
- [x] Copy, move, reassignment, and self-assignment of fixed arrays emit valid C; a call right-hand side is moved from a temporary
- [x] `len(m)` is the outermost extent for fixed multi-extent arrays; `len(s)` is the byte count for an owned `String`

**Verify**: `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test"`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-4, AR-7
**Objective**: Pin the emitted shapes and cover internals.

- [x] 1.3.1 Write implementation tests: `memcpy` copy shape, `cl_zero` after a move, the same-place no-op, the self-assignment save shape, the call-into-temporary assignment shape, `len` renderings, and counting-deleter release shape checks — `compiler/tests/array_value_integrity_impl_test.rs` ✅ (completed: 2026-09-28 18:18) — 7/7 pass; deviation: the fixed-array save branch is unreachable from accepted programs today (an exact same-place assignment is a no-op, an owned self-assignment reports `E10020`, and a self-referencing call takes the temporary path), so no Phase 1 test pins it; it stays as the approved defensive shape (`AR-7`), and the release counts pin the reachable call path. The Phase 1 review (RV-002) found that a conditional right-hand side whose arm reads the owned target (`s = ok ? s : t`) does reach it and is accepted today; that form is filed as `DEF-25`
- [x] 1.3.2 Full verification — `make check` (spec check plus the whole workspace) ✅ (completed: 2026-09-28 18:19) — `make check` exit 0: spec check OK (27 files, 104 compile-fail tests); 748 workspace tests passed across 28 binaries, no failures or warnings

**Deliverables**:
- [x] Implementation tests pass
- [x] No regressions; no warnings

**Verify**: `make check`

---

## Phase 2: Multi-extent element loops

> **Phase baseline tree**: `5bc7fd1f1273254fe2ae4860c9c79f5502a9d601`
> **Expected modification set** (strict scope): `compiler/tests/array_value_integrity_spec_test.rs`, `compiler/src/types/mod.rs`, `compiler/src/sema/stmt.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen/types.rs`, `compiler/src/codegen/render.rs`, `compiler/src/codegen/calls.rs` (authorized slice-guard fix), `compiler/tests/array_value_integrity_impl_test.rs`, `plans/array-value-integrity/99-execution-plan.md`
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 2.1: Specification Tests

**Reference**: [03-01 §New Types](03-01-multi-extent-element-loops.md) · AR-1, AR-6
**Objective**: Pin row iteration and row views before implementation.

- [x] 2.1.1 [spec-author] Add the element-loop specification tests to the suite (ST-10 – ST-15) — `compiler/tests/array_value_integrity_spec_test.rs` ✅ (completed: 2026-09-29 01:08)
- [~] 2.1.2 Run the spec suite and record the red phase — ST-10 – ST-13, ST-15 fail (`E10209` or wrong iteration count); ST-14 fails with the base-element type in the `E10040` message ✅ (completed: 2026-09-29 01:09) — red evidence: the 12 Phase 1 cases stay green; ST-10 – ST-13 and ST-15 reject with `E10209` (`type 'i32' cannot be indexed`, the loop binds the base element); ST-14 rejects with `E10040` naming `non-duplicable 'String'` instead of the row type `'String[2]'`

**Deliverables**:
- [ ] ST-10 – ST-15 in the spec suite
- [ ] Red-phase evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test"`

### Step 2.2: Implementation

**Reference**: [03-01 §Implementation Details](03-01-multi-extent-element-loops.md) · AR-1
**Objective**: Derive the row type from the remaining extents in both checkers, the lowering, and the borrow row's C declaration.

- [x] 2.2.1 Add `Ty::iterated_element`, use it in `sema/stmt.rs::check_for_in` and `own/loops.rs::check_for_in`/`bind_for_in_element`, and spell the borrow row's raw address temporary as a pointer-to-array — `compiler/src/types/mod.rs`, `compiler/src/sema/stmt.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen/types.rs` ✅ (completed: 2026-09-29 02:08) — a row access through a slice reinterprets its base-element pointer as a pointer-to-array (`codegen/render.rs`); authorized adjacent fix: the slice guard omits the lower-bound clause for a constant zero low bound (`codegen/calls.rs`), pinned by test `ai9`
- [ ] 2.2.2 Run the spec suite and record the green phase — ST-1 – ST-15 pass

**Deliverables**:
- [ ] Rows bind as `T[M]` for `T[N][M]` and `T[][M]`; the copy and view forms both work
- [ ] The borrow row address temporary is declared as a C pointer-to-array
- [ ] `E10040` names the row type

**Verify**: `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test"`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-1
**Objective**: Pin the loop lowering, the pointer-to-array declaration, and cover internals.

- [ ] 2.3.1 Add implementation tests: unit coverage for `iterated_element` (fixed, slice, nested, single-extent), the emitted row copy in the loop body, and the pointer-to-array borrow-row declaration — `compiler/tests/array_value_integrity_impl_test.rs`
- [ ] 2.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Implementation tests pass
- [ ] No regressions; no warnings

**Verify**: `make check`

---

## Phase 3: Projections and element moves

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 3.1: Specification Tests

**Reference**: [03-02 §Implementation Details](03-02-array-value-projections.md) · AR-5
**Objective**: Pin call-result projections and the element-move rejections.

- [ ] 3.1.1 [spec-author] Add the projection and element-move specification tests to the suite (ST-16 – ST-21, ST-23, ST-24, ST-27 – ST-30) — `compiler/tests/array_value_integrity_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase — ST-16 – ST-21, ST-23, ST-27 – ST-30 fail (invalid C or missing `E10026`); ST-24 passes as a guard

**Deliverables**:
- [ ] ST-16 – ST-21, ST-23, ST-24, ST-27 – ST-30 in the spec suite
- [ ] Red-phase evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test"`

### Step 3.2: Implementation

**Reference**: [03-02 §New Functions](03-02-array-value-projections.md) · AR-5
**Objective**: Materialize fixed-array call results before projections; reject ownerless element moves at every move decision.

- [ ] 3.2.1 Lift the fixed-array exclusion for call results in the materialize helpers and emit the destination call in both guard arms — `compiler/src/own/lower.rs`, `compiler/src/codegen/guards.rs`
- [ ] 3.2.2 Add `is_ownerless_element_move` and report `E10026` at the shared store preparation, the aggregate consume point, the `take` argument, and the return; cover projected-target assignments (local and global) by consuming the place value through the existing `consume_owned_place` after it is evaluated — `compiler/src/own/expr.rs`, `compiler/src/own/statements.rs`, `compiler/src/own/call_check.rs`
- [ ] 3.2.3 Run the spec suite and record the green phase — ST-1 – ST-30 pass

**Deliverables**:
- [ ] `mk()[i]` evaluates once and reads or lends the element; element moves report `E10026` in every move form, including projected-target assignment

**Verify**: `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test"`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-5
**Objective**: Pin the materialize shape, the short-circuit comma operand, the E10026 sites, and the element-move predicate.

- [ ] 3.3.1 Add implementation tests: materialized temporary with the destination call, the short-circuit comma operand, the E10026 sites (assignment, field-target assignment, aggregate-literal field), and unit coverage for `is_ownerless_element_move` — `compiler/tests/array_value_integrity_impl_test.rs`
- [ ] 3.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Implementation tests pass
- [ ] No regressions; no warnings

**Verify**: `make check`

---

## Phase 4: Close-out: probes, acceptance, documentation

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — docs and evidence phase)

### Step 4.1: Evidence and Close-out

**Reference**: [01 §Acceptance Criteria](01-requirements.md) · AR-8, AR-2, AR-4, AR-5
**Objective**: Prove memory safety, run the full acceptance gate, and close the records.

- [ ] 4.1.1 AddressSanitizer probes over the copy/move/projection programs (ST-2, ST-5, ST-7, ST-8, ST-16, ST-18, ST-23, ST-25, ST-26): emit, build with `clang -std=c99 -O0 -g -fsanitize=address`, run with `ASAN_OPTIONS=detect_leaks=0` (leak detection is off; the statement-temporary limitation is recorded as `DEF-23`), record the results on this line; count the emitted deleter call sites for the ST-25 and ST-26 release shapes (one per old element; the runs exit clean)
- [ ] 4.1.2 Full acceptance — `make verify` (every hosted-amiga and freestanding leg)
- [ ] 4.1.3 Update documentation: `AGENTS.md` test map (both new suites), roadmap rows for `DEF-15`/`DEF-19`/`DEF-21`/`DEF-22` (done + plan link) and keep the `DEF-23`/`DEF-24` filings current, register closure note — `AGENTS.md`, `plans/00-roadmap.md`
- [ ] 4.1.4 Check every acceptance criterion in `01-requirements.md`; set this plan's status Complete and the progress header to 24/24; final `make check`
- [ ] 4.1.5 Phase review evidence and plan summary — commit per the active commit mode

**Deliverables**:
- [ ] ASan-clean probes (configured run), release-count probes, and recorded results
- [ ] `make verify` all legs passed
- [ ] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (array stores + len)
    ↓
Phase 2 (element loops)      Phase 1 → Phase 3 (materialize)
    ↓                             ↓
Phase 4 (close-out) ←────────────┘
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security reviewed — compiler-internal change with no input, secret, or data path (AR-12)
6. ✅ Documentation updated
7. ✅ Code reviewed (strict-profile phase review)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
