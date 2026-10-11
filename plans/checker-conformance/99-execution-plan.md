# Execution Plan: Checker Conformance

> **Feature**: checker-conformance
> **Status**: Not Started
> **Progress**: 7/34 tasks (21%)
> **Created**: 2026-10-11 15:20
> **Last Updated**: 2026-10-11 15:10
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

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + semantics + emission-integrity
> **Reasoning**: medium — a new recorded duty list plus two focused check extensions

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-generic-instance-conformance.md)
**Objective**: Pin the iterable instances, the value-form member report, and the conditional fold.

- [ ] 2.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
- [ ] 2.1.2 Run the suite and record the red phase

### Step 2.2: Implementation

- [ ] 2.2.1 Record the iterable duty and audit it per instance — `compiler/src/sema/mod.rs`, `compiler/src/own/instance_check.rs`
- [ ] 2.2.2 Report the value-form member and resolve the conditional arm in the projection — `compiler/src/sema/expr.rs`, `compiler/src/codegen/expr.rs`
- [ ] 2.2.3 Run the suite to green; on any failure STOP and fix the implementation

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the phase implementation pins (the report counts and spans, the folded constant) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 2.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 3: Call, member, and cstring surfaces

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + semantics + emission-integrity
> **Reasoning**: medium — four checker extensions and one stdlib signature change
> **Gate**: AR-2 (the arity diagnostic code) must be resolved before Step 3.2.

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-call-member-cstring-surfaces.md)
**Objective**: Pin the `cstring` rejections, the arity reports, the unresolved-member reports, and the widened conversion.

- [ ] 3.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
- [ ] 3.1.2 Run the suite and record the red phase (the arity cases join once AR-2 is ruled)

### Step 3.2: Implementation

- [ ] 3.2.1 Reject the view at a `cstring` parameter, report the unresolved member callee, and check the argument count (AR-2-ruled) — `compiler/src/sema/call.rs`
- [ ] 3.2.2 Widen `toCString` to `str` and adapt its body — `stdlib/std.text.clo`
- [ ] 3.2.3 Run the suite to green; on any failure STOP and fix the implementation

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the phase implementation pins (the report counts, the accepted references' byte-identity, the stdlib body) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 3.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the three lenses

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 4: Null-shaped values

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + semantics
> **Reasoning**: low — one predicate and four rejection sites

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-null-shaped-values.md)
**Objective**: Pin the rejected uses and the accepted positions.

- [ ] 4.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
- [ ] 4.1.2 Run the suite and record the red phase

### Step 4.2: Implementation

- [ ] 4.2.1 Add the null-shape predicate with unit tests — `compiler/src/types/mod.rs`
- [ ] 4.2.2 Reject the null-shaped uses at the four sites — `compiler/src/sema/`
- [ ] 4.2.3 Run the suite to green; on any failure STOP and fix the implementation

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the phase implementation pins (the message text, the accepted references) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 4.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the two lenses

**Verify**: `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"`; `make check`

## Phase 5: Diagnostic hygiene

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + semantics
> **Reasoning**: medium — two reporting-path changes and one audit extension

### Step 5.1: Specification Tests

**Reference**: [03-05](03-05-diagnostic-hygiene.md)
**Objective**: Pin the single reports, the `E10401` attribution, and the display phrase.

- [ ] 5.1.1 [spec-author] Write the phase cases — `compiler/tests/checker_conformance_spec_test.rs`
- [ ] 5.1.2 Run the suite and record the red phase

### Step 5.2: Implementation

- [ ] 5.2.1 Dedupe the state reports per site — `compiler/src/own/`
- [ ] 5.2.2 Extend the capture recorder and retire the non-escape lend duty; render the generated-owner phrase — `compiler/src/sema/`, `compiler/src/own/`
- [ ] 5.2.3 Run the suite to green; on any failure STOP and fix the implementation

### Step 5.3: Impl Tests & Review

- [ ] 5.3.1 Write the phase implementation pins (the counts, the spans, the message) — `compiler/tests/checker_conformance_impl_test.rs`
- [ ] 5.3.2 Phase review evidence and progress update — commit per the active commit mode; dispatch the two lenses

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
