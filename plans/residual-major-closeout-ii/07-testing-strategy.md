# Testing Strategy: Residual Major Closeout II (Batch L)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Nullable-view arms | Every widening position builds under `-Werror`, runs the null selection without a dereference, and is ASan-clean; the non-null twins are byte-identical controls |
| Generic walk | The keyed instance symbols are pinned by emitted-C assertions; every declined projection resolves; the controls stay green |
| Instance surfaces | The substituted element's drop and judgments are pinned per instance; the duplicable out-slot reads back; the capture pair stays rejected |
| Ownership | The loan/move matrix runs ASan/LSan-clean (loop and early-exit variants); the conflict reports assert code, count, and span |
| Store emission | The conditional-base store rejection is pinned (one `E10221`, no TU); the guard-literal cases read the guard values and count the side effects; the skipped path runs nothing |
| Conversions/`ffi` | The rejections assert the exact code, count, and span; the legal instances and the `DEF-350` control spellings build and run |
| Member typing | The rejection spans and the emitted casts are pinned per destination; the controls byte-identical |
| Identity/folds | The emitted symbols and the deferred extent spelling are pinned; the `k18` matrix and the value folds stay green |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the ORIGIN suites, additions only** (AR-9):
  - P1 and `DEF-357` → `compiler/tests/module_initializer_integrity_{spec,impl}_test.rs`;
  - P2, P3, P6, P7 → `compiler/tests/checker_conformance_{spec,impl}_test.rs`;
  - P4, P5 → `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs`;
  - `DEF-356`/`DEF-359` → `compiler/tests/callable_release_integrity_{spec,impl}_test.rs`.
  A new `compiler/tests/residual_major_closeout_ii_{spec,impl}_test.rs` pair is created only if a
  phase cannot host a case naturally, and then it carries only that case (recorded in the
  register).
- The committed origin cases stay green as the regression oracle; a legitimate emission delta
  that invalidates a committed pin is re-pinned in the phase with the differential evidence
  recorded, never silently.
- Integration tests are the corpus runs on the host and (at close) the Amiga legs; E2E is the
  `clorraine` driver run over the spec cases.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the component specs (`03-01` – `03-08`), the frozen specification
> (Ch 02/03/04/06/10/12/16/17, spec/14 registry), and this plan's Ambiguity Register. IMMUTABLE
> ORACLE: do not modify an expectation to match the implementation.

### P1 — Nullable-view arms and initializer view stores (`03-01`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-1 | `nullguardcall` (`DEF-335`) | `let s: str? = c ? maybe() : stringOf("xy");` (call arm) and `let s: str? = c ? p : null;` (local and parameter place arms) with the null selection | Accepted; strict build; the null selection returns the null branch's value (`s == null`, the read folded); ASan-clean — no `cl_string_view(NULL)` | Red: call arm run 139 + ASan (`k1d10`), place arms run 139 (`def335f`/`def335g`). Controls: `k1d9` run 2, `k1m1` run 0, `k1d17` run 0 |
| ST-2 | `nullguardarg` (`DEF-336`) | `size(false, false ? stringOf("abc") : null)` (the null selection) | Accepted; strict build; the argument carries the guard; run 0; ASan-clean | Red: run 139 + ASan (`k1d15`). Control: the non-null conditional |
| ST-3 | `nullguardmember` (`DEF-337`) | `let t: str? = mkBox(false).name;` at module level (the nullable-member projection) | Accepted; strict build; run 0 (`t == null`); ASan-clean; the member move-out/release keeps its order | Red: run 139 + ASan (`k1e6`). Controls: the body twin `k1j1` run 0 and the non-null member |

### P2 — The generic walk's projected-argument resolver (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-4 | `walkdotreceiver` (`DEF-338`) | `let g: i32 = b.pick(7);` (a local-headed qualified receiver in an initializer dot call) | Accepted; strict build; the keyed instance emits; run proves the value | Red: strict build fails (`cl_pick__Box` type error; `d03_init_method_wrongkey2`). Control: the direct dot form |
| ST-5 | `walktemprooted` (`DEF-340`) | `keep(v.name)` inside `each<T>`'s borrow loop | Accepted; strict build; the keyed instance emits; run proves the value | Red: `implicit declaration of 'cl_keep__String'` (`b8`). Control: `id(v[i].y)` (`p2f`, run 7) |
| ST-6 | `walkconditional` (`DEF-342`) | `id(c ? a.y : b.y)`; the nested double-placeholder loop and the nested call result | Accepted; strict build; the keyed instance emits; the run proves the selected arm's value | Red: `implicit declaration of 'cl_id__i32'` (`p2b`). Controls: `id(s.y)` through a local (run 3) and `id(o.s.y)` (run 1) |

