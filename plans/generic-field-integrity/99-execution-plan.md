# Execution Plan: Generic Field Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-26 02:48
> **Progress**: 24/24 tasks (100%) — Done
> **CodeOps Artifact Schema**: 1

## Overview

Fix DEF-06…DEF-10 by substituting a generic struct instance's type arguments
into its field types in every stage that reads a field. A shared helper
(`types::substitute_instance`) plus `TypeTable::instance_field` becomes the single
owner of that substitution; the type table, checker, ownership stage, and emitter
route through it. Tests are specification-derived regressions in the existing
Rust suites, written red before each fix (AR-4).

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Types substitution foundation | 7 |
| 2 | Checker field reads | 6 |
| 3 | Ownership and emission | 8 |
| 4 | Full acceptance and closeout | 3 |

**Total: 24 tasks across 4 phases** (no fabricated hour estimates — scope is bounded by the task-size criteria in `quality-checklist.md`)

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
> Timestamps come from `date '+%Y-%m-%d %H:%M'` — never invented. Failure to keep the marks
> current means progress is invisible after crashes, context resets, or session handoffs.

---

## Phase 1: Types substitution foundation

> **Phase baseline tree**: 539a40bd88a832eff37fb5940c47dc028d7bceab
> **Expected modification set**: `compiler/src/types/mod.rs`, `compiler/src/types/table.rs`,
> `compiler/src/consts/mod.rs`, `compiler/src/sema/items.rs` (strict scope)
> **Lenses**: correctness, edge cases (constructor profile)
> **Phase review**: independent correctness review of the phase diff — no findings (2026-09-25)

### Step 1.1: Specification tests (red)

**Reference**: [03-01](03-01-types-substitution.md) · [07](07-testing-strategy.md) ST-8, ST-9 · AR-3, AR-6, AR-7, AR-13
**Objective**: Pin the type-table expectations before the helper exists.

- [x] 1.1.1 [spec-author] Write type-table regressions — ST-8 (`sizeof(Box<i32>)` folds), ST-9 (`offsetof(Pair<i32>, second)` folds), and `duplicable_at(Box<String>)` is false — in `compiler/src/types/table.rs` ✅ (completed: 2026-09-25 19:46)
- [x] 1.1.2 Run the type-table tests and confirm the new cases FAIL (red) — `cargo test --workspace --no-fail-fast` ✅ (completed: 2026-09-25 19:46)

**Deliverables**:
- New red tests compile and fail for the expected reason (generic size/offset unknown; `Box<String>` reported duplicable)

**Verify**: `make -C docker exec CMD="cargo test -p clc types::table"` shows the new cases red

### Step 1.2: Implementation (green)

**Reference**: [03-01](03-01-types-substitution.md) §Implementation Details · [AR-3, AR-6, AR-7, AR-14, AR-15]
**Objective**: Add the shared helper and apply it in the type table.

- [x] 1.2.1 Add `pub fn substitute_instance(declared, params, args)` to `compiler/src/types/mod.rs` (no-op on empty `params`/`args`) and route `sema/items.rs::embeds_by_value` through it ✅ (completed: 2026-09-25 19:48)
- [x] 1.2.2 Add `TypeTable::instance_field`; substitute in `duplicable_at` and `size_of_at`; change `offset_of` to take `&Ty` and update the `consts/mod.rs` caller and unit-test call sites — `compiler/src/types/table.rs`, `compiler/src/consts/mod.rs` ✅ (completed: 2026-09-25 19:48)
- [x] 1.2.3 Run the type-table tests and confirm the Step 1.1 cases PASS (green) ✅ (completed: 2026-09-25 19:48)

**Deliverables**:
- ST-8, ST-9 pass; `duplicable_at(Box<String>)` is false; non-generic table tests unchanged

**Verify**: `make -C docker exec CMD="cargo test -p clc types::table"`

### Step 1.3: Implementation tests and phase verification

**Reference**: [03-01](03-01-types-substitution.md) §Testing Requirements · [AR-14, AR-15]
**Objective**: Cover helper edge cases and verify the phase.

- [x] 1.3.1 Add implementation tests for the helper's empty/short-argument no-op and `offset_of` on a non-generic struct — `compiler/src/types/mod.rs` / `compiler/src/types/table.rs` ✅ (completed: 2026-09-25 19:49)
- [x] 1.3.2 Full phase verification — `make check` ✅ (completed: 2026-09-25 19:50)

**Deliverables**:
- All verification passing

**Verify**: `make check` (exit 0)

---

## Phase 2: Checker field reads

> **Phase baseline tree**: 20b2329fa12b6601a3cb81a871b1374d30f89213
> **Expected modification set**: `compiler/src/sema/mod.rs`, `compiler/src/sema/expr.rs`,
> `compiler/src/sema/regressions.rs` (strict scope)
> **Lenses**: correctness, security (unsafe/overlay boundary)
> **Phase review**: independent correctness/overlay-boundary review of the phase diff — no findings (2026-09-25)

