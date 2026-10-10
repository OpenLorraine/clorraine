# Execution Plan: Evaluation and Emission Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 17:39
> **Progress**: 39/49 tasks (80%)
> **CodeOps Artifact Schema**: 1

## Overview

Five phases close the 24 fix rows ([AR-B1](00-ambiguity-register.md)): module-initializer
integrity (DEF-117, 121, 122), strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120,
128), value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129), write-mutability
rules (DEF-119, 124), and ownership lifetimes and releases (DEF-95, 96, 125, 126, 127). DEF-103
closes with its evidence in the plan close. Each phase follows the specification-first ordering
and cites its owning [03-XX](00-index.md) document and the ST-cases in
[07-testing-strategy.md](07-testing-strategy.md).

Execution commit mode: **auto-commit** (user instruction) — commit and push each verified task
via the git-commit skill in push mode.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Module-initializer integrity (DEF-117, 121, 122) | 8 |
| 2 | Strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120, 128) | 12 |
| 3 | Value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129) | 12 |
| 4 | Write-mutability rules (DEF-119, 124) | 7 |
| 5 | Ownership lifetimes and releases (DEF-95, 96, 125, 126, 127) | 10 |

**Total: 49 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last Updated
>    stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Module-initializer integrity (DEF-117, DEF-121, DEF-122)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + emission-integrity auditor + memory-safety auditor
> (codegen and evaluation-order changes; AR-B16)
> **Reasoning**: high — emission contracts and double evaluation

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-module-initializer-integrity.md) §Proposed Changes ·
[AR-B5, AR-B13](00-ambiguity-register.md)
**Objective**: Pin the six init positions, the single-evaluation run, and the literal rejection
before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-7 — `compiler/tests/eval_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 12:13)
- [x] 1.1.2 Run the spec suite and record the red phase (expected: ST-1 – ST-6 fail against the broken emission; ST-7's `E10200` assertion fails while the `slice()` control passes) ✅ (completed: 2026-10-10 12:13)

**Red-phase evidence (2026-10-10 12:13)** — the suite (7 tests) fails exactly as designed: 0
passed, 7 failed. ST-1, ST-2, ST-4, and ST-5 fail with the destination-less calls (`too few
arguments to function 'cl_spec_eei_…_mk'`), ST-3 with the array assignment (`assignment to
expression with array type`), ST-6 with the double-evaluated guard (the second `idx()` call
reads out of range, so the run faults with `error E11211: index out of range` and never reaches
the expected exit), and ST-7's rejection is absent (the literal slice is accepted; its `slice()`
control passes). The first run also caught two missing `slice` imports in the test sources, fixed
before recording.

**Deliverables**:
- Specification suite with ST-1 – ST-7
- Red-phase evidence with per-pin justification

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-module-initializer-integrity.md) §Proposed Changes
**Objective**: Bind every fixed-array call through its destination in module initializers,
materialize impure guard operands once, and reject the backing-less slice literal.

- [x] 1.2.1 Add the module-init materialization pass for fixed-array calls across the six positions — `compiler/src/codegen/program.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-10 12:31)
- [x] 1.2.2 Apply the guarded-sub-operand materialization in the scalar single-assignment path — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-10 12:34)
- [x] 1.2.3 Stop adopting `None`-bearing expected extents in `check_array_literal`, so the fix reaches `E10200` for module and body spellings — `compiler/src/sema/expr.rs` ✅ (completed: 2026-10-10 12:40)
- [x] 1.2.4 Run the spec suite and `./spec/check.sh` to green (ST-1 – ST-7); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 12:40)

**Deliverables**:
- The init materialization and evaluation fixes and the checker rejection
- Green spec suite and green frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-module-initializer-integrity.md) §Testing Requirements
**Objective**: Pin the emitted destinations, the shared operand, and record the phase review
evidence.