### P3 — Instance substitution and element surfaces (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-7 | `elementstoredrop` (`DEF-339`) | `for (borrow v in xs) { v.name = stringOf("zzzz"); }` inside `each<T>` (loop and early-exit spellings) | Accepted; strict build; the overwritten owner releases once per store; run reads the sizes back; ASan/LSan-clean | Red: LeakSanitizer 11 B/store (`s2`). Control: the concrete twin (clean) |
| ST-8 | `elementconversion` (`DEF-345`) | `e04…e12` (a place/conditional/direct element return, an element `let`, an element argument, the loop element returns/lets) | Each spelling matches its concrete twin: accepted and building where the twin accepts; the frozen `E10200`/`E10212` where it rejects | Red: all seven accepted + strict build fails. Controls: the concrete twins (`dn4`/`dn5`, `d05c`/`d06`) |
| ST-9 | `duplicableoutslot` (`DEF-353`) | `wrap<i32>(borrow out, 7)` with `([borrow x, borrow out] () => { out = x; })()` | Accepted; strict build; run reads the stored value back (exit 0) | Red: run 249 (`out` stays 0; `p5_capture_dup_instance`). Control: the owning/`String?` instance |
| ST-10 | `capturepair` (`DEF-341`) | `let s: String = ([v] () => v.name)();` inside `each<T>`; the concrete twin | **Green control**: both spellings rejected with one `E10401` each (no TU); the pair pins the concrete parity | Verified green at the head (the batch-K C3 correction); the case fails if a fix regresses it (AR-8) |

### P4 — Ownership transfer judgments (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-11 | `viewcopyloans` (`DEF-315`) | `let ys: str[2] = xs;` (and `ys = xs;`) with a later `fill(ys[0], borrow slot)`; local/parameter sources, the loop and overwrite variants | The copy transfers (or judges) its element loans: the later element store is accepted with the loans intact or reports the frozen conflict; ASan-clean | Red: ASan heap-use-after-free (`hp_alias_copy`). Controls: `hp_alias_return`/`hp_alias_param` and the single-view loan twins |
| ST-12 | `rawreadowner` (`DEF-329`) | `let v: String = unsafe *q;`; the discarded read and a duplicable pointee | The owned read moves (empties the pointee) or reports the frozen move rule; ASan-clean; the discarded read stays clean | Red: ASan attempting double-free (`rawreadown`). Controls: the discarded read and `rawmove_use`/`rawdistinct_read` |

### P5 — Store emission: conditional bases and guarded view literals (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-13 | `condbasestore` (`DEF-322`) | `(c ? b1 : b2).ys[i] = 7;` both selections, and the RHS side-effect count | Rejected once with the frozen `E10221` (the scalar member twin's place rule; no TU; the RHS never runs) | Red: accepted, no store, run 251 (`def322_probe`). Controls: the read spelling (accepted, runs) and the scalar `E10221` twin |
| ST-14 | `guardviewliteral` (`DEF-323`) | `g && consume(ok ? Mixed { names: n1(), … } : Mixed { names: n2(), … })` with the `take` and plain parameter spellings | Both build strictly; only the taken arm's call runs (the counters prove it); the run values match; ASan-clean | Red: `take` strict build fails (`too few arguments`); plain run 10 (calls = 3). Controls: the direct-literal take (`ctl323`, run 0) and the scalar/owned twins |

### P6 — Call conversions and `ffi` typing (`03-06`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-15 | `inferrednullarg` (`DEF-344`) | `pair(1, x)` with `x = null`; the `pair(x, 1)`/`pair(x, x)` count controls; the explicit twin | The sibling-fixed null argument reports the frozen `E10012` once per the settled rule; the explicit twin keeps one report | Red: accepted + strict build fails (`dn1`). Controls: `dn2` one `E10012`, `dn3` two (the phase records the count ruling), the explicit twin |
| ST-16 | `instanceconversions` (`DEF-351`) | `pick<cstring>()`'s `return 5`; `consume2<cstring>(x)` at `T=str`; `sink<U>(a)` at `U=cstring`; `bs01`; the legal instances | Each offending instance reports once with the frozen `E10200`; the legal instances (`cstring`/`str`/`String`, identities) stay accepted and building | Red: all four accepted + strict build fails (`reverse1`/`gencallee`/`genmixed`/`bs01`). Controls: `genlit`'s `E10200` and the legal instances |
| ST-17 | `ffiresultcontrol` (`DEF-350`) | The `ffi` view/slice result spellings: `let s: str = Make()`, the argument, `len(Make())`, `.ptr`, the index, the inferred local, the template, the cstring return | **Green control**: the view/slice destinations report one `E10200`; `len(Make())`/`Make()[0]`/the cstring return build (compile; run with the recorded stubs); the `.ptr`/`.len` residuals are recorded as `DEF-352`'s scope | Verified green in core at the head (the batch-K F1 correction); the case pins the per-spelling table |

