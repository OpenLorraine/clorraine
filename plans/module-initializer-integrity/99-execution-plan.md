# Execution Plan: Module Initializer Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-11 14:44
> **Progress**: 29/33 tasks (88%)
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
| 3 | Initializer release and checks (DEF-130/152) | 7 |
| 4 | Body slice typedefs (DEF-137) | 6 |
| 5 | Batch close-out | 3 |

**Total: 33 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
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
- [x] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-11 02:10; three-lens review + one scoped re-review; two fix rounds — EIA-001/RV-001/002/003/004/005 and the head-only receiver resolution; ledger §Phase 1)

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 2: Initializer aggregate emission

> **Phase baseline tree**: `f81235752c2d5fabe88b20b54083c82853b17251` (the `77c268d` tree)
> **Lenses**: correctness + emission-integrity
> **Reasoning**: medium — three contained emission forms, each with a body contract to mirror

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-initializer-aggregate-emission.md) §Proposed Changes · [AR-7, AR-8, AR-9](00-ambiguity-register.md)
**Objective**: Pin the store form, the view wrap, and the fold before any change.

- [x] 2.1.1 [spec-author] Write specification tests ST-F5 – ST-F7 — `compiler/tests/module_initializer_integrity_spec_test.rs` ✅ (completed: 2026-10-11 09:09; seven cases: store from a place, from a call, nested, conditional arm, template wrap, String-call wrap, field fold)
- [x] 2.1.2 Run the spec suite and record the red phase (expected: invalid C / the wrapped-type error / the wrong `0`) ✅ (completed: 2026-10-11 09:09; red errors: missing braces / int-conversion for the stores, `ClStr` from `void *` for both str values, exit 254 for the fold)

### Step 2.2: Implementation

**Reference**: [03-02](03-02-initializer-aggregate-emission.md) §Proposed Changes
**Objective**: Give the init store the body store's destination decisions, shape-gated.

- [x] 2.2.1 Statement-form struct stores for fixed-array fields (DEF-134) — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 09:09; the literal binds into an init local wherever it stands, including nested fields and guarded conditional arms)
- [x] 2.2.2 Destination-aware `str` view wrap (DEF-135) — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 09:09)
- [x] 2.2.3 Structural qualified `.len` fold (DEF-143) — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 09:09; the shared projected read carries the field's type and keeps module paths out of the receiver)
- [x] 2.2.4 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-11 09:09; spec 18/18 plain and under AddressSanitizer)

### Step 2.3: Impl Tests & Review

- [x] 2.3.1 Write the Phase 2 implementation pins (store fragments, wrapped text, folded constant) — `compiler/tests/module_initializer_integrity_impl_test.rs` ✅ (completed: 2026-10-11 09:09; impl 10/10)
- [x] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-11 09:34; correctness review + one scoped re-review; two fix rounds — RV-001 and RV-002/003; ledger §Phase 2)

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 3: Initializer release and checks

> **Phase baseline tree**: `df4154dc81fefc7f57d0ef9371c0cd1bc18d9e93` (the `032d92c` tree)
> **Lenses**: correctness + memory-safety + semantics
> **Reasoning**: high — release placement and check extraction touch ownership and mutability rules

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-initializer-release-and-checks.md) §Proposed Changes · [AR-4, AR-10](00-ambiguity-register.md)
**Objective**: Pin the three leak shapes under ASan and the write rejections with their controls.

- [x] 3.1.1 [spec-author] Write specification tests ST-F8 – ST-F9 — `compiler/tests/module_initializer_integrity_spec_test.rs` ✅ (completed: 2026-10-11 10:43; seven cases: the four filed leak shapes, the two write rejections, the read-only control)
- [x] 3.1.2 Run the spec suite and record the red phase (expected: the filed leaks under host ASan; the accepted writes) ✅ (completed: 2026-10-11 10:43; four leaks (10/11+10/12 B) and both writes accepted)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-initializer-release-and-checks.md) §Proposed Changes
**Objective**: Record owning init locals and release them at the init function's end; run the extracted mutability predicates over initializer expressions.

- [x] 3.2.1 Init-owner registry, owned-argument materialization, and null-guarded end-of-init releases (DEF-130; includes the DEF-204 shape) — `compiler/src/codegen/program.rs`, `compiler/src/codegen.rs` ✅ (completed: 2026-10-11 10:43; owned results, arguments, templates, and array destinations bind and release in reverse order; a `take` argument's record is dropped; a view destination retains)
- [x] 3.2.2 Extract the write predicates and add the check-only initializer walk (DEF-152) — `compiler/src/own/expr.rs`, `compiler/src/own/place.rs`, `compiler/src/own/program.rs` ✅ (completed: 2026-10-11 10:43; the synthetic init checker walks updates and assignments; E10023/E10042 report with the body twins' spans, messages, and one-per-site count)
- [x] 3.2.3 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-11 10:43; spec 29/29 plain and under AddressSanitizer)

### Step 3.3: Impl Tests & Review

- [x] 3.3.1 Write the Phase 3 implementation pins (release text and null guards, one-report-per-site counts) — `compiler/tests/module_initializer_integrity_impl_test.rs` ✅ (completed: 2026-10-11 10:43; five pins: the four release shapes and the `take` exclusion; the report counts ride the spec suite's rejection assertions; impl 17/17)
- [x] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-11 10:56; three lenses reviewed; one fix round — the retention compounds, consumed-argument suppression, signature-first modes, the .len parity, and bookkeeping corrected; all counterexamples re-probed clean)

**Verify**: `make -C docker exec CMD="cargo test --test module_initializer_integrity_spec_test --test module_initializer_integrity_impl_test"`; `make check`

---

## Phase 4: Body slice typedefs

> **Phase baseline tree**: `a8dec2fa329b862d29394c8cc180593f003d62e3` (the `6bb8722` tree)
> **Lenses**: correctness + emission-integrity
> **Reasoning**: low — one collector walk mirroring an existing arm

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-body-slice-typedefs.md) §Proposed Changes · [AR-11](00-ambiguity-register.md)
**Objective**: Pin the body `sizeof(i32[])` spelling (strict build + run) before the change.

- [x] 4.1.1 [spec-author] Write specification test ST-F10 — `compiler/tests/module_initializer_integrity_spec_test.rs` ✅ (completed: 2026-10-11 14:44; the body `sizeof(i32[])` spelling with an ABI-robust size assertion)
- [x] 4.1.2 Run the spec suite and record the red phase (expected: the missing-typedef strict-build failure) ✅ (completed: 2026-10-11 14:44; `ClSlice_i32 undeclared` for `sizeof(ClSlice_i32)`, exactly as filed)

### Step 4.2: Implementation

- [x] 4.2.1 Collect written slice type arguments from body IR values — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-11 14:44; the walk covers statement operands, guard checks, call arguments and receivers, and terminator values; instance bodies walk with their substitutions)
- [x] 4.2.2 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-11 14:44; spec 37/37)

### Step 4.3: Impl Tests & Review

- [x] 4.3.1 Write the Phase 4 implementation pin (typedef plus use) — `compiler/tests/module_initializer_integrity_impl_test.rs` ✅ (completed: 2026-10-11 14:44; impl 18/18)
- [~] 4.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ⏳ (implemented: 2026-10-11 14:44; review dispatched)

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
