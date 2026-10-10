# Execution Plan: Module Initializer Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-11 01:49
> **Progress**: 8/34 tasks (24%)
> **CodeOps Artifact Schema**: 1

## Overview

Close the 9 module-initializer defects (DEF-130 – DEF-135, DEF-137, DEF-143, DEF-152) by
mirroring the verified body-path contracts into the init pipeline, extending the AST generics
walk to initializer expressions, lifting capture-less init lambdas, releasing owning init locals,
and running the write-mutability checks over initializers. All fixes are shape-gated; the
existing init pins are the byte-identity oracle. Execution runs under `--auto-commit
--auto-design` (the standing mandate; AR-16).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Initializer call resolution (DEF-131/132/133) | 9 |
| 2 | Initializer aggregate emission (DEF-134/135/143) | 8 |
| 3 | Initializer release and checks (DEF-130/152) | 8 |
| 4 | Body slice typedefs (DEF-137) | 6 |
| 5 | Batch close-out | 3 |

**Total: 34 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp
> 2. **On verify pass:** promote it to `[x]`
> 3. **Update the Progress header and Last Updated stamp after EVERY task**
> 4. **Resume** by scanning top-to-bottom: the first `[~]` task first, else the first `[ ]`
> 5. **On blocker:** mark `[!]` with `Blocked: <short reason>` on the same line

---

## Phase 1: Initializer call resolution

> **Phase baseline tree**: `58ba6e800b15ca59ba8c7bea82a36bcb4f3e9bd5` (the `a97725b` tree)
> **Lenses**: correctness + emission-integrity + semantics (lambda lifting)
> **Reasoning**: high — two lowerers to keep in sync, symbol resolution, byte-identity pins

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-initializer-call-resolution.md) §Proposed Changes · [AR-2, AR-3](00-ambiguity-register.md)
**Objective**: Pin the four call-resolution shapes (strict build plus run) before any change.

- [x] 1.1.1 [spec-author] Write specification tests ST-F1 – ST-F4 — `compiler/tests/module_initializer_integrity_spec_test.rs` ✅ (completed: 2026-10-11 01:31)
- [x] 1.1.2 Run the spec suite and record the red phase (expected: strict-build failures / unresolved callables at the batch base) ✅ (completed: 2026-10-11 01:31; red errors: empty callee, implicit `f`, undeclared `cl_pick__i32`, implicit `get`)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-initializer-call-resolution.md) §Proposed Changes
**Objective**: Record init instances and lambdas in the own stage; resolve callees and receivers in the init lowerer.

- [x] 1.2.1 Extend the initializer walk (instances + lambda collection) — `compiler/src/own/generics.rs`, `compiler/src/own.rs` ✅ (completed: 2026-10-11 01:31)
- [x] 1.2.2 Lift capture-less init lambdas through the synthetic minimal context — `compiler/src/own/lower.rs` ✅ (completed: 2026-10-11 01:31)
- [x] 1.2.3 Resolve init callees (lifted lambda, func-typed global, generic instance) and dot receivers — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 01:31)
- [x] 1.2.4 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-11 01:31; spec 4/4)
- [x] 1.2.5 Regression: the existing init pins stay green; the seeded unaffected initializers re-emit byte-identically ✅ (completed: 2026-10-11 01:31; make check 59 suites)

### Step 1.3: Impl Tests & Review

- [x] 1.3.1 Write the Phase 1 implementation pins (lifted identity, mangled symbol, instance symbol, receiver argument) — `compiler/tests/module_initializer_integrity_impl_test.rs` ✅ (completed: 2026-10-11 01:31; impl 4/4)
- [~] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ⏳ (implemented: 2026-10-11 01:31; three-lens review merged — EIA-001 regression + RV-001/002/003 fixed, RV-004/005 addressed; ledger §Phase 1; scoped re-review dispatched)

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 2: Initializer aggregate emission

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + emission-integrity
> **Reasoning**: medium — three contained emission forms, each with a body contract to mirror

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-initializer-aggregate-emission.md) §Proposed Changes · [AR-7, AR-8, AR-9](00-ambiguity-register.md)
**Objective**: Pin the store form, the view wrap, and the fold before any change.

- [ ] 2.1.1 [spec-author] Write specification tests ST-F5 – ST-F7 — `compiler/tests/module_initializer_integrity_spec_test.rs`
- [ ] 2.1.2 Run the spec suite and record the red phase (expected: invalid C / the wrapped-type error / the wrong `0`)

