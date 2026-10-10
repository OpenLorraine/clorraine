# Execution Plan: Store-Emission Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-10-10 09:41
> **Progress**: 24/26 tasks (92%)
> **CodeOps Artifact Schema**: 1

## Overview

Four phases close the batch defects ([AR-1](00-ambiguity-register.md)), one per defect: the
conditional-store judgment (DEF-113), the immutable-view write rule with its authorized spec
addition (DEF-114), the typedef emission order (DEF-115), and the module-initializer emission
(DEF-116). Each phase follows the specification-first ordering and cites its owning
[03-XX](00-index.md) document and the ST-cases in
[07-testing-strategy.md](07-testing-strategy.md).

Execution commit mode: **auto-commit** (user instruction) — commit and push each verified task
via the git-commit skill in push mode ([AR-16](00-ambiguity-register.md)).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Conditional-store judgment (DEF-113) | 6 |
| 2 | Immutable-view writes (DEF-114) | 7 |
| 3 | Typedef emission order (DEF-115) | 6 |
| 4 | Module initializers (DEF-116) | 7 |

**Total: 26 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the
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

## Phase 1: Conditional-store judgment (DEF-113)

> **Phase baseline tree**: 41bf2544e13bd8d7260b8dd4728c071ed70ad58b
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (ownership and
> lend-store language rules; `AGENTS.md` dispatch matrix, AR-14)
> **Reasoning**: high — ownership semantics on the lend-store path with memory-safety
> consequences

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-conditional-store-judgment.md) §Behavior Table · [AR-4, AR-5, AR-13](00-ambiguity-register.md)
**Objective**: Pin the corrected and preserved conditional-store judgments before any
implementation.

- [x] 1.1.1 [spec-author] Write specification tests from ST-1 – ST-9 — `compiler/tests/store_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 08:41)
- [x] 1.1.2 Run the spec suite and record the red phase (expected: ST-1 and ST-2 fail; ST-3 – ST-9 pass as deliberate regression pins, each recorded with its justification) ✅ (completed: 2026-10-10 08:41)

**Red-phase evidence (2026-10-10 08:41)** — the new suite (`store_emission_integrity_spec_test`,
nine tests) runs against the phase baseline and fails exactly as designed: 7 passed, 2 failed.
ST-1 and ST-2 fail (the concrete and generic conditional self-store are accepted today; the run
asserts the required rejection). ST-3 – ST-9 pass as deliberate regression pins:
- ST-3 (duplicable generic instance): the store is a copy; accepted, and the callee's own copy
  reads the chosen arm's value — pinned because the fix must leave the accepted shape's emission
  and behavior unchanged. (A scalar lend binds a copy, so the program observes the value through
  the callee, not through the caller's binding.)
- ST-4 (two taken arms): the conditional has no lend arm — the existing single report on the
  target must survive the refinement.
- ST-5 (direct store from a read-only lend): the source-side report must survive unchanged.
- ST-6 (mixed conditional with a read-only arm): the arm's single report must stay the only one.
- ST-7 (nullable out-slot conditional): the safe carve-out emission must stay accepted and
  memory-clean.
- ST-8 (nullable generic instance): the same carve-out, judged per instance.
- ST-9 (direct self-store): the target-side single report must survive.
Run: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"` → 7 passed,
2 failed (the two red cases).