### Step 2.1: Specification tests (red)

**Reference**: [03-02](03-02-checker.md) · [07](07-testing-strategy.md) ST-1, ST-2, ST-10, ST-11 · [AR-9, AR-13, AR-14]
**Objective**: Pin the overlay and raw-field expectations.

- [x] 2.1.1 [spec-author] Write checker regressions — ST-1 (`Outer<Regs>` parameter reports `E10207`), ST-2 (non-generic control), ST-10 (`raw Pair<u8>.second` is `u8`), ST-11 (a live parameter stays `T`) — in `compiler/src/sema/regressions.rs` ✅ (completed: 2026-09-25 19:54)
- [x] 2.1.2 Confirm ST-1 and ST-10 FAIL (red) — `cargo test --workspace --no-fail-fast` ✅ (completed: 2026-09-25 19:54)

**Deliverables**:
- ST-1 red (generic overlay accepted today); ST-10 red (`r.second` is `Unknown`)

**Verify**: `make -C docker exec CMD="cargo test -p clc sema::regressions"` shows the new cases red

### Step 2.2: Implementation (green)

**Reference**: [03-02](03-02-checker.md) §Implementation Details · [AR-9, AR-13]
**Objective**: Substitute in the overlay walk and route checker field reads through the helper.

- [x] 2.2.1 Substitute instance arguments in `overlay_reached`; replace `Checker::field_type`'s body with `TypeTable::instance_field` and follow a `TyKind::Raw` base — `compiler/src/sema/mod.rs`, `compiler/src/sema/expr.rs` ✅ (completed: 2026-09-25 20:00)
- [x] 2.2.2 Confirm the Step 2.1 cases PASS (green) ✅ (completed: 2026-09-25 20:00)

**Deliverables**:
- ST-1, ST-2, ST-10, ST-11 pass; the DEF-05 field-bound cases still pass

**Verify**: `make -C docker exec CMD="cargo test -p clc sema::regressions"`

### Step 2.3: Implementation tests and phase verification

**Reference**: [03-02](03-02-checker.md) §Testing Requirements
**Objective**: Cover nested/raw edges and verify the phase.

- [x] 2.3.1 Add an implementation test that `Pair<Pair<i32>>.first` is `Pair<i32>` and that a raw base peels to its pointee — `compiler/src/sema/regressions.rs` ✅ (completed: 2026-09-25 20:01)
- [x] 2.3.2 Full phase verification — `make check` ✅ (completed: 2026-09-25 20:02)

**Deliverables**:
- All verification passing

**Verify**: `make check` (exit 0)

---

## Phase 3: Ownership and emission

> **Phase baseline tree**: eb9cd5febdc1d64acdd37d5e8cddd8fb13ba769c
> **Expected modification set**: `compiler/src/own/expr.rs`, `compiler/src/own/tests.rs`,
> `compiler/src/codegen/drop.rs`, `compiler/src/codegen/expr.rs`,
> `compiler/tests/v1_language_impl_test.rs` (strict scope)
> **Lenses**: correctness, resource lifetime (drop)
> **Phase review**: RV-001 🔴 CRITICAL — a self-referential struct-literal reassignment (`b = Box<String> { value: b.value };`) now double-frees for generic owning structs: `emit_assign` (`compiler/src/codegen/function.rs:494`) treats the RHS field read as a borrow and drops the saved old value, which aliases the new value. Root cause is pre-existing (the non-generic `Label` form already double-frees at the baseline); this phase's substitution-aware `emits_drop` newly makes it reachable for generics. **User ruling (2026-09-25): track DEF-11 and fix it in a dedicated plan before Phase 4.** DEF-11 was fixed by `plans/partial-move-drop-coherence/` (2026-09-26); this plan may resume Phase 4.

### Step 3.1: Specification tests (red)

**Reference**: [03-03](03-03-ownership-codegen.md) · [07](07-testing-strategy.md) ST-3, ST-5, ST-6, ST-7, ST-12 · [AR-10, AR-11, AR-12]
**Objective**: Pin the move, drop, and zero-value expectations.

- [x] 3.1.1 [spec-author] Write regressions — ST-3 (`Box<String>` copied twice reports `E10020`), ST-5 (emitted drop releases `b.value`), ST-6 (`Pair<Pair<i32>> {}` emits nested zero states and compiles), ST-7 (`Point {}` still valid), ST-12 (non-generic emitted C unchanged) — `compiler/src/sema/regressions.rs`, `compiler/tests/v1_language_impl_test.rs` ✅ (completed: 2026-09-25 20:07; ST-3/ST-4 placed in `compiler/src/own/tests.rs` because `E10020` is reported by the ownership stage, not `sema`)
- [x] 3.1.2 Confirm ST-3, ST-5, ST-6 FAIL (red) — `cargo test --workspace --no-fail-fast` ✅ (completed: 2026-09-25 20:07)

