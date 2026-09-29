# Execution Plan: Runtime Value Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-29 22:35
> **Progress**: 20/27 tasks (74%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the seven filed runtime-value defects in three feature phases: materialization and
temporary release (DEF-24, DEF-23, DEF-25), callable-field integrity (DEF-26, DEF-27), and
the runtime boundary (DEF-28, DEF-29). Each phase follows the mandatory
specification-first ordering; the close-out phase runs the full acceptance gate, the
leak-detection probes, and the documentation/roadmap updates.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Materialization and temporary release | 8 |
| 2 | Function-value call integrity | 7 |
| 3 | Runtime boundary | 7 |
| 4 | Close-out: acceptance, probes, documentation | 5 |

**Total: 27 tasks across 4 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent
> MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** (`> **Progress**: X/Y tasks (Z%)`) and the Last
>    Updated stamp after EVERY task — never batch updates. Only `[x]` counts as complete.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is
>    resumed first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append
>    `Blocked: <short reason>` on the same line. The plan lifecycle is `Ready`,
>    `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Materialization and temporary release

> **Phase baseline tree**: 472e478872a619135a08d72eecbb85ba31b0c548
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 1.1: Specification Tests

**Reference**: [03-01 §Proposed Changes](03-01-materialization-and-temps.md) · AR-5, AR-6, AR-7, AR-16, AR-18
**Objective**: Pin the iterable materialization, the temporary release and its view lifetime rule, and the conditional stores before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from the 07 ST cases — `compiler/tests/runtime_value_integrity_spec_test.rs` (ST-1 – ST-11, ST-24 – ST-29) ✅ (completed: 2026-09-29 16:06)
- [x] 1.1.2 Run the spec suite and record the red phase — the invalid-C, missing-release, and missing-lifetime-rejection cases fail; the guard cases pass ✅ (completed: 2026-09-29 16:07)

**Deliverables**:
- [x] Spec suite with the ST-1 – ST-11 and ST-24 – ST-29 cases and the shared scratch/run helpers (including `assert_trap`)
- [x] Red-phase evidence recorded (suite output)

**Red-phase evidence** (2026-09-29 16:07, `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`):
7 passed, 10 failed. Failing: the invalid-C group (ST-1, ST-2, ST-4 – ST-8, ST-10, ST-29) and
the missing lifetime rejection (ST-25). ST-8 varies from the recorded prediction: the emitted C
compiles, but the callee's out-parameter variable receives the decayed pointer and `main` exits
`0` instead of `7` — still a red failure, with the observed variant recorded here. Passing
pre-fix (the predicted regression guards): ST-3, ST-9, ST-11, ST-24, ST-26 – ST-28.

**Verify**: `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`

### Step 1.2: Implementation

**Reference**: [03-01 §Proposed Changes](03-01-materialization-and-temps.md) · AR-5, AR-6, AR-7, AR-16, AR-18
**Objective**: Release owning destination temporaries at statement end with the temporary view lifetime rule, materialize call-rooted iterables, and store fixed-array conditionals through a branch.

- [x] 1.2.1 Add the statement-position release for owning destination temporaries (guard operands, the iterable temporary, the conditional read-position temporary) in reverse creation order; drain after each statement and before terminators; exclude the take-call and literal-adoption transfer temps; register the temporary as a loan owner so a later derived-view use reports `E10030` — `compiler/src/own/lower.rs`, `compiler/src/own/statements.rs`, `compiler/src/own.rs`, `compiler/src/own/place.rs`, `compiler/src/own/liveness.rs`, `compiler/src/ir.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-09-29 16:18)
- [x] 1.2.2 Materialize a call-rooted element-loop iterable before the loop and read the temporary (generic-parameter iterables keep the `mentions_type_param` behavior) — `compiler/src/own/loops.rs` ✅ (completed: 2026-09-29 16:18)
- [x] 1.2.3 Make the shared fixed-array value assignment branch-aware for conditionals (per-arm guards, copy for duplicable arms, move for owned arms, target-name resolution to the saved value), extend the fixed-array return emitter with the same branch shape, extend the place-projection predicate so a conditional argument with place arms stays a forwarding value, and materialize conditional values in read positions — `compiler/src/codegen/function.rs`, `compiler/src/codegen/cleanup.rs`, `compiler/src/own/lower.rs` ✅ (completed: 2026-09-29 16:18)
- [x] 1.2.4 Run the spec suite and record the green phase — ST-1 – ST-11 and ST-24 – ST-29 pass ✅ (completed: 2026-09-29 16:18)

**Deliverables**:
- [x] Call-rooted iterables compile and run; owning destination temporaries are released with the view lifetime rule; conditional fixed-array values compile in the routed positions, the return position, and as a materialized argument
- [x] No regressions; no warnings

