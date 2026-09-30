# Testing Strategy: Call Value Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Lowering and emission fixes (every defect) | One specification case minimum, plus implementation-shape tests |
| Ownership and leak cases | Specification run plus AddressSanitizer probes in close-out |
| Existing behavior | No regressions: the full workspace suite and corpus per phase |

- Test names state behavior: `should [expected behavior] when [condition]`.
- Host runs are this compiler project's end-to-end tier: a program is compiled with `clc`, the
  emitted C is built with the pinned C compiler, and the process result is asserted. Unit tests for
  the ownership predicates live in the source module test blocks.
- Targets match the project's existing suites (every defect has a concrete case; no percentage
  gate is added).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the frozen specification under `spec/`, the Ambiguity Register, and the
> filed defect expectations. These define behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the implementation.
>
> **RED-PHASE RULE:** at least one case per defect must fail on the committed compiler before its
> fix. A case that already passes (today ST-4, ST-7, and ST-8) is a regression pin: its pre-pass is
> recorded in the execution plan and it is not counted as red-phase evidence. A run without
> AddressSanitizer cannot see a leak-only failure, so the leak cases are also covered by the
> close-out probes below and by an emitted-shape implementation pin.

### Phase 1: Owned Conditional Stores (03-01)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | `let c: String = ok ? a : b;` with `ok` false and true; `c` is read through `len` | `c` holds the chosen arm's value; the process exits `0` with no double release | spec/03-ownership.md §3 · AR #2 · DEF-34 |
| ST-2 | `let c: String = ok ? mk() : b;` with `ok` true and false | `c` holds the chosen value in both directions (the place arm is the former failure); clean exit | spec/03-ownership.md §3 · AR #2, #19 · DEF-34 |
| ST-3 | `let c: String = ok ? a : mk();` with `ok` true and false | `c` holds the chosen value in both directions (the place arm is the former failure); clean exit | AR #2, #19 · DEF-34 |
| ST-4 | `eat(ok ? a : b)` where `eat(take arr: String[2])` sums the element lengths | The chosen arm's contents are read; clean exit (regression pin; the maybe-moved join is pinned by the closed plan's `rvi30`) | spec/06-functions.md §2 · AR #2, #19 · DEF-34 |
| ST-5 | `let c: String = ok ? a[0] : a[1];` | `E10026` rejection (element move) | spec/03-ownership.md §4 · AR #3 · DEF-34 |
| ST-6 | `let x: i32 = ok ? a[0] : a[1];` with duplicable elements | `x` is the chosen element; clean exit | spec/03-ownership.md §3 · AR #3 · DEF-34 |
| ST-7 | `ok ? sink(s) : t;` discarded, then `len(t)` is read | Clean exit; `t` stays owned and readable (regression pin; the leak is probed in close-out) | spec/03-ownership.md §3 · AR #2 · DEF-45 |
| ST-8 | `let r: String = ok ? sink(s) : t;` with `ok` true, then `r` is read | `r` holds `sink`'s value; clean exit (regression pin; leak-checked in close-out) | AR #2, #19, #16 · DEF-45 |
| ST-32 | `s = ok ? a : b;` where `s` already owns a `String` | The old `s` value is released once; `s` holds the chosen arm; clean exit | AR #2 · PF-006 · DEF-34 |
| ST-33 | `s = ok ? s : b;` (self-naming arm) with both `ok` values | The chosen value survives; no double release; clean exit | AR #2 · PF-006 · DEF-34 |
| ST-34 | `return ok ? sink(s) : t;` with both `ok` values, then the caller reads the result | The returned value stays valid; no use-after-free; clean exit | AR #2, #19 · PF-003 · DEF-34 |
| ST-35 | `eat(ok ? a : b)` where `eat(take s: String)` reads `len(s)` | The callee reads the chosen value; the caller releases nothing it transferred; clean exit | AR #2, #19 · PF-004 · DEF-34 |
| ST-36 | `let c: Box = ok ? a : b;` where `Box` owns a `String` and `ok` has both values | `c` holds the chosen value; no double release; clean exit | AR #2 · PF-007 · DEF-34 |
| ST-37 | `Box { name: ok ? a : b, tag: 1 }` and the take-call variant `name: ok ? sink(s) : t` | `name` holds the chosen value; no double release or use-after-free; clean exit | AR #2, #19 · PF-005 · DEF-34 |

