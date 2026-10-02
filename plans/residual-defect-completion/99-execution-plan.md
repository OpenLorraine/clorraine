# Execution Plan: Residual Defect Completion

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-03 01:50
> **Progress**: 34/34 tasks (100%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the six filed residual defects DEF-50 – DEF-55 in four implementation phases plus a
close-out. Each implementation phase follows the specification-first ordering: spec tests, a
recorded red phase, implementation, a green phase, implementation tests, and the project's verify
command.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Type keys and call arguments (DEF-50, DEF-54) | 7 |
| 2 | Lifted-lambda instances (DEF-52) | 8 |
| 3 | Call-result ownership (DEF-53, DEF-55) | 7 |
| 4 | Guarded drop hoist (DEF-51) | 7 |
| 5 | Close-out | 5 |

**Total: 34 tasks across 5 phases**

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`
> 3. **Update the Progress header** and the Last Updated stamp after EVERY task — never batch.
> 4. **Resume** by scanning the phase sections top-to-bottom: the first `[~]` task is resumed
>    first, else the first `[ ]` task.
> 5. **On blocker:** mark the task `[!]` and append `Blocked: <short reason>` on the same line.
>    The plan lifecycle is `Ready`, `Executing`, `Done`, or `Blocked`, derived from these markers.
>
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented.

---

## Phase 1: Type keys and call arguments (DEF-50, DEF-54)

> **Phase baseline tree**: 387dc536f0a2371822338156a4fc0802fc32f5a3
> **Lenses**: compiler-and-language (strict profile)

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-type-keys-and-call-arguments.md) · [AR-3](00-ambiguity-register.md)
**Objective**: Pin the extent-key and substituted-argument behaviors before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-6 — `compiler/tests/residual_defect_completion_spec_test.rs` ✅ (completed: 2026-10-02 09:41)
- [x] 1.1.2 Run the spec suite and record the red phase (DEF-50: C redefinition; DEF-54: C build
  type mismatch; ST-6 may pass pre-implementation and is recorded as a regression pin) ✅ (completed: 2026-10-02 09:43)

> **Red phase (1.1.2)**: 3 failed as required — the two extent-key cases fail the C build with
> `redefinition of 'struct ClGrid__i32'`; the substituted-argument case fails with
> `expected 'ClInner__String *' but argument is of type 'ClInner__String'`. 3 passed
> pre-implementation and are recorded as pins: the owned-element instantiation builds today, the
> `i32` substituted argument is duplicative so its view semantics already hold, and re-emission is
> already deterministic.

### Step 1.2: Implementation

**Reference**: [03-01](03-01-type-keys-and-call-arguments.md) §Proposed Changes
**Objective**: Make distinct extent lists key distinctly and substitute the argument type before
the pointer decision.

- [x] 1.2.1 Encode fixed-array extents and the slice marker in `type_key` — `compiler/src/codegen/types.rs` ✅ (completed: 2026-10-02 09:44)
- [x] 1.2.2 Substitute the argument type in the named-call pointer decision — `compiler/src/codegen/calls.rs` ✅ (completed: 2026-10-02 09:44)
- [x] 1.2.3 Run the spec suite and record the green phase ✅ (completed: 2026-10-02 09:45)

> **Green phase (1.2.3)**: all 6 Phase-1 specification cases pass.

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07-testing-strategy.md](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin internals, determinism, and the `type_key` unit shapes.

- [x] 1.3.1 Write implementation tests: `type_key` unit shapes in the module test block (scalar,
  multi-extent, nested, slice, struct element), emitted `&(p.v)`, byte-identical re-emission —
  `compiler/src/codegen/types.rs`, `compiler/tests/residual_defect_completion_impl_test.rs` ✅ (completed: 2026-10-02 09:47)
- [x] 1.3.2 Full verification (`make check`) ✅ (completed: 2026-10-02 09:49)

**Deliverables**:
- [x] `Grid<i32[2]>` and `Grid<i32[3]>` coexist and build; multi-extent and slice keys distinct
- [x] The substituted borrow argument builds and runs
- [x] All verification passing

**Verify**: `make check`

> **Post-phase review (Phase 1)**: correctness review — no findings; semantics review — one
> 🟠 MAJOR: a view built over a fixed grid (`slice<i32[3]>(grid, …)`) keyed its immediate element
> and emitted `ClSlice_arr3_i32` with no typedef, while the written `i32[][3]` parameter declared
> `ClSlice_i32`, so an accepted program failed the C build. User ruling: fix now. Resolved by
> keying every slice spelling from the base element (`slice_base_element` / `slice_element_key`,
> used by `c_type`, `type_key`, `collect_slice_types`, and the two view builders), with a unit
> pin and an emitted-shape pin; `make check` green; the scoped re-review reports no findings.
> Two pre-existing, non-blocking observations (a slice-of-struct typedef order and an
> unannotated `slice` local inference) are outside this plan's strict scope.

---

## Phase 2: Lifted-lambda instances (DEF-52)

> **Phase baseline tree**: f3a64e3ab9287f20c701730ea75cb51548d3c57a
> **Lenses**: compiler-and-language (strict profile)

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-lifted-lambda-instances.md) · [AR-2](00-ambiguity-register.md)
**Objective**: Pin the capture-less primary repro, two coexisting instances, and a capturing
parameter-typed lambda.

- [x] 2.1.1 [spec-author] Write specification tests from ST-7 – ST-9 — `compiler/tests/residual_defect_completion_spec_test.rs` ✅ (completed: 2026-10-02 21:13)
- [x] 2.1.2 Run the spec suite and record the red phase (C names `ClT` / the nested instance is missing) ✅ (completed: 2026-10-02 21:14)

> **Red phase (2.1.2)**: all 3 new cases fail as required — the emitted C rejects
> `static ClT cl_lambda_0(ClT v);` with `unknown type name 'ClT'` and the lambda symbol is
> undeclared. The Phase-1 cases stay green.

### Step 2.2: Implementation

**Reference**: [03-02](03-02-lifted-lambda-instances.md) §Proposed Changes
**Objective**: Carry the enclosing provenance, walk lifted bodies with the instance bindings, and
emit one specialization per needed instance.

- [x] 2.2.1 Record the enclosing-function provenance on lifted functions and walk them in the
  instance closure — `compiler/src/ir.rs`, `compiler/src/own/lower.rs`, `compiler/src/own/generics.rs` ✅ (completed: 2026-10-02 21:17)
- [x] 2.2.2 Emit per-instance lifted specializations, environment structs, and callable types,
  keeping the shared emission for non-parameter bodies — `compiler/src/codegen/program.rs`,
  `compiler/src/codegen/function.rs`, `compiler/src/codegen/types.rs` ✅ (completed: 2026-10-02 22:38)
- [x] 2.2.3 Render closure values, calls, and environments with the active specialization's
  lambda/closure/env spellings — `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-02 22:38)
