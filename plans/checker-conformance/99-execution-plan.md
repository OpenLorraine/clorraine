# Execution Plan: Checker Conformance

> **Feature**: checker-conformance
> **Status**: Not Started
> **Progress**: 35/38 tasks (92%)
> **Created**: 2026-10-11 15:20
> **Last Updated**: 2026-10-11 23:02
> **Commit mode**: `--auto-commit` — commit and push after each verified task group
> **Design authority**: `--auto-design` — one user-owned item open (AR-2, the arity diagnostic code)

## Phase 1: Initializer guard operands and typedef collectors

> **Phase baseline tree**: `477f27dd36dcbde5f40e6dc267ce1864388dbe11` (the `8170199` tree)
> **Lenses**: correctness + memory-safety + emission-integrity
> **Reasoning**: medium — one initializer materialization extension plus three collector additions

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-initializer-guards-typedefs.md)
**Objective**: Pin the two double-evaluation initializers, the inline-argument typedef, the lifted specialization, and the struct-instance element.

- [x] 1.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` ✅ (completed: 2026-10-11 15:10; the six cases: initdividend, initbase, inlinearg, lambdaspec, structelement, and the initownbase reference)
- [x] 1.1.2 Run the suite and record the red phase (expected: two calls, undeclared `ClSlice_*`) ✅ (completed: 2026-10-11 15:10; exactly as filed: initdividend/initbase exit 22 (two runs), inlinearg `'ClSlice_String' undeclared`, lambdaspec `'ClSlice_i32' undeclared`, structelement build failure; the initownbase reference green)

### Step 1.2: Implementation

- [x] 1.2.1 Materialize the impure dividend and index base in the initializer path — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 15:14; the index base keeps its fixed-array spelling, the dividend stores for `Div`/`Rem`, and the needs predicate mirrors both)
- [x] 1.2.2 Collect the value-result slice type, the lifted specializations, and the struct-instance element — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 15:14; the value walk records every value's own slice type, the specialization loop uses the emission's guard and substitution, and the mono records a written type argument's struct instances)
- [x] 1.2.3 Run the suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-11 15:14; spec 6/6)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the phase implementation pins (the single-call shapes, the typedefs and their order) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-11 15:14; impl 6/6)
- [x] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-11 16:22; three lenses reported no critical/major findings; the two minor items fixed directly; five pre-existing classes filed as DEF-219–DEF-223)

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 2: Generic instance conformance

> **Phase baseline tree**: `5a378fb` (the Phase 1 close-out tree)
> **Lenses**: correctness + semantics + emission-integrity
> **Reasoning**: medium — a new recorded duty list plus two focused check extensions

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-generic-instance-conformance.md)
**Objective**: Pin the iterable instances, the value-form member report, and the conditional fold.

- [x] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` ✅ (completed: 2026-10-11 17:03; thirteen cases: iterscalar, iterenum, iterstruct, iternull, iterok, iterenumelem, iterborrow, memval, memvalcall, condfold, condfolddrop, condfoldconcrete)
- [x] 2.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-11 17:03; seven red as filed: the rejections accepted (iterscalar/iternull failed the strict build; iterstruct/iterenum accepted), the fold cases returned 0; the five references green)

### Step 2.2: Implementation

- [x] 2.2.1 Record the iterable duty and audit it per instance — `compiler/src/ir/mono.rs`, `compiler/src/own.rs`, `compiler/src/own/loops.rs`, `compiler/src/own/instance_check.rs` ✅ (completed: 2026-10-11 17:03; the null arm lands in `check_for_in` with the shared predicate)
- [x] 2.2.2 Report the value-form member and resolve the conditional arm in the projection — `compiler/src/sema/mod.rs`, `compiler/src/sema/expr.rs`, `compiler/src/own/instance_check.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-11 17:03; the member report is per instance — the register's AR-5 amendment — because a declaration-site report rejects the designed late resolution and DEF-150's own shape)
- [x] 2.2.3 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-11 17:03; spec 18/18 after the AR-5 amendment)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the phase implementation pins (the report counts and spans, the folded constant) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-11 17:03; four pins: the folded constant, the late-resolved member, the scalar and borrow loop shapes; impl 11/11)
- [x] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-11 17:20; the semantics and correctness reviews found one critical and five major items; the fix round fixed the element binding per instance and the projection dispatch, pinned by new cases; the scoped re-review is dispatched)

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 3: Call, member, and cstring surfaces

> **Phase baseline tree**: `68d6428` (the Phase 2 close-out tree)
> **Lenses**: correctness + semantics + emission-integrity
> **Reasoning**: medium — four checker extensions and one stdlib signature change
> **Gate**: AR-2 (the arity diagnostic code) must be resolved before Step 3.2.

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-call-member-cstring-surfaces.md)
**Objective**: Pin the `cstring` rejections, the arity reports, the unresolved-member reports, and the widened conversion.

- [x] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` ✅ (completed: 2026-10-11 19:45; fifteen cases; the ruled `E10308` row, its compile-fail golden, and the harness inventory line land with it, and the `cstringview`/`cstringslicearg` expectations amend to the frozen crossing — the register's AR-15 note and the design's supersede note)
- [x] 3.1.2 Run the suite and record the red phase (the arity cases join once AR-2 is ruled) ✅ (completed: 2026-10-11 19:45; ten red exactly as filed: the three view crossings emitted invalid C, the four arities were accepted, the three member callees emitted implicit calls, and `toCString(view)` reported `E10200`; the five references were green)