### Step 2.2: Implementation

**Reference**: [03-02](03-02-initializer-aggregate-emission.md) §Proposed Changes
**Objective**: Give the init store the body store's destination decisions, shape-gated.

- [ ] 2.2.1 Statement-form struct stores for fixed-array fields (DEF-134) — `compiler/src/codegen/program.rs`
- [ ] 2.2.2 Destination-aware `str` view wrap (DEF-135) — `compiler/src/codegen/program.rs`
- [ ] 2.2.3 Structural qualified `.len` fold (DEF-143) — `compiler/src/codegen/program.rs`
- [ ] 2.2.4 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

### Step 2.3: Impl Tests & Review

- [ ] 2.3.1 Write the Phase 2 implementation pins (store fragments, wrapped text, folded constant) — `compiler/tests/module_initializer_integrity_impl_test.rs`
- [ ] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 3: Initializer release and checks

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + memory-safety + semantics
> **Reasoning**: high — release placement and check extraction touch ownership and mutability rules

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-initializer-release-and-checks.md) §Proposed Changes · [AR-4, AR-10](00-ambiguity-register.md)
**Objective**: Pin the three leak shapes under ASan and the write rejections with their controls.

- [ ] 3.1.1 [spec-author] Write specification tests ST-F8 – ST-F9 — `compiler/tests/module_initializer_integrity_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase (expected: the filed leaks under host ASan; the accepted writes)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-initializer-release-and-checks.md) §Proposed Changes
**Objective**: Record owning init locals and release them at the init function's end; run the extracted mutability predicates over initializer expressions.

- [ ] 3.2.1 Init-owner registry and null-guarded end-of-init releases (DEF-130) — `compiler/src/codegen/program.rs`, `compiler/src/codegen.rs`
- [ ] 3.2.2 Extract the write predicates and add the check-only initializer walk (DEF-152) — `compiler/src/own/expr.rs`, `compiler/src/own/place.rs`, `compiler/src/own/program.rs`
- [ ] 3.2.3 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

### Step 3.3: Impl Tests & Review

- [ ] 3.3.1 Write the Phase 3 implementation pins (release text and null guards, one-report-per-site counts) — `compiler/tests/module_initializer_integrity_impl_test.rs`
- [ ] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 4: Body slice typedefs

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness + emission-integrity
> **Reasoning**: low — one collector walk mirroring an existing arm

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-body-slice-typedefs.md) §Proposed Changes · [AR-11](00-ambiguity-register.md)
**Objective**: Pin the body `sizeof(i32[])` spelling (strict build + run) before the change.

- [ ] 4.1.1 [spec-author] Write specification test ST-F10 — `compiler/tests/module_initializer_integrity_spec_test.rs`
- [ ] 4.1.2 Run the spec suite and record the red phase (expected: the missing-typedef strict-build failure)

### Step 4.2: Implementation

- [ ] 4.2.1 Collect written slice type arguments from body IR values — `compiler/src/codegen/program.rs`
- [ ] 4.2.2 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

### Step 4.3: Impl Tests & Review

- [ ] 4.3.1 Write the Phase 4 implementation pin (typedef plus use) — `compiler/tests/module_initializer_integrity_impl_test.rs`
- [ ] 4.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 5: Batch close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: none new — the close-out runs the full verification over already-reviewed phases
> **Reasoning**: low — verification and documentation sync only

### Step 5.1: Final Verification & Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Run the full acceptance, sync the docs and the roadmap, and close the plan.

- [ ] 5.1.1 Run the full verification: `make check`, `make verify`, the strict-build sweep, and the ASan spec suite; record the evidence
- [ ] 5.1.2 Update `AGENTS.md` (test naming map with the new suites)
- [ ] 5.1.3 Sync the roadmap rows (DEF-130 – DEF-135, DEF-137, DEF-143, DEF-152 → Done with evidence), record the findings ledger, and close the plan

**Verify**: `make check`; `make verify`

---

## Dependencies

```
Phase 1 (call resolution)
    ↓
Phase 2 (aggregate emission)   independent of 1 except shared files
    ↓
Phase 3 (release + checks)     the release registry touches the materialization walk
    ↓
Phase 4 (slice typedefs)       independent collector arm
    ↓
Phase 5 (close-out)
```
