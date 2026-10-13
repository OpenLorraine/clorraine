# Testing Strategy: Conditional, Projection, and Update Residue

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Ownership/lifetime judgments | Every in-scope row has a red/green reproduction and an ASan run |
| Emission shapes | Every accepted in-scope spelling builds under `-Werror` and runs |
| Controls | The untouched twin spellings keep their pinned emission shapes (the re-emission determinism pins in the impl suite; base-vs-head identity is the emission-differential integration row) |

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
| ST-3 | **Green control (incidental closure):** `let s: String = (c ? a : Box { v: …, w: … }).v;` both arms | Accepted; run exit 0; ASan-clean — green at the planning baseline | R1, AR-23 |
| ST-4 | `if (g && consume((c ? a : mkBox()).v) > 0) { … }` with `g` true and false, and the `\|\|` and fresh-arm variants, in a loop | Accepted; run exit 0 (every path); ASan-clean (one release per execution; no double-free) | R1, AR-4 |
| ST-5 | `H { call: addOne, s: mkBox().v }.call(5)` in an `if` and in a loop condition | Accepted; run exit 0; ASan-clean (the projection root's sibling releases per execution) | R1, AR-4 |
| ST-6 | Control: the concrete twins of ST-1..ST-5 (`String` spelled out) | Accepted; run exit 0; ASan-clean; the re-emission determinism pins hold | R1 |

### P2 — View derivation and owners (`03-02`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-7 | `bytes((c ? b : d).v)` with `borrow` parameters (both selections); the view used beside a writable alias of the selected arm's backing | Rejected with the plain twin's judgment (`E10031` at the mutation; one report; the head accepts and reads freed storage) | R2, AR-19 |
| ST-8 | `use([mk().v])` (an argument) and `Holder { names: [mk().v] }` (a field) | Accepted; run exit 0; ASan-clean (no leak) | R2 |
| ST-9 | `func f(s: String): str[1] { return [s]; }` and `return [h.v]` | Accepted (the single-view twin `return s` is accepted); the `[h.v]` spelling reports its twin's exact judgment (no false `E10034`, no extra `E10036`) | R2 |
| ST-10 | *(parked — AR-2, AR-7)* `func pick(x: str, h: Box, c: bool): str { return c ? x : h.v; }` | Not authored: the rejection requires re-pinning an immutable suite pin (the conditional twin is pinned accepted); current behavior unchanged pending a ruling | R2, AR-7, AR-2 |
| ST-11 | *(parked — AR-2, AR-8)* `return (c ? a : mkBox()).v;` (a local-backed arm beside a generated-owner arm) | Not authored: the oracle pins exactly one `E10030` for a shape with a local arm; the dedupe needs a ruling | R2, AR-8, AR-2 |
| ST-28 | The chained view-returning callee: `let s: str = peek((c ? b : d).v);` through a `peek(x: str): str` callee; the view used beside a writable alias of the selected arm's backing | Rejected like the direct twin (`E10031` at the mutation; one report; the head accepts and reads freed storage) | R2, AR-19 |
| ST-29 | The two-arm owner walk: `bytes(c ? b.v : d.v)`; the view used beside a writable alias of the **second** arm's backing | Rejected like the first-arm twin (`E10031` at the mutation; one report; the head records only the first arm) | R2, AR-19 |

### P3 — Escaping stores (`03-03`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-12 | `names = mk().v;` and `names = [mk().v]` (a global `str` / `str[1]`), plus the parameter-backed control `names = p.v` | Rejected: exactly one `E10033` at each store; the parameter-backed control rejects the same way (the head's behavior) | R3, AR-5, AR-20 |
| ST-13 | `fill(borrow slot: str) { slot = mk().v; }` and the parameter-supported twin | The fresh-derived store rejected (exactly one `E10030` at the store); the parameter-supported twin accepted; run exit 0; ASan-clean | R3, AR-5, AR-20 |
| ST-14 | `g.names = [mk().v]` (a global struct field; a fresh element) and `slot = [mk().v]` (an out-slot; a fresh element) | Rejected: one `E10033` at the global store and one `E10030` at the out-slot store (the fresh element is an escaping store, so no emission is reached) | R3, AR-20 |
| ST-15 | `xs[1] = c ? b.v : a;` and `xs[1] = v;` (a local `str[2]`) | Accepted; run exit 0; ASan-clean; the global/field spellings still rejected (`E10033`) | R3, AR-6, AR-21 |
| ST-30 | The widened stores from a **local** backing (`b` a local `Box`): `names = b.v;` (a global `str?`) and `slot = b.v;` (a `borrow str?` out-slot), plus the parameter-backed out-slot control `slot = p.v` | Rejected: one `E10033` at the global store and one `E10030` at the out-slot store; the parameter-backed control accepted, run exit 0, ASan-clean | R3, AR-20, AR-21 |
| ST-31 | The loop and nested-scope element stores: `while (i < 3) { xs[1] = mk().v; }` and a nested block's backing stored into a local array | Rejected: one `E10030` per site (the backing's scope does not cover the array's) | R3, AR-21 |
| ST-32 | The accepted element store: `g.names = [s]` with a module-level static-backed `s` (and the parameter-backed out-slot twin `slot = [p]`) | Accepted; strict build; run exit 0; ASan-clean (the element-wise destination store; no initializer-list C) | R3, AR-20 |
| ST-33 | A lambda capturing a `str[1]` (the capture conversion) | Accepted; strict build; run exit 0; ASan-clean (the environment field uses the local binding's C shape) | R3 |

### P4 — Guarded and discarded literal positions (`03-04`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-16 | `while (i < 1 && (c ? H { … } : H { … }).call(5) > 0)` single-shot and loop; the array-literal element variant | Accepted; run exit 0; ASan-clean (no double-free) | R4, AR-12 |
| ST-17 | `ok ? Pair { a: stringOf("x"), b: t } : Pair { a: stringOf("y"), b: u };` | Accepted; run exit 0; ASan-clean; the place fields are reads — `t`/`u` usable afterwards (the direct `ok ? t : u;` control stays green) | R4, AR-22 |
| ST-18 | `let t: String = stringOf("t"); c ? mk().v : t;` | Accepted; run exit 0; ASan-clean; `t` readable afterwards | R4 |
| ST-34 | The `?:` and `\|\|` literal-receiver positions: `(g ? H { … } : H { … }).call(5)` and `g \|\| (c ? H { … } : H { … }).call(5) > 0` | Accepted; run exit 0 (the guarded path per selection); ASan-clean (one owner per field in every position) | R4, AR-12 |

### P5 — Fixed-array conditional arms and projections (`03-05`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-19 | `c ? a() : b();` (fixed-array call results, discarded) and a consuming store, both arms | Accepted; strict build; run exit 0 (both selections); ASan-clean | R5, AR-11 |
| ST-20 | A literal mixing a `str[2]` field with a `String` field, discarded | Accepted; strict build; run exit 0; ASan-clean | R5, AR-11 |
| ST-21 | `let a: u8[4] = (c ? b1 : b2).xs;` (store and return) and `for (x in mk().xs)` | Accepted; strict build; run exit 0 | R5, AR-11 |
| ST-22 | Control: the direct spellings (`a();`, `let a = mk().xs;`, the owned-field root) | Accepted; run exit 0; the re-emission determinism pins hold | R5 |
| ST-35 | The move-only conditional root: `let a: u8[4] = (c ? b1 : b2).xs;` with move-only rows | Accepted; strict build; run exit 0 (the per-arm address form on each path; the head emits `lvalue required`) | R5, AR-11 |

### P6 — Update order completion (`03-06`)

| # | Input / Scenario | Expected Output / Behavior | Source |
|---|------------------|----------------------------|--------|
| ST-23 | `arr[i] = i++;`, `i = i++;`, `i += i++;` | Accepted; the suite's strict flags pass; run exit 0 with the spec/04 §3 values (target location first, then the right-hand side; the update completes before the store); the emission pins carry the red (AR-24) | R6, AR-9, AR-24 |
| ST-24 | `P { a: i++, b: i }` and `[i++, i]` | Accepted; run exit 0 with the source-order values; the host orders initializer lists left-to-right pre-fix, so the red is the emission pin (AR-24) | R6, AR-10, AR-24 |
| ST-25 | `(i++).get(i)` and `while (i + id(i++) < 3)` (single-shot and loop) | Accepted; the suite's strict flags pass; run exit 0 per the left-to-right rule; the emission pins carry the red (AR-24) | R6, AR-9, AR-24 |
| ST-26 | `f(g(), i++)` with an owning `g()` result whose `g` observes the update (it encodes whether `i` already changed) | Accepted; run exit 0 — the argument order is left-to-right (the pre-fix right-to-left order fails the observation); the owning result transfers exactly once (ASan-clean) | R6, AR-9, AR-24 |
| ST-27 | Controls: the batch-H update cases (`i++ + i`, `f(i++, i)`, the standalone `i++`) | Green (unchanged) | R6 |

> **P6 red-phase note (AR-24):** the pinned container compilers reject the sequence shapes under
> the suite's own strict flags (`-Werror=sequence-point` / `-Werror,-Wunsequenced`, selected by
> `-Wall -Wextra -Werror`), so ST-23/ST-25 carry strict-build reds; the container also orders
> initializer lists left-to-right, so ST-24's red is the missing site/emission pins, and ST-26's
> order-observing case is its own red. (A host GCC 15 outside the container accepts the sequence
> shapes — record the container result in the red phase.)

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/conditional_projection_residue_spec_test.rs` | ST-1 – ST-35 (ST-3 green control; ST-10/ST-11 parked, not authored) | All phases |
| `compiler/tests/conditional_projection_integrity_*_test.rs` (existing) | Controls | Regression oracle |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/conditional_projection_residue_impl_test.rs` | The emitted binding/carrier/store shapes, the report counts and spans, the pin positions, and the re-emission determinism pins | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own | Every corpus/staged/example program builds under `-Werror` |
| Emission differential | codegen | Base vs head over the corpus; every delta classified (P3/P5/P6) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the P1/P3 cases | `clorraine run` on the case sources (host) | Exit per the ST expectation (behavior only; the driver has no sanitizer flag) |
| Ownership-phase suite runs | The new and batch-H ownership suites under `RDC_ASAN=1` (the suite helper) | ASan-clean (memory only; the switch is a test-suite switch) |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- Two-module libraries for the take/mode cases where a foreign function is needed.
- No mocks: the real compiler, the real C toolchain, and the real ASan run.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase; P6 per AR-24)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests
- [ ] Test coverage meets goals
