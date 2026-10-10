# Execution Plan: Callable Release Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 23:31
> **Progress**: 25/36 tasks (69%)
> **CodeOps Artifact Schema**: 1

## Overview

Five phases close the 13 defects ([AR-1](00-ambiguity-register.md)): generated identities
(DEF-138, 140, 165), callable effect modes (DEF-155 – DEF-157, DEF-166), owner release
completeness (DEF-139, 158, 159, 163, 164), the possibly-null widening (DEF-141), and the batch
close-out. Each phase follows the specification-first ordering and cites its owning
[03-XX](00-index.md) document and the ST-cases in [07-testing-strategy.md](07-testing-strategy.md).

Execution commit mode: **auto-commit + auto-design** (user instruction) — commit and push each
verified task via the git-commit skill in push mode; eligible technical mechanism decisions are
delegated under the shared auto-design policy (register provenance PD-1 – PD-4), rulings stay
reserved. Batch D stays parked; after this batch the loop continues with batch F
(module-initializer integrity) and batch G (checker conformance) until no defect row remains
open.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Generated identities and instance keys (DEF-138, 140, 165) | 8 |
| 2 | Callable effect modes (DEF-155, 156, 157, 166) | 9 |
| 3 | Owner release completeness (DEF-139, 158, 159, 163, 164) | 10 |
| 4 | Possibly-null widening (DEF-141) | 6 |
| 5 | Batch close-out | 3 |

**Total: 36 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
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

## Phase 1: Generated identities and instance keys

> **Phase baseline tree**: 6289af4bb724c394a842b3ea0e36cbe7acef8a66
> **Lenses**: correctness reviewer + memory-safety auditor (DEF-138 double-free) +
> emission-integrity auditor (type keys, env names, C rename)
> **Reasoning**: high — a critical memory defect and two symbol-identity changes

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-generated-identities.md) §Invariants · [AR-2, AR-3, AR-4,
AR-5](00-ambiguity-register.md)
**Objective**: Pin the three generated-identity cases — the collision program, the two func
instances, the inline capturing lambda — before any implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-3 — `compiler/tests/callable_release_integrity_spec_test.rs` ✅ (completed: 2026-10-10 19:59)
- [x] 1.1.2 Run the spec suite and record the red phase (expected: ST-1 ASan double-free; ST-2/ST-3 strict-build failures) ✅ (completed: 2026-10-10 19:59)

### Step 1.2: Implementation

**Reference**: [03-01](03-01-generated-identities.md) §Proposed Changes
**Objective**: Land identity-keyed generated bindings, the signature-bearing func key, and the
env-struct dedupe.

- [x] 1.2.1 Implement identity-keyed generated bindings and the collision-safe C spelling — `compiler/src/own.rs`, `compiler/src/own/lower.rs`, `compiler/src/codegen/*` ✅ (completed: 2026-10-10 20:04)
- [x] 1.2.2 Implement the signature-bearing func type key — `compiler/src/types/keys.rs` ✅ (completed: 2026-10-10 20:04)
- [x] 1.2.3 Implement the env-struct definition dedupe — `compiler/src/codegen/program.rs`, `compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-10 20:04)
- [x] 1.2.4 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 20:04)

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test"`; `make check`

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-generated-identities.md) §Testing Requirements
**Objective**: Pin the binding identity, the rename spelling, the keys, and the definition
counts; record the phase review evidence.

- [x] 1.3.1 Write the Phase 1 implementation tests (name-map absence, C rename, exact keys, one env definition, byte-identity) — `compiler/tests/callable_release_integrity_impl_test.rs` ✅ (completed: 2026-10-10 20:04)
- [x] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 20:27)

**Deliverables**:
- Identity-keyed generated bindings with collision-safe C spelling
- Signature-bearing func keys; single env definition per site/instance
- Phase 1 implementation tests

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_impl_test"`; `RDC_ASAN=1` spec case ST-1

---

## Phase 2: Callable effect modes

> **Phase baseline tree**: 37a1248794d536b7dcac2cd6f19ba93c8c20f455
> **Scope mode**: strict
> **Expected modification set**: `compiler/src/own/call.rs`, `compiler/src/own/call_check.rs`,
> `compiler/src/own/instance_check.rs`, `compiler/src/own/lower.rs` (only where instance judging
> needs it), `compiler/tests/callable_release_integrity_spec_test.rs`,
> `compiler/tests/callable_release_integrity_impl_test.rs`, `plans/callable-release-integrity/**`.
> The untracked `.opencode/` catalog install (2026-10-10 21:54) is unrelated pre-phase work:
> excluded from the phase review and from commits.
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (lend/move
> rules); the phase reviewer reads the emission diff for argument-shape changes
> **Reasoning**: high — double-frees and leaks across four callee kinds with rule-conformance
> stakes

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-callable-effect-modes.md) §Invariants · [AR-6](00-ambiguity-register.md)
**Objective**: Pin the twin-pair cases — field, global, param, and lambda-parameter callees —
before any implementation.

