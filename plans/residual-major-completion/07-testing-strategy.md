# Testing Strategy: Residual Major Completion (Batch M)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Call identity | The dot-call written argument reports once at its site; the collision reports once per import; the qualified call emits the module's symbol; the direct-name controls stay byte-identical |
| Substitution | The conditional reports at the substituted arms; the nested element key is concrete; the element member widening is byte-identical to the concrete twin |
| Context typing | The literal/negated literal takes the concrete context; the nullable type argument reports at its span; the qualified type path emits the unqualified instance |
| Store targets | The runtime-indexed element store emits once with its RHS, keeps the bounds trap, and drops the overwritten owner; the conditional-base store reports `E10221` once |
| Loans | The element-read store and the conditional/literal/single-view copies report `E10030`; the borrow-element take stays the two-report control |
| Deref/take | The impure deref operand runs once; the guarded take empties on the evaluated path; both selections ASan/LSan-clean |
| Literals/guards | The hidden destination materializes at every literal position; the owning nested conditional's fill stays inside the guard path |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the ORIGIN suites, additions only** (AR-26):
  - P1 (`DEF-360`, `361`, `363`), all of P2, and all of P3 →
    `compiler/tests/checker_conformance_{spec,impl}_test.rs`;
  - P1 (`DEF-390`) → `compiler/tests/callable_release_integrity_{spec,impl}_test.rs`;
  - all of P4–P7 → `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs`;
  - the `DEF-367` bounds fault → `compiler/tests/run_fault_spec_test.rs`.
  A new `compiler/tests/residual_major_completion_{spec,impl}_test.rs` pair is created only if a
  phase cannot host a case naturally, and then it carries only that case (recorded in the
  register).
- The committed origin cases and the frozen spec suites (`stage_spec_test.rs`,
  `compile_fail_spec_test.rs`, `codegen_run_spec_test.rs`, `run_fault_spec_test.rs`,
  `determinism_spec_test.rs`, `corpus_spec_test.rs`, the batch-I/J/K/L spec suites) stay green as
  the regression oracle; a legitimate emission delta that invalidates a committed pin is
  re-pinned in the phase with the differential evidence recorded, never silently.
- Integration tests are the corpus runs on the host and (at close) the Amiga legs; E2E is the
  `clorraine` driver run over the spec cases.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the component specs (`03-01` – `03-07`), the frozen specification
> (Ch 01/02/03/04/06/07/08/10/16/17, `spec/14` registry), and this plan's Ambiguity Register.
> IMMUTABLE ORACLE: do not modify an expectation to match the implementation.

### P1 — Call identity and written-argument checks (`03-01`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-1 | `dotwrittenarg` (`DEF-360`) | `b.pick(7)` on `pick<T>(take v: T, take w: T)` with `b: Box`; the `borrow`-receiver `b.put(7)` | One `E10200` at the written `7`; no TU; the matching dot calls build and run | Red: accepted, strict build fails (`cl_pick__Box` 11:25, `cl_put__Box` 11:20). Controls: `m360_control.clo`, the direct inferred/explicit twins (already `E10200`) |
| ST-2 | `directshadowcontrol` (`DEF-361`) | Local, parameter, expression-lambda, block-lambda, module-own, and generic-body direct-name shadows used as dot receivers | **Green control**: each builds strictly and runs 0; the import is never the receiver | Verified green at the head; the case fails if a fix regresses the body/lambda scope (AR-31) |
| ST-3 | `importdeclcollision` (`DEF-363`) | `import { b as g }` + `let g` (both orders); `import { b }` + `let b`; the renamed control | One `E11103` at the import item per collision; the renamed/unrelated control runs 0 | Red: all accepted, run 0. Control: `m363_control.clo` |
| ST-4 | `qualifiedcallidentity` (`DEF-390`) | `probe.m390.lib.f()` with a caller `f`; the signature-divergent shape; the mismatch control | The module's `f` is called (run 0); the signature shape builds and runs 0; the mismatch reports one `E10200` | Red: run 1 (wrong callee); strict build fails (`cl_probe_m390_sigmain_f`). Controls: `m390_control.clo`, the mismatch |