### P7 — Member typing on concrete paths (`03-07`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-18 | `unknownviewmembers` (`DEF-346`) | `let p = a.ptr;` on `u8[4]`; `let x = s.foo;` on `str` | Rejected once with the frozen `E10212` on the concrete path (the instance audit mirrors it) | Red: accepted, strict build fails (`def346a`/`def346b`). Controls: the typed `.ptr`/`.len` members and the scoped forms |
| ST-19 | `constptrmutdest` (`DEF-347`) | `let p: raw u8 = s.ptr;`, a `raw u8` return, `Holder { c: s.ptr }` | Accepted; strict build; the value reads back (the member already types `raw u8`, so only the C-side qualifier cast is added; the AR-7 stop-gate stays a phase decision, not a case outcome) | Red: accepted, strict build fails (`discarded-qualifiers`; `def347`). Control: `ctl347`'s `raw u8(s.ptr)` (run 0) |
| ST-20 | `rawenummember` (`DEF-348`) | `i32(p.A)` on a `raw E` | The member resolves through the enum rule (builds and runs) or reports the frozen `E10212`; the concrete and instance paths agree | Red: accepted, strict build fails (`def348`). Controls: the scoped `E.A` and the qualified `A.B.f` |

### P8 — Module/global identity and the const folder (`03-08`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-21 | `funcglobalresiduals` (`DEF-356`) | `import { g as bg } from holder; bg(5)`; the cross-module qualified `q.holder.g(5)`; the self-qualified and plain spellings | The aliased and qualified spellings build and run the holder's symbol; the self-qualified, plain, and qualified func-global controls stay byte-identical | Red: `cl_def356holder_bg` undeclared (`alias_fn`); `q.holder.g(5)` rejected `E10219`. Controls: the self-qualified (`selfq4`, run 0), the plain import (`plainfield`, run 0), the `k18` matrix |
| ST-22 | `constfoldabi` (`DEF-357`) | `const n: usize = sizeof(S); let pad: u8[n] = [; 0];` and the `str` twin; the value-position twins | The extent position honors the host layout: `len(pad)` reads the C size (8/16 on host), the run exits 0; the value folds stay green | Red: extent run 253/248 (`extent`/`extent_view`). Controls: `structpad`/`view` run 0 |
| ST-23 | `sconetyping` (`DEF-359`) | `f("x")` against an imported `func(i32)` global while a same-named `func(str)` exists elsewhere; the matching call and the qualified spelling | The mismatched call reports the frozen conversion once; the matching and qualified calls build and run | Red: accepted, strict build fails (`k18/sig/main2`). Controls: `main.clo` (`f(5)`) and the qualified spelling |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/module_initializer_integrity_{spec,impl}_test.rs` (append) | ST-1 – ST-3, ST-22 | P1, `DEF-357` |
| `compiler/tests/checker_conformance_{spec,impl}_test.rs` (append) | ST-4 – ST-9, ST-15 – ST-20 | P2, P3, P6, P7 |
| `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (append) | ST-10 – ST-14 | P3 (control), P4, P5 |
| `compiler/tests/callable_release_integrity_{spec,impl}_test.rs` (append) | ST-21, ST-23 | P8 |
| The origin suites (existing cases) | Regression oracle | Batches I/J/K and the close-outs |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| The origin `_impl_test.rs` files (append) | The arm guard spellings, the argument/initializer guards, the keyed instance symbols, the substituted drop and duty records, the loan/move shapes, the per-arm store and guard-literal fills, the conversion reports, the member rejection spans and casts, the alias/extent/checker shapes, and the re-emission determinism pins | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own + sema | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen + own | Base vs head over the corpus; every delta classified per phase |
| Memory suite runs | the suite `RDC_ASAN=1` switch | The P1/P3/P4/P5 cases ASan/LSan-clean |
| Profile legs | driver + toolchains | The freestanding and hosted-amiga legs stay green |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the phase cases | `clorraine run` on the case sources (host) | Exit per the ST expectation |
| The `ffi` compile-only cases | The strict build with the recorded stubs | The filed evidence (the strict build) passes |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- The multi-file fixtures for `DEF-356`/`DEF-359` (holder/other modules; the file list is the
  program).
- The `ffi` stubs for the `DEF-350` control spellings where a run is asserted (the recorded
  `*.stub.c` files), or compile-only evidence where none exists.
- The real compiler, C toolchain, ASan, the m68k toolchain, and the driver — no mocks.

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase; the controls
      verified green first — AR-3, AR-5)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests (the origin suites, the batch-I/J/K suites, the pinned
      oracles)
- [x] Test coverage meets goals
