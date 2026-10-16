# Execution Plan: Residual Major Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-16 23:10
> **Progress**: 11/39 tasks (28%) — Phase 2 in review
> **Planning baseline**: `80433a6` (the batch-J close-out commit)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 20 open 🟠 major rows — 19 red and `DEF-209` verified fixed at the planning head — in
six phases by origin machinery (initializer call/guard/view conditionals; element
placeholder/inference/bounds; member typing; call conversions/`ffi`; null destinations/capture
stores; standalone emission shapes) and a close-out. Every fix extends existing machinery; every
phase follows the specification-first ordering (spec cases → red → implement → green → impl
pins → review). The new cases append to the origin suites (AR-9); the probe-verified controls
(`DEF-209`, the element direct read, the `.len` twin, the concrete `E10216`) stay green as
pinned controls.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Initializer call resolution, guards, and nullable view conditionals | 6 |
| 2 | Generic element placeholder, inference, and bounds | 6 |
| 3 | Member typing: enum value form and view members | 6 |
| 4 | Call conversions and `ffi` result typing | 6 |
| 5 | Null-shaped destinations and capture slot stores | 6 |
| 6 | Standalone emission shapes | 6 |
| 7 | Close-out | 3 |

**Total: 39 tasks across 7 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp.
> 2. **On verify pass:** promote it to `[x]` with a timestamp.
> 3. **Update the Progress header** and the Last Updated stamp after EVERY task.
> 4. **Resume** by scanning top-to-bottom: the first `[~]` first, else the first `[ ]`.
> 5. **On blocker:** mark `[!]` with a short reason.

---

## Phase 1: Initializer call resolution, guards, and nullable view conditionals

> **Phase baseline tree**: `d949f7f` (the phase-1 red-cases baseline)
> **Lenses**: correctness + emission-integrity + memory-safety (AR-10) — the read-only
> `emission-integrity-auditor` and `memory-safety-auditor` join the phase
> **Reasoning**: high — the initializer lowering, the init-owner release scope, and the
> conditional view arms; mirror the body contracts, never re-architect

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-init-call-guards-view-conditionals.md) ·
[07](07-testing-strategy.md) ST-1..ST-6 · AR-3, AR-5
**Objective**: Pin the expression callee, the inferred generic instance (both spellings), the
guarded bind's release scope, the nullable view conditional's null arm, and the `DEF-209`
control.