### Phase 2: Fixed-Array Value Positions (03-02)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-9 | `rows[i] = value;` on `i32[2][2]` with `i = 1` | The second row equals `value`; clean exit | spec/08-arrays-strings.md · AR #13 · DEF-31 |
| ST-10 | `rows[0] = value;` on `i32[2][2]` | The first row equals `value`; clean exit | spec/08-arrays-strings.md · AR #13 · DEF-31 |
| ST-11 | `rows[i] = [x, y];` from variables | The stored row equals `[x, y]`; clean exit | spec/08-arrays-strings.md · AR #11 · DEF-31 |
| ST-12 | `rows[i] = ok ? a : b;` | The stored row equals the chosen arm; both arms keep their own storage | spec/08-arrays-strings.md · AR #11 · DEF-31 |
| ST-13 | `eat(ok ? mkA() : mkB())` over a duplicable `i32[2]` | The callee reads the chosen call's values; clean exit | spec/06-functions.md · AR #11 · DEF-32 |
| ST-14 | `eat(ok ? [1, 2] : [3, 4])` | The callee reads the chosen literal's values; clean exit | AR #11 · DEF-33 |
| ST-15 | `sink(mkA())` (a fixed-array call result as an argument) | The callee reads `mkA`'s values; clean exit | AR #11 · DEF-32 |
| ST-16 | `[a, b][0]` in value position and as a call argument | The element is read with no C diagnostic; the value is correct | spec/08-arrays-strings.md · AR #13 · DEF-35 |
| ST-17 | `Box { arr: ok ? a : b, tag: 1 }` then field reads | `arr` holds the chosen arm's elements and `tag` is `1`; clean exit | spec/07-structs.md · AR #7 · DEF-38 |
| ST-41 | `eat(Box { arr: ok ? a : b, tag: 1 })` (argument position) | The callee reads `arr` and `tag`; the emitted C builds and runs; clean exit | spec/07-structs.md · spec/04-expressions.md §3 · AR #7 · PF-011 · DEF-38 |