### Step 3.2: Implementation

- [x] 3.2.1 Reject the view at a `cstring` parameter, report the unresolved member callee, and check the argument count (AR-2-ruled) — `compiler/src/sema/call.rs`, `compiler/src/sema/mod.rs`, `compiler/src/codegen/calls.rs`, `compiler/src/codegen/function.rs` ✅ (completed: 2026-10-11 19:45; amended: the frozen ffi crossing rules accept a view at an uncontracted parameter and cross it as its pointer, so the emitter spells the `char *` cast instead of reporting `E10200` — see the ledger)
- [x] 3.2.2 Widen `toCString` to `str` and adapt its body — `stdlib/std.text.clo` ✅ (completed: 2026-10-11 19:45; the body reads the view's pointer, and the store conversion spells the `char *` cast the declaration expects)
- [x] 3.2.3 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-11 19:45; spec 40/40 after the two fixes — the place-shaped member read and the literal-crossing guard)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the phase implementation pins (the report counts, the accepted references' byte-identity, the stdlib body) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-11 19:45; revised in the fix round: four pins — the unwrapped literal, both call shapes, the callable field, and the widened `toCString` body and call sites; impl 17/17)
- [x] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses ✅ (completed: 2026-10-11 20:19; the emission audit found nothing, the correctness and semantics reviews one major and seven minors; the fix round reverted the `cstring` crossing to `E10200`, cached the ffi parameter types, suppressed the inference cascade, corrected the records, and pinned the two fixes; the scoped re-reviews verified all three fixes and their five minors are fixed; DEF-235 closed, DEF-237 withdrawn, DEF-239–DEF-244 filed)

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 4: Null-shaped values

> **Phase baseline tree**: `5a3727c` (the Phase 3 close-out tree)
> **Lenses**: correctness + semantics
> **Reasoning**: low — one predicate and four rejection sites

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-null-shaped-values.md)
**Objective**: Pin the rejected uses and the accepted positions.