- [x] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/module_initializer_integrity_spec_test.rs` (initcallexpr, initgenericinfer, initguardrelease, initviewinfer, nullarmview, memberprojcontrol) with their controls ✅ (completed: 2026-10-16 17:10; six cases appended to `compiler/tests/module_initializer_integrity_spec_test.rs` (+254 lines, additions only; 44 tests total) — the expression callee (both spellings, the emitted init collapsing to the argument), the inferred generic call (`cl_…_pick` unkeyed → implicit declaration), the guarded owned bind inside its block (`'__cl_itmp1' undeclared`), the inferred generic view builder (the unkeyed `viewOf` + invalid initializer), the nullable view conditional (module and body; the null arm segfaults), and the `DEF-209` member-projection control (green); every accepted program folds a read-back)
- [x] 1.1.2 Run the suite and record the red phase (the five reds re-measured; the `DEF-209`
      control verified green first — AR-3/AR-5) ✅ (completed: 2026-10-16 17:10; plain 39 passed / 5 failed and ASan 39 passed / 5 failed — the five reds (ST-1..ST-5); the `DEF-209` control green; parent-verified both legs)

### Step 1.2: Implementation

- [x] 1.2.1 Resolve the initializer expression callee like the body path; record the inferred
      generic instance (the initializer call and the view-builder spelling); construct the
      per-arm view for a nullable view conditional (the null arm carries the zero view); keep
      the guarded owned bind's release inside its declaration block — `compiler/src/codegen/program.rs`,
      `compiler/src/own/generics.rs`, `compiler/src/own/call.rs`, `compiler/src/codegen/expr.rs`,
      `compiler/src/codegen/function.rs`, `compiler/src/sema/call.rs` ✅ (completed: 2026-10-16 18:00; the expression callee lowers as the call receiver (`codegen/program.rs:2436`); the inferred generic instances are keyed at the initializer top level (`own/generics.rs:421/488`, `codegen/program.rs:2496`, `sema/call.rs:882`), with `instance_keyable` (`own/generics.rs:192`) rejecting type parameters, placeholders, and unstamped closure types; the guarded owned bind releases inside its decision block (`codegen/program.rs:1893-1902`); the nullable view conditional keeps its per-arm nullness (`codegen/program.rs:1055`, `codegen/function.rs:2065/2077`, `codegen/expr.rs:462`, `own/lower.rs:3520/4376`))
- [x] 1.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-16 18:00; the module-initializer spec 44/44 plain and under `RDC_ASAN=1`; the impl pins 19/19; the batch-I/J suites and the oracles green; `make check` exit 0; the differential shows 19 intended emission diffs and no accept/reject or diagnostic change; parent-verified 44/44 plain + ASan)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the keyed callee/instance spellings, the
      per-arm view construction, the release-in-block shape, the byte-identical explicit-type
      control, the re-emission determinism pin) —
      `compiler/tests/module_initializer_integrity_impl_test.rs` ✅ (completed: 2026-10-16 18:30; six pins added (25 tests total, additions only) — the expression-callee receiver call, the keyed inferred instances (the call and the view builder with the typed temporary), the in-block guarded release, the per-arm nullable view construction (module and body), and a discriminating re-emission pin; all six fail at `d949f7f` for the intended reason; 25/25 green, `make check` exit 0 (one pre-existing dead-code warning cleared))
- [x] 1.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses (correctness + emission-integrity + memory-safety; the reviewer
      packet includes the blast-radius step and the test-integrity step) ✅ (completed: 2026-10-16 21:00; all three lenses dispatched plus one correction round: K1R-1/F-1 (the receiver-inclusive inference at both sites) fixed — the wrong key and the two spurious instances gone, `d02` builds and runs; K1R-2 filed (`DEF-336`) and the memory lens's four pre-existing siblings filed (`DEF-335`–`337`, `DEF-338`); K1R-3/K1R-4 recorded (the baseline, the status, the wider 27-delta accounting); the closing verification: the module-initializer spec 45/45 plain and ASan, the impl pins 26/26, `make check` exit 0, 0 head-only strict failures, determinism 3,663/3,663)

**Verify**: `make check`; the ownership cases under `RDC_ASAN=1`; the strict-build probes

**Deliverables**:
- [ ] ST-1..ST-6 red then green (`DEF-209` as the green control)
- [ ] The origin suites and the pinned oracles stay green

---

## Phase 2: Generic element placeholder, inference, and bounds

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + emission-integrity + memory-safety (AR-10)
> **Reasoning**: high — the element type binding is a language-rule change; the instance key
> derivation, the capture environment, and the bounds guard follow it

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-element-placeholder-inference.md) ·
[07](07-testing-strategy.md) ST-7..ST-10 · AR-3, AR-5
**Objective**: Pin the element member read through a generic callee, the closure capture's
environment field, the placeholder/null-shape instance resolution, and the row-element index
judgment with their controls.

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (elementmembercall, elementcapture, elementinfer, rowindex) with their controls ✅ (completed: 2026-10-16 22:00; fourteen tests appended to `compiler/tests/checker_conformance_spec_test.rs` (+455 lines, additions only; 74 → 88 tests) plus one trap helper — the loop-element member read through a generic callee (`cl_id__unknown`) with the direct-read control, the element-typed capture environment with the concrete control, the element-placeholder inferred argument (`cl_inner__unknown`) with the concrete and explicit-null controls, and the row-index bounds (the constant `E10216` and the runtime `E11211` trap) with four controls incl. the in-bounds guard-non-fire and the concrete trap twin; every accepted program folds a read-back)
- [x] 2.1.2 Run the suite and record the red phase (the direct-read, concrete-index, and
      concrete-instance controls verified green first — AR-3/AR-5) ✅ (completed: 2026-10-16 22:00; plain 82 passed / 6 failed and ASan 82 passed / 6 failed — the six reds (the two implicit `__unknown` symbols, the capture type error, the accepted null-shaped inferred argument, the unguarded constant index, the missing runtime trap with the ASan overflow); the eight controls green; parent-verified both legs)

### Step 2.2: Implementation

- [x] 2.2.1 Bind the loop element in the type stage (`Param(element_param(T))`); extend the
      inference and key derivation with the `#element of T → arg.iterated_element()` mapping;
      resolve the null shape before the key derives; register the opaque-base index judgment
      and emit it on the substituted row — `compiler/src/sema/stmt.rs`,
      `compiler/src/sema/expr.rs`, `compiler/src/own/loops.rs`, `compiler/src/codegen.rs`,
      `compiler/src/types/keys.rs`, `compiler/src/own/instance_check.rs`,
      `compiler/src/codegen/guards.rs` ✅ (completed: 2026-10-16 22:45; the walk and the emitter resolve a projected-place argument at the instance (`own/generics.rs:994/1023/1055`, `codegen/expr.rs:728`); a bare-parameter iterable's element binds as the element of its array type (`sema/stmt.rs:209`), so the capture environment carries the element struct; the element placeholder maps through the shared `instance_bindings` (`types/mod.rs:710`, `codegen.rs:740`) and an inferred parameter resolved to the null shape reports the frozen null rule (`own/call_check.rs:403`); an opaque-base index is recorded (`sema/mod.rs:83/126`, `sema/expr.rs:640`) and resolved per instance (`own/instance_check.rs:1002`), so the constant out-of-range row reports `E10216` and the runtime index traps `E11211`; the walk's value resolver still declines on a runtime-index temporary and a non-local place base (recorded as left))