- [x] 1.3.1 Write the Phase 1 implementation pins (destination-carrying emission per position, the shared init local for the guard operand, byte-identical re-emission of unaffected initializers) — `compiler/tests/eval_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 12:46)
- [x] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 13:28)

**Phase 1 review evidence (2026-10-10)** — three dispatches per the AR-B16 matrix (correctness +
emission-integrity + memory-safety). The full review found four blocking walk defects (the `||`
left-decided result, hoisted lazy-position stores, two-pass source-order violation, the dropped
enclosing guard and the left's re-evaluation); the two scoped re-reviews found the lost eager
slice guard (ASan-confirmed out-of-bounds read), the `||`-under-guard conjunct, the unlanded
collector arm, and the predicate blind to the slice arm. Every blocking finding is fixed
in-batch with reproductions re-run and pins added; the memory audit found no phase-introduced
defect; pre-existing classes outside the phase's rows are filed (DEF-130 – DEF-137). Full
`make check` green (1510 tests); the specification suite runs green under `RDC_ASAN=1`.

**Deliverables**:
- Implementation pins for Phase 1
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 2: Strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120, 128)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + emission-integrity auditor (codegen and symbol-name changes;
> AR-B16)
> **Reasoning**: high — emitted-C contracts and order changes

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-strict-build-emission.md) §Proposed Changes ·
[AR-B13](00-ambiguity-register.md)
**Objective**: Pin each strict-build shape and the bounded-field run before any implementation.

- [x] 2.1.1 [spec-author] Write specification tests from ST-8 – ST-16 — `compiler/tests/eval_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 14:05)
- [x] 2.1.2 Run the spec suite and record the red phase (expected: ST-8, ST-10 – ST-16 fail against the strict build or run behavior; ST-9 reproduces the recorded shape first) ✅ (completed: 2026-10-10 14:08)

**Red-phase evidence (2026-10-10)** — 8 of the 9 new cases failed, the pointer-element control passed:
ST-8 `request for member 'len' in something not a structure or union`; ST-9 the same member
error on the named local and the call result; ST-10 the compound-literal assertion (the program
built, but `(ClP){0}` violates the contract); ST-11 `unknown type name 'bool'`; ST-12
`incompatible type for argument 1 of 'RawSecond'`; ST-13 `'__cl_ret' redeclared as different kind
of symbol`; ST-14 `array type has incomplete element type 'ClP'`; ST-16 `implicit declaration of
function 'call'`. ST-9's recorded `codegen_run_spec_test__036` ordinal does not map; the class was
reproduced as the `.len` member on a fixed array (the named local and the call result), which is
the same structural-length root as DEF-76.

**Deliverables**:
- Specification suite with ST-8 – ST-16
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-strict-build-emission.md) §Proposed Changes
**Objective**: Fix each emission shape; reproduce DEF-97 first and fix it with DEF-76 when the
root matches.

- [x] 2.2.1 Reproduce the DEF-97 shape from `codegen_run_spec_test`, then skip the owned `len` binding for a fixed-array argument — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-10 14:52)
- [x] 2.2.2 Emit the struct zero-helper family after the struct definitions and return it from `empty_return_value` — `compiler/src/codegen/program.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-10-10 14:55)
- [x] 2.2.3 Provide the boolean type in the freestanding preamble — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-10 14:57)
- [x] 2.2.4 Coerce a view argument to its pointer for an uncontracted extern parameter — `compiler/src/codegen/calls.rs` ✅ (completed: 2026-10-10 14:59)
- [x] 2.2.5 Seed the generated spellings into the name de-dup pass — `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-10 15:06)
- [x] 2.2.6 Split the slice-typedef emission (pointer-element before the struct definitions, complete-row-element after) and revise the order record — `compiler/src/codegen/program.rs`, `compiler/src/codegen.rs` ✅ (completed: 2026-10-10 15:10)
- [x] 2.2.7 Resolve the bounded field call through the instance field/closure call path — `compiler/src/own/call.rs` or `compiler/src/codegen/expr.rs` (chosen by probe) ✅ (completed: 2026-10-10 15:13)
- [x] 2.2.8 Run the spec suite, `./spec/check.sh`, and the independently affected recorded suites to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 15:22)

**Implementation evidence (2026-10-10)** — the structural `len` fix landed as a small family: the
own layer lowers a fixed-array `.len` (member, qualified name, and both `len` argument paths) to
its compile-time extent, so nothing binds and nothing evaluates; the emitter folds the same extent
for any residual member or place spelling. DEF-120's split defers only the row-element typedefs
that name a struct by value (pointer-element typedefs keep their place), and the generic instances
follow the same rule. Full `make check` green (1519 tests); `./spec/check.sh` OK (27 files, 108
compile-fail tests); `codegen_run_spec_test` 54/54 and `freestanding_spec_test` 16/16 green.

