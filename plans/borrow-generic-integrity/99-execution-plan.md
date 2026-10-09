# Execution Plan: Borrow/Generic Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-09 22:26
> **Progress**: 16/30 tasks (53%)
> **CodeOps Artifact Schema**: 1

## Overview

Four phases close the six batch defects ([AR-11](00-ambiguity-register.md)): the borrow-store
rules (DEF-78, DEF-82, DEF-83), the instance keys (DEF-79), the nullable view comparison
(DEF-81), and the may-lend callee identity (DEF-77). Each phase follows the specification-first
ordering and cites its owning [03-XX](00-index.md) document and the ST-cases in
[07-testing-strategy.md](07-testing-strategy.md).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Borrow-store rules (DEF-78, DEF-82, DEF-83) | 9 |
| 2 | Instance keys (DEF-79) | 7 |
| 3 | Nullable view comparison (DEF-81) | 6 |
| 4 | May-lend callee identity (DEF-77) | 8 |

**Total: 30 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
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

## Phase 1: Borrow-store rules (DEF-78, DEF-82, DEF-83)

> **Phase baseline tree**: `1463b89ef537fadba98cc69c124fc5b8dc954ecb` (clean tree at phase start; pre-Phase-1)
> **Scope**: strict — expected modification set: `compiler/tests/borrow_generic_integrity_spec_test.rs`, `compiler/tests/borrow_generic_integrity_impl_test.rs`, `compiler/src/own/expr.rs`, `compiler/src/own/instance_check.rs`, `compiler/src/own/call_check.rs`, `compiler/src/own/call.rs`, `compiler/src/own.rs`, `compiler/src/ir.rs`, `compiler/src/ir/mono.rs`, `plans/borrow-generic-integrity/99-execution-plan.md`
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (ownership/lend
> store rules; AGENTS.md dispatch matrix)
> **Reasoning**: high — ownership and memory-safety semantics on an assignment path

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-borrow-store-rules.md) §Proposed Changes · [AR-2, AR-5, AR-8, AR-9, AR-13](00-ambiguity-register.md)
**Objective**: Pin the corrected store rules and the preserved legal shapes before any
implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-8 — `compiler/tests/borrow_generic_integrity_spec_test.rs` ✅ (completed: 2026-10-09 21:39)
- [x] 1.1.2 Run the spec suite and record the red phase (expected: ST-1, ST-2, ST-4, ST-7, ST-8 fail; ST-3 passes (the dedupe pin, both spellings), ST-5 passes, and ST-6's concrete clauses pass while its generic-nullable clause fails; PF-004, PF-006) ✅ (completed: 2026-10-09 21:40)

**Red phase** (2026-10-09 21:40, 2 passed / 6 failed — the planned set exactly): failed as expected — the concrete store, the fresh-value store, the owning generic store, the out-slot self-assignment, the warning exemption, and the nullable instantiation (its generic clause); passed as planned — the both-lends dedupe pins (concrete and generic) and the duplicable-slot acceptance.

**Deliverables**:
- Specification suite with ST-1 – ST-8
- Red-phase evidence with per-case justification for the passing pins

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-borrow-store-rules.md) §Proposed Changes
**Objective**: Implement the target-side store rule, the duty audit, the out-slot self-assign
rejection, and the warning exemption.

- [x] 1.2.1 [DEF-78] Record `RequiresStorableSlot` and reject the concrete owning store, with the recording-time dedupe — `compiler/src/ir/mono.rs`, `compiler/src/ir.rs`, `compiler/src/own.rs`, `compiler/src/own/expr.rs` ✅ (completed: 2026-10-09 21:49)
- [x] 1.2.2 [DEF-78] Audit the duty per instance and exempt nullable substitutions in the not-lend audit — `compiler/src/own/instance_check.rs`, `compiler/src/own.rs` ✅ (completed: 2026-10-09 21:49)
- [x] 1.2.3 [DEF-82] Reject the whole-binding and exact projected out-slot self-assignment — `compiler/src/own/expr.rs` ✅ (completed: 2026-10-09 21:49)
- [x] 1.2.4 [DEF-83] Exempt nullable borrow parameters from `W10001` using the effective (explicit or inferred) parameter type — `compiler/src/own/call_check.rs`, `compiler/src/own/call.rs` ✅ (completed: 2026-10-09 21:49)
- [x] 1.2.5 Run the spec suite to green (ST-1 – ST-8) and the host-ASan probes for ST-6 (`RDC_ASAN=1` inside the container command; run probes individually — the full-suite sanitizer run is flaky); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-09 21:49)

