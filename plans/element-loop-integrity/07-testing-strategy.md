# Testing Strategy: Element Loop Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Ownership stage (guard predicate) | every argument form in the 03-01 behavior matrix |
| Loop emitter (step selection) | every `for` form nested in every loop kind, plus non-nested regression |
| Emitted-C shape | element-argument and nested-loop shapes pinned in the impl suite |

- Test names state behavior: `should [expected behavior] when [condition]`.
- The run oracle builds the emitted C with the project's warning flags
  (`-std=c99 -Wall -Wextra -Werror`) and asserts the process result; the value
  under test travels in `main`'s return code.
- **Bounded run (AR #10):** both new suites run programs under a 30-second bound
  (the `run_with_timeout` pattern from `codegen_ownership_impl_test.rs:126`), so a
  nested-loop hang fails instead of blocking `make check`.
- The two suites are the integration oracle (they build and run real emitted C);
  the full E2E is `make verify` (all hosted-amiga legs). No new harness is built.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived from the requirements (`01-requirements.md`), the component specs
> (`03-01`, `03-02`), and the frozen specification; never from implementation
> behavior.

### Element Read Ownership (DEF-16)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-1 | `let arr: String[2] = [stringOf("a"), stringOf("bb")]; return vlen(arr[0]);` | Exit 1; no abort | `03-01` matrix · R1 · AR #6 |
| ST-2 | Same array; `let i: i32 = 1; return vlen(arr[i]);` | Exit 2; no abort; bounds guard runs | `03-01` matrix · R1 · AR #13 |
| ST-3 | `total(borrow xs: String[2])` returns `vlen(xs[0]) + vlen(xs[1])`; caller passes a local `["a","bb"]` | Exit 3 | `03-01` matrix · R1 |
| ST-4 | `ps: P[2]` with `name: String`; `return vlen(ps[0].name) + ps[1].n;` where `ps[0].name = "a"`, `ps[1].n = 4` | Exit 5 | `03-01` matrix · R1 |
| ST-5 | `pick(borrow b: Bag)` where `Bag { xs: String[2] }`, returns `vlen(b.xs[0]) + vlen(b.xs[1])`; caller passes `["abc","de"]` | Exit 5 | `03-01` matrix · R1 |
| ST-6 | `byval(s: String)` (unmarked, read-only lend) returns 42; caller passes `arr[0]` of a local array | Exit 42; no abort | AR #9 · `spec/03` §2 |
| ST-7 | `sink(borrow s: String)` returns 42; caller passes `arr[0]` | Exit 42; no abort | `03-01` matrix · R1 |
| ST-8 | `println($"{arr[0]}")` where `arr: String[1] = [stringOf("ab")]` | Stdout `ab\n`; exit 0; no abort | `03-01` §Integration Points · R1 |
| ST-9 | `eat(take s: String); return eat(arr[0]);` | Compile-time `E10026`; no binary | R3 · `spec/03` §4 |
| ST-10 | `let a: String = arr[0];` | Compile-time `E10026`; no binary | R3 · `spec/03` §4 |
| ST-11 | `for (borrow e in xs) { total = total + vlen(e); }` over `["a","bb"]` | Exit 3 | R1 · `spec/03` §2 element views |
| ST-12 | `return vlen(stringOf("hi"));` (ownerless temporary) | Exit 2; the temporary is released exactly once | R2 · AR #6 |
| ST-13 | `let i: i32 = 2; return vlen(arr[i]);` over `String[2]` | Runtime trap: stderr contains `E11211`; the process aborts (non-zero, never a value exit) | `03-01` §Error handling · AR #13 |

### Nested Loop Step (DEF-17)

| # | Input / Scenario | Expected Output / Behavior | Source |
| --- | --- | --- | --- |
| ST-14 | `for (borrow x in xs)` over `i32[2] = [1,2]` containing `for (borrow y in ys)` over `[1,2,3]`; `s = s + x + y` | Exit 21 (Σ(x+y)); terminates | R4 · `spec/05` `for` step |
| ST-15 | `while (k < 2) { for (borrow x in xs) { s = s + x; } k = k + 1; }` over `[1,2]` | Exit 6; terminates | R4 |
| ST-16 | `while (k < 2) { for (let j = 0; j < 3; j = j + 1) { s = s + j; } k = k + 1; }` | Exit 6; terminates | R4 · `spec/05` three-clause `for` |
| ST-17 | Nested element loops with `if (y == 2) { continue; }` over `xs=[1,2]`, `ys=[1,2,3]`; `s = s + x + y` | Exit 14; `continue` resumes at the inner step | R4 |
| ST-18 | Nested element loops with `if (y == 2) { break; }`; `s = s + x + y` | Exit 5; `break` leaves the inner loop | R4 |
| ST-19 | Two sequential element loops: `xs=[1,2]`, `ys=[10,20,30]` | Exit 63 | R5 (regression) |
| ST-20 | One top-level element loop over `[1,2]` | Exit 3 | R5 (regression) |
| ST-21 | `for (borrow x in xs)` over `i32[2] = [1,2]` containing `for (let j = 0; j < 3; j = j + 1) { s = s + x + j; }` | Exit 15 (Σ(x+j)); terminates | R4 · AR #3 · `spec/05` three-clause `for` |
| ST-22 | `for (let j = 0; j < 2; j = j + 1)` containing `for (borrow x in xs)` over `[1,2]`; `s = s + x` | Exit 6; terminates | R4 · AR #3 |
| ST-23 | `for (let j = 0; j < 2; j = j + 1)` containing `for (let k = 0; k < 3; k = k + 1)`; `s = s + j + k` | Exit 9; terminates | R4 · AR #3 |
| ST-24 | `do { for (borrow x in xs) { s = s + x; } k = k + 1; } while (k < 2)` over `[1,2]` | Exit 6; terminates | R4 · AR #3 |
| ST-25 | `do { for (let j = 0; j < 3; j = j + 1) { s = s + j; } k = k + 1; } while (k < 2)` | Exit 6; terminates | R4 · AR #3 |

> ST-1 – ST-8, ST-14 – ST-18, and ST-21 fail before the fix (double-free
> aborts and bounded-run timeouts). ST-9 – ST-13 and ST-19 – ST-20 pass pre-fix:
> the two rejection cases, the runtime trap, and the regression guards. Their
> passing state in the red phase is expected and recorded; a deviation from
> this split is an oracle or test defect, never a reason to change a case.
> ST-22 – ST-25 complete the nesting matrix (element and three-clause `for`
> inside `while`, `for`, `do`/`while`, and element loops); they were added
> during the close-out review and share the already red-verified root cause
> (any nested `for` mis-selects its step), so their green pass is the
> recorded evidence.

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/element_loop_integrity_spec_test.rs` | ST-1 – ST-25 | Element read ownership (03-01) and nested loop step (03-02) |

The suite follows the existing ownership-spec pattern: compile with `clc`, build
the emitted C under the fixed warning flags, run under a 30-second bound, assert
the exit code or the rejection. Two helpers are added for this plan: a bounded run
(so a hang fails) and a rejection assertion (for ST-9/ST-10).

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/element_loop_integrity_impl_test.rs` | Emitted-C shape: an element argument has no `__cl_owned` binding; the ownerless-temporary argument still binds and releases; a template placeholder has no owned temporary; the nested inner `for` header carries its own increment with no dead step label or duplicate loop; two emissions are byte-identical | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | All suites | Every task's gate; existing ownership, NLL, codegen, and corpus suites guard non-nested behavior |
| Host ASan probes (close-out) | `clc` + emitted C | Rebuild the fixed programs with `-fsanitize=address`; zero reports |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| `make verify` | spec check + workspace tests + every hosted-amiga cross-toolchain leg | `verify: all legs passed` |

## Test Data

### Fixtures Needed

- Small programs inline in the test file (the repo pattern); no fixture files.
- `String`/`stringOf` from `std.text`, `println` from `std.io` where a program
  prints.

### Mock Requirements

- None; programs run against the real host C runtime.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation where the
  defect applies (ST-1 – ST-8, ST-14 – ST-18, ST-21), and the pre-passing cases
  (ST-9 – ST-13, ST-19 – ST-20) recorded as expected
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] `make verify` green before completion; host ASan probes clean
