# Testing Strategy: Residual Defect Completion

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Core behavior (the six defects' shapes) | 100% of the ST cases below |
| Supporting codegen paths touched by the fixes | 90% |
| Glue / emission shape | 80% |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Integration tests are the close-out probes (R10); E2E is the existing cross-toolchain
  `make verify` matrix, run once at close-out.
- Memory-cleanliness claims (R4, R5, R6) are pinned by sanitizer implementation tests and the
  close-out probes; the specification cases pin the program-visible behavior.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These cases are derived exclusively from `01-requirements.md`, the component specifications,
> and the Ambiguity Register. The immutable-oracle rule applies: a failing case means the
> implementation is wrong, never the test.
>
> In-code traceability comments quote the behavior in plain language, never the ST ids below.

### Phase 1 — Type keys and call arguments (DEF-50, DEF-54)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | One program with `Grid<i32[2]> { value: [1, 2] }` and `Grid<i32[3]> { value: [1, 2, 3] }`; `main` returns `a.value[0] + b.value[0]` | The C builds (no redefinition) and the program exits `2` | R1 |
| ST-2 | `Grid<i32[2][2]> { value: [[1,2],[3,4]] }` and `Grid<i32[2]> { value: [5,6] }` in one program; returns `g.value[1][1] + h.value[1]` | The C builds and the program exits `10` | R1 |
| ST-3 | `Grid<String[2]>` with two owned elements; returns the summed lengths | Builds, runs, and is sanitizer-clean; the element strings are released | R1 |
| ST-4 | `probe<T>(borrow p: Outer<T>)` calling `sink<T>(borrow p.v)` with `T = String`, `sink` returns `1` | The C builds and the program exits `1` | R2 |
| ST-5 | The ST-4 shape with `T = i32`; after the call the caller reads `p.v.s` again and returns it | Builds, exits with the stored value; the argument is a view (no move or release) | R2 |
| ST-6 | Emit the ST-1 program twice | The two emitted translations are byte-identical | R9 |

### Phase 2 — Lifted-lambda instances (DEF-52)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-7 | `outer<T>` with a capture-less `func(T): T` lambda calling `apply<T>(v)`; `main` calls `outer<i32>(3)` | The C builds and the program exits `3` | R3 |
| ST-8 | One program instantiating `outer<i32>` (returns `3`) and `outer<String>` (returns the length `len("abc") = 3`) | Both instances build; the program exits `3` | R3 |
| ST-9 | A capturing lambda in `outer<T>(x: T): T` capturing the parameter `x` and returning it; `main` returns `i32(len(outer<String>("abc")))` | Builds, exits `3`, sanitizer-clean | R3 |

### Phase 3 — Call-result ownership (DEF-55, DEF-53)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-10 | `takea(): String { return mkv().a; }` over a struct with two `String` fields where `a` is `"x"` and `b` is `"yz"` | The program returns `1` (length of `"x"`); sanitizer-clean (the sibling is released) | R4 |
| ST-11 | `let s: String = mkv().a;` then `return i32(len(s))` | Returns `1`; sanitizer-clean | R4 |
| ST-12 | `sink(mkv().a)` with `sink(take s: String): i32 { return i32(len(s)); }` | Returns `1`; the callee owns the projection and the temporary releases the sibling; sanitizer-clean | R4 |
| ST-13 | `at<T>(mk: func(): T[2], i: i32): i32 { return sink<T>(mk()[i]); }` with `sink<T>(take s: T): i32 { return 0; }`; instantiated `T = String`, `i = 0`, the first element `"ab"` | Returns `0`; the element is released exactly once; AddressSanitizer-clean (no double free) | R5 |
| ST-14 | `sink(mkv()[0])` with a concrete `take String` parameter (no generic) | Still rejected with `E10026` exactly as before; no accepted program changes | R7 |

### Phase 4 — Guarded drop hoist (DEF-51)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-15 | `while (i < 2 && grow()[0].count > 0) { i = i + 1; }` with `grow(): Array<String>[2]` | Exits `2`; LeakSanitizer-clean (the previous guarded value is released on each re-store) | R6 |
| ST-16 | The ST-15 shape in `do { ... } while (cond)` and `for` spellings | Each exits `2` and is LeakSanitizer-clean | R6 |
| ST-17 | A guarded operand whose type is a struct containing a growable `Array<String>` field | Exits with the expected value and is LeakSanitizer-clean | R6 |
| ST-18 | The ST-15 shape with `Array<i32>` (its element needs no drop) | Exits with the same count and emits the same expression-form drop as before | R6 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/residual_defect_completion_spec_test.rs` | ST-1 – ST-18 | All four components |

Written BEFORE implementation, one phase at a time; each phase's cases are added in that phase's
first step and verified red before its implementation.

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/residual_defect_completion_impl_test.rs` | Symbol-key unit shapes (multi-extent, slice, struct element); substituted argument renders `&(p.v)`; specialization symbols and environment structs; bind-temporary emitted shape and release counts; guard statement drop before the re-store; sanitizer-clean runs; byte-identical re-emission; `E10026` unchanged | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Close-out probes | All four | The six reproduction programs built and run under AddressSanitizer with leak detection on; recorded clean exit codes (R10) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Cross-toolchain acceptance | `make verify` | Every hosted-amiga and freestanding leg passes |

## Test Data

### Fixtures Needed

None new. The suites reuse the existing host build/run helpers and the `ScratchDir` pattern used
by the neighboring suites.

### Mock Requirements

None. All tests use the real compiler and the host C toolchain; no mocks.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement or AR entry
- [ ] Specification tests written BEFORE implementation, phase by phase
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Sanitizer probes record the pre-fix failure and are clean post-fix