### P2 — Substitution edges I (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-5 | `opaqueconditional` (`DEF-364`) | `id(c ? a : b)` with `T = i32`, `U = str`; the identical-arm and non-null controls | One `E10200` at the else arm; no TU; the controls run 0 | Red: accepted, strict build fails (`type mismatch in conditional expression`) |
| ST-6 | `nullconditionalarm` (`DEF-364`) | `id(c ? x : null)` with `x: i32`; the non-null sibling control | One `E10012` at the `null`; the sibling control runs 0 | Red: accepted, strict build fails (`pointer/integer type mismatch`, `void *` to `i32`) |
| ST-7 | `nestedelementkey` (`DEF-365`) | `id(v[0])` on a `str` element of `str[2]`; the `id(v.len)` control | Accepted; `cl_id__u8` emitted; no `#`; build/run 0 on host (and the m68k emission pins) | Red: accepted; `stray '#' in program`, `cl_id__` undeclared |
| ST-8 | `elementmemberview` (`DEF-368`) | `let s: str = xs[0].name;` at `T = S`; the concrete twin | Both build strictly and run 0; the emitted assignment is `cl_string_view(xs[0].name)` | Red: generic accepted, strict build fails (`ClStr` from `ClString`); concrete runs 0 |

### P3 — Substitution edges II (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-9 | `genericliteralarg` (`DEF-371`) | `set<S[1]>(borrow a, 0)`; the concrete `setS`; the out-of-range literal | Accepted, build/run 0; the out-of-range literal reports `E10202` | Red: one `E10200` (`i32`→`usize`) at 15:25; concrete runs 0 |
| ST-10 | `nullabletypearg` (`DEF-384`) | `Array<i32?>`; the verified `Array<String?>` and the legal nullable `raw`/`cstring`/view spellings | One `E10011` at the `i32?`; the nullable-capable instances build and run 0 | Red: accepted, strict build fails (`void *` to `i32` in `cl_push__i32Q`); `Array<String?>` runs 0 |
| ST-11 | `qualifiedtypepath` (`DEF-385`) | `std.mem.Array<i32>` and the unqualified `Array<i32>` | Both build and run 0; the emitted symbol is `ClArray__i32`; the unqualified control is byte-identical | Red: accepted, strict build fails (`i32` from `ClArray__i32`) |
| ST-12 | `negatedliteral` (`DEF-386`) | `let delta: i16 = -5; let a: i8 = -5;`; the positive control; `let a: i8 = -1000;`; the `u8 = -5` pin | The in-range literals build/run 0; the out-of-range one reports `E10202`; the `u8` one reads 251 | Red: `E10200` (`i32`→`i16`/`i8`); positive control runs 0 |

### P4 — Dropped store targets (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-13 | `indexedelementfieldstore` (`DEF-367`) | `a[i].y = bump();` local, slice (compile-only), borrow parameter, generic borrow, constant index; an owning-member variant; the out-of-range index | The store and RHS emit once; read-back 7; calls = 4; run 0; the owning member drops the old owner once (ASan/LSan-clean); the out-of-range traps `E11211` | Red (critical): accepted, run 191; calls = 1; constant index emits | 
| ST-14 | `conditionalmemberstore` (`DEF-377`) | The four/five conditional-base member/element stores; the read control | One `E10221` per store target; no TU; the read and ordinary stores stay accepted | Red: all accepted; runs 249/255; read control runs 0 |

### P5 — Element and store loans (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-15 | `elementreadloan` (`DEF-372`) | `let s: str = xs[0];` over a loaned element; the direct and member twins; an in-scope read | One `E10030` at the fill; the twins keep their report; the in-scope read runs 0 | Red: accepted; ASan heap-use-after-free (run 14 observed); twins `E10030` |
| ST-16 | `copysourceloans` (`DEF-373`) | The conditional array copy, the array-literal copy, the single-view conditional; the direct twin; in-scope copies | One `E10030` per fill; the direct twin keeps its report; the in-scope copies run 0 | Red: all accepted; ASan heap-use-after-free (runs 56/193/145) |
| ST-17 | `borrowelemtakecontrol` (`DEF-374`) | The generic and concrete `eat(take (c ? v.name : w.name))` | **Green control**: two `E10026` each at the same span | Verified green at the head (AR-32) |

### P6 — Deref and take channels (`03-06`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-18 | `derefoperandonce` (`DEF-375`) | `overwrite<T>(p, q, v)` with `v = *alt(p, q)` and an impure `alt`; the discarded and duplicable controls | Accepted; `alt` once; run 12; ASan/LSan-clean; the controls keep their shapes | Red: accepted; abort 134 + ASan double-free; `cl_alt` twice in the C |
| ST-19 | `guardeddereftake` (`DEF-376`) | `g && (eat(take (unsafe *q)) == 1)` with `g = true` and `g = false` | Accepted; run 0 / 1; ASan/LSan-clean | Red: accepted; abort 134 + ASan double-free |

