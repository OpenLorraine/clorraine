# Testing Strategy: Runtime Value Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| The materialization and temporary-release paths (`own/lower.rs`, `own/loops.rs`, `own/statements.rs`, `codegen/function.rs`) | every ST case below |
| The statement-scoped temporary view rule (`own/place.rs`, `own/liveness.rs`) | ST-24, ST-25 |
| The callable-field guard and func-pointer ABI paths (`codegen/function.rs`, `codegen/types.rs`, `codegen/cleanup.rs`) | every ST case below |
| The runtime-boundary paths (`codegen/function.rs`, `codegen/guards.rs`, the three shims) | every ST case below |
| Regression guards | existing laziness, guard traps, `E10040`, named calls, scalar func values |

- Test names state behavior: `should [expected behavior] when [condition]`.
- End-to-end coverage: the close-out phase runs `make verify` over every profile and
  toolchain leg, plus host AddressSanitizer probes with leak detection on.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived EXCLUSIVELY from the requirements
> (`01-requirements.md`), the component specs (`03-01`, `03-02`, `03-03`), and the frozen
> specification. They define expected behavior BEFORE any implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the
> implementation. If the implementation does not match a spec test case, the
> implementation is wrong — not the test.
>
> The `Source` column lives **in this plan document** — it is not a code comment. When the
> executor turns an ST case into a test, the test's in-code traceability comment quotes the
> behavior in **plain language** and never the `ST-` id or a `plans/` path.

