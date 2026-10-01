# Testing Strategy: Residual Defect Hardening

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Every defect and every suppression site | One specification case minimum (the site→case map below), plus implementation-shape tests |
| Leak-only behavior (DEF-48, DEF-49) | Specification run plus pre-fix AddressSanitizer evidence and a clean close-out probe |
| Existing behavior | No regressions: the full workspace suite, the corpus, and the determinism suite per phase |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Host runs are this compiler project's end-to-end tier: a program is compiled with `clc`, the
  emitted C is built with the pinned C compiler, and the process result is asserted. Unit tests
  for the predicates live in the source module test blocks.
- Targets match the repository's existing suites (every defect and site has a concrete case; no
  percentage gate is added).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the frozen specification under `spec/`, the Ambiguity Register, and the
> filed defect expectations. These define behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the implementation.
>
> **RED-PHASE RULE:** every defect needs a failing pre-fix observation. DEF-46 and DEF-47 emit
> invalid C, so their cases fail loudly. DEF-48 and DEF-49 are leak-only (the exit code and
> values are already correct), so each gets a pre-fix AddressSanitizer probe recorded in the
> execution plan plus an emitted-shape implementation pin; a run without AddressSanitizer cannot
> see the failure, exactly as the prior plan handled leak-only cases.

### Phase 1: Flat Array Literal Types and the Deferred Row Store (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | `ok && [c ? rows[0] : rows[1], [5, 6]][0][0] == 1` for every `ok` and `c` combination | The emitted C builds cleanly and the program computes the chosen element | spec/08-arrays-strings.md · R2 · AR #3 |
| ST-2 | `let lit = [[1, 2], [3, 4]];` then read `lit[1][0]` | `3`; the declaration is a flat `i32[2][2]` | spec/08-arrays-strings.md · R1 · AR #3 |
| ST-3 | `let pick = ok ? [a1, a2] : rows;` (no context on the literal) | The chosen array's elements; no `cannot convert` diagnostic | spec/04-expressions.md · R1 · AR #3 |
| ST-4 | A deferred literal with an owned element (`String[2][2]` rows) and a conditional row | Correct strings; one release per value; clean exit | spec/03-ownership.md · R2 · AR #3 |
| ST-5 | Regression pins: a literal whose element is a slice (`String[][2]`) and a literal with mismatched row lengths | Slice-element literal behaves as today; the mismatched literal is rejected exactly as today | spec/08-arrays-strings.md · AR #9 |
| ST-17 | A deferred filled literal with a non-literal row (`let g: i32[2][2] = [; c ? a : b];` shape) | The chosen row is stored through the expression form and the program computes the right elements | spec/08-arrays-strings.md · R2 · 03-01 fill form |

### Phase 2: Multi-Extent Return Copy (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-6 | `func copy(take a: String[2][2]): String[2][2] { return a; }`, caller reads every row | Every row equals the source's; the source is emptied; no double free or leak | spec/06-functions.md · R3 · AR #12 |
| ST-7 | The same shape with `i32[2][2]` | Every row equals the source's; the emitted C builds cleanly | spec/06-functions.md · R3 · AR #12 |

### Phase 3: Loop-Condition Owned-Call Release (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-8 | `while (grow().count > 0 && i < 3) { i = i + 1; }` with a `grow` that returns a fresh container each call | The body runs exactly three times and the program exits with the expected value; leak-freedom is pinned by the close-out host probe | spec/05-statements.md · R4 · AR #5 |
| ST-9 | The `do { ... continue; ... } while (grow().count > 0 && i < 3);` and `for` spellings | Same iteration count; `continue` re-evaluates the condition; leak-freedom is pinned by the close-out host probe | spec/05-statements.md §12 · R4 · AR #5 |

### Phase 4: Substitution-Aware Ownership (03-04)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-10 | `func keep<T>(take a: T[2]): i32 { return 0; }` with `T = String` and a `String[2]` argument | The parameter's elements are released; AddressSanitizer reports nothing | spec/06-functions.md · R5 · AR #17 |
| ST-11 | `func dup<T>(take a: T): T { let b: T = a; return b; }` with `String` | The value moves through `b`; the caller reads it; no double free | spec/03-ownership.md · R5 · AR #17 |
| ST-12 | `func reuse<T>(take a: T): i32 { let b: T = a; sink(borrow a); return 0; }` with `String` and with `i32` | `String` instance: `E10020` once per function and binding, at the body's use; `i32` instance: compiles and runs | spec/14-diagnostics.md:51 · R5, R8 · AR #11, #17 |
| ST-13 | `keep<i32>` and `dup<i32>`; and `func peek<T>(borrow a: T): i32` with `String` | No diagnostics, no extra release, behavior unchanged; the borrow instance frees nothing in the callee | R6 · AR #11, #17 |
| ST-14 | A generic body whose guarded operand is an impure param-typed call (`func at<T>(mk: func(): T[], i: i32): T { return mk()[i]; }`) with an owning substitution | `mk` runs once per evaluation; no leak | spec/04-expressions.md · R5 · site `own/lower.rs:619` |
| ST-15 | A generic body iterating an ownerless param-typed iterable (`func all<T>(mk: func(): T[]): i32 { for e in mk() { ... } }`) with an owning substitution | The iterable is evaluated once, not per iteration; no leak | spec/05-statements.md · R5 · 03-04 site table |
| ST-16 | A generic body passing an ownerless param-typed call result into a `take` parameter, owning substitution | The callee owns and releases it; the caller releases nothing | spec/06-functions.md · R5 · 03-04 site table |
| ST-18 | A generic loop condition holding a param-typed owned call result (a condition that calls a param-typed `mk()` behind a predicate), instantiated with an owning type | The call result is materialized and released per iteration; leak-freedom is pinned by the close-out host probe | spec/05-statements.md · R4, R5 · 03-03/03-04 |
| ST-19 | A param-typed `take` argument place (`func caller<T>(take a: T) { sink<T>(a); }` with `String`) | The caller empties its place, the callee owns and releases exactly once; AddressSanitizer reports nothing | spec/03-ownership.md · R5 · sites `own/call_check.rs:281`, `own/call.rs:352` |
| ST-20 | A param-typed operand used twice in a guarded operation within a generic body, owning substitution | The operand is materialized once and released once | spec/04-expressions.md · R5 · site `own/lower.rs:1051` |
| ST-21 | A generic body reading a param-typed fixed-array place under a guard, owning substitution | The read place is materialized; no leak | spec/04-expressions.md · R5 · site `own/lower.rs:980` |
| ST-22 | A generic body with a conditional param-typed move whose source is used afterwards (`func pick<T>(take a: T, c: bool): i32 { let b: T = c ? a : a; sink(borrow a); return 0; }` shape) | `String` instance: `E10020` once at the body's use; `i32` instance: accepted | spec/03-ownership.md · R5 · sites `own/expr.rs:50/85/108` |

