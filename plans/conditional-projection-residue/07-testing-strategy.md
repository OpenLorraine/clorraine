# Testing Strategy: Conditional, Projection, and Update Residue

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Ownership/lifetime judgments | Every in-scope row has a red/green reproduction and an ASan run |
| Emission shapes | Every accepted in-scope spelling builds under `-Werror` and runs |
| Controls | The untouched twin spellings stay byte-identical where the phase changes no shape |

- Test names state behavior (`should_…`), as the existing suites do.
- Integration tests are the corpus runs on the host and (at close) the Amiga legs; E2E is the
  `clorraine` driver run over the spec cases.
- The new suites:
  `compiler/tests/conditional_projection_residue_spec_test.rs` (specification tests, from the
  ST cases below) and `compiler/tests/conditional_projection_residue_impl_test.rs` (implementation
  pins). The batch-H suites are the regression oracle and stay green.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the component specs (`03-01` – `03-06`), the frozen specification
> (Ch 03/04/07/17, spec/14 registry), and the Ambiguity Register. IMMUTABLE ORACLE: do not modify
> an expectation to match the implementation.

### P1 — Arm roots and per-instance duty (`03-01`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-1 | The generic projection store `let s: T = (c ? a : mkPair<T>(x, y)).v;` instantiated with `String`, both arms, run in a loop | Accepted, strict build; run exit 0; ASan-clean (the root's sibling releases once; the moved value is readable) | R1, AR-3 |
| ST-2 | `let s: String = c ? a.v : mkBox().v;` both arms | Accepted; run exit 0; ASan-clean (the fresh root's sibling releases; the selected value correct) | R1 |
| ST-3 | `let s: String = (c ? a : Box { v: …, w: … }).v;` both arms | Accepted; run exit 0; ASan-clean | R1 |
| ST-4 | `if (g && consume((c ? a : mkBox()).v) > 0) { … }` with `g` true and false, and the `\|\|` and fresh-arm variants, in a loop | Accepted; run per the paths; ASan-clean (one release per execution; no double-free) | R1, AR-4 |
| ST-5 | `H { call: addOne, s: mkBox().v }.call(5)` in an `if` and in a loop condition | Accepted; run exit 0; ASan-clean (the projection root's sibling releases per execution) | R1, AR-4 |
| ST-6 | Control: the concrete twins of ST-1..ST-5 (`String` spelled out) | Accepted; run exit 0; ASan-clean; byte-identical emission | R1 |

### P2 — View derivation and owners (`03-02`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-7 | `let s: u8[] = bytes((c ? b : mk()).v); b.v = stringOf("zz"); s[0]` | Rejected with the plain twin's judgment (the live view's mutation reports); no UAF program | R2 |
| ST-8 | `use([mk().v])` (an argument) and `Holder { names: [mk().v] }` (a field) | Accepted; run exit 0; ASan-clean (no leak) | R2 |
| ST-9 | `func f(s: String): str[1] { return [s]; }` and `return [h.v]` | Accepted (the single-view twin `return s` is accepted) and the twin's exact judgment for `[h.v]`; no false `E10034`, no extra `E10036` | R2 |
| ST-10 | `func pick(x: str, h: Box, c: bool): str { return c ? x : h.v; }` | Rejected with exactly one `E10036` | R2, AR-7 |
| ST-11 | `return (c ? a : mkBox()).v;` (a local-backed arm beside a generated-owner arm) | Rejected with exactly one report (`E10035`) | R2, AR-8 |

### P3 — Escaping stores (`03-03`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-12 | `names = mk().v;` and `names = [mk().v]` (a global `str` / `str[1]`) | Rejected with exactly one `E10033` | R3, AR-5 |
| ST-13 | `fill(borrow slot: str) { slot = mk().v; }` and the parameter-supported twin | The fresh-derived store rejected (one report); the parameter-supported twin accepted and run | R3, AR-5 |
| ST-14 | `g.names = [mk().v]` (a global struct field) and a lambda capturing a `str[1]` | Accepted; strict build; run exit 0 | R3 |
| ST-15 | `xs[1] = c ? b.v : a;` and `xs[1] = v;` (a local `str[2]`) | Accepted; run exit 0; ASan-clean; the global/field spellings still rejected | R3, AR-6 |

### P4 — Guarded and discarded literal positions (`03-04`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-16 | `while (i < 1 && (c ? H { … } : H { … }).call(5) > 0)` single-shot and loop; the array-literal element variant | Accepted; run exit 0; ASan-clean (no double-free) | R4, AR-12 |
| ST-17 | `ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };` | Accepted; run exit 0; ASan-clean; `t`/`u` readable afterwards | R4 |
| ST-18 | `let t: String = stringOf("t"); c ? mk().v : t;` | Accepted; run exit 0; ASan-clean; `t` readable afterwards | R4 |

### P5 — Fixed-array conditional arms and projections (`03-05`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-19 | `c ? a() : b();` (fixed-array call results, discarded) and a consuming store, both arms | Accepted; strict build; run per arm; ASan-clean | R5, AR-11 |
| ST-20 | A literal mixing a `str[2]` field with a `String` field, discarded | Accepted; strict build; run exit 0; ASan-clean | R5 |
| ST-21 | `let a: u8[4] = (c ? b1 : b2).xs;` (store and return) and `for (x in mk().xs)` | Accepted; strict build; run exit 0 | R5 |
| ST-22 | Control: the direct spellings (`a();`, `let a = mk().xs;`, the owned-field root) | Accepted; run; byte-identical emission | R5 |

### P6 — Update order completion (`03-06`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-23 | `arr[i] = i++;`, `i = i++;`, `i += i++;` | Accepted; strict build; run values per spec/04 §3 (target location first, then the right-hand side) | R6, AR-9 |
| ST-24 | `P { a: i++, b: i }` and `[i++, i]` | Accepted; run values per source order | R6, AR-10 |
| ST-25 | `(i++).get(i)` and `while (i + id(i++) < 3)` (single-shot and loop) | Accepted; strict build; run per the left-to-right rule | R6, AR-9 |
| ST-26 | `f(g(), i++)` with an owning `g()` result | Accepted; the argument order matches left-to-right; the owning result transfers exactly once (ASan-clean) | R6, AR-9 |
| ST-27 | Controls: the batch-H update cases (`i++ + i`, `f(i++, i)`, the standalone `i++`) | Green (unchanged) | R6 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/conditional_projection_residue_spec_test.rs` | ST-1 – ST-27 | All phases |
| `compiler/tests/conditional_projection_integrity_*_test.rs` (existing) | Controls | Regression oracle |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/conditional_projection_residue_impl_test.rs` | The emitted binding/carrier/store shapes, the report counts and spans, the pin positions, and the byte-identity controls | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own | Every corpus/staged/example program builds under `-Werror` |
| Emission differential | codegen | Base vs head over the corpus; every delta classified (P5/P6) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the P1/P3 cases | `clorraine run` on the case sources (host) | Exit per the ST expectation; ASan-clean under `RDC_ASAN=1` |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- Two-module libraries for the take/mode cases where a foreign function is needed.
- No mocks: the real compiler, the real C toolchain, and the real ASan run.

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