- [x] 2.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-16 22:45; the checker-conformance spec 88/88 plain and under `RDC_ASAN=1`; the impl pins 25/25; the suite family green; `make check` exit 0; the differential shows 9 intended emission diffs and 4 head-only rejects (the two new rejections), no head-only accepts; parent-verified 88/88 plain + ASan)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the element declarations and borrow address
      temporary, the keyed instance symbols, the environment field declarator, the guard
      condition, the re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-16 23:10; five pins added (30 tests total, additions only) — the projected-place argument keying (the keyed symbol present, the opaque key absent), the element-typed capture environment, the placeholder inference keying, the runtime row guard with the constant `E10216` rejection, and a discriminating re-emission pin; all five fail at `15051a3` for the intended reason; 30/30 green, `make check` exit 0)
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the memory lens probes the element reads/captures on both paths

**Verify**: `make check`; the element cases under `RDC_ASAN=1`; the strict builds

**Deliverables**:
- [ ] ST-7..ST-10 red then green (or the pinned report outcome where the case is a rejection)
- [ ] The element-loop origin pins and the `checker_conformance` oracle stay green

---

## Phase 3: Member typing: enum value form and view members

> **Phase baseline tree**: `15051a3` (the phase-2 red-cases baseline)
> **Lenses**: correctness + semantics + emission-integrity (AR-10)
> **Reasoning**: medium — the member dispatch's enum arm and the typed view members; the
> scoped forms and the `toCString` acceptance are the pinned controls

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-member-typing.md) · [07](07-testing-strategy.md) ST-11..ST-12 ·
AR-6
**Objective**: Pin the value-form enum member rejection on both paths, the `.ptr`/`.len` view
member typing, and the `toCString` acceptance.

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (enumvaluemember, viewmembers) with the scoped/`.len`/`toCString` controls
- [ ] 3.1.2 Run the suite and record the red phase (the scoped enum, `.len`, and `toCString`
      controls verified green first)

### Step 3.2: Implementation

- [ ] 3.2.1 Report the value-form enum member read with `E10212` on the concrete and instance
      paths (the scoped form unchanged); type `.ptr` as the view pointer and `.len` as `usize`
      in the member dispatch and judge the conversions, keeping the `cstring` pointer-member
      conversion — `compiler/src/sema/expr.rs`, `compiler/src/own/expr.rs`,
      `compiler/src/own/instance_check.rs`, `compiler/src/codegen/function.rs`
- [ ] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the rejection spans on both paths, the typed
      pointer-member conversion emission, the member controls byte-identical, the re-emission
      determinism pin) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses

**Verify**: `make check`; the pinned `checker_conformance` phrase/conversion cases

**Deliverables**:
- [ ] ST-11..ST-12 red then green (or the pinned reject/accept outcome)
- [ ] The stdlib and the `checker_conformance` oracle stay green

---

## Phase 4: Call conversions and `ffi` result typing

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + semantics + emission-integrity (AR-10)
> **Reasoning**: high — the per-instance conversion duty and the `ffi` result type; the legal
> instances are the bounded surface (AR-8)

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-call-conversions-ffi-typing.md) ·
[07](07-testing-strategy.md) ST-13..ST-14 · AR-8
**Objective**: Pin the conversion duty's offending instances, the legal-instance controls, and
the `ffi` result's declared type.

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (genericconvert, ffiresult) with the legal-instance and valid-use controls
- [ ] 4.1.2 Run the suite and record the red phase (the legal instances verified green first;
      the `DEF-242` stop condition measured — AR-8)