**Deliverables**:
- The seven emission fixes
- Green suites and frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`; `make -C docker exec CMD="cargo test --test codegen_run_spec_test --test freestanding_spec_test"`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-strict-build-emission.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [x] 2.3.1 Write the Phase 2 implementation pins (the absent `__cl_owned` binding, the `cl_zero_*` spellings, the seeded de-dup spelling, the typedef order, the bound-field call shape, byte-identity where unaffected) — `compiler/tests/eval_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 15:32)
- [x] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 14:40)

**Phase 2 review evidence (2026-10-10)** — three dispatches per the AR-B16 matrix (correctness,
emission-integrity, memory-safety) plus one scoped re-review. The full review found two
regressions (the DEF-98 coercion wrapped an already-converted cstring literal; a structural-`.len`
store was silently dropped) and a residual fold gap (the `len(xs)`/`xs.len()`/projection
spellings); the scoped re-review confirmed the fixes and found two adjacent shapes (value-position
`.len` updates and address-of emitted invalid C; a func-typed `len` field was misrouted). Every
blocking finding is fixed in-batch with pins, one deliberate spelling pin moved with its fix, and
the pre-existing classes are filed (DEF-138 – DEF-144). The emission auditor's 3543-pair sweep
shows every corpus diff attributable and the strict builds failure-free for every accepted unit;
the memory audit's 43 ASan probes are clean. Final: 18 specification cases, 31 implementation
pins, `./spec/check.sh` OK, full `make check` green (1528 tests).

**Deliberate pin updates (recorded)**: the spec suite's ST-9 extension (the three `.len`
spellings; exit 32 → 38 — the pre-fix program still exits 32 on head and the extension is
base-red) and `array_value_integrity_impl_test.rs`'s `ai6` spelling (the folded `usize` literal's
`u` suffix).

**Deliverables**:
- Implementation pins for Phase 2
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 3: Value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (checker and typing changes; AR-B16)
> **Reasoning**: high — language-rule judgments and one new diagnostic

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-value-typing-conformance.md) §Proposed Changes ·
[AR-B4, AR-B6, AR-B7, AR-B8, AR-B9](00-ambiguity-register.md)
**Objective**: Land the approved `E10043` row and golden, then pin the corrections before any
implementation.