### P7 — Literal destinations and guards (`03-07`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-20 | `hiddenviewdestination` (`DEF-378`) | The read position, the grid element, and the guarded row conditional with a call-valued `str[2]` field | All build strictly and run 0; each call runs once; the concrete brace-list control is unchanged | Red: all accepted; strict build fails (`too few arguments ... expected 1, have 0`) |
| ST-21 | `guardnestedowning` (`DEF-379`) | The owning nested conditional under `g && consume(...)`, both selections | Calls = 2; run 0; ASan/LSan-clean | Red: accepted; run 1; calls = 3 |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/checker_conformance_{spec,impl}_test.rs` (append) | ST-1 – ST-3, ST-5 – ST-12 | P1 (`DEF-360`, `361`, `363`), P2, P3 |
| `compiler/tests/callable_release_integrity_{spec,impl}_test.rs` (append) | ST-4 | P1 (`DEF-390`) |
| `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (append) | ST-13 – ST-21 | P4, P5, P6, P7 |
| `compiler/tests/run_fault_spec_test.rs` (append) | ST-13's bounds case | P4 (`DEF-367`) |
| The frozen spec suites and the origin suites (existing cases) | Regression oracle | Batches I/J/K/L and the close-outs |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| The origin `_impl_test.rs` files (append) | The dot-call report code/span, the `E11103` site, the qualified symbol, the conditional/type-argument spans, the keyed element symbol, the member widening text, the store sequence and RHS count, the bounds guard, the drop order, the loan records and `E10030` sites, the single deref call, the guarded emptying, the literal destination form, the call counters, and the re-emission determinism pins | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own + sema | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen + own | Base vs head over the corpus; every delta classified per phase (the `DEF-365`/`368`/`378`/`385` keys and destinations, the `DEF-390` symbols, the `DEF-367` store) |
| Memory suite runs | the suite `RDC_ASAN=1` switch | The P2/P4/P5/P6/P7 cases ASan/LSan-clean |
| Profile legs | driver + toolchains | The hosted-amiga leg for `DEF-365`/`368`/`378`/`384`/`385`; the freestanding compile for `DEF-365` |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the phase cases | `clorraine run` on the case sources (host) | Exit per the ST expectation |
| Bounds fault | The `DEF-367` out-of-range store under the run-fault harness | `error E11211: index out of range` |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- The multi-file fixtures for `DEF-390` (the lib/sig modules; the file list is the program).
- The direct-name matrix for `DEF-361` (body, lambda expression/block, module initializer,
  generic body) and the aliased oracle sibling.
- The `str[2]` call-valued literal fixtures for `DEF-378` and the owning nested literal for
  `DEF-379`.
- The `DEF-367` store fixtures (the local array, the uncalled borrow-slice function, the borrow
  parameter, the generic borrow, the constant index, the out-of-range index).
- The real compiler, C toolchain, ASan, the m68k toolchain, and the driver — no mocks.

## Verification Gates

| Gate | Command | Applies |
| ---- | ------- | ------- |
| Fast verification | `make check` | Every phase |
| Focused suite (P1–P3) | `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | P1, P2, P3 |
| Focused suite (P1 `DEF-390`) | `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test --test callable_release_integrity_impl_test"` | P1 |
| Focused suite (P4–P7) | `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | P4–P7 |
| ASan suite leg | the suites' `RDC_ASAN=1` switch | P2, P4, P5, P6, P7 |
| Host ASan probes | `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The `DEF-372`/`373`/`375`/`376`/`379` reds (before the fix) and the accepted controls (after) |
| Strict build | `clc --profile host -o out.c prog.clo` + `cc -std=c99 -O0 -g -Wall -Wextra -Werror` | Every accepted shape |
| Hosted-amiga leg | `clc --profile hosted-amiga` + `m68k-amigaos-gcc` | `DEF-365`, `368`, `378`, `384`, `385` |
| Freestanding compile | `clc --profile freestanding` | `DEF-365` |
| Full acceptance | `make verify` | Close-out |

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (red phase; the controls
      verified green first — AR-3, AR-5)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests (the origin suites, the frozen spec suites, the batch-I/J/K/L
      oracles)
- [ ] Test coverage meets goals
