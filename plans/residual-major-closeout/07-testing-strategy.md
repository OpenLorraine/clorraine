# Testing Strategy: Residual Major Closeout

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Initializer emission | Every in-scope accepted spelling builds under `-Werror`, runs, and reads back the correct value; the memory shapes are ASan-clean |
| Generic element surface | The element type, instance key, environment field, and bounds judgment are pinned by emitted-C assertions and run values |
| Member and call typing | The rejections assert the exact code, count, and span; the accepted controls build and run |
| Destinations | The null-shaped and capture-store cases build/run (or reject once) on every profile the row names |
| Standalone emissions | The callee symbol, the shim alignment, the `sizeof` value, and the void-lambda abort are pinned per site; the controls byte-identical |
| Controls | `DEF-209`'s fixed shapes and every probe-verified green twin stay green (AR-5) |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the ORIGIN suites, additions only** (AR-9): the initializer rows
  (including `DEF-211`) into `compiler/tests/module_initializer_integrity_{spec,impl}_test.rs`;
  the element, member, conversion, and destination rows into
  `compiler/tests/checker_conformance_{spec,impl}_test.rs`; `DEF-170` into
  `compiler/tests/callable_release_integrity_{spec,impl}_test.rs`; `DEF-189` into
  `compiler/tests/freestanding_spec_test.rs`; `DEF-255` into
  `compiler/tests/def_backlog_closeout_{spec,impl}_test.rs`. A new
  `compiler/tests/residual_major_closeout_{spec,impl}_test.rs` pair is created only if a phase
  cannot host a case naturally, and then it carries only that case (recorded in the register).
- The committed origin cases stay green as the regression oracle; a legitimate emission delta
  that invalidates a committed pin is re-pinned in the phase with the differential evidence
  recorded, never silently.
- Integration tests are the corpus runs on the host and (at close) the Amiga legs; E2E is the
  `clorraine` driver run over the spec cases (the hosted-amiga `File?` leg for `DEF-245`).

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the component specs (`03-01` – `03-06`), the frozen specification
> (Ch 03/04/06/07/09/11/12/17, spec/14 registry), and this plan's Ambiguity Register. IMMUTABLE
> ORACLE: do not modify an expectation to match the implementation.