### Phase 3: Func Pointers and Callable Resolution (03-03)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-18 | `let g: func(i32): i32[2] = (x: i32) => { return [x, x + 1]; };` then `g(5)[1]` | `6`; the emitted C builds cleanly | spec/02-types.md (function types) · spec/17-codegen.md · DEF-30 |
| ST-19 | An array-returning lifted lambda invoked through a generic callable bound `F: func(i32): i32[2]` (the only path that exercises the closure struct's `fn` field); plus a capture-less lambda in a plain `func`-typed field | The returned array is correct; clean exit | spec/02-types.md · spec/17-codegen.md · DEF-30 |
| ST-20 | `func make(): func(i32): i32 { return double; }` then `make()(4)` | `8`; valid C declarator | DEF-43 |
| ST-21 | A lambda capturing a func-typed local and calling it by name; a same-named module function also exists | The captured value is called (not the module function) | DEF-40 |
| ST-22 | A module-level `let global_fn: func(i32): i32 = double;` called by name | `global_fn(3)` returns `6` | DEF-41 |
| ST-23 | `show(stringOf("hi"))` where `let show: func(str): void = print_str;` | The implicit `String` → `str` widening is emitted; the program builds and runs | spec/02-types.md · AR #12 · DEF-42 |
| ST-24 | `let bump: func(borrow Entity): void = nudge; bump(e);` | The callee writes through the pointer; the caller sees the update | spec/grammar.ebnf.md §4 · AR #4, #14 · DEF-44 |
| ST-25 | The same `borrow` func type as a struct field, invoked through the field | The write is visible; clean exit | AR #4, #14 · DEF-44 |
| ST-39 | `((x: i32) => { return [x, x + 1]; })(5)[1]` (direct invocation) | `6`; the emitted C builds cleanly | spec/02-types.md · spec/17-codegen.md · PF-012 · DEF-30 |
| ST-40 | `let g: func(Entity): void = nudge;` where `nudge` takes `borrow Entity` | `E10210` (mode mismatch) | spec/02-types.md · AR #14 · PF-008 · DEF-44 |

### Phase 4: Guards and Cleanup (03-04)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-26 | `func f(take a: String[2])` containing `let b: String[2] = a;` then returning `len(b[0])` | Correct length; no over-read, no NULL dereference; clean exit | spec/06-functions.md §2 · AR #10 · DEF-36 |
| ST-27 | `do { i = i + 1; } while (vlen(mk()[0]) == 2 && i < 3);` | The condition re-evaluates after every iteration; the loop terminates with the correct `i` | spec/05-statements.md · AR #8 · DEF-37 |
| ST-38 | `do { if (i == 1) { continue; } i = i + 1; } while (vlen(mk()[0]) == 2 && i < 3);` | Every iteration — fall-through and `continue` alike — re-evaluates the guarded condition; correct `i`; clean exit | spec/05-statements.md §12 · AR #8 · PF-002 · DEF-37 |
| ST-28 | A `do`/`while` whose condition carries a fixed-array guarded operand | Each iteration evaluates the guarded operand; no stale-temporary read | AR #8 · DEF-37 |
| ST-29 | `while (i < 3 && growable()[0].count > 0)` where `growable(): Array<i32>[2]` | The loop body runs the correct number of times; no leak (ASan in close-out) | AR #9, #16 · DEF-39 |
| ST-30 | The embedded guard spelling with a fixed-array value still drops before re-materializing | The fixed-array case keeps its behavior (regression) | AR #9 · DEF-39 |
| ST-31 | The statement spelling of a growable guarded operand still drops correctly | The statement case keeps its behavior (regression) | AR #9 · DEF-39 |

## Test Categories

### Specification Tests (from ST-cases above)

> Written BEFORE implementation. Filed as `compiler/tests/call_value_integrity_spec_test.rs`.

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/call_value_integrity_spec_test.rs` | ST-1 – ST-41 | All four phases (per-phase sets in the ST tables) |

### Implementation Tests (edge cases, internals)

> Written AFTER implementation. Filed as `compiler/tests/call_value_integrity_impl_test.rs`;
> focused ownership predicates also get unit tests in the existing `#[cfg(test)]` blocks.

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/call_value_integrity_impl_test.rs` | Emitted-C shapes: owned branch store (empty, live, and self-naming destinations; handles and aggregates), arm zeroing, transfer-temp registration and release (discard and adoption), non-releasing take-argument temporary, return store, struct-literal field store; element-store byte copy (constant and runtime index, one shared core); materialized argument/projection/struct literal; declarator shapes (return, closure field, lifted lambda, direct invocation) and the hidden destination plumbing; capture/global spelling; widening; borrow pointer and the callee-type plumbing; parameter copy/zero on both sides; goto-free flag-first do-while (fall-through and `continue`); growable drop expression (no-drop elements) | High |
| `compiler/src/own/tests.rs` (existing block) | Updated transfer-temp release-set pin (AR #15) | High |
| `compiler/tests/runtime_value_integrity_impl_test.rs` (existing) | Updated old-behavior pin (AR #15) | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | Whole workspace | All existing suites (spec, impl, corpus, determinism) regress clean each phase |
| `make verify` | Cross-toolchain legs | Hosted-amiga (`m68k-amigaos-gcc`, `vamos`), vbcc build-only, FS-UAE programs, runtime faults |

### End-to-End Tests

N/A — this compiler project's end-to-end tier is `make verify` (above); adding a new harness is out
of scope (AR #6, minimum-sufficient baseline).

## Test Data

### Fixtures Needed

- The same host-run helper pattern the existing suites use (`run_host`, `assert_exit`,
  `assert_rejected_with`, `emit_host`, `after`); the helpers are private to each test binary, so
  the new suites define their own copies rather than sharing a module (a shared support module
  would be new machinery).
- No new fixtures; every case is a self-contained `.clo` source string.

### Mock Requirements

None — real compiler, real C compiler, real runtime shims.

### AddressSanitizer Probes (close-out)

Run on the host: the pinned container's clang 14 has no ASan runtime.

```
clang -std=c99 -O0 -g -fsanitize=address <program.c> -o <program>.asan
ASAN_OPTIONS=detect_leaks=1 ./<program>.asan
```

Probe programs, one per memory-safety defect: ST-1/ST-32/ST-33 shapes (DEF-34), ST-26 (DEF-36),
ST-27/ST-38 (DEF-37), ST-29 (DEF-39), and a function-scoped or looped discard shape for DEF-45
(the plain main-scoped ST-7/ST-8 temporaries are still reachable at exit, so leak detection stays
quiet there). "Exit 0 and no AddressSanitizer output" asserts leak-freedom and no double-free.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] At least one case per defect verified to FAIL before its fix (red phase); pre-passing cases
  recorded as regression pins
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] AddressSanitizer probes clean (close-out, AR #16)