- [x] 3.1.1 [spec] Add the `E10043` registry row (`spec/14-diagnostics.md`), the golden `spec/tests/compile-fail/E10043_str_loop.clo`, and the compile-fail driver registration (AR-B6) ✅ (completed: 2026-10-10 15:55)
- [x] 3.1.2 [spec-author] Write specification tests from ST-17 – ST-25 — `compiler/tests/eval_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 16:10)
- [x] 3.1.3 Run the spec suite and `./spec/check.sh`; record the red phase (expected: ST-17, ST-18, ST-21 – ST-25 fail or reject wrongly; ST-19/20 and the controls pass as deliberate pins) ✅ (completed: 2026-10-10 16:12)

**Red-phase evidence (2026-10-10)** — `./spec/check.sh` OK with the new golden (109 compile-fail
tests; the `E10043_str_loop` golden waits for the implementation). All nine ST-17 – ST-25 cases
fail at HEAD, and the two controls pass in isolation:
ST-17 returns the fallback 3 where 7 is expected (the projection folds to `(i32)(0)`); ST-18 returns
0 where the element count 2 is expected (the same fold on a growable `Array` field); ST-19 and
ST-20 fail the strict build on `int x = NULL` (`int-conversion`); ST-21/22 (view equality) and
ST-23/24 (the `str` loop) and ST-25 (the bare-parameter member call) are accepted and emit C the
compiler rejects (or nothing meaningful). The plan's note that ST-19/20 pass at HEAD was written
before the declaration spelling was confirmed; both are red on the declaration, which is the
phase's DEF-108 fix. Controls: the fixed-array/slice loop program exits 12 and the concrete +
callable-bound member program exits 19.

**Deliverables**:
- The registry row and golden
- Specification suite with ST-17 – ST-25
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-value-typing-conformance.md) §Proposed Changes
**Objective**: Resolve the substituted projection, keep the nullable inference building, and land
the three rejections.

- [x] 3.2.1 Reproduce DEF-99 and fix the `len` fold (with DEF-111 when they share the root) — `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-10-10 16:42)
- [x] 3.2.2 Resolve the generic field projection through the substituted base — the value path per the chosen lever ✅ (completed: 2026-10-10 16:42)
- [x] 3.2.3 Keep the nullable inference and make the declaration and `null` spelling agree — `compiler/src/sema/stmt.rs`, `compiler/src/codegen/types.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-10 16:50)
- [x] 3.2.4 Reject `str`/`static str` pair equality with `E10222` (null-pair exemption and `cstring` stay) — `compiler/src/sema/ops.rs` ✅ (completed: 2026-10-10 16:52)
- [x] 3.2.5 Reject a `str` element loop with `E10043` — `compiler/src/sema/stmt.rs` or `compiler/src/own/loops.rs` ✅ (completed: 2026-10-10 16:53)
- [x] 3.2.6 Reject a member call on a bare-parameter receiver with `E10212` — `compiler/src/sema/call.rs` ✅ (completed: 2026-10-10 16:54)
- [x] 3.2.7 Run the spec suite and `./spec/check.sh` to green (ST-17 – ST-25 and the golden); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 17:02)

**Implementation evidence (2026-10-10)** — DEF-99 and DEF-111 are one root: the projected read
carries the opaque type and the emitter's `len` arm fell to the zero fallback. The fix resolves a
value's type after substitution (`resolved_value_ty`) by walking member, index, and projected-place
chains through the same field lookup the place renderer uses, so `(*b).s` resolves to the instance
field (`(cl_string_view((*b).s)).len`) and the growable `Array` field reads `.count`. DEF-108 keeps
the nullable inference and makes `c_type`/`empty_state` agree for the null literal's shape (`void *`
with `NULL`), and the null-shaped operand in a view pairing reads the local against the view's
pointer. DEF-107 removes `str`/`static str` from `comparable` (the frozen operator table; the
null-pair exemption and `cstring` stay). DEF-118 reports `E10043` for any non-array iterable in the
element-loop check (fixed arrays and slices are untouched). DEF-129 reports `E10212` for a member
callee on a bare `Param` receiver, at the callee's span. One deliberate unit-test update: the sema
test that asserted view equality now asserts the rejection and keeps the null pairing. Full
`make check` green (1538 tests); the `E10043` golden green in the compile-fail suite; ST-17 – ST-25
green.

**Deliverables**:
- The typing/projection fixes and the three checker rejections
- Green spec suite and frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-value-typing-conformance.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [x] 3.3.1 Write the Phase 3 implementation pins (the per-instance projection spelling, unchanged concrete emissions, the one-report counts and spans for E10222/E10043/E10212, the null-local declaration, byte-identity of the controls) — `compiler/tests/eval_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 17:20)
- [x] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 18:05)

**Phase 3 review evidence (2026-10-10)** — two dispatches per the phase's lens (correctness +
semantics) plus one scoped re-verification. Both reviews confirmed four of the five judgments and
found one blocking scope defect: the new `E10043` guard rejected `Param`/`Unknown` iterables at the
declaration, regressing generic element loops instantiated at arrays and slices (accepted and
running on the base). The fix keeps those iterables on their accepted path and rejects only
concrete non-iterable kinds; the scoped re-verification found no findings and confirmed the
concrete-kind parity, the once-per-site counts, and the base-identical residual (DEF-145). The
deliberate unit-test update (the sema view-equality test) is the only test change that
accommodates the new behavior. Seven pre-existing classes are filed (DEF-145 – DEF-151). Final:
27 specification cases, 34 implementation pins, `./spec/check.sh` OK, full `make check` green
(1541 tests).

**Deliverables**:
- Implementation pins for Phase 3
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 4: Write-mutability rules (DEF-119, DEF-124)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (language-rule changes; AR-B16)
> **Reasoning**: high — user-ruled write checks and one registry wording

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-write-mutability-rules.md) §Proposed Changes ·
[AR-B10, AR-B11](00-ambiguity-register.md)
**Objective**: Land the approved `E10042` wording and pin the six cases before any implementation.

