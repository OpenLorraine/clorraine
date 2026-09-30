# Execution Plan: Call Value Integrity

> **Document**: 99-execution-plan.md
> **Parent**: [Index](00-index.md)
> **Last Updated**: 2026-09-30 15:38
> **Progress**: 2/39 tasks (5%)
> **CodeOps Artifact Schema**: 1

## Overview

Fix the sixteen filed defects DEF-30 – DEF-45 in four implementation phases (owned conditional
stores, fixed-array value positions, func pointers and callable resolution, guards and cleanup)
plus a close-out phase, following the specification-first ordering. Governing designs:
[03-01](03-01-scalar-conditional-ownership.md), [03-02](03-02-fixed-array-value-positions.md),
[03-03](03-03-func-pointers-and-callable-resolution.md), [03-04](03-04-guards-and-cleanup.md).
Specification cases: [07](07-testing-strategy.md). Decisions: `00-ambiguity-register.md`.

**🚨 Update this document after EACH completed task!**

---

## Implementation Phases

| Phase | Title | Tasks |
| ----- | ----- | ----- |
| 1 | Owned Conditional Stores | 8 |
| 2 | Fixed-Array Value Positions | 9 |
| 3 | Func Pointers and Callable Resolution | 9 |
| 4 | Guards and Cleanup | 8 |
| 5 | Close-out | 5 |

