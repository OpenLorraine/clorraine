# Testing Strategy: Def Backlog Closeout

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Fixed ownership/place paths (03-01, 03-02, 03-03, 03-04) | Every changed shape has a spec case and an impl case |
| Diagnostic path (03-05) | Registry golden + nested/global/mode cases |
| Memory behavior | Every case that can leak/double-free/UAF has a deterministic host-ASan probe |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Integration tests are the repository's existing suites; no new harness is built.
- The in-container full-suite `RDC_ASAN=1` run is timing-flaky in this environment and is **not** a
  gate; host ASan probes are the deterministic memory oracle (AR-15).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from `01-requirements.md`, the `03-XX` component docs, and the Ambiguity
> Register. Immutable oracle: a failing spec case means the implementation is wrong.

### Take and Move-Out Integrity (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | `flag && sink(mkv().a)` with a true and a false guard, host ASan | Accepted; exit pinned; no `AddressSanitizer` output on either path; the emitted guard zeroes the taken member and the result's remaining fields are released exactly once | R1, AR-8 |
| ST-2 | `sink(mkv().a)` unguarded, host ASan | Clean (regression); the taken member transfers and the result releases once | R1, AR-8 |
| ST-3 | `Box<T> { v: mk()[i] }` with a run-time `i`, concrete and `<String>`, host ASan | C builds and runs; field equals the `i`-th element; the root call is emitted before the move; `__cl_tmp0[i]` is emptied; exactly one release of the result | R2, AR-9 |
| ST-4 | `Box<T> { v: mk()[0] }` constant index | C builds and runs; the slot `[0]` is emptied; clean | R2, AR-9 |
| ST-5 | `let s: T = slice(mk(), 0, 2)[i]` and a non-zero start (`slice(mk(), 1, 2)[i]`), host ASan (reproduction-gated) | Clean: either already fixed (pinned) or the element moves out before the temporary release | R3, AR-2 |
| ST-6 | `consume<T>(slice(mk(), 0, 2)[i])`, host ASan (reproduction-gated) | Clean: no double free, element transferred; the emitted C builds with no unknown-typed symbol | R3, AR-2 |
| ST-7 | Concrete slice element-move forms rejected today (e.g. a concrete take/store through a slice view) | The existing `E10026` element-move limit stays; previously rejected forms still report it | R3, AR-2 |

### Expression Temporaries (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-8 | `return i32(len(stringOf("abc")));` and `return i32(len(storeAt<String>(mkString, 1)));`, host ASan | Returns the pinned length; clean (no leak) | R5, AR-6 |
| ST-9 | `stringOf("a").len()` dot form, host ASan | Returns 1; clean | R5, AR-6 |
| ST-10 | `flag && len(stringOf("abc")) == 3` with true and false guard, host ASan | Clean on both paths; the call is not evaluated when the guard is false | R5, AR-6 |
| ST-11 | `let n: i32 = i32(len(s));` where `s` is a local `String` place | Unchanged behavior and emitted shape (regression); a place argument needs no binding | R5, AR-6 |

### Borrow Out-Slot ABI (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-12 | `fill<T>(take v: T, borrow slot: String?)` called with a non-null slot; callee assigns `v`; host ASan | Callee's value visible to the caller; the old value is dropped exactly once; clean | R4, AR-3 |
| ST-13 | Same with `borrow slot: File?` | Pointer parameter; exactly one free; clean | R4, AR-3 |
| ST-14 | Same with `borrow slot: str?` and a `str?` place | The pointer receives `&slot`; callee write-through reaches the caller. A `String` value passed to the slot is rejected (`E10200`) | R4 |
| ST-15 | Null-start out-slot case and a non-nullable duplicable-struct `borrow` parameter | Null start unchanged; non-nullable duplicable struct stays by value (W10001 path unchanged). Structs cannot be nullable (TY-4/E10011), so no nullable-struct sub-case exists | R4, AR-3, AR-19 |