**Verification (2026-10-09 21:45–21:49)**: spec suite 8/8 green; ASan probe on the nullable-slot case clean; `make check` green. Runtime note: the pinned container's ASan runtime crash-loops on PIE images under the host kernel's full page-address entropy (a trivial ASan C control hangs; the same control and the emitted program with `-no-pie` start cleanly), so the suite's sanitizer helper links `-no-pie` (AR-14, runtime delegated).

**Deliverables**:
- The source changes
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"` and, for the memory cases, `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-borrow-store-rules.md) §Testing Requirements
**Objective**: Pin the internal shapes and run the phase verification.

- [x] 1.3.1 Write implementation tests (emitted out-slot store shapes; a loop-body store reporting exactly one `E10034`; no duplicated counts) — `compiler/tests/borrow_generic_integrity_impl_test.rs` ✅ (completed: 2026-10-09 21:51)
- [x] 1.3.2 Full phase verification — `make check` ✅ (completed: 2026-10-09 21:51)

**Deliverables**:
- Implementation suite for P1
- `make check` green

**Verify**: `make check`

### Phase 1 Review (2026-10-09 22:04)

Independent review (correctness reviewer + memory-safety auditor + semantics reviewer) found one
critical issue and one phase-introduced major issue, both fixed within the phase: the not-lend
audit's nullable exemption was narrowed to nullable out-slot destinations (a generic `return` of
a nullable lend had been newly accepted and double-freed), and the one-report skip now applies
when any right-side move source is already judged (a mixed conditional had produced two
`E10034`). A scoped re-review of the fix diff confirmed both resolutions and found one follow-on
critical — a read-only source into a nullable slot was still exempted and aliased — fixed with
the source-mode gate and pinned by ST-23 (no third review pass, per the phase-review budget cap).
Pre-existing defects and the runtime-indexed residual are filed as DEF-100/DEF-101/DEF-102; the
remaining minor notes are accepted with rationale. Rulings and evidence:
[05-findings.md](05-findings.md).

---

## Phase 2: Instance keys (DEF-79)

> **Phase baseline tree**: `05900efb61c9b30f361c076c9ca06bbed4c86998` (clean tree at phase start; pre-P2)
> **Scope**: strict — expected modification set: `compiler/tests/borrow_generic_integrity_spec_test.rs`, `compiler/tests/borrow_generic_integrity_impl_test.rs`, `compiler/src/types/keys.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/types.rs`, `compiler/src/codegen/expr.rs`, `plans/borrow-generic-integrity/99-execution-plan.md`
> **Lenses**: correctness reviewer + memory-safety auditor + emission-integrity auditor
> (symbol-name and emission change; AGENTS.md dispatch matrix)
> **Reasoning**: high — C symbol/ABI spellings across the corpus

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-instance-keys.md) §Proposed Changes · [AR-4, AR-6](00-ambiguity-register.md)
**Objective**: Pin the distinct-instance behavior, the unchanged function-instance spellings, and
determinism.

- [x] 2.1.1 [spec-author] Write specification tests from ST-9 – ST-12 — `compiler/tests/borrow_generic_integrity_spec_test.rs` ✅ (completed: 2026-10-09 22:22)
- [x] 2.1.2 Run the spec suite and record the red phase (expected: ST-9, ST-11, and ST-10's nested clause fail on the redefinition; ST-10's top-level clause and ST-12 pass as pins) ✅ (completed: 2026-10-09 22:22)

**Red phase** (2026-10-09 22:22, 13 passed / 3 failed — the planned set exactly): failed as expected — the distinct-struct case, the nested-and-slice case, and the nested function-instance clause, each on `error: redefinition of 'struct ClWrap__String'` from the strict host C build; passed as planned — the top-level function-instance clause and the byte-identical re-emission pin (plus all P1 cases).