**Deliverables**:
- Specification suite with ST-1 – ST-9
- Red-phase evidence with per-pin justification

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`

---

### Step 1.2: Implementation

**Reference**: [03-01](03-01-conditional-store-judgment.md) §Proposed Changes
**Objective**: Exclude the exact-target arm from the conditional suppression so the store rule
judges the conditional like the direct store.

- [x] 1.2.1 Make `lend_moves_judged` target-aware and filter the conditional walks' self leaves (`own/expr.rs`), updating its doc comment in plain language ✅ (completed: 2026-10-10 08:45)
- [x] 1.2.2 Run the spec suite to green (ST-1 – ST-9); on any failure STOP and fix the implementation, never the test; run the host ASan probes for the ST-1/ST-2 shapes and the accepted ST-3/ST-7/ST-8/ST-9 alternatives ✅ (completed: 2026-10-10 08:45)

**Green-phase evidence (2026-10-10 08:45)** — the suite passes 9/9 in the pinned container. The
host AddressSanitizer probes are clean for the accepted memory shapes: the duplicable generic
instance (exit 7), the nullable out-slot conditional (exit 4), and the nullable generic instance
(exit 4) each build with `-fsanitize=address -no-pie` and run with leak detection on, reporting
nothing. The two amended shapes are rejected before emission (no translation unit), so no runtime
probe applies to them. `make check` is fully green: `spec check: OK (27 files, 105 compile-fail
tests)` and every workspace test binary reports zero failures.

**Deliverables**:
- The checker change
- Green spec suite and clean ASan probes

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`; the per-case host probes are the memory oracle (emit, `cc -std=c99 -g -fsanitize=address -no-pie`, run with leak detection); the in-container `RDC_ASAN=1` run is a non-gating convenience (AR-13)

---

### Step 1.3: Implementation Tests & Hardening

**Reference**: [03-01](03-01-conditional-store-judgment.md) §Testing Requirements
**Objective**: Pin the internal shapes and record the phase review evidence.

- [x] 1.3.1 Write the Phase 1 implementation pins (report counts and names per shape, the duty naming, byte-identical re-emission) — `compiler/tests/store_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 08:46)
- [x] 1.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ([AR-16](00-ambiguity-register.md)) ✅ (completed: 2026-10-10 08:53)

**Phase review evidence (2026-10-10 08:53)** — three independent reviews ran on the phase diff
(correctness reviewer, memory-safety auditor, semantics reviewer; AR-14): no critical or major
findings. The correctness reviewer reproduced every report shape and confirmed the blast radius is
confined to the single suppression test; the memory-safety auditor reproduced the base-compiler
heap-use-after-free on the amended shapes, confirmed the head rejects them before emission, and
verified every accepted shape builds and runs ASan-clean (23 related suites also pass); the
semantics reviewer confirmed conformance to the lend-store rule and every pinned spelling, with no
evaluation change. Minor rulings are recorded in [05-findings.md](05-findings.md) (one bookkeeping
correction, applied).

**Deliverables**:
- Implementation pins for Phase 1
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test store_emission_integrity_impl_test"`

---

## Phase 2: Immutable-view writes (DEF-114)

> **Phase baseline tree**: edbb6fbd788922c9b456006f1bd3ab9de987e6b2
> **Lenses**: correctness reviewer + memory-safety auditor + semantics reviewer (a language-rule
> change with an authorized spec addition; AR-14)
> **Reasoning**: high — a new diagnostic and the only authorized frozen-spec change

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-immutable-view-writes.md) §Proposed Changes · [AR-2, AR-6, AR-7, AR-12](00-ambiguity-register.md)
**Objective**: Land the authorized registry row and golden, then pin the rule before any
implementation.