- [x] 2.1.1 [spec-author] Write specification tests from ST-4 – ST-7 — `compiler/tests/callable_release_integrity_spec_test.rs` ✅ (completed: 2026-10-10 22:03; delegated to spec-test-author)
- [x] 2.1.2 Run the spec suite and record the red phase ✅ (completed: 2026-10-10 22:03; red: 4 rejection-parity cases plain — take-field, borrow-field, take-global, shadowing parameter — plus the lambda expression-body store after the AR-17 correction, and the call-path release case under RDC_ASAN=1; the pre-existing six cases and the two new E10034 controls stay green)

### Step 2.2: Implementation

**Reference**: [03-02](03-02-callable-effect-modes.md) §Proposed Changes
**Objective**: Thread each callee kind's own signature into the call's modes; keep resolution
identity-based.

- [x] 2.2.1 Implement func-typed field-call modes — `compiler/src/own/call.rs`, `compiler/src/own/call_check.rs` ✅ (completed: 2026-10-10 22:22)
- [x] 2.2.2 Implement module func-typed global resolution and modes — `compiler/src/own/call.rs`, `compiler/src/own/call_check.rs` ✅ (completed: 2026-10-10 22:22)
- [x] 2.2.3 Implement `Param` callee modes from the callable bound and identity-based take resolution — `compiler/src/own/call_check.rs` (`func_binding_callee`, `callable_signature`) ✅ (completed: 2026-10-10 22:22)
- [x] 2.2.4 Judge expression-bodied lambdas like returns and materialize closure-call lend arguments — `compiler/src/own/lower.rs`, `compiler/src/own/statements.rs` (per AR-17) ✅ (completed: 2026-10-10 22:22)
- [x] 2.2.5 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 22:22; spec 17/17 plain and under RDC_ASAN=1; `make check` 58 suites green after four deliberate fixture re-pins)

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test"`; `make check`

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-callable-effect-modes.md) §Testing Requirements
**Objective**: Pin the mode vectors, materialization shapes, and report counts; record the
phase review evidence.

- [x] 2.3.1 Write the Phase 2 implementation tests (per-kind mode vectors, temporary materialization, callee spellings, report counts) — `compiler/tests/callable_release_integrity_impl_test.rs` ✅ (completed: 2026-10-10 22:30; six pins: the field/global take move-outs, the field/global/param/closure owned-temporary bindings, and the bound instance symbol)
- [x] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 23:12; full round + one scoped re-review; the re-review's findings fixed in the second fix round and verified directly per the review budget)

**Deliverables**:
- Every callee kind applies its own signature's modes (per instance for generics)
- Twin parity for take/borrow/owned-temporary shapes
- Phase 2 implementation tests

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_impl_test"`; `RDC_ASAN=1` spec cases ST-4 – ST-7

---

## Phase 3: Owner release completeness

> **Phase baseline tree**: `a9b42d7b83e7baa00a61a2097d8aabf481511f04` (the `056f978` tree)
> **Lenses**: correctness + memory-safety + emission-integrity + semantics (view lifetimes)
> **Reasoning**: high — five release holes; one release per created owner is the invariant

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-owner-release-completeness.md) §Invariants · [AR-7, AR-8](00-ambiguity-register.md)
**Objective**: Pin the five release cases (ASan runs plus the view-array diagnostic parity)
before any implementation.

- [x] 3.1.1 [spec-author] Write specification tests from ST-8 – ST-12 — `compiler/tests/callable_release_integrity_spec_test.rs` ✅ (7 cases: temporary receiver, defer view, defer call, mixed conditional view, both owning arms, view-array parity, discarded result)
- [x] 3.1.2 Run the spec suite and record the red phase (expected: leaks under ASan; the view-array case accepted-and-leaking) ✅ (plain: 1 red — view-array accepted; ASan: 6 red with the filed byte counts; ST-9a's 11 B proven on the host where the container's LSan misses it)

### Step 3.2: Implementation

**Reference**: [03-03](03-03-owner-release-completeness.md) §Proposed Changes
**Objective**: Extend the owner/release machinery to each hole.