- [x] 4.1.1 [spec] Amend the `E10042` registry row wording (`spec/14-diagnostics.md`) and write specification tests from ST-26 – ST-31 — `compiler/tests/eval_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 18:30)
- [x] 4.1.2 Run the spec suite and `./spec/check.sh`; record the red phase (expected: ST-26, ST-27, ST-29 fail — accepted today; ST-28 and the controls pass as deliberate pins) ✅ (completed: 2026-10-10 18:32)

**Red-phase evidence (2026-10-10)** — `./spec/check.sh` OK with the amended `E10042` row. ST-26
(both the fixed-array and slice const spellings), ST-27, and ST-29 are accepted at HEAD; ST-28
already reports `E10042` (the value-update pin) and the ST-30/ST-31 controls pass. One spelling
finding: a module-level `const xs: u8[2] = [1, 2];` is rejected at its initializer (`E10224`, the
array literal is not a constant expression), so the const cases use a local `const` binding — the
taxonomy is the same (the const check reads the binding, not its position). The `cstring` write
case is worth noting: the accepted program builds and segfaults at run time when the `cstring`
names a `static str` literal's bytes, which is exactly the class the ruling rejects.

**Deliverables**:
- The registry wording
- Specification suite with ST-26 – ST-31
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 4.2: Implementation

**Reference**: [03-04](03-04-write-mutability-rules.md) §Proposed Changes
**Objective**: Route the value-position updates through the statement checks and add `cstring`
to the immutable-view predicate.

- [x] 4.2.1 Route `check_update_value_write` through `read_place`+`write_place` (locals) and the const/immutable checks (globals), one report per site — `compiler/src/own/expr.rs` ✅ (completed: 2026-10-10 18:47)
- [x] 4.2.2 Add `TyKind::CString` to `report_immutable_view_write` — `compiler/src/own/place.rs` ✅ (completed: 2026-10-10 18:48)
- [x] 4.2.3 Run the spec suite and `./spec/check.sh` to green (ST-26 – ST-31); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 18:52)

**Implementation evidence (2026-10-10)** — the value-position update now runs the statement
taxonomy: locals through `read_place` then `write_place` (the immutable `const` report, the
read-only-locals through `write_place` and the immutable-view report, plus the place-state reads),
globals through the `const` check with its through-`raw` carve-out and the immutable-view check.
`report_immutable_view_write` now matches `TyKind::CString`. The four rejection shapes re-run:
`const a: u8[2]` value update `E10023`; `const s: u8[]` value update `E10023`; the unmarked
parameter `E10024`; the `cstring` element write `E10042`; the writable slice value update stays
accepted and runs (exit 3). One report per site; `./spec/check.sh` OK; 33 specification cases
green; full `make check` green (1547 tests) with no other suite affected.

**Deliverables**:
- The write-check routing and the predicate extension
- Green spec suite and frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 4.3: Implementation Tests & Hardening

**Reference**: [03-04](03-04-write-mutability-rules.md) §Testing Requirements
**Objective**: Pin the internals and record the phase review evidence.

- [x] 4.3.1 Write the Phase 4 implementation pins (the exact code per shape, one report per site, the registry wording, byte-identity of the accepted controls) — `compiler/tests/eval_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 19:05)
- [x] 4.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 19:32)