- [x] 2.1.1 [spec] Add the `E10042` registry row (`spec/14-diagnostics.md`) and the golden `spec/tests/compile-fail/E10042_write_immutable_view.clo` — exactly the authorized modification set (AR-12), plus the driver's asserted-code registration (AR-19, runtime) ✅ (completed: 2026-10-10 08:57)
- [x] 2.1.2 [spec-author] Write specification tests from ST-10 – ST-18 (including the local-write case ST-16b) — `compiler/tests/store_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 08:57)
- [x] 2.1.3 Run the spec suite and `./spec/check.sh`; record the red phase (expected: ST-10 – ST-15 and ST-18 fail; ST-16 and ST-17 pass as accepted controls; the new golden fails until the compiler reports `E10042`) ✅ (completed: 2026-10-10 08:57)

**Red-phase evidence (2026-10-10 08:57)** — the extended suite (18 tests) fails exactly as
designed against the phase baseline: 11 passed, 7 failed. ST-10 – ST-15 and ST-18 fail (the
compiler still accepts every immutable-view element write; the run asserts the required
`E10042`), while ST-16 and ST-17 pass as accepted controls and the Phase 1 cases stay green.
The new golden fails in the compile-fail driver until the implementation lands
(`E10042_write_immutable_view.clo: no 'error E10042: ...' diagnostic line (exit Some(0))`).
`./spec/check.sh` is green (27 files, 106 compile-fail tests). ST-11 and ST-16b describe the
same local-write program; one case pins both rows (AR-20).

**Deliverables**:
- The registry row and golden
- Specification suite with ST-10 – ST-18
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 2.2: Implementation

**Reference**: [03-02](03-02-immutable-view-writes.md) §Proposed Changes
**Objective**: Reject every write through an element of a `str`/`static str` view.

- [x] 2.2.1 Add the immutable-view write check and call it from the local write path (`write_place`) and the global write branches (`check_assign`/`check_update`) — `compiler/src/own/place.rs`, `compiler/src/own/expr.rs` ✅ (completed: 2026-10-10 09:00)
- [x] 2.2.2 Run the spec suite and `./spec/check.sh` to green (ST-10 – ST-18 and the golden); on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 09:00)

**Green-phase evidence (2026-10-10 09:00)** — the suite passes 18/18 in the pinned container (the
seven immutable-view rejections report exactly one `E10042` naming the written path; the rebind
and writable-slice controls build and run). The compile-fail driver asserts the new golden green,
and `make check` passes end to end: `spec check: OK (27 files, 106 compile-fail tests)` with every
workspace suite reporting zero failures.

**Deliverables**:
- The checker change
- Green spec suite and green frozen-suite check

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`; `./spec/check.sh`

---

### Step 2.3: Implementation Tests & Hardening

**Reference**: [03-02](03-02-immutable-view-writes.md) §Testing Requirements
**Objective**: Pin the internal shapes and record the phase review evidence.

- [x] 2.3.1 Write the Phase 2 implementation pins (one report per write form, no translation unit for a rejected write, the accepted controls' emission unchanged, byte-identical re-emission) — `compiler/tests/store_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 09:02)
- [x] 2.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 09:16)

**Phase review evidence (2026-10-10 09:16)** — three independent reviews ran on the phase diff
(correctness reviewer, memory-safety auditor, semantics reviewer; AR-14). The semantics review was
clean (conformance to the mutability table, the taxonomy ordering, and the spec boundary), and the
memory audit found no defect in the reviewed change. The correctness review found a critical gap
(a value-position update, `i32(s[0]++)`, bypassed the new rule) and the memory audit a
pre-existing `raw`-pointer spelling; both were fixed in the remediation commit (a value-position
update now judges the rule through a shared helper with the statement path routed so each site
reports once; the projection walk's field step reaches through a raw pointer), with ST-14b and
implementation pins added. The pre-existing residuals were filed on the roadmap (DEF-118: a `str`
element loop is accepted and emits C that does not build; DEF-119: value-position updates bypass
the const and read-only write rules). The one scoped re-review accepted both remediations with no
findings (the base compiler accepts and the head rejects every closed shape; legal updates emit
byte-identically; suites 19/19 + 5/5; the workspace runs 52 suites green). Rulings are recorded in
[05-findings.md](05-findings.md).

**Deliverables**:
- Implementation pins for Phase 2
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test store_emission_integrity_impl_test"`

---

## Phase 3: Typedef emission order (DEF-115)

> **Phase baseline tree**: 2e484dc9da88a79ed4b4ec471707fcacf6ee57ff
> **Lenses**: correctness reviewer + memory-safety auditor + emission-integrity auditor
> (codegen/emission change; AR-14)
> **Reasoning**: medium — a deterministic ordering change verified by strict builds

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-typedef-order.md) §Testing Requirements · [AR-3, AR-8](00-ambiguity-register.md)
**Objective**: Pin the three ordering instances and the closure-capture regression before any
implementation.

