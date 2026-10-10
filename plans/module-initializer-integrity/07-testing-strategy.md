# Testing Strategy: Module Initializer Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Fixed defect shapes (spec cases) | 100% — every R1 – R10 filed reproduction |
| Init emission contracts (impl pins) | every mirrored shape pinned by an emitted fragment |
| Regression (unaffected initializers) | byte-identity pins + the corpus differential per phase |

- Test names state behavior: `should [expected behavior] when [condition]`.
- The new suites are `compiler/tests/module_initializer_integrity_spec_test.rs` and
  `compiler/tests/module_initializer_integrity_impl_test.rs`; the existing
  `eval_emission_integrity_*` / `store_emission_integrity_*` init pins stay untouched oracles.
- E2E: the spec run cases execute the emitted C on the host under the strict flags — the
  project's established end-to-end form for the compiler.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases derive EXCLUSIVELY from `01-requirements.md`, the `03-XX` specifications,
> and the Ambiguity Register. They define expected behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the implementation.

### Initializer Call Resolution

| # | Input / Scenario | Expected Output / Behavior | Source |
|------|----------------------------|----------------------------------------|-------------------|
| ST-F1 | `let g: i32 = (() => 3)();` in a module | Strict build passes; the program runs and `g` drives the pinned result | R1 |
| ST-F2 | `func ready(): i32 { return 7; } let f: func(): i32 = ready; let g: i32 = f();` | Strict build passes (the global's mangled symbol); runs with `g == 7` | R2 |
| ST-F3 | `func pick<T>(a: T, b: T): T; let g: i32 = pick<i32>(1, 2);` | Strict build passes (the instance is declared); runs with `g == 1` | R3 |
| ST-F4 | `struct Box { get: func(): i32; } let b: Box = Box { get: ready }; let g: i32 = b.get();` | Strict build passes; the receiver reaches the callable; runs with `g == 7` | R4 |

### Initializer Aggregate Emission

| # | Input / Scenario | Expected Output / Behavior | Source |
|------|----------------------------|----------------------------------------|-------------------|
| ST-F5 | `struct BoxA { arr: i32[2]; tag: i32; } let garr: i32[2] = [1, 2]; let boxed: BoxA = BoxA { arr: garr, tag: 3 };` and the call-result variant `BoxA { arr: dup(), tag: 4 }` | Strict build passes (statement-form store with `memcpy`); a run reads both elements correctly | R5 |
| ST-F6 | `let n: i32 = 4; let label: str = "n=${n}";` plus an owned-`String`-valued `str` initializer | Strict build passes (the value reaches the `str` global through the view wrapper); a run compares the bytes | R6 |
| ST-F7 | `let b: BoxA = …; let count: usize = b.arr.len();` | Strict build passes; `count` equals the declared extent (never a silent `0`); a run asserts it | R7 |

### Initializer Release and Checks

| # | Input / Scenario | Expected Output / Behavior | Source |
|------|----------------------------|----------------------------------------|-------------------|
| ST-F8 | The three filed leak shapes: a member projection with an owned member (`mkBox().count`), a `String[2]` element projection, and an owned conditional arm — each in an initializer, in a loop of program starts where expressible | Each program runs under host ASan with `detect_leaks=1` clean; base evidence shows the filed `Direct leak` | R8 |
| ST-F9 | `const G: i32 = 0; let h: i32 = i32(G++);` and `let S: str = "ab"; let t: i32 = i32(S[0]++);` with the body-spelling twins and read-only controls | One report per site with the same code/message/help as the body twin; controls accepted | R9 |

### Body Slice Typedefs

| # | Input / Scenario | Expected Output / Behavior | Source |
|------|----------------------------|----------------------------------------|-------------------|
| ST-F10 | `func size(): usize { let n: usize = sizeof(i32[]); return n; }` | Strict build passes (the `ClSlice_i32` typedef exists); a run returns the pinned size | R10 |

### Regression

| # | Input / Scenario | Expected Output / Behavior | Source |
|------|----------------------------|----------------------------------------|-------------------|
| ST-F11 | A seeded set of unaffected initializer shapes (scalar, call, member, element, conditional spellings) | Each re-emits byte-identically to the batch base; the existing eval-emission/store-emission init pins stay green untouched | R11 |

## Test Categories

### Specification Tests (from ST-cases above)
> Written BEFORE implementation. Filed as `module_initializer_integrity.spec.test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/module_initializer_integrity_spec_test.rs` | ST-F1 – ST-F11 | all four components |

### Implementation Tests (edge cases, internals)
> Written AFTER implementation. Filed as `module_initializer_integrity.impl.test.rs`.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/module_initializer_integrity_impl_test.rs` | Emitted-fragment pins: lifted lambda identity, mangled global symbol, instance symbol, receiver argument, statement stores, view wrap, folded `len`, end-of-init releases and their null guards, the typedef pair, byte-identity | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus differential | compiler | every repository + inline program, base vs head, classification of each delta (phase reviews) |
| Strict-build sweep | compiler | every head-accepted unit under `-Wall -Wextra -Werror` |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Init program run | emit with `clc`, strict-build with `cc`, run | the pinned exit/values; ASan-clean where the case requires |
| Full acceptance | `make check`, `make verify` | all suites and all hosted-amiga legs pass |

## Test Data

### Fixtures Needed

- The filed reproductions embedded as raw-string sources in the spec suite (the project's
  established inline-fixture form).
- The batch base worktree for red-phase and byte-identity comparisons (per phase, deleted after).

### Mock Requirements

- None — the compiler is exercised end-to-end; the host C toolchain is the real dependency.

## Verification Checklist
- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Test coverage meets goals