- [x] 2.2.4 Run the spec suite and record the green phase ✅ (completed: 2026-10-02 22:38)

> **Green phase (2.2.4)**: all 9 specification cases pass (6 from Phase 1, 3 from Phase 2).
> ST-8's String result is measured in place; binding it surfaced a pre-existing, out-of-scope
> defect filed as roadmap DEF-56 and recorded as runtime AR-11.

### Step 2.3: Implementation Tests & Hardening

- [x] 2.3.1 Write implementation tests: specialization symbol and environment/callable shapes,
  closure value/call/env rendering for the capture-less and capturing cases, shared emission
  unchanged, byte-identical re-emission — `compiler/tests/residual_defect_completion_impl_test.rs` ✅ (completed: 2026-10-02 22:37)
- [x] 2.3.2 Full verification (`make check`) ✅ (completed: 2026-10-02 22:41)

**Deliverables**:
- [x] A generic call inside a lifted lambda is instantiated per enclosing instance
- [x] Multiple instances and a capturing parameter-typed lambda build and run
- [x] All verification passing

**Verify**: `make check`

> **Post-phase review (Phase 2)**: correctness and semantics reviews both found: one 🔴 CRITICAL
> (the specialized lifted body kept an unsubstituted `return_ty`, so a fixed-array return silently
> returned zero and a cleanup return spelled `ClT`) and one 🟠 MAJOR (the specialization predicate
> missed lambda captures and nested value positions). A second 🟠 MAJOR (a callable-bound closure
> argument kept a site-only callable type) and a pre-existing trailing-comma zero-argument closure
> call were ruled out of scope and filed as roadmap DEF-57 and DEF-58 (runtime AR-12). Fixes:
> substituted `return_ty`; the predicate now checks the lambda site's captures and walks nested
> values (receiver calls included) in the statement scan, and the instance walker visits receiver
> values; four compile-and-run pins were added. The scoped re-review found one remaining receiver
> gap in the same predicate arm; the one-line correction was applied, `make check` is green, and
> per the review cap no third pass was run.

