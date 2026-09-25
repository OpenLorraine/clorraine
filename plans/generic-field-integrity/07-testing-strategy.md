# Testing Strategy: Generic Field Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Substitution helper and type-table queries | 100% of the new branches (generic, non-generic, empty args) |
| Checker, ownership, emitter field reads | One regression per defect plus one control |
| Non-generic behavior | Unchanged; pinned by the existing suites |

E2E: N/A — the compiler's own suites (`v1_language_spec_test.rs`,
`codegen_run_spec_test.rs`, corpus) already build and run emitted C; this plan
adds regression cases to the existing suites rather than a new harness (AR-4,
AR-22).

### Test placement (AR-4)

Per the user's ruling, the new tests are **specification-derived regression
tests** placed in existing suites, written and verified **red** before the fix:

| Suite | Kind | ST cases |
| ----- | ---- | -------- |
| `compiler/src/sema/regressions.rs` | checker unit regressions | ST-1, ST-2, ST-3, ST-4, ST-10, ST-11 |
| `compiler/src/types/table.rs` (`mod tests`) | type-table unit tests | ST-8, ST-9 |
| `compiler/tests/v1_language_impl_test.rs` | emitted-C / end-to-end | ST-5, ST-6, ST-7, ST-12 |

No `spec/` file and no `*_spec_test.rs` file is modified.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Expectations are derived from the frozen `spec/` and the diagnostics registry,
> never from implementation behavior.

### Checker (sema)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `struct Regs { r: volatile u16; }`, `struct Outer<T> { inner: T; }`, `func takes(o: Outer<Regs>): void { }` | Reports `E10207` (`overlay type 'Regs' cannot be used by value`) | `spec/14-diagnostics.md:91`, AR-9 |
| ST-2 | Same with non-generic `struct Outer { inner: Regs; }` | Reports `E10207` (control; already correct) | `spec/14-diagnostics.md:91` |
| ST-3 | `struct Box<T> { value: T; }`, `let a = Box<String> { value: stringOf("x") }; let b = a; let c = a;` | Reports `E10020` on the second use of `a` | `spec/14-diagnostics.md:51`, AR-10 |
| ST-4 | `let a = Box<i32> { value: 1 }; let b = a; let c = a;` | Accepted (a `Box<i32>` is duplicable) | `spec/07-structs.md:61` |
| ST-10 | `r: raw Pair<u8>`, `takesI32(r.second)` inside `unsafe` | Reports `E10200` (`cannot convert 'u8' to 'i32' implicitly`); the field is `u8`, not unknown | AR-13 |
| ST-11 | `func f<T>(p: Pair<T>): T { return p.first; }` | Accepted; the field stays the live parameter `T` | AR-14 |

### Type table

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-8 | `const N: usize = sizeof(Box<i32>);` in a constant context | Folds to `4`; no `E10224` | `spec/14-diagnostics.md:108`, AR-13 |
| ST-9 | `offsetof(Pair<i32>, second)` | Folds to `4` | AR-13 |

### Emitter

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-5 | `let b = Box<String> { value: stringOf("x") };` at function scope | Emitted C releases the field, e.g. `if (b.value) { FreeString(b.value); }` | AR-11 |
| ST-6 | `let z = Pair<Pair<i32>> {};` | Emitted C uses nested zero states (`{ {0}, {0} }`) and compiles under the host `-Werror` flags | `spec/07-structs.md:44`, AR-12 |
| ST-7 | `let p = Point {};` for a non-generic struct | Accepted and compiles (STR-7 zero value; control) | `spec/07-structs.md:44` |
| ST-12 | Compile a small non-generic program that copies and drops a struct | Emitted C is unchanged from `HEAD` (byte comparison in the test) | AR-4 |

## Test Categories

### Specification-Derived Regression Tests (from ST-cases above)

> Written BEFORE implementation, in the existing suites per AR-4.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/src/sema/regressions.rs` | ST-1, ST-2, ST-3, ST-4, ST-10, ST-11 | Checker |
| `compiler/src/types/table.rs` unit tests | ST-8, ST-9 | Type table |
| `compiler/tests/v1_language_impl_test.rs` | ST-5, ST-6, ST-7, ST-12 | Emitter |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/src/types/table.rs` unit tests | `substitute_instance` empty/short-argument no-op; `offset_of` non-generic unchanged | High |
| `compiler/src/sema/regressions.rs` | `Pair<Pair<i32>>.first` field type is `Pair<i32>`; raw peel types the field | Medium |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `v1_language_impl_test` field cases | checker + emitter | The existing DEF-05 cases still pass after consolidation |
| `codegen_run_spec_test` / corpus | full pipeline | Non-generic emitted C and runtime behavior unchanged |

### End-to-End Tests

N/A — the existing run/build suites already execute emitted C; see Coverage Goals.

## Test Data

### Fixtures Needed

None beyond inline Clorraine snippets written into the existing Rust test helpers
(`assert_codes`/`assert_accepts`, `emit`, `diagnostics`).

### Mock Requirements

None.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a `spec/` rule or diagnostic registry entry
- [ ] Regression tests written BEFORE implementation
- [ ] Regression tests verified to FAIL before implementation (red phase)
- [ ] All regression tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests (non-generic emitted C unchanged)
- [ ] No `spec/` or `*_spec_test.rs` file modified
