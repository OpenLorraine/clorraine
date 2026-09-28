# Testing Strategy: Array Value Integrity

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Element typing and loop lowering (`own/loops.rs`, `sema/stmt.rs`, `types`) | every ST case below |
| Fixed-array store path (`codegen/function.rs`) | every shape: init, reassign, move, save |
| Projection materialization and `len` (`own/lower.rs`, `codegen/guards.rs`, `codegen/calls.rs`) | every ST case below |
| Regression guards | single-extent loops, slices, growable arrays, call returns, existing diagnostics |

- Test names state behavior: `should [expected behavior] when [condition]`.
- End-to-end coverage: the close-out phase runs the new programs under
  AddressSanitizer and `make verify` over every profile and toolchain leg.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> These test cases are derived EXCLUSIVELY from the requirements
> (`01-requirements.md`), the component specs (`03-01`, `03-02`), and the
> frozen specification. They define expected behavior BEFORE any
> implementation exists.
>
> **IMMUTABLE ORACLE RULE:** Do NOT modify these expectations to match the
> implementation. If the implementation does not match a spec test case, the
> implementation is wrong — not the test.
>
> The `Source` column lives **in this plan document** — it is not a code
> comment. When the executor turns an ST case into a test, the test's
> in-code traceability comment quotes the behavior in **plain language** and
> never the `ST-` id or a `plans/` path.

### Phase 1 — Fixed-array values and `len` (`03-01`, `03-02`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-1 | `len(m)` for `m: i32[2][3] = [[1,2,3],[4,5,6]]` | `2` (outermost element count) | R5 · AR-6 · `spec/04` §9 |
| ST-2 | `let a: i32[2] = [1,2]; let b: i32[2] = a;` then `b[0]+b[1]` and `a[0]` | `3`; `a` still usable | R2 · AR-2 · `spec/03` §3, V8/V9 |
| ST-3 | `struct S { v: i32[2]; }`; `let c: i32[2] = s.v;` | `c[0]+c[1] == 3` | R2 · AR-2 |
| ST-4 | `let r: i32[3] = m[0];` for the ST-1 `m` | `r[0]+r[1]+r[2] == 6` | R2 · AR-2 |
| ST-5 | `let a: String[2] = [stringOf("ab"), stringOf("cde")]; let b: String[2] = a;` then `len(b[1])` | `3`; process exits 0 (no double free) | R2 · AR-4 · `spec/03` §3 |
| ST-6 | `let b: String[2] = a; let c: String[2] = a;` | `E10020` (`'a' was moved and cannot be used`) | R6 · `spec/03` §3 |
| ST-7 | `b = a;` where both are initialized `String[2]` arrays with live elements | `len(b[1]) == 3`; exits 0; the old `b` elements are dropped once | R2 · `spec/03` O3 |
| ST-8 | `let a: i32[2] = [1,2]; a = a;` | `a[0]+a[1] == 3` | R3 · AR-7 |
| ST-9 | `let b: i32[2] = mk();` where `mk(): i32[2]` returns `[1,2]` | `b[0]+b[1] == 3` | R6 · `spec/06` F2 (returns by value) |
| ST-22 | `len(stringOf("hello"))` and `s.len()` for the same `String` | `5` both ways | R5 · AR-5 · `spec/02` §9, `spec/04` §11 |
| ST-25 | `let s: String[2] = [stringOf("ab"), stringOf("cde")]; s = mk();` where `mk(): String[2]` returns `[stringOf("pq"), stringOf("rs")]`; then `len(s[1])` | `2` (the new array); Phase 4's emitted deleter count shows one release site per old element | R2 · AR-4 · `spec/03` O3 |
| ST-26 | `s = dup(borrow s);` where `dup(borrow source: String[2]): String[2]` returns `[stringOf(source[1]), stringOf(source[0])]` | `len(s[0]) == 3` ("cde"); exits 0 (alias-safe; the old value released) | R3 · AR-7 · `spec/03` O3 |

### Phase 2 — Multi-extent element loops (`03-01`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-10 | `for (row in m) { s += row[0]+row[1]+row[2]; }` for the ST-1 `m` | `21`; terminates | R1 · `spec/08` §5 LOOP-1 |
| ST-11 | `for (borrow row in m) { row[0] = row[0] + 10; }` then `m[0][0]+m[1][0]` | `25` (writes reach the array) | R1 · `spec/08` LOOP-2 |
| ST-12 | `m3: i32[2][2][2] = [[[1,2],[3,4]],[[5,6],[7,8]]];` `for (row in m3) { s += row[0][0] + row[1][1]; }` | `18` (2 iterations) | R1 · `spec/08` AR-3 |
| ST-13 | `for (row in xs) { s += row[1]; }` where `xs: i32[][3]` is `borrow m` from ST-1 | `7` (2 + 5); slice of rows | R1 · `spec/08` AR-3, §5 |
| ST-14 | `for (row in fs)` where `fs: String[2][2]` and no `borrow` | `E10040` (`element loop requires 'borrow' for non-duplicable 'String[2]'`) | R1 · `spec/08` LOOP-3 |
| ST-15 | `for (row in m) { for (x in row) { s += x; } }` for the ST-1 `m` | `21`; every inner iteration advances | R1 · `spec/08` §5 |