---

## Phase 3: Call-result ownership (DEF-53, DEF-55)

> **Phase baseline tree**: 53cad897f6a26abacf335fcb8035a992146cd28b
> **Lenses**: compiler-and-language (strict profile)

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-call-result-ownership.md) · [AR-4](00-ambiguity-register.md)
**Objective**: Pin the `return`, `let`, and take-argument field projections, the maybe-owning
element take, and the concrete `E10026` compatibility case.

- [x] 3.1.1 [spec-author] Write specification tests from ST-10 – ST-14 — `compiler/tests/residual_defect_completion_spec_test.rs` ✅ (completed: 2026-10-02 23:02)
- [x] 3.1.2 Run the spec suite and record the red phase (LSan leaks on ST-10 – ST-12; the ASan
  double free on ST-13; ST-14 passes pre-implementation as a compatibility pin) ✅ (completed: 2026-10-02 23:02)

> **Red phase (3.1.2)**: ST-13 fails with `free(): double free detected` for the ownerless element
> take. ST-10 – ST-12 compute the right value pre-fix and pass, because their defect is a leak
> that the plain host harness cannot see; those are recorded as value pins, and the leak is pinned
> by this plan's sanitizer implementation tests and the close-out probes. ST-14 reports `E10026`
> as required.

### Step 3.2: Implementation

**Reference**: [03-03](03-03-call-result-ownership.md) §Proposed Changes
**Objective**: Bind the ownerless call result before an owning projection read and before the
maybe-owning element take.

- [x] 3.2.1 Bind field/element projection reads of ownerless call results — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-02 23:09)
- [x] 3.2.2 Bind the maybe-owning element take in lowering and emit the element move-out, keeping
  the checker suppression and the concrete `E10026` rejection — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-02 23:09)
- [x] 3.2.3 Run the spec suite and record the green phase ✅ (completed: 2026-10-02 23:09)

> **Green phase (3.2.3)**: all 14 specification cases pass, and `make check` is green across the
> workspace. One regression surfaced and was fixed during the phase: the projection binding had
> to stay confined to field/element reads so a whole fixed-array call assignment keeps its
> hidden-destination shape (`array_value_integrity_impl_test`).

### Step 3.3: Implementation Tests & Hardening

- [x] 3.3.1 Write implementation tests: emitted binding shape, single-release counts, sanitizer
  runs for the leak and double-free cases, `E10026` unchanged — `compiler/tests/residual_defect_completion_impl_test.rs` ✅ (completed: 2026-10-02 23:20)
- [x] 3.3.2 Full verification (`make check`) ✅ (completed: 2026-10-02 23:21)

**Deliverables**:
- [x] `return mkv().a`, `let s = mkv().a`, and `sink(mkv().a)` are sanitizer-clean
- [x] `sink<T>(mk()[i])` under an owning substitution is AddressSanitizer-clean
- [x] The concrete element take keeps `E10026`
- [x] All verification passing