- [x] 3.1.1 [spec-author] Write specification tests from ST-19 – ST-22 — `compiler/tests/store_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 09:18)
- [x] 3.1.2 Run the spec suite and record the red phase (expected: ST-19 – ST-21 fail at the strict build; ST-22 passes as a regression control) ✅ (completed: 2026-10-10 09:18)

**Red-phase evidence (2026-10-10 09:18)** — the extended suite (23 tests) fails exactly as
designed against the phase baseline: 20 passed, 3 failed. ST-19, ST-20, and ST-21 fail at the
strict build (`unknown type name 'ClP'` and `'ClE'` diagnostics from the host C compiler), while
ST-22 passes as the emission-order regression control and every earlier case stays green.

**Deliverables**:
- Specification suite with ST-19 – ST-22
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`

---

### Step 3.2: Implementation

**Reference**: [03-03](03-03-typedef-order.md) §Proposed Changes
**Objective**: Emit every typedef before its uses.

- [x] 3.2.1 Reorder the emission phases: handle typedefs, enum typedefs, struct forward typedefs, slice typedefs, struct definitions — `compiler/src/codegen.rs`, `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-10 09:20)
- [x] 3.2.2 Run the spec suite to green (ST-19 – ST-22); confirm the strict build and byte-identical re-emission; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 09:20)

**Green-phase evidence (2026-10-10 09:20)** — the suite passes 23/23: the three ordering
instances build under the strict flags and run, and the closure control stays green. The emitted
order is the required one (`typedef struct ClP ClP;` precedes `ClSlice_P`, which precedes
`struct ClP { ... }`; the enum typedef and its member macros precede the struct forward typedef).
Re-emitting a covered program is byte-identical, and `make check` is fully green (52 suites;
`spec check: OK (27 files, 106 compile-fail tests)`).

**Deliverables**:
- The emission reorder
- Green spec suite with strict builds

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`

---

### Step 3.3: Implementation Tests & Hardening

**Reference**: [03-03](03-03-typedef-order.md) §Testing Requirements
**Objective**: Pin the emitted order and record the phase review evidence.

- [x] 3.3.1 Write the Phase 3 implementation pins (typedef order positions in the emitted text, byte-identical re-emission) — `compiler/tests/store_emission_integrity_impl_test.rs` ✅ (completed: 2026-10-10 09:21)
- [x] 3.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode ✅ (completed: 2026-10-10 09:32)

**Phase review evidence (2026-10-10 09:32)** — three independent reviews ran on the phase diff
(correctness reviewer, memory-safety auditor, emission-integrity auditor; AR-14). No findings were
introduced by the change: the correctness review enumerated every emitted dependency edge and
proved the diff is declaration-order-only (identical line multisets); the memory audit found every
reorder-touched shape strict-clean and ASan-clean with byte-identical bodies; the emission-integrity
audit diffed 796 accepted programs plus the corpus (0 content diffs, 0 head-only build regressions,
no duplicate typedefs) and confirmed the six previously failing shapes now build. Two pre-existing
findings were filed on the roadmap (SR-008 → DEF-120: a multi-extent row slice of a user struct
still emits an incomplete-element typedef on base and head; SR-009 → DEF-116, the batch's own
Phase 4). A minor note (the doubled `struct_units()` computation) was dismissed as a pure, stable
computation. Rulings are recorded in [05-findings.md](05-findings.md).

**Deliverables**:
- Implementation pins for Phase 3
- Review evidence recorded

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test store_emission_integrity_impl_test"`

---

## Phase 4: Module initializers (DEF-116)

> **Phase baseline tree**: 0bbe213f2125ad16f6801f2d54fa63506d1f4b25
> **Lenses**: correctness reviewer + memory-safety auditor + emission-integrity auditor
> (codegen/emission change; AR-14)
> **Reasoning**: high — evaluation-order semantics (operand-once) plus emission shapes

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-module-initializers.md) §Proposed Changes · [AR-9, AR-10, AR-11, AR-18](00-ambiguity-register.md)
**Objective**: Pin the fill, intrinsic, and operand-once behaviors before any implementation.

- [x] 4.1.1 [spec-author] Write specification tests from ST-23 – ST-30 — `compiler/tests/store_emission_integrity_spec_test.rs` ✅ (completed: 2026-10-10 09:34)
- [x] 4.1.2 Run the spec suite and record the red phase (expected: ST-23 – ST-29 all fail at the strict build or the run) ✅ (completed: 2026-10-10 09:34)

