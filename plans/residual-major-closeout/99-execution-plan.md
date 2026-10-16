# Execution Plan: Residual Major Closeout

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-17 21:00
> **Progress**: 36/39 tasks (92%) — Phase 6 complete
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
- [x] 2.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the memory lens probes the element reads/captures on both paths ✅ (completed: 2026-10-17 02:30; all four lenses dispatched plus two correction rounds and one bounded re-verification: the index judgment, the nested placeholder fixed point, the capture duty, the audit switch, the chained index and the conditional keying; `DEF-343` closed fixed; rows filed `DEF-339`–`DEF-345`; the closing verification: the checker-conformance spec 97/97 plain and ASan, the impl pins 30/30, `make check` exit 0, 0 head-only strict failures, determinism 2,481/2,481, the re-verification CONFIRMED with 0 findings)

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

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (enumvaluemember, viewmembers) with the scoped/`.len`/`toCString` controls ✅ (completed: 2026-10-17 03:00; five tests appended to `compiler/tests/checker_conformance_spec_test.rs` (+150 lines, additions only; 97 → 102 tests) — the concrete and instance value-form enum member reads (the strict build's `request for member 'A'`; E10212 post-fix), their scoped/qualified controls, the `.ptr` view member at an integer destination (`-Werror=int-conversion`; silently wrong in the memory leg) and the `.len` control; every accepted program folds a read-back; the `memenum` comment update delegated to the implementation)
- [x] 3.1.2 Run the suite and record the red phase (the scoped enum, `.len`, and `toCString`
      controls verified green first) ✅ (completed: 2026-10-17 03:00; plain 99 passed / 3 failed and ASan 99 passed / 3 failed — the three reds (the two enum paths and the `.ptr` conversion); the controls green in both legs, 0 sanitizer reports; parent-verified)

### Step 3.2: Implementation

- [x] 3.2.1 Report the value-form enum member read with `E10212` on the concrete and instance
      paths (the scoped form unchanged); type `.ptr` as the view pointer and `.len` as `usize`
      in the member dispatch and judge the conversions, keeping the `cstring` pointer-member
      conversion — `compiler/src/sema/expr.rs`, `compiler/src/own/expr.rs`,
      `compiler/src/own/instance_check.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-17 03:40; the enum value-form read always reports the registered `E10212` and types unknown (`sema/expr.rs:570`), with `projection_resolves` returning false for an enum substitution on the instance path (`own/instance_check.rs:1099`); `.ptr` types as `raw u8`/`raw <elem>` and `.len` as `usize` (`sema/expr.rs:583`), a byte pointer reaches `cstring` implicitly (`types/mod.rs:668`), and the store's pointer-member conversion keys on the member's type (`codegen/function.rs:2095`, the name heuristic removed); the mirrors (`own/expr.rs:792`) accept `.ptr`/`.len` on substituted views; the `memenum` comment updated (the body unchanged); left: a fixed array's `.ptr` stays untyped (pre-existing fallback))
- [x] 3.2.2 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-17 03:40; the checker-conformance spec 102/102 plain and under `RDC_ASAN=1`; the impl pins 30/30; the suite family green; `make check` exit 0; the differential shows 6 head-only rejects (the three new cases twice) and no emission or diagnostic diffs; parent-verified 102/102 plain + ASan)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the rejection spans on both paths, the typed
      pointer-member conversion emission, the member controls byte-identical, the re-emission
      determinism pin) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-17 04:10; four pins added (34 tests total, additions only) — the typed view pointer member and its conversions (the two sanctioned `(char *)(s.ptr)` occurrences; the `raw u8`→`usize` rejection once with `E10200`), the typed `.len` member, the value-form enum rejection on both paths (one `E10212` each; the scoped and qualified controls), and a re-emission pin; two of the five (the `.len` and determinism pins) are base-stable shape guards, reported; 34/34 green, `make check` exit 0)
- [x] 3.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses ✅ (completed: 2026-10-17 06:30; all three lenses dispatched plus two correction rounds and two bounded confirmations (correctness PASS WITH MINORS, 0 blockers; the semantics scope ruling implemented): the global widening reverted and the conversion scoped to the member read, the fixed-array instance guard, the instance use conversion, and the tightening to the slots and returns (the argument/FFI/struct-literal positions report `E10200`; the conversion case re-scoped per the ruling with two new rejection cases); rows filed `DEF-346`–`DEF-349`; the closing verification: the checker-conformance spec 108/108 plain and ASan, the impl pins 34/34, `make check` exit 0, the per-position matrix parent-verified)

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

- [x] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (genericconvert, ffiresult) with the legal-instance and valid-use controls ✅ (completed: 2026-10-17 07:10; ten tests (thirteen programs) appended to `compiler/tests/checker_conformance_spec_test.rs` (+408 lines, additions only; 108 → 118 tests) — the generic conversion at an FFI and an ordinary cstring argument, at a concrete return and declaration, the null-shaped return/declaration (the frozen `E10012` per the design), the FFI integer result at a cstring destination, and the legal-instance/identity/integer-destination controls; adjustments recorded (the `cl_probe_convert` ffi probe instead of the shim `Open`; the `take` identity control; the `count` parameter name); the AR-8 legal-instance controls measured green, so the duty is expressible)
- [x] 4.1.2 Run the suite and record the red phase (the legal instances verified green first;
      the `DEF-242` stop condition measured — AR-8) ✅ (completed: 2026-10-17 07:10; plain 111 passed / 7 failed and ASan 111 passed / 7 failed — the seven reds (the `ClStr`-at-`char *` arguments, the `ClStr`/`void *` at concrete returns and declarations, the pointer-from-integer destinations); the controls green in both legs, 0 sanitizer reports; parent-verified)

### Step 4.2: Implementation

- [x] 4.2.1 Record the conversion duty at the parameter-typed argument/return/declaration
      sites and judge it per instance (the iterable-duty shape); carry the `ffi` declaration's
      resolved return type into the call — `compiler/src/sema/call.rs`,
      `compiler/src/sema/mod.rs`, `compiler/src/own/call_check.rs`,
      `compiler/src/own/instance_check.rs` ✅ (completed: 2026-10-17 07:50; the per-instance conversion duty: an `OpaqueConversion` record at `check_into_position` (`sema/mod.rs:161`, `sema/expr.rs:447`) whenever the source mentions a parameter or is null-shaped, the FFI cstring-argument duty (`sema/call.rs:782`), and the instance audit + frozen report helper (`own/instance_check.rs:1182`/`:1219`) — the six conversion reds report once (`E10200`, or the frozen `E10012` for the null-shaped sources) while the legal instances stay accepted (**AR-8 stop-gate not triggered; no park**); the FFI result carries its declared type (`sema/call.rs:83`, `ffi_result_type :717`), so `Uses(Count())` reports one `E10200`; the call-site null arguments keep the pre-existing own-stage rule (the duplicate-report drift this first round had was corrected in the K4R-2 follow-up round); left: the argument cast in `codegen/calls.rs` is now unexercised in accepted emissions (a reviewer design call) and the generic member-read return still pairs `E10036` with the instance report)
- [x] 4.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park
      the row with the evidence if the stop condition fires) ✅ (completed: 2026-10-17 07:50; the checker-conformance spec 118/118 plain and under `RDC_ASAN=1`; the impl pins 34/34; the suite family green; `make check` exit 0; the differential shows 10 intended rejections, 3 intended emission diffs and 1 diagnostic diff, no head-only accepts; parent-verified 118/118 plain + ASan)

### Step 4.3: Impl Tests & Review

- [x] 4.3.1 Write the phase implementation pins (the duty records, the per-instance report
      counts and spans, the emitted `ffi` prototype unchanged, the accepted-instance
      emissions byte-identical, the re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-17 08:20; seven pins added (41 tests total, additions only) — the deferred argument/return/declaration conversions (one `E10200` with the span and text), the null shapes (one `E10012`, no `E10200`), the legal-instance emissions, the FFI prototype typing, the FFI result arithmetic typing, and a re-emission pin; four fail at `2cda910` for the intended reason, three are disclosed base-stable guards; 41/41 green, `make check` exit 0)
- [x] 4.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; record any park in the ledger ✅ (completed: 2026-10-17 10:30; all four lenses dispatched plus four correction rounds: the receiver duty (K4R-1), the null-shape destination gate (K4R-2/K4S-1), the `T?` destination skip (K4S-2), and the ABI-pointer typing for the view-typed FFI result (F1); rows filed/extended `DEF-350`–`DEF-352`; the closing verification: the checker-conformance spec 124/124 plain and ASan, the impl pins 43/43, `make check` exit 0, 0 head-only strict failures for accepted programs, the F1 repro parent-verified)

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

- [x] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` (nullshape, captureslot) with the pinned null controls and the non-capture out-slot store ✅ (completed: 2026-10-17 11:10; four tests appended to `compiler/tests/checker_conformance_spec_test.rs` (+228 lines, additions only; 124 → 128 tests) plus the hosted-amiga emit/build helpers — the null shape at a nullable view destination (the store and the argument), the hosted-amiga handle destination (the `File?` store and return), the writable capture into a nullable out-slot (one `E10401`), and the non-capture control; adjustments recorded: the ST-15 length read folds as a null-fallthrough exit 0 (the frozen TY-6 null rules reject a live read), the ST-16 control uses a `take` source (the by-value lend is the pinned `E10034`), and the read-only capture control is referenced from the existing pin)
- [x] 5.1.2 Run the suite and record the red phase (the comparison/copy/literal controls
      verified green first; the ASan red re-measured) ✅ (completed: 2026-10-17 11:10; plain 125 passed / 3 failed and ASan 125 passed / 3 failed — the three reds (the host `ClStr` store and argument from `void *`, the hosted-amiga `BPTR` from `void *`, the capture double free 134); the control green in both legs, 0 sanitizer reports beyond the intended double free; parent-verified)

### Step 5.2: Implementation

- [x] 5.2.1 Lower the null shape to the destination's null value at the store/argument/return
      positions (views and handles; the hosted-amiga handle included); judge the capture move
      at the out-slot store instead of exempting it — `compiler/src/codegen/expr.rs`,
      `compiler/src/codegen/function.rs`, `compiler/src/sema/expr.rs`,
      `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs` ✅ (completed: 2026-10-17 11:50; the typed-null lowering: a null-shaped value at a null-capable destination (views, slices, handles, `cstring`, raw pointers, `func`) lowers to the destination's own empty value through the store/return/argument/array-element paths (`codegen/expr.rs:434`, `function.rs:2084`/`:469`/`:2439`, `calls.rs`), so the host and the hosted-amiga `BPTR` spellings build and run (**AR-13 not triggered; no park**); the borrowed-capture store records into the instance audit (`sema/expr.rs:331`, `sema/ops.rs:373`), so a writable capture into a nullable out-slot reports one `E10401` (the control unchanged); left: the inferred-scalar null argument (`pair(1, x)`, the filed `DEF-344` family), the conditional null-shaped arm's pre-existing rendering, the concrete capture spellings' `E10034`)
- [x] 5.2.2 Run the suite to green; on any failure STOP and fix the implementation (or park
      per AR-13) ✅ (completed: 2026-10-17 11:50; the checker-conformance spec 128/128 plain and under `RDC_ASAN=1`; the hosted-amiga emission cross-builds clean with the container's m68k toolchain (the leg ran, not skipped); the impl pins 43/43; `make check` exit 0; the differential shows 7 intended emission diffs and 1 intended rejection, no head-only accepts; parent-verified 128/128 plain + ASan)

### Step 5.3: Impl Tests & Review

- [x] 5.3.1 Write the phase implementation pins (the zero-view/`NULL` spellings, the
      `BPTR`/handle spelling, the capture report span/count, the non-capture control
      byte-identical, the re-emission determinism pin) —
      `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-17 12:20; four pins added (47 tests total, additions only) plus a hosted-amiga emit helper — the view store/argument typed null with the conditional precedence, the hosted-amiga `BPTR` zero (four spellings), the capture-store rejection with the non-capture control, and a discriminating re-emission pin; all four fail at `8be0c9c` for the intended reason; 47/47 green, `make check` exit 0)
- [x] 5.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; run the hosted-amiga leg ✅ (completed: 2026-10-17 14:40; all three lenses dispatched plus four correction rounds and two bounded confirmations: the conditional walk alignment (K5S-1), the capture gate (K5S-2/K5R-2), the predicate narrowing (K5S-3), the nested walk skip (K5R-1), the computed null shape running (F1), and the zero-profile pointer casts (K5C-1); the final confirmation PASS — 0 blockers; rows filed `DEF-353`–`DEF-355`; the closing verification: the checker-conformance spec 133/133 plain and ASan, the impl pins 50/50, `make check` exit 0, the 12/12 profile legs, the driver builds on all three profiles)

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

- [x] 6.1.1 [spec-author] Write the phase cases — `compiler/tests/callable_release_integrity_spec_test.rs` (globalcallee), `compiler/tests/freestanding_spec_test.rs` (fsentry), `compiler/tests/module_initializer_integrity_spec_test.rs` (arraysizeof), `compiler/tests/def_backlog_closeout_spec_test.rs` (voidintrinsic) with their controls ✅ (completed: 2026-10-17 15:20; nine tests appended across four suites (+310 lines, additions only) plus two helpers — the current-module func-global control and the imported/qualified red pair (the two-file `export` form, the preflight note), the freestanding local `str[2]` host-leg red, the whole-type `sizeof` red with the scalar/struct/view/const controls, and the void-lambda abort reds (assert + panic) with the block-bodied and `return panic` controls; the reds re-derived (the `DEF-211` read-back folds exit 228); a noted head observation: the `DEF-170` pair emits `W10006: unused import 'h'` despite the call)
- [x] 6.1.2 Run the suites and record the red phase (the scalar/struct `sizeof`, the
      current-module func-global, and the freestanding controls verified green first) ✅ (completed: 2026-10-17 15:20; the four suites run with exactly the five designed reds (the freestanding 16/1, the module-initializer 46/1, the callable-release 53/1, the backlog-closeout 47/2) and every control green; parent-verified the four focused suites)

### Step 6.2: Implementation

- [x] 6.2.1 Resolve the imported/qualified func-global identity for the callee spelling;
      align the stack before the `main` call in the freestanding shim; render the whole
      fixed-array type size for `sizeof`; judge a void intrinsic statement in a lambda's
      expression body — `compiler/src/codegen/expr.rs`,
      `compiler/src/codegen/function.rs`, `compiler/runtime/shim_freestanding.c`,
      `compiler/src/codegen/calls.rs`, `compiler/src/own/lower.rs`,
      `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-10-17 16:00; the imported/qualified func-global callee resolves through the module binding path (`codegen/expr.rs:426`, the callable-signature fallback, the import-use scanner `load/scan.rs:119`; the `W10006` also gone); the freestanding `_start` realigns its frame on x86-64 (`compiler/runtime/shim_freestanding.c:311`, `force_align_arg_pointer`; m68k untouched); `sizeof` of a fixed array spells the C array type (`codegen/calls.rs:370`); a void expression-bodied lambda lowers as a statement (`own/lower.rs:1032`) so the abort fires; left: the known container-ASan harness flake in the backlog-closeout suite (pre-existing, the base flakes identically, the emissions byte-identical))
- [x] 6.2.2 Run the suites to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-17 16:00; the four suites plain all green (callable-release 54/54, freestanding 17/17, module-initializer 47/47, backlog-closeout 49/49); the load unit tests 10/10; `make check` exit 0; the differential shows three intended emission diffs (the two void-lambda traps and the whole-type `sizeof`); parent-verified the four suites)

### Step 6.3: Impl Tests & Review

- [x] 6.3.1 Write the phase implementation pins (the mangled callee spellings, the `_start`
      alignment shape, the emitted `sizeof` expression, the void return/trap shape, the
      byte-identical controls, the re-emission determinism pins) in the origin `_impl_test.rs`
      files ✅ (completed: 2026-10-17 16:40; seven pins added across three impl suites (callable-release 21/21, module-initializer 29/29, backlog-closeout 34/34; additions only) plus two helpers — the imported/qualified func-global pair (the `emit_pair` helper for the two-file form) with the current-module control, the whole-type `sizeof` with the scalar/struct/view controls and a re-emission pin, and the void expression lambda’s statement lowering with a re-emission pin; five discriminate, two are the disclosed base-stable controls; `DEF-189`’s alignment has no impl-suite home (no freestanding impl suite) — the spec case’s host no-libc run and the m68k link are the behavior pins, reported; 84/84 green across the three suites; `make check` exit 0)
- [x] 6.3.2 Phase review evidence and progress update — commit per the active commit mode;
      dispatch the lenses; the emission lens runs the base/head differential ✅ (completed: 2026-10-17 21:00; all three lenses dispatched plus three correction rounds and two bounded confirmations: the identity threading and the declarator (C1), the function preference/void scope/element call/head policy (F-a..F-e), and the scope-aware gate (K6C-1); the final re-confirmation PASS — 0 blockers; rows filed `DEF-356`–`DEF-359`; the closing verification: the phase-6 suites green, `make check` exit 0 (2243 passed), the differential clean)

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