> **Post-phase review (Phase 3)**: correctness and semantics reviews found: one 🟠 MAJOR
> introduced by this phase (the new take-root binding ignored `defer_move_outs`, so a projected
> take inside `&&`/`||`/`?:` hoisted the root call out of its guard), one 🟠 MAJOR harness issue
> (the new in-test sanitizer runs are nondeterministic in the pinned container), and one
> pre-existing 🔴 CRITICAL (an ownerless element read under a maybe-owning substitution uses
> freed memory; not worsened by this phase). The user ruled: gate the take-root binding on
> `defer_move_outs == 0` and file the guarded take as DEF-60; run sanitizers opt-in behind
> `RDC_ASAN` with plain-run pins; file the element read as DEF-59 (runtime AR-13). A guard pin was
> added, `make check` is deterministic green, and the scoped re-review reports no findings.

**Verify**: `make check`

---

## Phase 4: Guarded drop hoist (DEF-51)

> **Phase baseline tree**: ed277e31e04c86276466dc49d7691512c2d3c1cb
> **Lenses**: compiler-and-language (strict profile)

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-guarded-drop-hoist.md) · [AR-8](00-ambiguity-register.md)
**Objective**: Pin the `while`, `do`, and `for` guarded shapes, the struct-containing form, and
the expression-form regression shape.

- [x] 4.1.1 [spec-author] Write specification tests from ST-15 – ST-18 — `compiler/tests/residual_defect_completion_spec_test.rs` ✅ (completed: 2026-10-03 01:01)
- [x] 4.1.2 Run the spec suite and record the red phase (ST-15 – ST-17 LSan leaks; ST-18 passes
  pre-implementation as a regression pin) ✅ (completed: 2026-10-03 01:01)

> **Red phase (4.1.2)**: all 19 specification cases pass against the plain harness, because
> DEF-51 is leak-only: the guarded values are computed correctly and the missing drop is
> invisible without a sanitizer. The defect is recorded with host AddressSanitizer probes of the
> ST-15 and ST-17 programs: exit `1` with `70 byte(s) leaked in 6 allocation(s)` and
> `35 byte(s) leaked in 3 allocation(s)` respectively, while ST-18 (`Array<i32>`, no drop-needing
> element) is clean. The implementation pin in the call-value-integrity suite that recorded the
> skipped drop is rewritten in 4.2.2 (AR-10).

### Step 4.2: Implementation

**Reference**: [03-04](03-04-guarded-drop-hoist.md) §Proposed Changes
**Objective**: Hoist the statement-form drop for guard operands whose type has no expression-form
drop.

- [x] 4.2.1 Add the conditional-subtree walk and the statement-form drop hoist in `emit_guards` — `compiler/src/codegen/guards.rs` ✅ (completed: 2026-10-03 01:01)
- [x] 4.2.2 Rewrite the implementation test that pinned the old skip to the fixed shape — `compiler/tests/call_value_integrity_impl_test.rs` (AR-10) ✅ (completed: 2026-10-03 01:01)
- [x] 4.2.3 Run the spec suite and record the green phase ✅ (completed: 2026-10-03 01:01)

> **Green phase (4.2.3)**: all 19 specification cases pass, the rewritten call-value pin passes
> (61/61), and the host ASan probes of ST-15 and ST-17 exit `2` with no leaks (`detect_leaks=1`);
> ST-18 stays clean with its expression-form drop.

### Step 4.3: Implementation Tests & Hardening

- [x] 4.3.1 Write implementation tests: statement-form drop before the re-store, expression-form
  shapes unchanged, LeakSanitizer-clean runs, byte-identical re-emission — `compiler/tests/residual_defect_completion_impl_test.rs` ✅ (completed: 2026-10-03 01:03)
- [x] 4.3.2 Full verification (`make check`) ✅ (completed: 2026-10-03 01:05)

**Deliverables**:
- [x] A guarded operand of a drop-needing growable type releases its previous value on every
  re-store for the `while`, `do`, and `for` spellings and the struct-containing form