### Step 4.2: Implementation

- [ ] 4.2.1 Record the conversion duty at the parameter-typed argument/return/declaration
      sites and judge it per instance (the iterable-duty shape); carry the `ffi` declaration's
      resolved return type into the call — `compiler/src/sema/call.rs`,
      `compiler/src/sema/mod.rs`, `compiler/src/own/call_check.rs`,
      `compiler/src/own/instance_check.rs`
- [ ] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park
      the row with the evidence if the stop condition fires)

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the duty records, the per-instance report
      counts and spans, the emitted `ffi` prototype unchanged, the accepted-instance
      emissions byte-identical, the re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; record any park in the ledger

**Verify**: `make check`; the ffi/intrinsic origin cases

**Deliverables**:
- [ ] ST-13..ST-14 red then green (or the registered park)
- [ ] The legal instances and the `checker_conformance` oracle stay green

---

## Phase 5: Null-shaped destinations and capture slot stores

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + memory-safety + semantics (AR-10)
> **Reasoning**: high — a lowering change at every destination and a double-free store
> judgment; the stop conditions (AR-9/AR-13) apply

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-null-destinations-capture-stores.md) ·
[07](07-testing-strategy.md) ST-15..ST-16 · AR-9, AR-13
**Objective**: Pin the null-shaped value at the view/handle destinations (host and
hosted-amiga) and the capture source at the nullable out-slot store.

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (nullshape, captureslot) with the pinned null controls and the non-capture out-slot store
- [ ] 5.1.2 Run the suite and record the red phase (the comparison/copy/literal controls
      verified green first; the ASan red re-measured)

### Step 5.2: Implementation

- [ ] 5.2.1 Lower the null shape to the destination's null value at the store/argument/return
      positions (views and handles; the hosted-amiga handle included); judge the capture move
      at the out-slot store instead of exempting it — `compiler/src/codegen/expr.rs`,
      `compiler/src/codegen/function.rs`, `compiler/src/codegen/types.rs`,
      `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs`
- [ ] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park
      per AR-13)

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the zero-view/`NULL` spellings, the
      `BPTR`/handle spelling, the capture report span/count, the non-capture control
      byte-identical, the re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; run the hosted-amiga leg

**Verify**: `make check`; `RDC_ASAN=1`; the hosted-amiga build

**Deliverables**:
- [ ] ST-15..ST-16 red then green
- [ ] The pinned null rules stay green; the hosted-amiga leg passes

---

## Phase 6: Standalone emission shapes

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: correctness + emission-integrity + semantics (AR-10)
> **Reasoning**: high — the shim entry, the intrinsic value, the callee symbol, and a void
> judgment; each site is independent and bounded

### Step 6.1: Specification Tests

**Reference**: [03-06](03-06-standalone-emission-shapes.md) ·
[07](07-testing-strategy.md) ST-17..ST-20 · AR-7
**Objective**: Pin the func-global callee symbol, the freestanding host run and m68k link, the
fixed-array `sizeof` value, and the void lambda over an abort intrinsic.

- [ ] 6.1.1 [spec-author] Write the phase cases — `compiler/tests/callable_release_integrity_spec_test.rs` (globalcallee), `compiler/tests/freestanding_spec_test.rs` (fsentry), `compiler/tests/module_initializer_integrity_spec_test.rs` (arraysizeof), `compiler/tests/def_backlog_closeout_spec_test.rs` (voidintrinsic) with their controls
- [ ] 6.1.2 Run the suites and record the red phase (the scalar/struct `sizeof`, the
      current-module func-global, and the freestanding controls verified green first)

### Step 6.2: Implementation

- [ ] 6.2.1 Resolve the imported/qualified func-global identity for the callee spelling;
      align the stack before the `main` call in the freestanding shim; render the whole
      fixed-array type size for `sizeof`; judge a void intrinsic statement in a lambda's
      expression body — `compiler/src/codegen/expr.rs`,
      `compiler/src/codegen/function.rs`, `compiler/runtime/shim_freestanding.c`,
      `compiler/src/codegen/calls.rs`, `compiler/src/own/lower.rs`,
      `compiler/src/codegen/cleanup.rs`