> **Realization notes:** ST-14 and ST-15 are exercised with a `func(): T[2]` fixed-array call
> (a `T[]` slice is a view and cannot carry the owning duty) and borrow-based uses
> (`sink<T>(borrow mk()[i])`, `for (borrow e in mk())`); ST-20 is exercised with the numeric
> index instantiation the compound store requires. The cases' expectations and sites are
> unchanged; the full rationale is in the execution plan's Phase 4 record. The close-out probe
> runner also covers the `for` spelling of ST-9.

### Site → case map (03-04 completeness proof)

| Site | Pinned by |
| ---- | --------- |
| `own/lower.rs:619` | ST-14 |
| `own/lower.rs:705` | ST-11, ST-12 |
| `own/lower.rs:980` | ST-21 |
| `own/lower.rs:1051` | ST-20 |
| `own/lower.rs:1090`, `:1104`, `:1117` | ST-16 |
| `own/expr.rs:50`, `:85`, `:108` | ST-22 |
| `own/loops.rs:503` | ST-15 |
| `own/call_check.rs:281`, `own/call.rs:352` | ST-19 |
| `own/lower.rs:865` | Task 4.2.6 probe (acceptance ruling if reachable) |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as
> `compiler/tests/residual_defect_hardening_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/residual_defect_hardening_spec_test.rs` | ST-1 – ST-22 | All four phases (per-phase sets in the ST tables) |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as
> `compiler/tests/residual_defect_hardening_impl_test.rs`; focused predicates also get unit tests
> in the existing `#[cfg(test)]` blocks.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/residual_defect_hardening_impl_test.rs` | Emitted-C shapes: the flat declarator and the per-row expression store (conditional, place, call, and fill rows, plus the mixed-arm conditional tail and the adjusted-parameter row move); the return-arm `memcpy` and source emptying; the guarded loop forms for `while`/`do`/`for`, the guarded fixed-array condition, and the flat `for` step; the maybe-owning drop, the non-owning no-op move branch, and the emptied `take` argument place; `E10020` reported once per function and binding | High |
| `compiler/src/types/` (existing block) | `Ty::array` and `substitute` folding: fixed elements fold, slice elements and slice outer extents do not; update the depth-cap and nested-array tests (explicit `TyKind::Array`) | High |
| `compiler/src/own/` (existing blocks) | `maybe_owns`; the parameter-mode gate; `RequiresDuplicable` recording and discharge; the loan-owner predicate probe | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | Whole workspace | All existing suites (spec, impl, corpus, determinism) regress clean each phase |
| `make verify` | Cross-toolchain legs | Hosted-amiga, vbcc build-only, FS-UAE programs, runtime faults |

### End-to-End Tests

N/A — this compiler project's end-to-end tier is `make verify` (above); adding a new harness is
out of scope (AR #14, minimum-sufficient baseline).

## Test Data

### Fixtures Needed

- The same host-run helpers the existing suites use (`run_host`, `assert_exit`,
  `assert_rejected_with`, `emit_host`, `after`); the helpers are private to each test binary, so
  the new suites define their own copies rather than adding a shared module (AR #14).
- No new fixtures; every case is a self-contained `.clo` source string.

### Mock Requirements

None — real compiler, real C compiler, real runtime shims.

### AddressSanitizer Probes (close-out)

Run on the host: the pinned container's clang 14 has no ASan runtime.

```
clang -std=c99 -O0 -g -fsanitize=address <program.c> -o <program>.asan
ASAN_OPTIONS=detect_leaks=1 ./<program>.asan
```

Probe programs: the ST-4 deferred literal with owned rows (DEF-46), the ST-6 multi-extent owned
return (DEF-47), ST-8 and ST-9 (DEF-48), and ST-10, ST-11, ST-14, ST-15, ST-16, ST-18, ST-19, and
ST-21 (DEF-49). Each
probe exits with the case's expected value and prints no sanitizer report. The pre-fix runs of the
DEF-48 and DEF-49 probes are recorded in the execution plan as the red-phase evidence.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Every defect has a failing pre-fix observation (invalid C for DEF-46/47; recorded ASan
      probe for DEF-48/49)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] AddressSanitizer probes clean (close-out)