**Phase 4 review evidence (2026-10-10)** — two dispatches per the phase's lens (correctness +
semantics). No critical or major findings. Both reviewers confirmed statement/value parity for
every probed shape (E10023/E10024/E10042 with one report per site and the same precedence), the
`cstring` extension (the frozen view-mutability table already declares `cstring` immutable), the
ruling fidelity (exactly the ruled routing, no emission change — all accepted programs are
byte-identical), and the test integrity (the new cases are red on the base). Three pre-existing
classes are filed: DEF-152 (module-initializer value updates bypass the own-stage checks — the
semantics reviewer's repro accepts a const mutation and emits invalid C for a `str` write),
DEF-153 (a moved/uninitialized update reports its state error more than once — the statement form
already did at base; the value form now matches), and DEF-154 (`toCString` accepts only `String`
where the frozen appendix documents `String` or `str`). The helper's doc comment was aligned with
the measured reality (one write-mutability report per update; the place-state reports follow the
statement form). Final: 33 specification cases, 37 implementation pins, `./spec/check.sh` OK, full
`make check` green (1550 tests).

**Deliverables**:
- Implementation pins for Phase 4
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`

---

## Phase 5: Ownership lifetimes and releases (DEF-95, 96, 125, 126, 127)

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (ownership,
> captures, releases; AR-B16)
> **Reasoning**: high — memory-safety consequences and capture-rule changes

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-ownership-lifetimes.md) §Proposed Changes ·
[AR-B2, AR-B12, AR-B13](00-ambiguity-register.md)
**Objective**: Pin the leak-free runs and the ruled rejections before any implementation.

- [ ] 5.1.1 [spec-author] Write specification tests from ST-32 – ST-41 — `compiler/tests/eval_emission_integrity_spec_test.rs`
- [ ] 5.1.2 Run the spec suite and record the red phase (expected: ST-32, ST-34, ST-36 and the block/conditional rejections fail; the controls pass as deliberate pins — the leak oracles are the ASan probes in the green step for the plain-run cases)

**Deliverables**:
- Specification suite with ST-32 – ST-41
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`

---

### Step 5.2: Implementation

**Reference**: [03-05](03-05-ownership-lifetimes.md) §Proposed Changes
**Objective**: Close each memory hole and the capture-duty gaps.

- [ ] 5.2.1 Apply the func-value call's own modes and materialize owned temporary arguments — `compiler/src/own/call.rs`, `compiler/src/own/call_check.rs`
- [ ] 5.2.2 Suppress the not-lend duty for capture bindings and record the capture-move duty at the block return and take sites; update the closure batch's block-body pin deliberately — `compiler/src/own/place.rs`, `compiler/src/sema/stmt.rs`, `compiler/src/sema/call.rs`, `compiler/src/sema/expr.rs`
- [ ] 5.2.3 Recurse conditional arms in the capture rejection and the recorder with per-site dedupe — `compiler/src/sema/expr.rs`
- [ ] 5.2.4 Arm the superseded-result gate for an ordinary store whose deferred scopes can preempt (byte-identity where unreachable) — `compiler/src/codegen/cleanup.rs`
- [ ] 5.2.5 Give a view bound from an owned projection temporary a binding-scoped owner, with `StructLit` descent in the temp-owner walk — `compiler/src/own/statements.rs`, `compiler/src/own/lower.rs`
- [ ] 5.2.6 Run the spec suite to green (ST-32 – ST-41) and the host ASan probes for the DEF-95, DEF-96, DEF-125, and DEF-127 shapes; on any failure STOP and fix the implementation, never the test

**Deliverables**:
- The five ownership/release fixes
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"`; the host ASan probes are the memory oracle (emit, `cc -std=c99 -g -fsanitize=address -no-pie`, run with leak detection; AR-B14)

---

### Step 5.3: Implementation Tests & Hardening

**Reference**: [03-05](03-05-ownership-lifetimes.md) §Testing Requirements
**Objective**: Pin the internals, record the phase review evidence, and close the plan.

- [ ] 5.3.1 Write the Phase 5 implementation pins (the owner binding and release counts, the preemption-edge drop shape, the argument materialization shape, the duty/report counts, byte-identity where pinned) — `compiler/tests/eval_emission_integrity_impl_test.rs`
- [ ] 5.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode; run `make check` and the `make verify` legs; record the DEF-103 closure (marker site, recursion, top-level analogue, fail-closed proof) and sync the roadmap to Done

**Deliverables**:
- Implementation pins for Phase 5
- Review evidence recorded; `make check` and `make verify` green
- DEF-103 closed with its evidence; roadmap synced

**Verify**: `make -C docker exec CMD="cargo test --test eval_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test eval_emission_integrity_impl_test"`; `make check`; `make verify`

---

## Dependencies

```
Phase 1 (init emission + evaluation)
    ↓  (the init path's fixes precede the emission-shape work)
Phase 2 (emission shapes; DEF-128 shares the call-emission surface with P1's DEF-121 fix)
    ↓  (independent of P3–P5 code surfaces; the spec suite accumulates)
Phase 3 (typing/checker; DEF-99/111 share the fold root)
    ↓  (independent surfaces)
Phase 4 (write rules)
    ↓
Phase 5 (ownership/releases; DEF-126 extends the closure batch's duties)
```

The phases are independent in behavior; the sequence keeps the shared suite file and the
`codegen/expr.rs`/`own/lower.rs` surfaces touched by one phase at a time.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All five phases completed (49/49 tasks)
2. ✅ All verification passing (`make check` per phase; `make verify` at close-out)
3. ✅ No warnings/errors (strict builds for every covered emission; the corpus legs green)
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Memory hardening — host ASan probes clean for DEF-95, 96, 121, 125, 127
6. ✅ Documentation — the approved registry edits and goldens; plain-language comments updated
7. ✅ Code reviewed per the repository's phase-review dispatch (AR-B16)
8. ✅ DEF-103 closed with its evidence; roadmap rows at Done; post-completion project re-analysis
   (handled by the exec-plan skill)