- [x] Types with expression-form drops keep their emitted shape
- [x] All verification passing

> **Post-phase review (Phase 4)**: the correctness and semantics reviews report no phase findings.
> The hoist is exactly-once on the probed paths (skipped, skipped-then-evaluated, every
> iteration), expression-form shapes are unchanged, and generic substitutions resolve. One
> 🟡 MINOR coverage gap (no `?:`-arm pin) was closed with an additional implementation pin. Two
> pre-existing defects found while probing were filed as roadmap DEF-61 (a generic body passes a
> default-lend parameter to a take position without `E10034`) and DEF-62 (a `do ... while` early
> return leaks the condition temporary), recorded as runtime AR-14.

**Verify**: `make check`

---

## Phase 5: Close-out

> **Phase baseline tree**: c21b60704e55c82afde2d388f4eda11e2d8b2a99
> **Lenses**: compiler-and-language (strict profile)

### Step 5.1: Acceptance, Probes, Docs, and Summary

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · [AR-7](00-ambiguity-register.md)
**Objective**: Prove acceptance, record the close-out probes, and sync the durable docs.

- [x] 5.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg) ✅ (completed: 2026-10-03 01:43)
- [x] 5.1.2 Host AddressSanitizer close-out probes for the six defects with leak detection on —
  `build/closeout-probes/` (R10) ✅ (completed: 2026-10-03 01:45)
- [x] 5.1.3 Update documentation: the `AGENTS.md` test map (both new suites) and the roadmap rows
  for DEF-50 – DEF-55 (Done with this plan linked) — `AGENTS.md`, `plans/00-roadmap.md` (R11) ✅ (completed: 2026-10-03 01:46)

> **Close-out evidence**: `make verify` reports `verify: all legs passed` (hosted-amiga
> cross-toolchain, fault, and freestanding legs included). The close-out probe runner
> `build/closeout-probes/run-residual-completion.sh` emits, builds with AddressSanitizer and leak
> detection, and runs all eight reproductions (DEF-50 – DEF-55, including the three projection
> forms): every probe exits its expected code with no sanitizer report.
- [x] 5.1.4 Check every acceptance criterion in [01-requirements.md](01-requirements.md); set the
  plan status Complete and the progress header to 34/34; final `make check` ✅ (completed: 2026-10-03 01:48)

> **Acceptance check**: R1 – R11 and all five acceptance criteria are checked in
> [01-requirements.md](01-requirements.md). R8 is verified by `git diff --name-only d438347..HEAD
> -- spec/` returning zero files. The final `make check` is green (38 suites).
- [x] 5.1.5 Phase review evidence and plan summary — commit per the active commit mode ✅ (completed: 2026-10-03 01:50)

**Deliverables**:
- [x] `make verify` all legs passed
- [x] ASan probes clean with leak detection on
- [x] Documentation and roadmap synced; acceptance criteria checked

> **Post-phase review (Phase 5)**: the close-out diff is docs-only, so one correctness review ran
> (auditors skipped by the profile). It reproduced the acceptance evidence and found one
> 🟡 MINOR wording mismatch (the checked 5.1.4 line named a `34/34` header while the last task was
> still open); the header now reads `34/34` at completion. No other findings.

---

## Dependencies

```
Phase 1 (DEF-50, DEF-54)
    ↓
Phase 2 (DEF-52)
    ↓
Phase 3 (DEF-53, DEF-55)
    ↓
Phase 4 (DEF-51)
    ↓
Phase 5 (Close-out)
```

Phases run in order per [AR-9](00-ambiguity-register.md): the substitution cluster lands first,
then the call-result binding that the element-take case shares, then the guard hoist. Each phase
is independently verifiable with `make check`.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, then `make verify` at close-out)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Security hardened — N/A (compiler-internal; no input surface)
6. ✅ Documentation updated (`AGENTS.md`, the roadmap)
7. ✅ Code reviewed (per the repository's quality profile)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