**Total: 39 tasks across 5 phases** (no fabricated hour estimates — scope is bounded by the
task-size criteria in the make-plan skill's quality checklist)

> **⚠️ EXECUTION RULE — APPLIES TO EVERY AGENT EXECUTING THIS PLAN:**
>
> The task checkboxes in the phase sections below are the **single source of truth** for
> progress. Every task line appears exactly once in this document. The executing agent MUST:
>
> 1. **On implementation:** mark the task `[~]` with a timestamp —
>    `- [~] 1.1.1 Task description ⏳ (implemented: YYYY-MM-DD HH:MM)`
> 2. **On verify pass:** promote it to `[x]` —
>    `- [x] 1.1.1 Task description ✅ (completed: YYYY-MM-DD HH:MM)`. Every implementation task
>    runs `make check` in the pinned container before it is promoted; `make check` per task is the
>    binding cadence (AR #17).
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

## Phase 1: Owned Conditional Stores

> **Phase baseline tree**: `d606000124794de3146191c65e34afc28c44a856` (recorded 2026-09-30 15:34
> from a temporary-index snapshot of committed, staged, unstaged, and untracked phase-start state)
> **Lenses**: compiler-and-language (strict profile)
> **Scope mode**: strict · **Expected modification set**: `compiler/tests/call_value_integrity_spec_test.rs`,
> `compiler/src/own/{statements,expr,lower}.rs`, `compiler/src/own.rs`, `compiler/src/codegen/{function,calls,expr,cleanup}.rs`,
> `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs`, `compiler/tests/call_value_integrity_impl_test.rs`,
> `plans/call-value-integrity/*`, `plans/00-roadmap.md`

### Step 1.1: Specification Tests

**Reference**: [03-01](03-01-scalar-conditional-ownership.md) §Proposed Changes 1–7 · AR #2, #3, #15, #19
**Objective**: Pin the owned conditional and deferred-take behavior from ST-1 – ST-8 and ST-32 – ST-37.

- [x] 1.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-1 – ST-8 and ST-32 – ST-37 — `compiler/tests/call_value_integrity_spec_test.rs` ✅ (completed: 2026-09-30 15:38)
- [x] 1.1.2 Run the specification tests — verify at least one case per Phase-1 defect fails before implementation (red phase); record ST-4/ST-7/ST-8 as pre-passing regression pins ✅ (completed: 2026-09-30 15:38)

**Red phase (2026-09-30 15:38):** 12 passed / 17 failed on the unmodified compiler
(`cargo test --test call_value_integrity_spec_test`; every failure is a run-stage double-free).
DEF-34 red cases: ST-1 (both directions), ST-2 (place arm), ST-3 (place arm), ST-32 (both),
ST-33 (both), ST-34 (place arm), ST-35 (both), ST-36 (both), ST-37 (place arm and the
struct-field place arm). Pre-passing regression pins: ST-4, ST-5, ST-6, ST-7, ST-8
(call-arm direction), and the call-arm directions of ST-2/ST-3/ST-34/ST-37. DEF-45 is
leak-only and invisible without AddressSanitizer; its red evidence is the close-out ASan probe
plus the emitted-shape implementation pin.

### Step 1.2: Implementation

**Reference**: [03-01](03-01-scalar-conditional-ownership.md) §Proposed Changes 1–3 · AR #2, #19
**Objective**: Emit the branch store for owned conditionals (handles and owned aggregates; places, calls, and mixed arms).

- [ ] 1.2.1 Owned conditional branch store: destination predicate (non-array `needs_drop`, aggregates included), old-target/save and self-naming-arm rules, nested arms, arm sources, and emission — `compiler/src/own/statements.rs`, `compiler/src/own/expr.rs`, `compiler/src/codegen/function.rs`
- [ ] 1.2.2 Transfer temporary release registration (release-on-discard) and adoption move-out at binding, assignment, `take` argument (non-releasing argument temporary), `return`, and struct-literal field — `compiler/src/own/lower.rs`, `compiler/src/own.rs`, `compiler/src/codegen/calls.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/codegen/cleanup.rs`
- [ ] 1.2.3 Update the two existing pins that assert the old transfer behavior — `compiler/tests/runtime_value_integrity_impl_test.rs`, `compiler/src/own/tests.rs` (AR #15)
- [ ] 1.2.4 Run the specification tests — verify they PASS (green phase); a failing case means the implementation is wrong

### Step 1.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests · AR #15
**Objective**: Pin the emitted shapes and run the phase verification.

- [ ] 1.3.1 Write implementation tests (branch-store shape, arm zeroing, transfer registration/release, argument temporary) — `compiler/tests/call_value_integrity_impl_test.rs`
- [ ] 1.3.2 Full verification — `make check`

**Deliverables**:
- [ ] Roadmap rows `DEF-30` – `DEF-45` set to Executing with the plan link (AR #18)
- [ ] ST-1 – ST-8 and ST-32 – ST-37 pass; at least one case per Phase-1 defect failed first (red
  phase recorded); ST-4/ST-7/ST-8 recorded as regression pins
- [ ] The two updated pins pass; the closed plans' specification suites are unchanged
- [ ] `make check` passes with no warnings

**Verify**: `make check`

---

## Phase 2: Fixed-Array Value Positions

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: compiler-and-language (strict profile)

### Step 2.1: Specification Tests

**Reference**: [03-02](03-02-fixed-array-value-positions.md) §Proposed Changes 1–4 · AR #7, #11, #13
**Objective**: Pin the fixed-array store, argument, projection, and struct-field behavior from ST-9 – ST-17 and ST-41.

- [ ] 2.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-9 – ST-17 and ST-41 — `compiler/tests/call_value_integrity_spec_test.rs`
- [ ] 2.1.2 Run the specification tests — verify at least one case per Phase-2 defect fails before implementation (red phase); record pre-passing cases as regression pins

### Step 2.2: Implementation

**Reference**: [03-02](03-02-fixed-array-value-positions.md) §Proposed Changes 1–4
**Objective**: Give every fixed-array value position a valid C place.

- [ ] 2.2.1 Element stores: refactor the fixed-array assign core to a rendered target and route constant- and runtime-index element stores through it; return the full element type from the projected type — `compiler/src/own/expr.rs`, `compiler/src/own.rs`, `compiler/src/codegen/function.rs` (DEF-31)
- [ ] 2.2.2 Materialize duplicable fixed-array values and struct literals with a place-less field at place-requiring positions (gate the skip) — `compiler/src/own/lower.rs` (AR #11; DEF-32, DEF-33, DEF-38)
- [ ] 2.2.3 Materialize fixed-array literal sources at projections (value and argument forms) — `compiler/src/own/lower.rs`, `compiler/src/codegen/expr.rs` (DEF-35)
- [ ] 2.2.4 Struct-literal initialization: materialize when any field lacks an initializer form; zero-init then store every field in source order (single rule) — `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs` (AR #7; DEF-38)
- [ ] 2.2.5 Run the specification tests — verify they PASS (green phase)

### Step 2.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted shapes and run the phase verification.

- [ ] 2.3.1 Write implementation tests (element-store byte copy, materialized argument/projection, struct field store) — `compiler/tests/call_value_integrity_impl_test.rs`
- [ ] 2.3.2 Full verification — `make check`

**Deliverables**:
- [ ] ST-9 – ST-17 and ST-41 pass; at least one case per Phase-2 defect failed first (red phase
  recorded); pre-passing cases recorded as regression pins
- [ ] `make check` passes with no warnings

**Verify**: `make check`

---

## Phase 3: Func Pointers and Callable Resolution

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: compiler-and-language (strict profile)

### Step 3.1: Specification Tests

**Reference**: [03-03](03-03-func-pointers-and-callable-resolution.md) §Proposed Changes 1–5 · AR #4, #12, #14
**Objective**: Pin the declarator, resolution, widening, and borrow behavior from ST-18 – ST-25, ST-39, and ST-40.

- [ ] 3.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-18 – ST-25, ST-39, and ST-40 — `compiler/tests/call_value_integrity_spec_test.rs`
- [ ] 3.1.2 Run the specification tests — verify at least one case per Phase-3 defect fails before implementation (red phase); record pre-passing cases as regression pins

### Step 3.2: Implementation

**Reference**: [03-03](03-03-func-pointers-and-callable-resolution.md) §Proposed Changes 1–5
**Objective**: Use the shared declarator everywhere and resolve func-value callees.

- [ ] 3.2.1 Declarator reuse: one shared signature builder (optional environment prefix, hidden destination), return position, closure struct field, lifted-lambda signature with `result_var`, and destination-aware closure call sites — `compiler/src/codegen.rs`, `compiler/src/codegen/types.rs`, `compiler/src/codegen/program.rs`, `compiler/src/codegen/function.rs`, `compiler/src/codegen/expr.rs` (DEF-30, DEF-43)
- [ ] 3.2.2 Capture and module-global resolution for calls by name (lowering and emission) — `compiler/src/own/call.rs`, `compiler/src/codegen/expr.rs`, `compiler/src/codegen/function.rs` (DEF-40, DEF-41)
- [ ] 3.2.3 Func-value expected parameter types and widening, with the resolved callee type passed into the argument renderer — `compiler/src/codegen/calls.rs`, `compiler/src/codegen/function.rs` (AR #12; DEF-42)
- [ ] 3.2.4 Function-type parameter modes: `FuncParam { ty, mode }` in `FuncTy`; `FuncSig::ty`, `resolve_ty_with`, and the callable-bound path zip modes; mode-aware signature; exact-signature equality (mode mismatch = `E10210`) and display; all `TyKind::Func` consumers; call-site pointer passing from the resolved callee type — `compiler/src/types/mod.rs`, `compiler/src/types/table.rs`, `compiler/src/sema/{expr,call,mod}.rs`, `compiler/src/own/{lower,generics,call}.rs`, `compiler/src/abi.rs`, `compiler/src/codegen/{types,calls,function}.rs` (AR #4, #14; DEF-44)
- [ ] 3.2.5 Run the specification tests — verify they PASS (green phase)

### Step 3.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted shapes and run the phase verification.

- [ ] 3.3.1 Write implementation tests (declarator shapes, capture/global spelling, widening, borrow pointer) — `compiler/tests/call_value_integrity_impl_test.rs`
- [ ] 3.3.2 Full verification — `make check`

**Deliverables**:
- [ ] ST-18 – ST-25, ST-39, and ST-40 pass; at least one case per Phase-3 defect failed first (red
  phase recorded); pre-passing cases recorded as regression pins
- [ ] `make check` passes with no warnings

**Verify**: `make check`

---

## Phase 4: Guards and Cleanup

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: compiler-and-language (strict profile)

### Step 4.1: Specification Tests

**Reference**: [03-04](03-04-guards-and-cleanup.md) §Proposed Changes 1–3 · AR #8, #9, #10
**Objective**: Pin the parameter move, do-while condition, and growable guard behavior from ST-26 – ST-31 and ST-38.

- [ ] 4.1.1 [spec-author] Write specification tests from the 07 ST-cases ST-26 – ST-31 and ST-38 — `compiler/tests/call_value_integrity_spec_test.rs`
- [ ] 4.1.2 Run the specification tests — verify at least one case per Phase-4 defect fails before implementation (red phase); record pre-passing cases as regression pins

### Step 4.2: Implementation

**Reference**: [03-04](03-04-guards-and-cleanup.md) §Proposed Changes 1–3
**Objective**: Correct the parameter zero walk, the do-while condition entry, and the embedded growable drop.

- [ ] 4.2.1 Array-parameter copy and zero walk on both the source and the target of a move; `is_pointer_param` and the parameter ABI stay unchanged; `cl_zero` stays for true locals — `compiler/src/codegen/function.rs` (AR #10; DEF-36)
- [ ] 4.2.2 Goto-free flag-first do-while when the condition carries guards (fall-through, `continue`, and embedded-only guards all reach the guards/test; unguarded keeps today's emission) — `compiler/src/codegen/control.rs` (AR #8; DEF-37)
- [ ] 4.2.3 Growable-array drop expression form for element types that need no drop; the limitation for drop-needing elements is recorded in Won't Have and pinned — `compiler/src/codegen/drop.rs`, `compiler/src/codegen/guards.rs` (AR #9; DEF-39)
- [ ] 4.2.4 Run the specification tests — verify they PASS (green phase)

### Step 4.3: Implementation Tests & Hardening

**Reference**: [07](07-testing-strategy.md) §Implementation Tests
**Objective**: Pin the emitted shapes and run the phase verification.

- [ ] 4.3.1 Write implementation tests (parameter copy/zero on both sides, flag-first do-while with `continue`, limited growable drop expression) — `compiler/tests/call_value_integrity_impl_test.rs`
- [ ] 4.3.2 Full verification — `make check`

**Deliverables**:
- [ ] ST-26 – ST-31 and ST-38 pass; at least one case per Phase-4 defect failed first (red phase
  recorded); pre-passing cases recorded as regression pins
- [ ] `make check` passes with no warnings

**Verify**: `make check`

---

## Phase 5: Close-out

> **Phase baseline tree**: _(recorded by the exec-plan skill)_
> **Lenses**: correctness, maintainability, standards (strict profile — docs and evidence phase)

### Step 5.1: Acceptance and Documentation

**Reference**: [01](01-requirements.md) §Acceptance Criteria · AR #16, #17, #18
**Objective**: Full acceptance, memory-safety probes, documentation, and the plan summary.

- [ ] 5.1.1 Full acceptance — `make verify` (every hosted-amiga and freestanding leg)
- [ ] 5.1.2 Host AddressSanitizer probes for the memory-safety defects DEF-34, DEF-36, DEF-37, DEF-39, and DEF-45: build with `clang -std=c99 -O0 -g -fsanitize=address` and run with `ASAN_OPTIONS=detect_leaks=1` (host side; the pinned container has no ASan runtime), using the probe programs in `07-testing-strategy.md` (AR #16)
- [ ] 5.1.3 Update documentation: `AGENTS.md` test map (both new suites) and the roadmap rows for `DEF-30` – `DEF-45` (done + plan link) — `AGENTS.md`, `plans/00-roadmap.md` (AR #18)
- [ ] 5.1.4 Check every acceptance criterion in `01-requirements.md`; set this plan's status Complete and the progress header to 39/39; final `make check`
- [ ] 5.1.5 Phase review evidence and plan summary — commit per the active commit mode

**Deliverables**:
- [ ] `make verify` all legs passed
- [ ] ASan probes clean with leak detection on
- [ ] Documentation and roadmap synced; acceptance criteria checked

**Verify**: `make verify`

---

## Dependencies

```
Phase 1 (owned conditional stores)
    ↓
Phase 2 (fixed-array values) — independent of Phase 1 internals; ordered after for a clean baseline
    ↓
Phase 3 (func pointers) — independent of 1–2; ordered after for context economy
    ↓
Phase 4 (guards and cleanup) — independent of 1–3
    ↓
Phase 5 (close-out)
```

## Success Criteria

**Feature is complete when:**

1. ✅ All phases completed
2. ✅ All verification passing (`make check`, `make verify`)
3. ✅ No warnings/errors
4. ✅ No dead code — no unused parameters, functions, classes, or modules
5. ✅ Security hardened — input validation, injection prevention, auth, rate limiting, data protection (N/A for an off-line compiler; the mandatory security check is the ASan ownership verification, AR #16)
6. ✅ Documentation updated
7. ✅ Code reviewed (strict-profile phase reviews per the exec-plan skill)
8. ✅ Post-completion project re-analysis (handled by the exec-plan skill)