**Red-phase note**: ST-5 and ST-6 fail (no generic field drop; nested zero value emits `{ 0, 0 }`). ST-3 and ST-4 already pass — the underlying duplicability defect (DEF-07) was fixed in Phase 1, so the ownership stage already reports `E10020`; the spec expectation is unchanged and remains the oracle. ST-7 and ST-12 pass as controls.

**Deliverables**:
- ST-3 red (no `E10020`), ST-5 red (no field drop), ST-6 red (invalid C / ICE)

**Verify**: the new cases fail for the expected reasons

### Step 3.2: Implementation (green)

**Reference**: [03-03](03-03-ownership-codegen.md) §Implementation Details · [AR-10, AR-11, AR-12, AR-14]
**Objective**: Substitute in the ownership read, drop emission, and the zero-value initializer.

- [x] 3.2.1 Route `own/expr.rs::member_type` through `TypeTable::instance_field` — `compiler/src/own/expr.rs` ✅ (completed: 2026-09-25 20:10)
- [x] 3.2.2 Substitute in `codegen/drop.rs::emit_drop_expr` and `emits_drop` (apply `self.substitute` after the instance substitution) — `compiler/src/codegen/drop.rs` ✅ (completed: 2026-09-25 20:10)
- [x] 3.2.3 Substitute the omitted-field zero state in `struct_literal` via `self.field_type(ty, field)`, and consolidate `codegen/expr.rs::field_type` onto `instance_field` — `compiler/src/codegen/expr.rs` ✅ (completed: 2026-09-25 20:10)
- [x] 3.2.4 Confirm the Step 3.1 cases PASS (green) ✅ (completed: 2026-09-25 20:10)

**Deliverables**:
- ST-3, ST-5, ST-6, ST-7, ST-12 pass

**Verify**: the Step 3.1 cases pass

### Step 3.3: Implementation tests and phase verification

**Reference**: [03-03](03-03-ownership-codegen.md) §Testing Requirements
**Objective**: Cover emit edges and verify the phase.

- [x] 3.3.1 Add implementation tests: `Box<String>` drop releases the field; a nested generic zero value round-trips through `clorraine build` — `compiler/tests/v1_language_impl_test.rs` ✅ (completed: 2026-09-25 20:11)
- [x] 3.3.2 Full phase verification — `make check` ✅ (completed: 2026-09-25 20:12)

**Deliverables**:
- All verification passing

**Verify**: `make check` (exit 0)

---

## Phase 4: Full acceptance and closeout

> **Phase baseline tree**: caf87ce7cab1fb3300c0ead75a769fe0e31b1ff8
> **Expected modification set**: `compiler/src/own/generics.rs`, `compiler/src/codegen/program.rs`,
> `plans/00-roadmap.md`, `AGENTS.md`, `plans/generic-field-integrity/**` (strict scope, expanded by
> AR-23)
> **Lenses**: regression, documentation
> **Phase review**: independent correctness review of the phase diff — no findings (2026-09-26)

### Step 4.1: Acceptance and documentation

**Reference**: [01-requirements.md](01-requirements.md) §Acceptance Criteria · [AR-4, AR-5]
**Objective**: Prove the whole feature and record completion.

- [x] 4.1.1 Search the touched modules for any remaining duplicated zip-substitute block or unsubstituted field read; confirm none remains and no dead code was introduced ✅ (completed: 2026-09-26 02:45; touched modules clean; user-ruled consolidation of the two pre-existing copies in `own/generics.rs` and `codegen/program.rs` onto `substitute_instance`; `make check` green)
- [x] 4.1.2 Full acceptance run — `make verify` ✅ (completed: 2026-09-26 02:48; all legs passed; one transient vbcc "No config file!" flake on the first attempt, green on retry)
- [x] 4.1.3 Mark DEF-06…DEF-10 Fixed on `plans/00-roadmap.md` and set this plan's roadmap row to Done; update `AGENTS.md` only if a factual note changed ✅ (completed: 2026-09-26 02:48; DEF-06–DEF-10 marked Fixed with regressions; PLAN-10 row Done; AGENTS.md unchanged — no factual note changed)

**Deliverables**:
- `make verify` green; roadmap and plan status updated

**Verify**: `make verify` (exit 0)

---

## Dependencies

```
Phase 1 (helper + type table)
    ↓
Phase 2 (checker uses instance_field)
    ↓
Phase 3 (ownership + emitter use instance_field)
    ↓
Phase 4 (full acceptance)
```

---

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check` per task; `make verify` final)
3. ✅ No warnings/errors
4. ✅ No dead code — the duplicated zip-substitute blocks are gone; the name-only `offset_of` is removed
5. ✅ Security N/A (AR-17) — no external input surface; the unsafe/overlay gate is unchanged
6. ✅ Documentation updated — roadmap DEF-06…DEF-10 fixed; plan marked Done
7. ✅ Code reviewed (strict profile — independent review of each phase's diff)
8. ✅ No `spec/` or `*_spec_test.rs` file modified
9. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