- [x] 3.2.1 Root temporary func-field receivers (DEF-139) — `compiler/src/own/call.rs`, `compiler/src/own/lower.rs` ✅
- [x] 3.2.2 Enable defer-body materialization (DEF-158) — `compiler/src/own/statements.rs` ✅ (plus the `defer_body` scope rule and the speculative-pass gate split)
- [x] 3.2.3 Own conditional view arms (DEF-159) — `compiler/src/own/lower.rs` ✅ (per-arm branch roots in the three binders; the projection-arm predicate also closes DEF-175's conditional spelling)
- [x] 3.2.4 Cover view-element arrays in the elision/escape checks (DEF-163) — `compiler/src/own/place.rs`, `compiler/src/borrow.rs` ✅ (the predicate extension drives the existing chec (the `is_elided_view` recursion covers the loan/provenance and E10036 paths)
- [x] 3.2.5 Release discarded owned call statements (DEF-164) — `compiler/src/own/statements.rs` ✅
- [x] 3.2.6 Run the spec suite to green; on any failure STOP and fix the implementation, never the test ✅ (spec 35/35 plain and ASan; `make check` 58 suites)

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test"`; `make check`

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-owner-release-completeness.md) §Testing Requirements
**Objective**: Pin the owner shapes, release counts, and placement; record the phase review
evidence.

- [x] 3.3.1 Write the Phase 3 implementation tests (owner binding shapes, release counts, registered-release placement, byte-identity) — `compiler/tests/callable_release_integrity_impl_test.rs` ✅ (7 pins; impl 18/18)
- [~] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ⏳ (implemented: 2026-10-10 23:31; implementation committed; four-lens review in flight)

**Deliverables**:
- Five release holes closed with exactly-one-release semantics
- View-array return diagnostic parity
- Phase 3 implementation tests

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_impl_test"`; `RDC_ASAN=1` spec cases ST-8 – ST-12

---

## Phase 4: Possibly-null widening

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness reviewer + semantics reviewer (the narrowing rule is frozen)
> **Reasoning**: medium — one rejection site with carve-out fidelity

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-possibly-null-widening.md) §Proposed Changes · [AR-9](00-ambiguity-register.md)
**Objective**: Pin the `E10010` report and the two accepted controls before implementation.

- [ ] 4.1.1 [spec-author] Write specification tests from ST-13 — `compiler/tests/callable_release_integrity_spec_test.rs`
- [ ] 4.1.2 Run the spec suite and record the red phase (expected: the first case is accepted and segfaults)

### Step 4.2: Implementation

**Reference**: [03-04](03-04-possibly-null-widening.md) §Proposed Changes
**Objective**: Check the widening operand with the existing narrowing predicate.

- [ ] 4.2.1 Implement the possibly-null widening check at every conversion site — `compiler/src/own/expr.rs`, `compiler/src/own/lower.rs`
- [ ] 4.2.2 Run the spec suite to green; on any failure STOP and fix the implementation, never the test

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test"`; `make check`

### Step 4.3: Implementation Tests & Hardening

**Reference**: [03-04](03-04-possibly-null-widening.md) §Testing Requirements
**Objective**: Pin the report count, the message/help, and the accepted spellings' byte-identity;
record the phase review evidence.

- [ ] 4.3.1 Write the Phase 4 implementation tests (one report per site, message/help, controls' byte-identity, module-initializer spelling) — `compiler/tests/callable_release_integrity_impl_test.rs`
- [ ] 4.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode

**Deliverables**:
- The widening use reports `E10010`; no silent zero
- Phase 4 implementation tests

**Verify**: `make -C docker exec CMD="cargo test --test callable_release_integrity_impl_test"`; `make check`

---

## Phase 5: Batch close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: none new — the close-out runs the full verification over already-reviewed phases
> **Reasoning**: low — verification and documentation sync only

### Step 5.1: Final Verification & Documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria
**Objective**: Run the full acceptance, sync the docs and the roadmap, and close the plan.

- [ ] 5.1.1 Run the full verification: `make check`, `make verify`, the corpus strict-build sweep, and the ASan spec suite; record the evidence
- [ ] 5.1.2 Update `AGENTS.md` (test naming map with the new suites)
- [ ] 5.1.3 Sync the roadmap rows (DEF-138 – DEF-141, DEF-155 – DEF-159, DEF-163 – DEF-166 → Done with evidence), record the findings ledger, and close the plan

**Deliverables**:
- Full verification green
- AGENTS.md and roadmap updated
- Plan closed with its review evidence

**Verify**: `make check`; `make verify`

---

## Dependencies

```
Phase 1 (generated identities)
    ↓
Phase 2 (callable effect modes)   Phase 1's identity work underlies callee identity
    ↓
Phase 3 (owner release completeness)
    ↓
Phase 4 (possibly-null widening)
    ↓
Phase 5 (close-out)
```

Phases 2 – 4 are independent in code but share the spec suite and the review cadence, so they
run in order; Phase 1 lands first because DEF-138 is the critical row and its identity contract
underlies the callee-resolution work.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed (36/36 tasks)
2. ✅ All verification passing (`make check`, `make verify`, ASan spec suite)
3. ✅ No warnings/errors in the workspace build
4. ✅ No dead code — no unused helpers or stale branches
5. ✅ Security addressed — memory safety (the batch's security property) proven by ASan probes
6. ✅ Documentation updated (AGENTS.md test map)
7. ✅ Code reviewed (the per-phase reviews recorded in the findings ledger)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill); batch F follows
