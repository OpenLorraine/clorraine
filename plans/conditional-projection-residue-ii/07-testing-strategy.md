# Testing Strategy: Conditional, Projection, and Update Residue II

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Ownership/lifetime judgments | Every in-scope row has a red/green reproduction and an ASan run |
| Emission shapes | Every accepted in-scope spelling builds under `-Werror` and runs; the corpus differential classifies the deltas |
| Reports and marks | The exact counts, codes, and spans (or the absence) are asserted; the pinned oracles stay green |
| Controls | The landed partial fixes and the untouched twins keep their verified shapes (AR-3, AR-5) |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the SAME suite pair batch I created**:
  `compiler/tests/conditional_projection_residue_spec_test.rs` (specification tests, from the ST
  cases below) and `compiler/tests/conditional_projection_residue_impl_test.rs` (implementation
  pins) — additions only; the committed batch-I cases stay green as the regression oracle (a
  legitimate emission delta that invalidates a committed pin is re-pinned in the phase with the
  differential evidence recorded, never silently). The machinery is the same,
  the batch-I suite is the regression oracle, and a new pair would duplicate the harness (AR-12).
- The batch-I suite test count at the planning head is the baseline for "additions only"; the
  close-out re-counts.
- Integration tests are the corpus runs on the host and (at close) the Amiga legs; E2E is the
  `clorraine` driver run over the spec cases.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the component specs (`03-01` – `03-05`), the frozen specification
> (Ch 03/04/07/17, spec/14 registry), and this plan's Ambiguity Register. IMMUTABLE ORACLE: do
> not modify an expectation to match the implementation.

