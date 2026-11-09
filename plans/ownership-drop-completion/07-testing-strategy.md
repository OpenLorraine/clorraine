# Testing Strategy: Ownership and Drop Model Completion

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Shape predicate and consumers (`types`, `borrow`) | 95% |
| Ownership and lowering (`own`) | 90% |
| Drop emission (`codegen/drop`) | 90% |
| Checks (`sema`) | 90% |

- Test names state behavior: `should [expected] when [condition]`.
- E2E: the build-and-run oracle builds the emitted C and runs it, so a double
  free aborts in the C library. It uses the existing oracle pattern from
  `compiler/tests/partial_move_impl_test.rs`; it does not create a new harness.
- All new tests live in the Rust suites; no new `spec/` golden is added (AR-12).
  The existing `E10205` golden stays and keeps passing.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from `spec/03-ownership.md`, `spec/08-arrays-strings.md`,
> `spec/11-memory.md`, `spec/16-borrow-rules.md`, `spec/appendix-stdlib.md`, the
> decisions ledger, and the register. Expectations come from the specification,
> never from imagined implementation behavior.

### Arrays and Ownership

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `func pair(): String[2] { let a=…; let b=…; return [a, b]; }`, then run | the program compiles and runs; each element is released exactly once | spec/08 §6 |
| ST-2 | `let arr: String[1] = [a]; use(a);` | `E10020` — `a` was moved into the array | O1/O3; spec/08 §6 |
| ST-3 | `let row: i32[4] = [1, 2, 3, 4];` | accepted; no element drop is emitted (a copy) | V8 |
| ST-9 | `let grid: String[2][2] = [[a, b], [c, d]];` then run | all four elements are released exactly once, first-to-last | O2 |
| ST-13 | `Ty::is_slice` on `u8[]`, `u8[4]`, `u8[3][2]`, `u8[][2]` | true only for `u8[]` (and any type with an omitted extent) | spec/08 §1, §3 |
| ST-14 | `is_handle_or_aggregate` on a fixed array and a slice | true for the fixed array; false for the slice | X4 |
| ST-15 | `func head(arr: u8[2]): u8[] { return slice(arr, 0, 1); }` | accepted — a fixed array stays a lend candidate | EL-2; PF-006 |
| ST-16 | `let a: String[2]? = null;` | rejected — a fixed array is not nullable | T8; PF-004 |

### Growable Container

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-7 | `Array<String>` with two pushed elements, then run | elements release first-to-last, then the buffer; no double free | spec/appendix-stdlib §3; O2 |
| ST-8 | `let xs: Array<Buffer> = arrayNew<Buffer>();` | accepted; no `E90000` | AR-2 |
| ST-17 | `Array<String>` with two elements, then `xs.set(0, v)`; run | the replaced element is dropped; no leak | spec/appendix-stdlib §3 (:62); PF-007 |

### View Fields and Fill

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-4 | `struct Bad { part: u8[]; }` | `E10205` | spec/08:289; T4/T9 |
| ST-5 | `struct Bad { part: str; }` | `E10205` | T4; `spec/07-structs.md:142` |
| ST-6 | `struct Ok { n: String; l: static str; p: cstring; r: u8[8]; }` | accepted | T4 |
| ST-10 | `let xs: String[2] = [; a];` | `E10215` — fill cannot repeat a move-only element | AR-10 |
| ST-11 | `let xs: u8[4] = [; 0];` | accepted | spec/08 §2 (AL) |
| ST-12 | a fixed array passed to an uncontracted extern | `E10101`; the same call with a slice is accepted | X4 |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. `compiler/tests/ownership_drop_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/ownership_drop_spec_test.rs` | ST-1 (checker/emitter half), ST-2, ST-4, ST-5, ST-6, ST-8, ST-10, ST-11, ST-12, ST-13, ST-14, ST-15, ST-16 | classification, checks, return compile |
| `compiler/src/types/mod.rs` (`mod tests`) | ST-13 | shape predicate |
| `compiler/src/borrow.rs` (`mod tests`) | ST-14 | classification predicate |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. `compiler/tests/ownership_drop_impl_test.rs` builds
> and runs the emitted C, so a second release aborts. It is the runtime oracle for
> the spec cases whose behavior is only observable at run time (PF-015); the
> spec-level expectation for each is also pinned in the spec file above.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/ownership_drop_impl_test.rs` | ST-1, ST-3, ST-7, ST-9, ST-17 runtime drop, ordering, and replacement | High |
| `compiler/src/codegen/drop.rs` (`mod tests`) | emitted shape for a fixed array (per dimension) and the growable loop | Medium |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | full workspace | spec suite plus Rust suites |
| `make verify` | full acceptance | hosted-amiga legs and the corpus |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Fixed-array drop | emit C for `pair(): String[2]`, compile, run | exit 0; no `free(): double free` |
| Growable drop | emit C for `Array<String>` push, compile, run | exit 0; elements released once |

## Test Data

### Fixtures Needed

- Small `.clo` sources for the oracle, held as inline strings in
  `ownership_drop_impl_test.rs`, matching the `partial_move_impl_test.rs` style.
- No new `spec/tests/` fixture is added (AR-12).

### Mock Requirements

None. The oracle uses the real host C compiler and the real runtime shim.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a specification rule or an AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] `make check` passes after every task; `make verify` passes before completion
- [x] No regressions in existing tests or the corpus