- [x] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` ✅ (completed: 2026-10-11 21:35; six cases — the store, the return, the argument, the comparisons, the nullable declarations, the null-shaped copy; the iterable row is the Phase 2 `iternull` case)
- [x] 4.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-11 21:35; three red exactly as filed — the store and the return were accepted and failed the strict build, the argument was already rejected as `E10010` (the change is the code) — and the three references green)

### Step 4.2: Implementation

- [x] 4.2.1 Add the null-shape predicate with unit tests — `compiler/src/types/mod.rs` ✅ (completed: 2026-10-11 21:35; the predicate and its unit tests landed in Phase 2 and are reused unchanged)
- [x] 4.2.2 Reject the null-shaped uses at the four sites — `compiler/src/sema/` ✅ (completed: 2026-10-11 21:35; amended: the nullability gate is the own stage's shared `check_value_into`, extended with the null-shaped-value and null-shaped-target branches; the iterable site is Phase 2's)
- [x] 4.2.3 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-11 21:35; spec 49/49; the `nullarg` failure was the case's own `take` keyword, renamed to `consume`)

### Step 4.3: Impl Tests & Review

- [x] 4.3.1 Write the phase implementation pins (the message text, the accepted references) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-11 21:35; three pins: the comparison shapes, the null-shaped copy, the nullable zero initializers; impl 20/20)
- [x] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the two lenses ✅ (completed: 2026-10-11 21:47; the correctness lens found no critical/major and one records minor, fixed; the semantics lens found nothing in the phase and confirmed the recorded reading; its six pre-existing gaps are filed as DEF-245–DEF-249 and in DEF-242's note)

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 5: Diagnostic hygiene

> **Phase baseline tree**: `f84a26b` (the Phase 4 close-out tree)
> **Lenses**: correctness + semantics
> **Reasoning**: medium — two reporting-path changes and one audit extension

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-diagnostic-hygiene.md)
**Objective**: Pin the single reports, the `E10401` attribution, and the display phrase.

- [x] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs` ✅ (completed: 2026-10-11 22:25; twenty-one cases with the fix round — the update dedupe (four rejection spellings and one accepted), the capture `let`/assignment/duplicable/composite/array/unsafe/projection forms, the generated-owner phrase, and the ruled pins; the `captureexpr` case pins one E10401 for the expression-bodied twin, corrected from the design's "unchanged" note)
- [x] 5.1.2 Run the suite and record the red phase ✅ (completed: 2026-10-11 22:25; eleven red at the baseline — nine designed plus the two reconciliation pins — the three-report and two-report doubles, both capture sites, both composite forms, and the `__cl_owned0` message — with the accepted controls green)

### Step 5.2: Implementation

- [x] 5.2.1 Dedupe the state reports per site — `compiler/src/own/` ✅ (completed: 2026-10-11 22:25; the per-site ledger funneled through `report_state_once`; amended: the key is the binding, the span's start, and the code — the element case's projections and end offset differ between its two reads)
- [x] 5.2.2 Extend the capture recorder and retire the non-escape lend duty; render the generated-owner phrase — `compiler/src/sema/`, `compiler/src/own/` ✅ (completed: 2026-10-11 22:25; the recorder covers the `let`/assignment/composite-field sites with a per-lambda record set; the lend duty is retired for every by-value capture site and the dead `escape` flag merged away; `own/liveness.rs` renders the phrase)
- [x] 5.2.3 Run the suite to green; on any failure STOP and fix the implementation ✅ (completed: 2026-10-11 22:25; the uninitialized element case failed the first green run — the ledger key was coarsened to the span start — and the fix rounds' rework (the move walk, the element/fill/`const` sites, the projection judgment, the field-specific precedence) ended at spec 74/74)

### Step 5.3: Impl Tests & Review

- [x] 5.3.1 Write the phase implementation pins (the counts, the spans, the message) — `compiler/tests/checker_conformance_impl_test.rs` ✅ (completed: 2026-10-11 22:25; five pins — the moved update's one report at 10:13, the capture move's one report at 7:20 with no E10034, the phrase with no compiler spelling, and the accepted update/duplicable-capture emissions; impl 25/25)
- [x] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the two lenses ✅ (completed: 2026-10-11 22:28; the correctness lens found no critical/major and one records minor, fixed; the semantics lens found nothing in the phase and ruled both flagged behaviors conformant — the out-slot rejection is the frozen reading and the second-site silence matches the recorded reading — with three ruled pins added: `captureoutslot`, `capturetwosite`, `statededupepartial`) The review round found a critical, a major, and records minors (the array-literal and `unsafe` capture paths accepted double-frees; duplicable projections falsely rejected; the out-slot record wrong); the fix round corrected the move walk, the element sites, the projection judgment, the field-specific precedence, and the records; a scoped re-review follows. The round-2 scoped re-review found the `const` initializer hole (fixed with `captureconst`/`captureconstdup`), corrected the bare-discard label, and pinned the fill position; the array-element projection boundary routes into the pre-existing DEF-227 emitter gap and is recorded there.

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 6: Batch close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: none new — the close-out verifies the reviewed phases

### Step 6.1: Final Verification & Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Run the full acceptance, sync the docs and the roadmap, and close the plan.

- [ ] 6.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and the ASan suite; record the evidence
- [ ] 6.1.2 Update `AGENTS.md` (test naming map with the new suites)
- [ ] 6.1.3 Sync the roadmap rows (the batch's rows → Done with evidence), record the findings ledger, and close the plan

**Verify**: `make check`; `make verify`

---

## Dependencies

```
Phase 1 (emission guards + typedefs)   independent
Phase 2 (generic instances)            independent; shares sema/expr.rs with Phase 4
Phase 3 (call/member/cstring)          independent; AR-2 gate before Step 3.2
Phase 4 (null-shaped values)           the null-shape predicate lands here; Phase 2 uses it
Phase 5 (diagnostic hygiene)           independent
Phase 6 (close-out)                    after every phase
```