### Phase 1 — Materialization and temporary release (`03-01`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `let sum = 0; for (x in mk()) { sum = sum + x; }` with `mk(): i32[2] = [1, 2]` | accepted; `sum == 3` | R1 · AR-6 · `spec/05` §9 |
| ST-2 | `for (borrow r in mkRows()) { r[0] = r[0] + 10; total = total + r[0]; }` with rows `[1,2,3]`, `[4,5,6]` | accepted; `total == 25` (writes reach the materialized rows) | R1 · AR-6 · `spec/05` §9 |
| ST-3 | `return vlen(mk()[1]);` with `mk(): String[2] = ["a", "bb"]` | accepted; returns `2`; the statement's temporary elements are released (ASan close-out) | R2 · AR-5 · `spec/17-codegen.md:80` |
| ST-4 | `for (borrow s in mkStrings()) { total = total + vlen(s); }` with `mkStrings(): String[2] = ["a", "bb"]` | accepted; `total == 3`; the loop's materialized iterable is released after the loop (ASan close-out) | R1, R2 · AR-5, AR-6 · `spec/17-codegen.md:80` |
| ST-5 | `let c: i32[2] = ok ? a : b;` (`ok == true`, `a = [1,2]`, `b = [3,4]`) | accepted; `c[0] + c[1] == 3` | R3 · AR-7 · `spec/03` §3–4 |
| ST-6 | `let c: String[2] = ok ? a : b;` (`ok == true`) then `vlen(b[0])` | accepted; `c` holds `a`'s elements; `b` still owns its elements (returns `2 + 1 == 3`) | R3 · AR-7 · `spec/03` §3–4 |
| ST-7 | `let ok = false; s = ok ? s : t;` with owned `String[2]` values | accepted; `s` takes `t`'s values; the old `s` is released exactly once (ASan close-out) | R3 · AR-7 · `spec/03` §4 |
| ST-8 | `return ok ? a : b;` from a function returning `i32[2]` | accepted; the returned row is the chosen arm | R3 · AR-16 · `spec/03` §3 |
| ST-9 | `eat(ok ? a : b)` where `eat(take arr: i32[2])` and `ok == false` | accepted; the call receives `b`'s bytes | R3 · AR-16 · `spec/03` §3 |
| ST-10 | `let c: i32[2] = ok ? a : panicPair();` with `ok == true` | accepted; `panicPair` does not run (the un-chosen arm is never evaluated) | R3 · AR-7 · `spec/04` short-circuit rule |
| ST-11 | `if (false && vlen(mk()[0]) > 1) { return 1; } return 0;` with `mk(): String[2]` | accepted; returns `0`; the unevaluated temporary's release is a no-op (no drop of garbage) | R2 · AR-5 · `spec/17-codegen.md:80` |
| ST-24 | `let n: i32 = i32(len(mk()[0])); return n;` with `mk(): String[2] = ["ab", "cde"]` | accepted; returns `2`; the temporary's elements are released after the statement (ASan close-out) | R2 · AR-18 · `spec/17-codegen.md:80` |
| ST-25 | `let v: str = mk()[0]; return i32(len(v));` with `mk(): String[2]` | `E10030` — the view's use has no release-free path past the temporary's statement | R2 · AR-18 · `spec/16` BR-1 |
| ST-26 | `eat(ok ? a : b)` where `eat(take arr: String[2])`, `ok == false`, `a = ["ab","cde"]`, `b = ["fg","hij"]`; then `return i32(len(a[0]));` | accepted; the call receives `b`'s elements and releases them; `a` still owns its elements (returns `2`; ASan clean) | R3 · AR-7, AR-16 · `spec/03` §3 |
| ST-27 | `let x: i32 = (ok ? a : b)[1];` with `a = [1,2]`, `b = [3,4]`, `ok == false` | accepted; returns `4` (the chosen row's element) | R3 · AR-16 · `spec/04` |
| ST-28 | `for (x in ok ? a : b) { sum = sum + x; }` with `a = [1,2]`, `b = [3,4]`, `ok == false` | accepted; `sum == 7` | R3 · AR-16 · `spec/05` §9 |
| ST-29 | `eat(ok ? a : b)` where `eat(borrow arr: String[2])`, `ok == false`, `a = ["ab","cde"]`, `b = ["fg","hij"]`; then `return i32(len(a[0]));` | accepted; the call reads `b`'s elements; both `a` and `b` still own their elements (returns `2`; ASan clean) | R3 · AR-16 · `spec/03` §3 |

### Phase 2 — Callable fields (`03-02`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-12 | `ops[i].run(1);` with `i == 0` and `run: func(i32): i32` | accepted; the call runs and the program returns `0` | R4 · AR-8 |
| ST-13 | `ops[i].run(1);` with `i == 5` over an `Ops[2]` | `E11211` bounds trap before the call | R4 · AR-8 · `spec/14` |
| ST-14 | `let a: i32[2] = ops[i].run(1);` with `i` out of range | `E11211` bounds trap before the call | R4 · AR-8 · `spec/14` |
| ST-15 | `return ops[i].run(1);` with `i` out of range | `E11211` bounds trap before the call | R4 · AR-8 · `spec/14` |
| ST-16 | `struct Maker { make: func(i32): i32[2]; }`; `let a: i32[2] = m.make(3); return a[1];` | accepted; the C compiles; returns `4` | R5 · AR-9 · `spec/17` by-value aggregate return |
| ST-17 | `let g: func(i32): i32[2] = pair; let a: i32[2] = g(3); g(3); return a[0] + a[1];` | accepted; the C compiles; returns `7`; the discarded call passes a release temporary (no owning element in this case, so the ASan set excludes it) | R5 · AR-9 |

### Phase 3 — Runtime boundary (`03-03`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-18 | `panic("boom")` | the process aborts and stderr contains `panic: boom` | R6 · AR-10 · `spec/17-codegen.md:344` |
| ST-19 | `panic(viewOf("boom"))` with `func viewOf(s: str): str { return s; }` | aborts with `panic: boom` (a `str` view, no NUL assumption) | R6 · AR-10 · `spec/02` §9 |
| ST-20 | `panic(stringOf("owned"))` | aborts with `panic: owned` (a `String` handle) | R6 · AR-10 · `spec/08` §6 |
| ST-21 | `let ok = false; let r = ok && sink(s);` where `sink(take s: String)` counts its calls | accepted; the call and the move do not run; the counter stays `0` | R7 · AR-11 · `spec/04` short-circuit rule |
| ST-22 | `let ok = true; let r = ok && sink(s);` | accepted; the call runs once and `s` is moved exactly once (the counter is `1`; no double release) | R7 · AR-11 · `spec/03` §4 |
| ST-23 | `let r: i32 = cond ? sinkCount(s) : 0;` with `cond == false` | accepted; the arm call and the move do not run; the counter stays `0` | R7 · AR-11 · `spec/04` short-circuit rule |

### Recorded red-phase predictions

The suite is expected to fail before the fixes exactly as follows; a deviation is an oracle
or test defect, never a reason to change a case.

| State | Cases | Expected pre-fix observation |
| ----- | ----- | ---------------------------- |
| Fails: invalid C | ST-1, ST-2, ST-4 – ST-8, ST-10, ST-16, ST-17, ST-29 | `clc` accepts but the host C compiler rejects the emitted C (`mk()[i]` over the out-param call; `memcpy` over a decayed ternary, including the `__cl_owned` argument binding; the return fallback `__cl_ret = (cond ? a : b)`; the wrong func-pointer type) |
| Fails: no bounds trap | ST-13 – ST-15 | the receiver runs unguarded, so the expected `E11211` trap text never appears (a crash or a wrong call is still a failure) |
| Fails: abort-path build | ST-18 – ST-20 | the C compiler rejects `cl_panic(ClStr)` / `cl_panic(void *)` |
| Fails: eager guarded call | ST-21, ST-23 | the counter is `1` where `0` is due |
| Fails: missing lifetime rejection | ST-25 | the view binding is accepted and the expected `E10030` never appears |
| Passes pre-fix (guards) | ST-3, ST-9, ST-11, ST-12, ST-22, ST-24, ST-26, ST-27, ST-28 | ownerless reads, decayed conditional arguments and iterables, unevaluated guarded temporaries, in-range receiver calls, and evaluated guarded calls already run; ST-24/ST-26 release behavior is checked by the ASan close-out |

## Test Categories

### Specification Tests (from ST cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/runtime_value_integrity_spec_test.rs` | ST-1 – ST-29 | `03-01`, `03-02`, `03-03` |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/runtime_value_integrity_impl_test.rs` | Emitted shape: an inline statement-position release for an owning `__cl_tmpN`; the absence of a release for the take-call and literal-adoption transfer temps; the iterable temporary; the branch store and per-arm guards for a conditional in the assignment and return positions; the receiver trap in the three call shapes; the destination pointer declarator and a discarded local func-value call; the two-argument `cl_panic` for each message form; the move-out inside the guarded path only | High |

Unit coverage: `own::tests` for the iterable materialization predicate, the conditional
store shape, the destination release set (pinned to the materialization sites), and the
temporary-view loan registration, plus the deferred move-out; `codegen` unit tests for
the func-pointer declarator where the module already tests declarators.

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | all | Spec check plus the full Rust workspace test run |
| `make verify` | all | Every hosted-amiga and freestanding leg |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Cross-profile build | `make verify` | all legs passed |
| Leak-free temporaries | Host AddressSanitizer probes with leak detection on for ST-3, ST-4, ST-7, ST-24, ST-26, ST-29 | clean |
| View lifetime | ST-25 rejects with `E10030`; no sanitizer case is needed because the rejection is compile-time | rejected |

## Test Data

- Scratch directories per case, following the existing helper pattern in
  `array_value_integrity_spec_test.rs` (`ScratchDir`, `emit_host`, `build_host_c`,
  `run_with_timeout`, `run_host`, `assert_exit`, `assert_rejected`, `assert_rejected_with`,
  `assert_trap`).
- Timeout for every run case: 30 seconds.
- Runtime traps use the existing `E11211` fault text; panic cases assert the exit status
  and the stderr prefix.
- No mocks; the real compiler and C toolchain run.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] AddressSanitizer probes clean with leak detection on