**Deliverables**:
- Specification cases ST-9 – ST-12 added
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-instance-keys.md) §Proposed Changes
**Objective**: Fold nullability into the key and remove the wrapper.

- [x] 2.2.1 Fold nullability into `type_key`; preserve the text-conversion spelling — `compiler/src/types/keys.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-09 22:24)
- [x] 2.2.2 Remove `instance_arg_key`; point its two call sites at `type_key` — `compiler/src/codegen/types.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/expr.rs` ✅ (completed: 2026-10-09 22:24)
- [x] 2.2.3 Run the spec suite to green (ST-9 – ST-12, including the nested function-instance key); on any failure STOP and fix the implementation ✅ (completed: 2026-10-09 22:25)

**Test-source corrections (2026-10-09 22:24)**: the first ST-11 sources were not expressible as written — `slice` needs `import { slice } from std.mem;`, and a generic struct instance cannot carry a view field (`E10205`, spec/07 §9), so the slice witness rides a top-level function type argument (`pick<String[]?>` versus `pick<String[]>`). With the corrected source, the baseline probe confirms the red reason is the strict-build redefinition (`ClWrap__String`); ST-11's struct pair (`Outer<Wrap<String?>>` versus `Outer<Wrap<String>>`) is the nested red clause and the slice pair is a green pin. Recorded as AR-15 (runtime).

**Deliverables**:
- The key change and the wrapper removal
- Green spec suite

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-instance-keys.md) §Testing Requirements
**Objective**: Pin the symbol spellings and byte stability; hand the emission sweep to the phase
review.

- [x] 2.3.1 Write implementation tests (typedef and symbol names, top-level function-instance byte pins, the nested `Q` key, the `Q`-limitation note, re-emission) — `compiler/tests/borrow_generic_integrity_impl_test.rs` ✅ (completed: 2026-10-09 22:26)
- [x] 2.3.2 Full phase verification — `make check` ✅ (completed: 2026-10-09 22:26)

**Deliverables**:
- Implementation suite for P2
- `make check` green
- The emission sweep runs in the phase review (base↔head; nested function-instance renames are expected fixing diffs — PF-003)

**Verify**: `make check`

---

## Phase 3: Nullable view comparison (DEF-81)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (operator rule;
> AGENTS.md dispatch matrix)
> **Reasoning**: medium — one sema rule with an existing emitter path; bounded

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-nullable-view-comparison.md) §Proposed Changes · [AR-7](00-ambiguity-register.md)
**Objective**: Pin the acceptance, the narrowing run, and the preserved rejections.

- [ ] 3.1.1 [spec-author] Write specification tests from ST-13 – ST-16 and ST-7's projected clause (the out-slot self-assign rule becomes expressible with the view acceptance) — `compiler/tests/borrow_generic_integrity_spec_test.rs`
- [ ] 3.1.2 Run the spec suite and record the red phase (expected: ST-13, ST-14, ST-16, and ST-7's projected clause fail; ST-15 passes as the preserved-rejection pin)

**Deliverables**:
- Specification cases ST-13 – ST-16 added
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-nullable-view-comparison.md) §Proposed Changes
**Objective**: Accept the view-versus-`null` pairing in the equality arm.

- [ ] 3.2.1 Implement the pairing rule — `compiler/src/sema/ops.rs`
- [ ] 3.2.2 Run the spec suite to green (ST-13 – ST-16); on any failure STOP and fix the implementation

**Deliverables**:
- The sema rule change
- Green spec suite

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-nullable-view-comparison.md) §Testing Requirements
**Objective**: Pin the emitted `.ptr` comparison and the preserved diagnostic.

- [ ] 3.3.1 Write implementation tests (the emitted `.ptr` comparison shape for both operand orders; ST-15 owns the E10222 rejection) — `compiler/tests/borrow_generic_integrity_impl_test.rs`
- [ ] 3.3.2 Full phase verification — `make check`

**Deliverables**:
- Implementation suite for P3
- `make check` green

**Verify**: `make check`

---

## Phase 4: May-lend callee identity (DEF-77)

> **Phase baseline tree**: _(recorded by the exec-plan skill at phase start)_
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer + emission-integrity auditor (borrow gate, IR parameter names, and emitted C names; AGENTS.md dispatch matrix; PF-001)
> **Reasoning**: high — ownership binding semantics; the leak oracle is ASan

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-may-lend-callee-identity.md) §Proposed Changes · [AR-3](00-ambiguity-register.md)
**Objective**: Pin the leak repro and the unchanged sibling behaviors.

- [ ] 4.1.1 [spec-author] Write specification tests from ST-17 – ST-20 — `compiler/tests/borrow_generic_integrity_spec_test.rs`
- [ ] 4.1.2 Run the spec suite and record the red phase (expected: ST-17 fails under ASan — run it with `RDC_ASAN=1`; ST-18 – ST-20 pass as pins)

**Deliverables**:
- Specification cases ST-17 – ST-20 added
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 4.2: Implementation

**Reference**: [03-04](03-04-may-lend-callee-identity.md) §Proposed Changes
**Objective**: Extend the callee-name masking guard to every binding kind.

- [ ] 4.2.1 Extend the mask condition in `unique_ir_name` and carry the masked parameter spelling through the IR and emitter — `compiler/src/own.rs`, `compiler/src/codegen.rs`, `compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs`
- [ ] 4.2.2 Run the spec suite to green (ST-17 – ST-20) and the ST-17 ASan probe (`RDC_ASAN=1` inside the container); update the two existing pinned tests before the green gate (`runtime_value_integrity_impl_test.rs`, `call_value_integrity_impl_test.rs`); on any failure STOP and fix the implementation

**Deliverables**:
- The guard and plumbing changes
- Green spec suite and clean ASan probe

**Verify**: `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test"` and, for the leak case, `make -C docker exec CMD="RDC_ASAN=1 cargo test --test borrow_generic_integrity_spec_test"`