### Phase 3 — Projections and element moves (`03-02`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|---|---|---|
| ST-16 | `let x: i32 = mk()[1];` where `mk(): i32[2]` returns `[7,9]` | `9` | R4 · AR-5 · `spec/04` §9 |
| ST-17 | `mk2()[1][2]` where `mk2(): i32[2][3]` returns `[[1,2,3],[4,5,6]]` | `6` | R4 · AR-5 |
| ST-18 | `vlen(mk()[1])` where `vlen(borrow s: String)` and `mk(): String[2]` returns `[stringOf("ab"), stringOf("cde")]` | `3` (the call runs once) | R4 · AR-5 |
| ST-19 | `let s: String = mk()[0];` | `E10026` | R4 · AR-5 · `spec/03` §4 |
| ST-20 | `consume(take mk()[0]);` | `E10026` | R4 · AR-5 · `spec/03` §4 |
| ST-21 | `func first(): String { return mk()[0]; }` | `E10026` | R4 · AR-5 · `spec/03` §4 |
| ST-23 | `let ok: bool = false; if (ok && boom()[0] == 1) { return 1; } return 0;` with `boom(): i32[2] { panic("boom"); }` | exits `0`; `boom` never runs | R4 · AR-5 · `spec/04` short-circuit |
| ST-24 | `len("hello")`, `len([1,2])`, `len(growable)` where the growable holds 3 items | `5`, `2`, `3` | R6 · `spec/04` §9, §11 |
| ST-27 | `s = mk()[0];` for `s: String`, `mk(): String[2]` | `E10026` | R4 · AR-5 · `spec/03` §4 |
| ST-28 | `let h: Holder = Holder { s: mk()[0] };` (struct-literal field) | `E10026` | R4 · AR-5 · `spec/03` §4 |
| ST-29 | `h.s = mk()[0];` (field-target assignment of an ownerless element) | `E10026` | R4 · AR-5 · `spec/03` §4 |
| ST-30 | `h.s = a[0];` (field-target assignment of a place element; accepted today) | `E10026` | R4 · AR-5 · `spec/03` §4 |

> **⚠️ AUTHORING RULE:** Derive expectations from the specification. Do NOT
> imagine or infer what the implementation will produce. If an expected output
> cannot be determined from the spec, that is an ambiguity — add it to the
> register and resolve it with the user before defining the test case.

### Recorded red-phase split

The spec suite is expected to fail before the fix exactly as follows; a
deviation is an oracle or test defect, never a reason to change a case.

| State | Cases | Observed pre-fix (2026-09-28, host) |
| ----- | ----- | ----------------------------------- |
| Fails: wrong `len` value | ST-1 (`6`), ST-22 (`0`), ST-25 (`0`), ST-26 (`0`) | runs, wrong result |
| Fails: C compiler rejects | ST-2 – ST-5, ST-7, ST-8, ST-16 – ST-18, ST-23 | `clc` accepts, `cc` reports array assignment / hidden out-parameter errors |
| Fails: wrong shape or missing diagnostic | ST-10 – ST-13, ST-15 | `E10209` on the row index, or 6 base-element iterations |
| Fails: missing rejection | ST-19 – ST-21, ST-27 – ST-30 | `clc` accepts (no `E10026`) |
| Passes pre-fix (guards) | ST-6, ST-9, ST-14, ST-24 | `E10020`, call-return binding, `E10040`, and the view/growable lengths already correct; the call-reassignment release shape is pinned red by the Phase 1 implementation tests |

## Test Categories

### Specification Tests (from ST cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/array_value_integrity_spec_test.rs` | ST-1 – ST-30 | `03-01`, `03-02` |

### Implementation Tests (edge cases, internals)

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/array_value_integrity_impl_test.rs` | Emitted shape: `memcpy` copy, `cl_zero` after a move, the same-place no-op, the self-assignment save, the call-into-temporary assignment, the materialized temporary with the destination call, the short-circuit comma operand, both `len` renderings, the pointer-to-array row declaration, and counting-deleter release shape checks (one emitted deleter call site per released element); `iterated_element` and `is_ownerless_element_move` unit coverage | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| `make check` | all | Spec check plus the full Rust workspace test run |
| `make verify` | all | Every hosted-amiga and freestanding leg |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| ASan copy/move probes | Emit, build with `clang -std=c99 -O0 -g -fsanitize=address`, run the ST-2, ST-5, ST-7, ST-8, ST-16, ST-18, ST-23, ST-25, and ST-26 programs with `ASAN_OPTIONS=detect_leaks=0` | 0 ASan error reports; exits match the oracle; leak detection is off (limitation `DEF-23`) |
| Deleter-count probes | Emit the ST-25 and ST-26 programs and count the emitted deleter call sites for the old elements | One release site per old element; the runs exit clean |
| Cross-profile build | `make verify` | all legs passed |

## Test Data

- Scratch directories per case, following the existing helper pattern in
  `compiler/tests/element_loop_integrity_spec_test.rs` (`ScratchDir`,
  `emit_host`, `build_host_c`, `run_with_timeout`, `run_host`,
  `assert_exit`, `assert_stdout`, `assert_rejected`).
- Timeout for every run case: 30 seconds, so a mis-selected loop step cannot
  hang the suite.
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
- [ ] Test coverage meets goals