### P1 — View owners, loans, and stores (`03-01`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-1 | `dupresidual` (`DEF-281`) | `holdNum(c ? a.n : mkBox().n)` (take), the return twin, and the read positions `(c ? mk() : b).ys[i]` / `mk().ys[i]`, in a loop | Accepted; strict build; run exit 0; ASan-clean (the fresh root's residual releases once; the copied scalar reads back) | Red: ASan 11 B leaked (probe `d281_probe`); the read twins 48 B/4 allocs. Controls: the place-arm selection; the concrete `String`-member twin |
| ST-2 | `runtimestore` (`DEF-285`) | `xs[j] = b.v;` (a local `str[2]`, a run-time index) and the re-executed `while { xs[j] = mk().v; }` | The owner releases exactly once and the element loan is recorded: a writable alias of the backing reports the same judgment as the constant-index store; the loop store rejects where the constant twin does | Red: 11 B stranded per store; the loan skipped (33 B over three iterations). Controls: `xs[1] = b.v;` (the constant twin, accepted); the alias mutation report |
| ST-3 | `crossframe` (`DEF-286`) | `outer(borrow slot) { let b = …; fill(b.v, borrow slot); }` plus the `slot = [p, q]` and `str`-twin spellings; the parameter-backed and same-frame twins | Rejected: one `E10030` report naming the escaping store — the direct twin's code (BR-8); the twins accepted, strict build, run exit 0, ASan-clean. Stop-gated: if the rule cannot avoid over-rejecting the twins, the phase stops and the row is parked with the evidence (AR-6) | Red: accepted + ASan heap-use-after-free (byte-identical base/head). Controls: the parameter-backed out-slot store (accepted) |
| ST-4 | `modulefresh` (`DEF-287`) | A module binding initialized from a fresh call (`let g: str = …`-shaped) | Accepted; strict build; run exit 0; the post-init read-back stays valid; ASan-clean (the view destination retains the fresh owner for the module's storage — the pinned module-initializer rule) | Red: 11 B leaked per program start. Controls: the static/parameter-backed module bindings |
| ST-5 | `containerescape` (`DEF-288`) | `xs[1] = b.v; return xs;` and the alias spelling; `xs[1] = b.v; g = xs;` | Rejected like the single-view twin: one report per site (`E10035` at the return, `E10033` at the global store) | Red: accepted + read freed storage (byte-identical C). Controls: `return s` (one `E10035`); the parameter-backed twins (accepted, ASan-clean) |
| ST-6 | `staleloan` (`DEF-290`) | `let h = Holder { names: [b.v] }; h.names = ["AA"]; b.v = …` and the whole-carrier reassignment | Accepted: the overwritten carrier's loans drop, so the backing write compiles; a live view keeps its loan | Red: false `E10031` at the backing write. Controls: the constant-index element twin (loan replaced); the live-view mutation still rejected |
| ST-7 | `allnull` (`DEF-291`) | `slot = c ? null : null;` into a `borrow str?` out-slot, plus the global twin | Accepted (an empty storage-atom set names no storage); strict build; the null view semantics hold | Red: false `E10030` at the store (probe `d291_shape`); the global twin `E10033`. Controls: a single `null`; the null-arm mixes (unchanged) |

### P2 — Guarded, discarded, and carrier roots (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-8 | `guardcallroot` (`DEF-284`) | `g ? use(Holder { names: [id(mk().v)] }) : 0` with `g` true and false, in a loop | Accepted; run exit 0 (both paths); the call runs once on the taken path and never on the skipped; ASan-clean | Red: 11 B leaked and the skipped call ran (base-equal). Controls: the direct (unguarded) element position |
| ST-9 | `carrierliteral` (`DEF-292`) | `let h: Holder = g ? Holder { names: [mk().v] } : Holder { names: ["zz"] };` both selections | Accepted; run exit 0; ASan-clean (the taken arm's fresh element root binds for the local's frame; the read-back is correct) | Red: 11–22 B leaked. Controls: the loop-condition and guarded literal spellings (green) |
| ST-10 | `discardresidual` (`DEF-293`) | `c ? i32(mk().v.len()) : t;` over a fresh root with a residual sibling field (`struct Box { v: String; w: String; }` — the residual is what leaks; the sole-field twin is clean at the planning head) | Accepted; run exit 0; ASan-clean (the root's residual releases; the place arm stays readable) | Red: 11 B leaked (with the residual sibling field). Controls: the discarded conditional with an owned selected type (green) |
| ST-11 | `barediscard` (`DEF-297`) | `[mk().v];` as a bare statement | Accepted; run exit 0; ASan-clean | Red: the strict build rejects the emitted discard statement (measured at the planning head: `(void)({ … });` — `expected ';'`); the leak shape is re-derived at the red run. Controls: the conditional discard spelling (green); a place leaf stays a read |

### P3 — Fixed-array and literal emissions (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-12 | `condfield` (`DEF-298`) | `Holder { row: c ? a.row : mkBox().row }` guarded and plain, both selections | Accepted; strict build; run exit 0 (the per-arm field fill; the values correct) | Red: `int-conversion`/`missing-braces` strict failure (base/head). Controls: the direct literal field |
| ST-13 | `condrootfield` (`DEF-299`) | `Holder { row: (c ? a : b).row }` | Accepted; strict build; run exit 0 (the field's read-back proves the copied rows) | Red: a strict-build failure (the filed projection emits `lvalue required as unary '&' operand`; the DEF-298 twin shows `int-conversion`/`missing-braces`). Controls: `DEF-275`'s store/return spellings (pinned green) |
| ST-14 | `guardviewfield` (`DEF-300`) | `g && len(Holder { names: [mk().v] }.names[0]) > 0` | Accepted; strict build (no `missing-field-initializers`); run exit 0 | Red: strict failure (base/head). Controls: the unguarded field spelling |
| ST-15 | `condcallliteral` (`DEF-302`) | `ok ? Mixed { names: n1(), tag: … } : Mixed { names: n2(), tag: … };` | Accepted; strict build; run exit 0; each arm's call runs once on its path | Red: `too few arguments`. Controls: the single-literal call-filled field (fixed in batch I) |
| ST-16 | `condrowreturn` (`DEF-303`, critical) | The `String[2]` conditional-root return `(c ? b1 : b2).xs` by value, both selections | Accepted; strict build; run exit 0 (the read-back proves the copied rows); ASan-clean (per-arm copies into the hidden destination) | Red: run segfault 139 (probe `d303b_shape`; byte-identical emission). Controls: the `String[2]` store twin (correct); the direct return |
| ST-17 | `guardfallback` (`DEF-304`) | `g && consume((c ? b1 : b2).names)` and `g && consume(c ? mk1() : mk2())` | Accepted; strict build; run exit 0 (the consumed value reads back; the skipped path runs nothing) | Red: `assignment to expression with array type` / `too few arguments`. Controls: the unguarded consuming store (fixed in batch I) |
| ST-18 | `arrayelement` (`DEF-305`) | `Outer { items: [Inner { row: n1() }, …] };` | Accepted; strict build; run exit 0 (the element's field reads back) | Red: `too few arguments`. Controls: the direct struct field |
| ST-19 | `nestedread` (`DEF-306`, residual) | `(c ? (d ? b1 : b2).ys : b3.ys)[i]` | Accepted; strict build; run exit 0 with the selected arm's values | Red: `lvalue required as unary '&' operand` (probe `d306_read`, container and host). Controls: the store/iterable/return distribution twins (probe-verified green) |

### P4 — Loop conditions, update order, and store lowering (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-20 | `loopreceiver` (`DEF-294`, AR-10) | `while (i < 1 && (c ? mkH() : a).call(5) > 0) { … }`, both selections | Accepted; run exit 0 with the arm values; the emission pin carries the receiver settlement; the strict-build legs are recorded per AR-10 (the pinned container must accept; the host result is measured — the suite's single `cc` asserts its own environment's leg) | Red: host GCC 15 `-Werror=sequence-point`; the pinned container accepts (measured) — the phase records the outcome (fixed or control, AR-10). Controls: the direct receiver; the plain loop condition |
| ST-21 | `loopmember` (`DEF-295`) | `while (i < k && i32(len(mkH().s)) > 0) { … }` single-shot and re-executed | Accepted; strict build; run exit 0; ASan-clean (one release per iteration/exit) | Red: run abort 134; ASan double-free (probe `d295b_shape`). Controls: the store/argument twins of the projection |
| ST-22 | `guardupdate` (`DEF-308`, residual) | `g && f(i++, i)`, `g \|\| f(i++, i) > 0`, and `c ? f(i++, i) : 0` | Accepted; strict build on both toolchains; run exit 0 with the left-to-right values; the skipped path runs nothing | Red: `-Werror=sequence-point` (container and host; probe `d308_guard`/`d308_ternary`). Controls: the plain loop condition `while (f(i++, i) < 30)` (probe-verified green) |
| ST-23 | `arrayfill` (`DEF-309`) | `let xs: i32[3] = [; i++];` with an observing counter | Accepted; strict build; run exit 0 with the fill evaluated once; every element holds the fill's value | Red: the fill runs per element. Controls: the fill-free literal; the element-list literal |
| ST-24 | `rawderefstore` (`DEF-310`) | `unsafe { *q = 9; }` and `unsafe { *q = i++; }` with read-back | Accepted; strict build; run exit 0 (the write lands; the read-back is 9) | Red: run exit 254 — the store emitted nothing (probe `d310_major`). Controls: the field write through a raw (works); the deref read |

### P5 — Diagnostics and marks (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-25 | `onereportnested` (`DEF-282`) | `return (c ? a : (d ? mkBox() : mkBox2())).v;`; the `gfieldcall` spelling; the mixed `Pair` array | Rejected with exactly one report at the span (the `E10030` pair collapses); the cross-site pair withholds the consequence report where the callee judgment stands; the counts are pinned | Red: two identical `E10030` at 12:5 (probe `d282_shape`); the extra store report. Controls: `callable_release_integrity`'s exactly-one `E10030` pin; the batch-I one-report controls |
| ST-26 | `deferphrase` (`DEF-283`) | The deferred mixed view return `defer { … return (c ? l : mkBox()).v; }` | Rejected with one report rendering `a compiler-generated owner` (never the internal name) | Red: `borrow of '__cl_owned1'` (probe `d283_shape`). Controls: the `checker_conformance` phrase test; the direct mixed return |
| ST-27 | `iterableark` (`DEF-307`) | `for (borrow x in (c ? b1 : b2))` with owned (`String[2]`) elements; a later use of each arm | Accepted; strict build; run exit 0; ASan-clean (the arms stay usable; the emission reads in place) | Red: `E10020: 'b1' was moved` (probe `d307b_shape`). Controls: the `u8[4]` twin (accepted); the pinned `fixediterowned` behavior |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/conditional_projection_residue_spec_test.rs` (append) | ST-1 – ST-27 | All phases |
| `compiler/tests/conditional_projection_residue_*_test.rs` (existing) | Regression oracle | Batch I |
| `compiler/tests/callable_release_integrity_spec_test.rs` | The one-report pin (`DEF-282`) | Oracle |
| `compiler/tests/checker_conformance_spec_test.rs` | The generated-owner phrase (`DEF-283`) | Oracle |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| `compiler/tests/conditional_projection_residue_impl_test.rs` (append) | The binding/loan shapes, the per-arm destination and copy shapes, the pin positions, the report counts and spans, and the re-emission determinism pins | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen | Base vs head over the corpus; every delta classified (P3/P4) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the P1/P3 cases | `clorraine run` on the case sources (host) | Exit per the ST expectation |
| Ownership-phase suite runs | The residue suites under `RDC_ASAN=1` (the suite helper) | ASan-clean |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- Two-module libraries for the cross-frame (`DEF-286`) and take/mode cases where a foreign
  function is needed.
- No mocks: the real compiler, the real C toolchain, and the real ASan run.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase; the partial-fix
      controls verified green first — AR-3)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests (the batch-I and batch-H suites, the pinned oracles)
- [x] Test coverage meets goals