---

### Step 4.3: Implementation Tests & Hardening

**Reference**: [03-04](03-04-may-lend-callee-identity.md) §Testing Requirements
**Objective**: Pin the suffixed names, close the phase, and run the final acceptance.

- [ ] 4.3.1 Write implementation tests (suffixed IR/emitted names for colliding bindings; the updated pins of the two existing tests; unchanged module-call emission) — `compiler/tests/borrow_generic_integrity_impl_test.rs`
- [ ] 4.3.2 Full phase verification — `make check`
- [ ] 4.3.3 Final acceptance — `make verify` (all legs)
- [ ] 4.3.4 Close-out: correct the DEF-78/DEF-79 roadmap row notes, add the two new suites to the AGENTS.md test-naming map, file the residual defects (DEF-100 generic whole-binding self-assign, DEF-101 concrete nullable out-slot return release, DEF-102 runtime-indexed out-slot element self-assign), and sync the roadmap (PF-005, PF-014; phase-1 review rulings in [05-findings.md](05-findings.md))

**Deliverables**:
- Implementation suite for P4
- `make check` and `make verify` green

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (borrow-store rules)
    ↓
Phase 2 (instance keys)
    ↓
Phase 3 (nullable view comparison)
    ↓
Phase 4 (may-lend callee identity)
```

The phases run sequentially; P1, P2, and P3 are independent in code. P4 carries the masked
parameter spelling through `codegen/expr.rs`, which P2 also owns (the sequential order absorbs
the overlap; PF-001). Within Phase 1, tasks 1.2.1 → 1.2.2 are sequential; 1.2.3 and 1.2.4 are
independent of each other and of 1.2.1/1.2.2.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per phase; `make verify` final)
3. ✅ The six verified repros of [02-current-state.md](02-current-state.md) behave per R1 – R9
4. ✅ No warnings/errors; no change under `spec/`
5. ✅ Host-ASan probes clean for the P1 and P4 shapes
6. ✅ No dead code — `instance_arg_key` is removed rather than left unused
7. ✅ Code reviewed per the repo's strict quality profile (independent phase reviews per AGENTS.md)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