- [ ] 6.2.2 Run the suites to green; on any failure STOP and fix the implementation

### Step 6.3: Impl Tests & Review

- [ ] 6.3.1 Write the phase implementation pins (the mangled callee spellings, the `_start`
      alignment shape, the emitted `sizeof` expression, the void return/trap shape, the
      byte-identical controls, the re-emission determinism pins) in the origin `_impl_test.rs`
      files
- [ ] 6.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the emission lens runs the base/head differential

**Verify**: `make check`; the freestanding host/m68k legs; the emission differential

**Deliverables**:
- [ ] ST-17..ST-20 red then green
- [ ] The shim's other freestanding cases and the origin suites stay green

---

## Phase 7: Close-out

> **Phase baseline tree**: _(recorded at phase start)_
> **Lenses**: none new (per the `AGENTS.md` dispatch matrix)
> **Reasoning**: low — verification and bookkeeping

### Step 7.1: Verification and Sync

- [ ] 7.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and
      the ASan suites; record the evidence
- [ ] 7.1.2 Update the roadmap rows: the 20 rows move to Done with per-row evidence
      (`DEF-209` recorded as incidentally fixed with its control pins); the six older parked
      rows stay open (AR-2); the batch-J follow-ons stay untouched
- [ ] 7.1.3 Record the close-out section in the findings ledger and mark the plan Complete in
      `00-index.md`

**Verify**: `make check`; `make verify`

**Deliverables**:
- [ ] The 20 rows accounted for: Done with evidence (or the registered park/control outcomes)
- [ ] The six older parked rows' rulings collected (AR-2)
- [ ] The plan marked Complete

---

## Dependencies

```
Phase 1 (init call/guard/view)
        ↓
Phase 2 (element placeholder/inference/bounds)
        ↓
Phase 3 (member typing) → Phase 4 (conversions/ffi)
        ↓
Phase 5 (null destinations/capture stores)
        ↓
Phase 6 (standalone emission shapes) → Phase 7 (close-out)
```

Arrow legend: `A → B` marks a machinery dependency (B extends the binding, key, or judgment A
touches); the vertical order is the executable sequence — the phases run 1 → 2 → 3 → 4 → 5 →
6 → 7 in order. P3/P4 share the instance machinery P2 settles; P5's store judgment follows the
duty pattern P4 exercises.

---

## Verification Commands

| Command | Purpose |
|---------|---------|
| `make check` | Per-phase gate: `./spec/check.sh` plus `cargo test --workspace` in the pinned container |
| `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"` | The P1/P6(`DEF-211`) focused suites |
| `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | The P2–P5 focused suites |
| `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test --test callable_release_integrity_impl_test --test def_backlog_closeout_spec_test --test def_backlog_closeout_impl_test --test freestanding_spec_test"` | The P6 focused suites |
| The suites' `RDC_ASAN=1` switch | The memory runs (the suite helper) |
| `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo"` + `cc -std=c99 -O0 -g -Wall -Wextra -Werror` | The strict-build probes (the pinned container) |
| `make -C docker exec CMD="cargo test --test freestanding_spec_test"` | The `DEF-189` host no-libc run and the m68k link |
| The hosted-amiga driver build (`m68k-amigaos-gcc -mcrt=nix20 ... -lamiga -lm`) | The `DEF-245` handle spelling |
| `make verify` | Close-out: every hosted-amiga leg plus the runtime-fault programs |

## Reviewer Dispatch Conventions

- Correctness and semantics via the project briefs (`.opencode/agents/correctness-reviewer.md`,
  `.opencode/agents/semantics-reviewer.md`); memory-safety and emission-integrity via the
  dedicated read-only subagents (`memory-safety-auditor`, `emission-integrity-auditor`).
- Every reviewer packet includes the **blast-radius step** (every call site, exhaustive match,
  and parallel path of the changed construct) and the **test-integrity step** (an assertion must
  fail without the fix); the read-only auditors stay read-only.
- Phase dispatch follows the change surface (AR-10) and the `AGENTS.md` matrix.

## Success Criteria

**Feature is complete when:**

1. [ ] All phases completed
2. [ ] All verification passing (`make check`; `make verify` at close-out)
3. [ ] No warnings/errors; no dead code
4. [ ] Security hardened — not applicable beyond the compiler's own input validation (unchanged)
5. [ ] Documentation updated (the roadmap rows and the findings ledger)
6. [ ] Code reviewed (every phase's declared lenses)