### Defer Condition Release (03-04)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-16 | `do { defer { return i; } } while (guarded)` with a guarded condition, host ASan | Exit pinned; clean (condition temporary released on the deferred return) | R6, AR-7 |
| ST-17 | Direct early return in a do-while body (existing ST cases) | Unchanged release list and clean probe (regression) | R6, AR-7 |
| ST-18 | Deferred return inside a `while` body | Unchanged (condition already registered before the body) | R6, AR-7 |
| ST-19 | Break, fall-through, and a post-loop return after a do-while with a deferred return | No appended release on the post-loop return; no duplicate release; clean | R6, AR-7 |

### Capture Diagnostic (03-05)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-20 | `[x] () => n` where `n` is an enclosing parameter | `E10405`: `lambda reads 'n' without listing it`, suggestion `add 'n' to the capture list`; compile-fail golden | R7, AR-4, AR-14 |
| ST-21 | `[x, n] () => n` | Accepted; compiles and runs | R7, AR-4 |
| ST-22 | Capture-less lambda reading a module global and a module function | Accepted; compiles | R7, AR-4 |
| ST-23 | Inner lambda reads an outer lambda's capture without listing it / with listing | Unlisted: `E10405`; listed: accepted | R7, AR-5 |
| ST-24 | Move/write/`take` capture cases | E10401 / E10402 / E10403 unchanged | R7, AR-4 |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as `compiler/tests/def_backlog_closeout_spec_test.rs`
> (module prefix `spec.closeout.stNN`); ST-20 additionally lives as the compile-fail golden
> `spec/tests/compile-fail/E10405_unlisted_capture.clo`, validated by `spec/check.sh`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/def_backlog_closeout_spec_test.rs` | ST-1 – ST-19, ST-21 – ST-24 | 03-01, 03-02, 03-03, 03-04, 03-05 |
| `spec/tests/compile-fail/E10405_unlisted_capture.clo` | ST-20 | 03-05 |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as `compiler/tests/def_backlog_closeout_impl_test.rs`.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/def_backlog_closeout_impl_test.rs` | Emitted-shape assertions: guard zeroing for the member shape, aggregate fill/index slot, slice-root regression pins, intrinsic-argument release shape, pointer declarators/arguments per nullable kind, deferred-return release list, nested capture/global cases | High |

### Host AddressSanitizer Probe Matrix

Emit with `target/debug/clc --profile host -o build/closeout/<name>.c <program>.clo`, build with
`cc -std=c99 -O0 -g -fsanitize=address`, run bounded with
`ASAN_OPTIONS=detect_leaks=1:symbolize=0:handle_segv=0 timeout 30`.

| Probe | ST | Expected |
| ----- | -- | -------- |
| `def63_guarded_member.clo` | ST-1 | exit pinned, no ASan output |
| `def64_aggregate_index.clo` | ST-3 | exit pinned, no ASan output |
| `def66_slice_store.clo` | ST-5 | exit pinned, no ASan output |
| `def66_slice_take.clo` | ST-6 | exit pinned, no ASan output |
| `def67_len_call.clo` | ST-8 | exit 3, no ASan output |
| `def67_len_guard.clo` | ST-10 | exit pinned, no ASan output |
| `def65_nonnull_slot.clo` | ST-12 | exit pinned, no ASan output |
| `def65_str_slot.clo` | ST-14 | exit pinned, no ASan output |
| `def68_defer_return.clo` | ST-16 | exit pinned, no ASan output |
| `def67_storeat_regression.clo` | ST-11 | exit pinned, no ASan output |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Close-out probe runner | `build/def-backlog-closeout-probes/run-def-backlog-closeout.sh` | Every original defect program exits with its pinned code under ASan and no sanitizer output |

## Test Data

### Fixtures Needed

- Small `.clo` programs per probe, embedded in the suites where practical, matching the filed
  reproductions and their variants (true/false guard, constant/run-time index, concrete/substituted
  type).
- The `E10405` compile-fail golden.

### Mock Requirements

- None; real host toolchain and runtime shim.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase), with justified passes
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] Host-ASan probe matrix clean for every fixed shape
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] `make check` green per phase; `make verify` green at close-out