**Green-phase evidence** (2026-09-29 16:18, `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`):
17 passed, 0 failed. The full workspace `make check` also passes with no warnings, so the
release machinery, the iterable materialization, and the conditional branch stores introduced
no regression. Two implementation findings were resolved on the way: the pre-existing
`__cl_owned` binding must not adopt a projection of a released temporary (it would double-free),
and the per-statement release floor keeps an enclosing statement's temporary pending while a
nested statement runs.

**Verify**: `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-5, AR-7, AR-16
**Objective**: Pin the emitted temporary release, iterable temporary, and branch store shapes.

- [x] 1.3.1 Add implementation tests: an inline statement-position release for an owning `__cl_tmpN` and the absence of a release for the take-call/literal-adoption transfer temps; the materialized iterable temporary; the conditional branch with per-arm guards in the assignment and return positions; the release-set unit test — `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs` ✅ (completed: 2026-09-29 16:22)
- [x] 1.3.2 Full verification — `make check` ✅ (completed: 2026-09-29 16:22)

**Deliverables**:
- [x] Implementation tests pass
- [x] No regressions; no warnings

**Full-verification evidence** (2026-09-29 16:22, `make check`): every workspace suite passed —
the new implementation suite 8/8, the runtime value integrity specification suite 17/17, and
all pre-existing suites unchanged (32 test binaries, 0 failures, 0 warnings).

> **Phase 1 review (recorded 2026-09-29 21:26)**: independent correctness/maintainability/
> standards review of the phase diff (baseline tree `472e4788`). Two critical and three major
> findings, all ruled by the user: F1 a conditional's owned-arm move was invisible to the
> checker (maybe-moved rule, AR-19; ST-6/ST-26 revised, ST-30 added), F2 the temporary view
> lifetime missed call, conditional, and return derivations (completed provenance rule),
> F3 a release was attached to one runtime path (per-path release copies), F4 a loop-header
> temporary was re-materialized without releasing the previous value (release before the new
> store), F5 a conditional argument with literal or duplicable arms (filed as `DEF-33`). The
> fixes are commits `aafa1c0` and `6b6e703`.
>
> **Phase 1 re-review (the one permitted pass, recorded 2026-09-29 21:26)**: scoped to the
> fix diff (`4ac201b..6b6e703`). F3 and F5 confirmed. F1/F2/F4 were incomplete against their
> rulings: R1 a `Materialize` wrapper hid the owning temporary below it, R2 a generic
> reborrow was not recognized as a lend source, R3 a conditional assigned to a projected
> target was unmarked, R4 a conditional of ownerless elements bypassed `E10026`, R5 the
> embedded guard spelling has no expression-form drop for a growable array. The user ruled:
> fix R1–R4; file R5 and the pre-existing findings. R1–R4 are fixed in this round and the
> reviewer's probes now reject with `E10030` (f2_slice, f2_slice_cond, f2_generic),
> `E10020` (f1_field_use), and `E10026` (f1_take_elem); the specification suite passes 18/18,
> the implementation suite 10/10, the ownership unit tests 59/59, and `make check` passes 32
> suites with 0 failures and 0 warnings. R5 is filed as `DEF-39`; the pre-existing findings
> are `DEF-34` (expanded to the scalar handle form), `DEF-37`, and `DEF-38`. No further
> independent review is permitted for this phase (one re-review); this block records the
> final dispositions.

**Verify**: `make check`

---

## Phase 2: Function-value call integrity

> **Phase baseline tree**: 452e31c896d392d469f33589369f7762d3dacc37
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 2.1: Specification Tests

**Reference**: [03-02 §Proposed Changes](03-02-callable-fields.md) · AR-8, AR-9
**Objective**: Pin the receiver bounds traps and the func-pointer ABI before any implementation.

- [x] 2.1.1 [spec-author] Add the callable-field specification tests to the suite (ST-12 – ST-17) — `compiler/tests/runtime_value_integrity_spec_test.rs` ✅ (completed: 2026-09-29 21:28)
- [x] 2.1.2 Run the spec suite and record the red phase — the trap and C-acceptance cases fail as predicted; the in-range guards pass ✅ (completed: 2026-09-29 21:28)

**Deliverables**:
- [x] ST-12 – ST-17 in the spec suite
- [x] Red-phase evidence recorded

**Red-phase evidence** (2026-09-29 21:28, `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`):
24 tests, 19 passed / 5 failed. Failing: ST-13 (the out-of-range statement call runs unguarded, so
the `E11211` trap text never appears), ST-14 and ST-15 (their result type is `func(i32): i32[2]`,
so the C build fails on the func-pointer ABI before the missing trap can surface — the observed
variant of the predicted missing-trap failure), and ST-16/ST-17 (invalid C: the func pointer lacks
the hidden destination parameter). Passing pre-fix: ST-12 (the in-range receiver call already runs
through the existing expression guard).

**Verify**: `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`

### Step 2.2: Implementation

**Reference**: [03-02 §Proposed Changes](03-02-callable-fields.md) · AR-8, AR-9
**Objective**: Guard the receiver on every call path and render the hidden out-param ABI for func values.

- [x] 2.2.1 Guard the receiver wherever arguments are guarded: the statement call path, the fixed-array call assign path, and the fixed-array return path — `compiler/src/codegen/function.rs`, `compiler/src/codegen/cleanup.rs` ✅ (completed: 2026-09-29 21:31)
- [x] 2.2.2 Render the hidden destination ABI for the filed `func` spellings with a fixed-array return and resolve call destinations for func values (including the discard shape and the local func-typed callee lookup) — `compiler/src/codegen/types.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-09-29 21:31)
- [x] 2.2.3 Run the spec suite and record the green phase — ST-12 – ST-17 pass ✅ (completed: 2026-09-29 21:31)