**Red-phase evidence (2026-10-10 09:34)** — the extended suite (31 tests) fails exactly as
designed against the phase baseline: 23 passed, 8 failed. Every new case fails at the strict
build: the array-typed initializers emit an invalid array assignment (`backing = { 1, 2 };` and
the nested, fill, and place-copy variants), the module `slice` call renders an unlowered bare
name, and the `len`-of-a-module-array case fails on its array-assignment line too — so the red
set is all eight cases (the plan's ST-23 – ST-29 list extends by ST-30).

**Deliverables**:
- Specification suite with ST-23 – ST-29
- Red-phase evidence

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`

---

### Step 4.2: Implementation

**Reference**: [03-04](03-04-module-initializers.md) §Proposed Changes
**Objective**: Fill array-typed initializers element-wise, lower intrinsic calls, and evaluate
impure operands once.

- [x] 4.2.1 Implement the element-wise array fill (literal, fill, place-copy, and conditional values) in `emit_global_init` — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-10 09:41)
- [x] 4.2.2 Implement the intrinsic-call detection in `lower_global` and the init-local materialization for impure intrinsic operands — `compiler/src/codegen/program.rs` ✅ (completed: 2026-10-10 09:41)
- [x] 4.2.3 Run the spec suite to green (ST-23 – ST-29) and the strict-build runs; on any failure STOP and fix the implementation, never the test ✅ (completed: 2026-10-10 09:41)

**Green-phase evidence (2026-10-10 09:41)** — the suite passes 31/31 on the first run: the array
fill stores elements in declaration order (literal, nested rows, fill tail, place copy), the
`slice` intrinsic constructs the view behind its bounds guard, an impure offset is stored once
into `i32 __cl_itmp0` before the construction, the conditional's condition is stored once into
`bool __cl_itmp0` with per-element ternaries over the indexed arms, and the dot-`len` folds to the
extent. The strict builds and runs assert the exits, and `make check` is fully green (52 suites;
`spec check: OK (27 files, 106 compile-fail tests)`).

**Deliverables**:
- The fill walk, intrinsic lowering, and init-local materialization
- Green spec suite with strict builds and runs

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"`

---

### Step 4.3: Implementation Tests & Hardening

**Reference**: [03-04](03-04-module-initializers.md) §Testing Requirements
**Objective**: Pin the emitted shapes and record the phase review evidence.

- [ ] 4.3.1 Write the Phase 4 implementation pins (fill statement order, init-local materialization shape, intrinsic rendering in an init function, byte-identical re-emission) — `compiler/tests/store_emission_integrity_impl_test.rs`
- [ ] 4.3.2 Phase review evidence in the findings ledger and plan progress update — commit per the active commit mode; run `make check` for the phase close-out

**Deliverables**:
- Implementation pins for Phase 4
- Review evidence recorded; `make check` green

**Verify**: `make -C docker exec CMD="cargo test --test store_emission_integrity_spec_test"` and `make -C docker exec CMD="cargo test --test store_emission_integrity_impl_test"`; `make check`

---

## Dependencies

```
Phase 1 (DEF-113, own/expr.rs)
    ↓
Phase 2 (DEF-114, own/place.rs + own/expr.rs)   — independent of P1, sequenced after it
    ↓
Phase 3 (DEF-115, codegen order)
    ↓
Phase 4 (DEF-116, codegen/program.rs)           — shares program.rs with P3; sequenced after it
```

Phases are independent in behavior; the sequence keeps shared files (`own/expr.rs`,
`codegen/program.rs`) touched by one phase at a time.

---

## Success Criteria

**Feature is complete when:**

1. ✅ All four phases completed (26/26 tasks)
2. ✅ All verification passing (`make check` per phase; `make verify` at close-out)
3. ✅ No warnings/errors (strict builds for every covered emission)
4. ✅ No dead code — no unused parameters, functions, or modules
5. ✅ Memory hardening — host ASan probes clean for the DEF-113 family
6. ✅ Documentation — the authorized registry row and golden; plain-language comments updated
7. ✅ Code reviewed per the repository's phase-review dispatch
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