### P1 — Initializer call resolution, guards, and view conditionals (`03-01`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-1 | `initcallexpr` (`DEF-196`) | `let h: i32 = mk()(5);` and the conditional-callee twin `(c ? addOne : addOne)(5)` in a module initializer | Accepted; strict build; `h` is the call's result (6), run exit 0 | Red: emitted `h = (5);`, run exit 255 (probe `def196`/`def196c`). Controls: the explicit func-value call `f(5)` (works) |
| ST-2 | `initgenericinfer` (`DEF-197`) | `let g: i32 = pick(1, 2);` with a generic `pick<T>` | Accepted; strict build; the instance declares and the keyed callee emits; the run reads `g` back (the call's result) and exits 0 | Red: implicit `cl_..._pick` declaration (probe `def197`). Control: `pick<i32>(1, 2)` explicit (works) |
| ST-3 | `initguardrelease` (`DEF-219`) | `let hit: bool = yes && (firstByte(stringOf("ab")) == 97);` | Accepted; strict build (no undeclared `__cl_itmp1`); run proves `hit`; ASan-clean | Red: `'__cl_itmp1' undeclared` (probe `def219`). Control: the unguarded owned bind |
| ST-4 | `initviewinfer` (`DEF-220`) | `let e: i32 = viewOf(slice(xs, 0, 4))[1];` | Accepted; strict build; the inferred `viewOf` instance declares; the run reads `e` back (2) and exits 0 | Red: implicit `cl_..._viewOf` declaration + invalid initializer (probe `def220`). Control: the explicit-type-argument call |
| ST-5 | `nullarmview` (`DEF-207`) | `let s: str? = c ? stringOf("abc") : null;` with the null test and the length read, both selections, body and module level | Accepted; strict build; run 9 on the null arm and 3 on the taken arm; ASan-clean (each arm builds its own view) | Red: run segfault 139 at `cl_string_ptr(NULL)` (probe `def207`/`def207b`). Controls: the non-null conditional view |
| ST-6 | `memberprojcontrol` (`DEF-209`) | `mkBox().count` in the let, return, argument, and module-initializer spellings | **Green control**: accepted; strict build; the run reads the projection back and exits 0; ASan-clean (the root binds and releases its owned member) | Verified green at the head (probe `def209`/`def209b`/`def209c`/`def209d`/`def209e`); the case fails if a P1 change regresses the binding |

### P2 — Element placeholder, inference, and bounds (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-7 | `elementmembercall` (`DEF-229`) | `id(v.y)` inside `each<S[2]>` | Accepted; strict build; the keyed instance emits; run proves the summed values | Red: `cl_id__unknown` implicit declaration (probe `def229`). Control: the direct read `i32(v.y)` (probe `def229ctl`, green) |
| ST-8 | `elementcapture` (`DEF-230`) | `[v] () => v.y` inside `each<S[2]>` | Accepted; strict build; the environment field carries the element struct; run proves the values | Red: the env field `i32 v` for a `ClS` element (probe `def230`). Control: the concrete non-generic capture |
| ST-9 | `elementinfer` (`DEF-231`) | `inner(v)` inside `outer<i32[2]>`; `each(x)` with `x = null` | The placeholder case: accepted, strict build, and the run reads the call's result back. The null-shape case: rejected once with the frozen null-shaped argument code (`E10012`) | Red: `cl_inner__unknown`/`cl_each__unknownQ` implicit declarations (probe `def231a`/`def231b`). Controls: the concrete instances |
| ST-10 | `rowindex` (`DEF-232`) | `row[5]` and `row[i]` inside `rowsum<i32[2][3]>`; the concrete `row[5]` twin | The constant case reports `E10216` at the instance; the run-time case emits the `E11211` trap (the out-of-bounds read never runs to completion) | Red: no guard, run exit 1 (probe `def232`). Controls: the concrete `row[5]` (`E10216`, probe `def232conc`) and a concrete in-bounds index |

### P3 — Member typing (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-11 | `enumvaluemember` (`DEF-233`) | `let e: E = E.A; let n = i32(e.A);` and the instance twin `read<T>(x) { i32(x.A) }` at `T=E` | Rejected once with `E10212` on both paths (the scoped `E.A` stays accepted) | Red: accepted, strict build fails `request for member 'A'` (probe `def233a`/`def233b`). Controls: `E.A`, the scoped `Type.MEMBER`, and the qualified `A.B.f` forms (accepted) |
| ST-12 | `viewmembers` (`DEF-238`) | `func f(s: str): usize { return s.ptr; }`; the `.len` twin; the `toCString` body | The `.ptr` integer destination reports the frozen conversion code (`E10200`); `.len` and `toCString` stay accepted and building | Red: accepted, strict build fails `-Werror=int-conversion` (probe `def238`). Controls: the `.len` twin (probe `def238ctl`, green) and the pinned `checker_conformance` `toCString` cases |

### P4 — Call conversions and `ffi` result typing (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-13 | `genericconvert` (`DEF-242`) | `call<str>(v) { unsafe { Open(x, 0); } }`; the ordinary `cstring` twin; `make<i32>()`'s `return x`; `f<i32>()`'s `let y: T = x` | Each offending instance reports once with the frozen `E10200`; the legal instances (`T = cstring`/`str`/`String`) stay accepted | Red: accepted, strict build fails (ClStr at `char *`; `void *` at `i32`) (probe `def242a`/`def242b2`/`def242c`/`def242e`). Controls: the legal instances and the `str`-parameter twin (probe `def242b`, green) |
| ST-14 | `ffiresult` (`DEF-243`) | `Uses(Count())` with `ffi { func Count(): i32; func Uses(s: cstring): i32; }` | The result carries the declared `i32`; the cstring destination reports per the frozen table (or a valid integer use runs) | Red: accepted, strict build fails `pointer from integer` (probe `def243`). Control: a valid `i32`-destination use |

### P5 — Null destinations and capture slot stores (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-15 | `nullshape` (`DEF-245`) | `let x = null; let s: str? = x; show(x);` and the `File?` store and return on hosted-amiga | Accepted; strict build on the host and the hosted-amiga leg; the run reads the null back (`s == null` and `len(s) == 0`) and exits 0 | Red: `void *` into the view/`BPTR`, strict build fails (probe `def245`/`def245amiga`; the `File?` return is red on hosted-amiga: `returning 'void *'` from a `BPTR` function). Controls: the pinned null comparisons, the copy between null-shaped locals, and the single-`null` declarations |
| ST-16 | `captureslot` (`DEF-251`) | `wrap<String?>(borrow out, s)` with `([borrow x, borrow out] () => { out = x; })()` | Rejected once with the frozen capture code (`E10401`); the run no longer double-frees | Red: accepted, run abort 134 / ASan `attempting double-free` (probe `def251`). Controls: the writable non-capture out-slot store (`put<String?>(borrow d, s)`, accepted) and the read-only capture |

### P6 — Standalone emission shapes (`03-06`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-17 | `globalcallee` (`DEF-170`) | `import { g, h } from lib; g(5); h.call(6);` and the qualified `lib.g(5)` spelling | Accepted; strict build; the mangled global symbols emit; run proves the values | Red: `implicit declaration of 'g'`, `'h' undeclared` (probe `def170`/`def170c`). Control: the current-module func-global call |
| ST-18 | `fsentry` (`DEF-189`) | The freestanding `let a: str[2] = [; "ab"];` shape | The host no-libc build runs with the program's exit code; the m68k link is clean | Red: host run segfault 139 (probe `def189`). Controls: `println`, `exit7`, and the aggregate/float freestanding cases |
| ST-19 | `arraysizeof` (`DEF-211`) | `sizeof(i32[2])` and `sizeof(i32[2][3])`; the scalar/struct and omitted-extent twins | The whole-type values (8, 24) read back by the run; strict build; run exit 0 | Red: `sizeof(i32)` → 4, run exit 252 (probe `def211`). Controls: `sizeof(i32)`, `sizeof(S)`, and a `usize`-context constant |
| ST-20 | `voidintrinsic` (`DEF-255`) | `let f: func(): void = () => assert(false, "nope");` and the `panic` twin | Accepted; strict build; the run aborts with the frozen fault (`E11213` for the assert) | Red: accepted, strict build fails `'return' with a value` (probe `def255`/`def255b`). Controls: the block-bodied twin (aborts correctly) and `return panic(...)` (`E10217`) |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/module_initializer_integrity_{spec,impl}_test.rs` (append) | ST-1 – ST-6, ST-19 | P1, `DEF-211` |
| `compiler/tests/checker_conformance_{spec,impl}_test.rs` (append) | ST-7 – ST-16 | P2 – P5 |
| `compiler/tests/callable_release_integrity_{spec,impl}_test.rs` (append) | ST-17 | P6 |
| `compiler/tests/freestanding_spec_test.rs` (append) | ST-18 | P6 |
| `compiler/tests/def_backlog_closeout_{spec,impl}_test.rs` (append) | ST-20 | P6 |
| The origin suites (existing cases) | Regression oracle | Batches E/F/G and the close-out |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| The origin `_impl_test.rs` files (append) | The spelled callee symbols, the per-arm view construction, the release scope, the element declarations and keyed instances, the guard conditions, the enum rejection spans, the typed pointer-member conversion, the duty records, the zero-view/`NULL` spellings, the shim alignment, the `sizeof` expression, the void return shape, and the re-emission determinism pins | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen | Base vs head over the corpus; every delta classified (P1, P2, P3, P5, P6) |
| Hosted-amiga leg | driver + m68k toolchain | The `File?` null-shape build (`DEF-245`) |
| Freestanding legs | shim + m68k | The host no-libc run and the m68k link (`DEF-189`) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the P1 cases | `clorraine run` on the case sources (host) | Exit per the ST expectation |
| Memory-phase suite runs | The origin suites under `RDC_ASAN=1` (the suite helper) | ASan-clean |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- A two-module library for `DEF-170`'s imported and qualified globals (the `lib` module file
  in the case's scratch, as the probe `def170lib.clo` does).
- A stub or the real `std.text`/`std.io` imports for the view/handle cases; the real compiler,
  C toolchain, ASan, the m68k toolchain, and the driver — no mocks.

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase; the controls
      verified green first — AR-3, AR-5)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests (the origin suites, the batch-H/I/J suites, the pinned
      oracles)
- [ ] Test coverage meets goals