**Deliverables**:
- [x] Runtime-index receivers trap; func values with fixed-array returns compile and run
- [x] No regressions; no warnings

**Green-phase evidence** (2026-09-29 21:31, `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`):
24 passed, 0 failed; `make check` also passes (32 suite binaries, 0 failures, 0 warnings), so the
receiver guards and the function-pointer ABI change no existing behavior.

**Verify**: `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-8, AR-9
**Objective**: Pin the emitted receiver traps and the pointer declarator.

- [x] 2.3.1 Add implementation tests: the `E11211` trap in the three call shapes, the `__cl_ret` pointer declarator, and the func-value destination — `compiler/tests/runtime_value_integrity_impl_test.rs` ✅ (completed: 2026-09-29 21:32)
- [x] 2.3.2 Full verification — `make check` ✅ (completed: 2026-09-29 21:32)

**Deliverables**:
- [x] Implementation tests pass
- [x] No regressions; no warnings

**Full-verification evidence** (2026-09-29 21:32, `make check`): every workspace suite passed —
the implementation suite 15/15, the specification suite 24/24, and all pre-existing suites
unchanged (32 test binaries, 0 failures, 0 warnings).

> **Phase 2 review (recorded 2026-09-29 22:20)**: independent correctness/maintainability/
> standards review of the phase diff (baseline tree `452e31c8`). Three major findings, no
> critical: F1 a shadowed or sanitized func-typed local resolved to the wrong binding (the
> phase-aggravated discard shape became a silent wrong call), F2 a nested receiver was
> guarded against the product of the extents, F3 a fixed-array func parameter rendered as
> its element type. The user ruled: fix all three (commit `cf65905`).
>
> **Phase 2 re-review (the one permitted pass, recorded 2026-09-29 22:20)**: scoped to the
> fix diff (`f042a66..cf65905`). F2 and F3 verified; F1 held for locals but func-typed
> parameters still resolved to the module callee (R1, major — a new hard C error on the
> array-returning variant). Five pre-existing minors in the same family (R2–R6: captured func
> value, module func global, `String`→`str` widening through a func value, func type in
> return position, dropped `borrow` mode). The user ruled: fix R1 and file R2–R6. R1 is fixed
> in commit `efab229`; its probes now run correctly (`f1_param_shadow` 1,
> `f1_param_shadow_array` 2, `f1_param_shadow_generic` 1, `f1_param_text` 1,
> `f1_before_decl` 15, `f1_discard_mix` 5), the specification suite passes 24/24, the
> implementation suite 21/21, and `make check` passes 32 suites with 0 failures and 0
> warnings. R2–R6 are filed as `DEF-40`–`DEF-44`. No further independent review is permitted
> for this phase (one re-review); this block records the final dispositions.

**Verify**: `make check`

---

## Phase 3: Runtime boundary

> **Phase baseline tree**: bcefe96106febf637b752d491ad20ceb0c92dcc1
> **Lenses**: correctness, maintainability, standards (strict profile — independent review of the phase diff)

### Step 3.1: Specification Tests

**Reference**: [03-03 §Proposed Changes](03-03-runtime-boundary.md) · AR-10, AR-11, AR-17
**Objective**: Pin the panic message forms and the deferred guarded moves before any implementation.

- [x] 3.1.1 [spec-author] Add the runtime-boundary specification tests to the suite (ST-18 – ST-23) — `compiler/tests/runtime_value_integrity_spec_test.rs` ✅ (completed: 2026-09-29 22:26)
- [x] 3.1.2 Run the spec suite and record the red phase — the panic-message builds and the guarded-move cases fail; the evaluated guarded call passes ✅ (completed: 2026-09-29 22:26)

**Deliverables**:
- [x] ST-18 – ST-23 in the spec suite
- [x] Red-phase evidence recorded

**Red-phase evidence** (2026-09-29 22:26, `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`):
30 tests, 25 passed / 5 failed. Failing: ST-18 – ST-20 (the C build rejects the emitted program: the
diverging tail falls out of a non-void `main` as a bare `return;`, and the `str` form additionally
passes a view where `cl_panic` takes `const char *`) and ST-21/ST-23 (the guard is `false` but the
counter is `1`: the guarded call and its move ran eagerly). Passing pre-fix: ST-22 (the evaluated
guarded call already runs once). Packet notes: the `&&` operand calls return `bool` (the frozen
`spec/04` requires boolean operands; the plan row wrote the parameter list without a return type);
the panics need the diverging-tail return handled alongside the new runtime signature.

**Verify**: `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`

### Step 3.2: Implementation

**Reference**: [03-03 §Proposed Changes](03-03-runtime-boundary.md) · AR-10, AR-11, AR-17
**Objective**: Carry pointer + length for panic messages and defer guarded take move-outs.

- [x] 3.2.1 Change `cl_panic` to take pointer + length in all three shims and render the three message forms (literal, `str`, `String`) once — `compiler/runtime/shim_host.c`, `compiler/runtime/shim_hosted_amiga.c`, `compiler/runtime/shim_freestanding.c`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-09-29 22:34)
- [x] 3.2.2 Carry a take call's move-out places on the materialized value inside guarded operands and emit them only on the evaluated path — `compiler/src/own/lower.rs`, `compiler/src/codegen/guards.rs`, `compiler/src/ir.rs` ✅ (completed: 2026-09-29 22:34)
- [x] 3.2.3 Run the spec suite and record the green phase — ST-18 – ST-23 pass ✅ (completed: 2026-09-29 22:34)

**Deliverables**:
- [x] Panic messages compile and print; guarded take moves run only when evaluated
- [x] No regressions; no warnings

**Green-phase evidence** (2026-09-29 22:34, `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`):
30 passed, 0 failed; `make check` also passes (32 suite binaries, 0 failures, 0 warnings).
Besides the two planned changes, the diverging `panic` tail needed the value-less return in a
non-void function to yield the type's empty state, so the emitted C satisfies the return-type
check after `cl_panic` (recorded in the red-phase note).

**Verify**: `make -C docker exec CMD="cargo test --test runtime_value_integrity_spec_test"`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [07 §Test Categories](07-testing-strategy.md) · AR-10, AR-11
**Objective**: Pin the emitted panic call and the guarded move-out shape.

- [x] 3.3.1 Add implementation tests: the two-argument `cl_panic` for each message form, the move-out inside the guarded path only, and the once-only message rendering — `compiler/tests/runtime_value_integrity_impl_test.rs` ✅ (completed: 2026-09-29 22:35)
- [x] 3.3.2 Full verification — `make check` ✅ (completed: 2026-09-29 22:35)

**Deliverables**:
- [x] Implementation tests pass
- [x] No regressions; no warnings

**Full-verification evidence** (2026-09-29 22:35, `make check`): every workspace suite passed —
the implementation suite 26/26, the specification suite 30/30, and all pre-existing suites
unchanged (32 test binaries, 0 failures, 0 warnings).

**Verify**: `make check`

---

## Phase 4: Close-out: acceptance, probes, documentation

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — docs and evidence phase)

### Step 4.1: Evidence and Close-out

**Reference**: [01 §Acceptance Criteria](01-requirements.md) · AR-12, AR-14
**Objective**: Run the full acceptance gate, the leak probes, and close the records.

- [ ] 4.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg)
- [ ] 4.1.2 Host AddressSanitizer probes with leak detection on for ST-3, ST-4, ST-7, ST-24, ST-26, and ST-29 (owning temporaries and the owning conditional arguments)
- [ ] 4.1.3 Update documentation: `AGENTS.md` test map (both new suites), roadmap rows for `DEF-23` – `DEF-29` (done + plan link) — `AGENTS.md`, `plans/00-roadmap.md`
- [ ] 4.1.4 Check every acceptance criterion in `01-requirements.md`; set this plan's status Complete and the progress header to 27/27; final `make check`
- [ ] 4.1.5 Phase review evidence and plan summary — commit per the active commit mode

**Deliverables**:
- [ ] `make verify` all legs passed
- [ ] ASan probes clean with leak detection on
- [ ] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (materialization and temporary release)
    ↓
Phase 2 (function-value call integrity)
    ↓
Phase 3 (runtime boundary)
    ↓
Phase 4 (close-out)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security reviewed — compiler-internal change with no input, secret, or data path (AR-13)
6. ✅ Documentation updated
7. ✅ Code reviewed (strict-profile phase review)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
