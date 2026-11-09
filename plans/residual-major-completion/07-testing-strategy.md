# Testing Strategy: Residual Major Completion (Batch M)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)

## Testing Overview

### Coverage Goals

| Code type | Target |
| --------- | ------ |
| Call identity | The dot-call written argument reports once at its site and a valid generic caller is not rejected; the collision reports once per import for every declaration kind; the qualified call uses the module's symbol and modes/params/return; the six direct-name controls stay byte-identical |
| Substitution | The conditional reports at the substituted arms and in the four direct contexts; the nested element key is concrete; the element member widening is byte-identical to the concrete twin |
| Context typing | The literal takes the concrete context; the negated literal's fit is folded; the nullable type argument reports at each nested site with no duplicates; the qualified type path emits the unqualified instance |
| Store targets | The runtime-indexed element/member store emits in plain/compound/update/deep-chain forms, materializes the target before the RHS, resolves the per-instance owning type, keeps the bounds trap, and never silently drops; the five conditional-rooted forms and the unsafe-wrapped **conditional** forms report one `E10221`; the unsafe **raw-prefix** forms are accepted with a store-emitting read-back |
| Loans | The element-read store and the conditional/literal/single-view copies report `E10030`; the destination record is the union of both arms (mixed positions); the borrow-element take stays the two-report control |
| Deref/take | The impure deref operand runs once in the store, return, aggregate/struct-literal field, array-literal element, and conditional arm; the guarded take empties on the evaluated path and the false path does not call the pointer; both selections ASan/LSan-clean |
| Literals/guards | The hidden destination materializes at every literal position (the unguarded codegen value/read path consults `struct_literal_needs_stores`); the owning nested conditional's carrier/fill stays inside the guard path with the drop/re-fill analyzed |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the ORIGIN suites, additions only** (AR-26):
  - P1 (`DEF-360`, `361`, `363`), all of P2, and all of P3 →
    `compiler/tests/checker_conformance_{spec,impl}_test.rs`;
  - P1 (`DEF-390`) → `compiler/tests/callable_release_integrity_{spec,impl}_test.rs`;
  - all of P4–P7 → `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs`;
  - the `DEF-367` bounds fault → `conditional_projection_residue_spec_test.rs` with the inline
    `assert_traps` helper (precedent `borrow_store_integrity_spec_test.rs:363`); the frozen
    `run_fault_spec_test.rs` data and the Amiga fault list stay untouched.
  **No new suite pair is created.** A case with no home needs a register entry and user approval
  (AR-26, AR-35).
- The committed origin cases and the frozen spec suites (`stage_spec_test.rs`,
  `compile_fail_spec_test.rs`, `codegen_run_spec_test.rs`, `run_fault_spec_test.rs`,
  `determinism_spec_test.rs`, `corpus_spec_test.rs`, the batch-I/J/K/L spec suites) stay green as
  the regression oracle. **Only `_impl_test.rs` byte-identity pins may be re-pinned**, and
  only when the per-phase differential proves the delta intended; **spec cases are never edited**
  (AR-35).
- Integration tests are the corpus runs on the host and (at close) the Amiga legs; E2E is the
  `clorraine` driver run over the spec cases.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the component specs (`03-01` – `03-07`), the frozen specification
> (Ch 01/02/03/04/06/07/08/10/16/17, `spec/14` registry), and this plan's Ambiguity Register.
> IMMUTABLE ORACLE: do not modify an expectation to match the implementation.

### P1 — Call identity and written-argument checks (`03-01`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-1 | `dotwrittenarg` (`DEF-360`) | `b.pick(7)` with `b: Box`; the `borrow`-receiver `b.put(7)`; the direct inferred/explicit twins; the valid caller `outer<i32>(b, c)` with `b: Box`, `c: Box`, and `b.pick(c)` | One `E10200` at each written `7` and at each direct twin; no TU; the matching dot calls and the `c: Box` caller build and run | Red: accepted, strict build fails (`cl_pick__Box` 11:25, `cl_put__Box` 11:20); the direct twins report one `E10200` each; the `c: Box` caller is falsely rejected with two `E10200` (8:12/8:19); the `c: i32` shapes are rejection cases. Control: `m360_control.clo` |
| ST-2 | `directshadowcontrol` (`DEF-361`) | Six fixtures: local, parameter, generic-body local, expression-lambda, block-lambda direct-name shadows used as dot receivers, and the no-import baseline (`m361_control`) | **Green control**: each builds strictly and runs 0; the import is never the receiver. The module-own shape is **not** in the case (it is the `DEF-363` collision) | Verified green at the head; the case fails if a fix regresses the body/lambda scope (AR-31); `d_comp`/`d_capture` are unrelated dialect errors |
| ST-3 | `importdeclcollision` (`DEF-363`) | `import { b as g }` + `let g` (both orders); `import { b }` + `let b`; the func/struct/const declaration-kind matrix (both orders); the renamed control | One `E11103` at the import item per collision; the renamed/unrelated control runs 0 | Red: all accepted; the kind matrix accepted with unused-import warnings. Control: `m363_control.clo` |
| ST-4 | `qualifiedcallidentity` (`DEF-390`) | `probe.m390.lib.f()` with a caller `f`; the signature-divergent shape; the mismatch control; the qualified func-typed global `m.hook()`; the qualified-head local-shadow fixture (a local named like the one-segment module head with a callable field) | The module's `f` is called with its own modes/params/return (run 0); the signature shape builds and runs 0; the mismatch reports one `E10200`; `m.hook()` runs 0; the local-shadow fixture calls the field (run 0) | Red: run 1 (wrong callee); strict build fails (`cl_probe_m390_sigmain_f`). Controls: `m390_control.clo`, the mismatch, `m390_hook*`, `m390_shadow` |

### P2 — Substitution edges I (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-5 | `opaqueconditional` (`DEF-364`) | `id(c ? a : b)` with `T = i32`, `U = str`; the identical-arm and non-null controls; the concrete disagreeing call | One `E10200` at the else arm; no TU; the controls run 0 | Red: accepted, strict build fails (`type mismatch in conditional expression`) |
| ST-6 | `nullconditionalarm` (`DEF-364`) | `id(c ? x : null)` with `x: i32`; the direct let/return/field/argument contexts (`m364_ctx`); the non-null sibling control | One `E10012` at the `null` in each context; the sibling control runs 0 | Red: accepted, strict build fails (`pointer/integer type mismatch`, `void *` to `i32`); `m364_direct`/`m364_ctx` accepted and fail the C build |
| ST-7 | `nestedelementkey` (`DEF-365`) | `id(v[0])` on a `str` element of `str[2]`; the `id(v.len)` control | Accepted; `cl_id__u8` emitted; no `#`; build/run 0 on host (and the m68k emission pins) | Red: accepted; `stray '#' in program`, `cl_id__` undeclared |
| ST-8 | `elementmemberview` (`DEF-368`) | `let s: str = xs[0].name;` at `T = S`; the concrete twin | Both build strictly and run 0; the emitted assignment is `cl_string_view(xs[0].name)` | Red: generic accepted, strict build fails (`ClStr` from `ClString`); concrete runs 0 |

### P3 — Substitution edges II (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-9 | `genericliteralarg` (`DEF-371`) | `set<S[1]>(borrow a, 0)`; the concrete `setS`; the out-of-range input `1000000000000` | Accepted, build/run 0; the out-of-range input reports `E10202` at the literal | Red: one `E10200` (`i32`→`usize`) at 15:25; concrete runs 0 |
| ST-10 | `nullabletypearg` (`DEF-384`) | The field annotation, the explicit call type argument (`arrayNew<i32?>()`), `Array<Array<i32?>>`, `Array<Wrap<i32?>>`; the duplicate-count case; the verified `Array<String?>` control | One `E10011` at each nested type-argument site and at the explicit call type argument (the count case has **five** sites, no duplicates); the nullable-capable instances build and run 0; the `ClArray__StringQ` key is pinned | Red: the annotation, call argument, and nested cases are accepted/run 0; the count case reports the three existing sites but neither `Array<i32?>` nor the call argument; the combined probe fails the strict build |
| ST-11 | `qualifiedtypepath` (`DEF-385`) | `std.mem.Array<i32>` and the unqualified `Array<i32>` | Both build and run 0; the emitted symbol is `ClArray__i32`; the unqualified control is byte-identical | Red: accepted, strict build fails (`i32` from `ClArray__i32`) |
| ST-12 | `negatedliteral` (`DEF-386`) | The eight-position matrix at `i16` (let, module value, argument, return, field, array element, conditional arm, assignment); `i8 = -5`; `i8 = -128`; `i8 = -129`; `u8 = -5`; `f32 = -5`; `bool = -5`; the positive control | The matrix, `i8 = -5`, and `-128` build/run 0; `i8 = -129` and `u8 = -5` report one `E10202` each; `f32 = -5`/`bool = -5` report `E10200`; no wrapping pin | Red: the matrix reports eight `E10200`; `-5`, `-128`, `-129`, and `u8 -5` report `E10200`; `f32`/`bool` report `E10200`; positive control runs 0 |

### P4 — Dropped store targets (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-13 | `indexedelementfieldstore` (`DEF-367`) | `a[i].y = bump();` local, slice (compile-only), borrow parameter, generic borrow, constant index; compound; update; deep chain; nested generic member (`w.mids[i].inner.name`); multi-index (`w.grid[i][j].name`); target order; the generic owning member/element/whole-element stores; the out-of-range index | The store and RHS emit once; read-backs correct; calls = 4; run 0; compound/update/chain/nested/two-index/order run 0; the owning stores drop the old owner once and are LSan-clean; the out-of-range traps `E11211` via the residue-suite `assert_traps`; hosted-amiga compiles the accepted forms | Red (critical): accepted; basic run 191/calls = 1; compound 253; update 255; chain 249; nested 255; two-index 255; whole store leak (LSan); order 247; bounds run 0 |
| ST-14 | `conditionalmemberstore` (`DEF-377`) | The **five** conditional-base member/element stores (`m377_member`, `m377_index`, `m377_array`, `m377_indexbase`, `m377_deep`); the two `unsafe`-wrapped conditional forms (`(unsafe (c?a:b)).v.x`, `(unsafe (c?a:b))[i].x`); the accepted prefix raw forms `unsafe p.f`/`unsafe p.f.x`; the rejected prefix conditional `unsafe (c?a:b).v`; the read control; the block controls | One `E10221` for each of the five red forms, each unsafe-wrapped conditional form, and the prefix conditional; the `unsafe p.f`/`p.f.x` prefix forms are accepted and **read back the stored value**; no TU; the read and `s.f.g`/`s.arr[i]`/`p.f.g` (inside `unsafe { }`) stay accepted | Red: the five forms accepted; runs 255/249/249/249/255; the unsafe conditionals run 0; the raw prefixes report `E10221` at head; the conditional prefix reports `E10221` |
### P5 — Element and store loans (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-15 | `elementreadloan` (`DEF-372`) | `let s: str = xs[0];` over a loaned element; the direct and member twins; an in-scope read | One `E10030` at the fill; the twins keep their report; the in-scope read runs 0 | Red: accepted; ASan heap-use-after-free (the sanitizer report is the oracle); twins `E10030` |
| ST-16 | `copysourceloans` (`DEF-373`) | The conditional array copy, the array-literal copy (fill form), the single-view conditional, a conditional whose arm is an array literal, a conditional element inside an array literal (`b373_mix`), the different-arm-count case (`b373_diff`), the fill-form case (`b373_fill`); the direct twin; in-scope copies | One `E10030` per affected fill; the mixed-position case reports at both fills (the destination record is a union); the different-arm-count and fill-form cases report at every affected fill; the direct twin keeps its report; the in-scope copies run 0 | Red: all accepted; ASan heap-use-after-free (run varies; the sanitizer report is the oracle; the `b373_mix`/`b373_diff`/`b373_fill` runs also vary) |
| ST-17 | `borrowelemtakecontrol` (`DEF-374`) | The generic and concrete `eat(take (c ? v.name : w.name))` | **Green control**: two `E10026` each at the same span | Verified green at the head (AR-32) |

### P6 — Deref and take channels (`03-06`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-18 | `derefoperandonce` (`DEF-375`) | The store (`overwrite<T>(p, q, v)`), the return (`return *alt(p, q)`), the struct-literal field (`Box { s: *alt(p, q) }`), the array element (`[*alt(p, q)]`), and the conditional arm (`flag ? *alt(p, q) : x`); the discarded read (`b375_discard`), the non-taking argument (`b375_arg`), and the concrete duplicable control (`m375_dup`) | Accepted; `alt` once in all five consuming shapes; the store/return/field/array/conditional read-backs run 12; the discarded/argument controls run 0; `m375_dup` reads once and runs 0; ASan/LSan-clean; a named generic duplicable emission delta is allowed | Red: the five consuming shapes abort 134 + ASan double-free; `cl_alt` twice in the store C; the controls are clean |
| ST-19 | `guardeddereftake` (`DEF-376`) | `g && (eat(take (unsafe *q)) == 1)` with `g = true` and `g = false`; the impure-pointer variants | Accepted; both selections run 0; the false path does not call the impure pointer (calls = 0); ASan/LSan-clean | Red: `g = true`/`m376_impure_true` abort 134 + ASan double-free; `m376_impure_false` runs 10 (calls = 1) |

### P7 — Literal destinations and guards (`03-07`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-20 | `hiddenviewdestination` (`DEF-378`) | The read position, the unguarded position, the grid element, and the guarded row conditional with a call-valued `str[2]` field | All build strictly and run 0; each call runs once; the unguarded codegen value/read path consults `struct_literal_needs_stores`; the concrete brace-list control is unchanged | Red: all accepted; strict build fails (`too few arguments ... expected 1, have 0`); the read position is red |
| ST-21 | `guardnestedowning` (`DEF-379`) | The owning nested conditional under `g && consume(...)`, both selections; a loop fixture that re-executes the guard twice (`b379_loop`); the eager-order (unguarded) control | Calls = 2 and run 0 in the single-pass fixture; calls = 4 and run 0 in the loop fixture; ASan/LSan-clean; the `__cl_owned0` fill is inside the guarded path; a re-executed guard releases the previous value once | Red: accepted; single-pass run 1/calls = 3; loop run 2/calls = 6; the carrier fill precedes the guard |

## Test Categories

### Specification Tests (from ST-cases above)

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/checker_conformance_{spec,impl}_test.rs` (append) | ST-1 – ST-3, ST-5 – ST-12 | P1 (`DEF-360`, `361`, `363`), P2, P3 |
| `compiler/tests/callable_release_integrity_{spec,impl}_test.rs` (append) | ST-4 | P1 (`DEF-390`) |
| `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (append) | ST-13 – ST-21 (ST-13's bounds case with the inline `assert_traps` helper, precedent `borrow_store_integrity_spec_test.rs:363`) | P4, P5, P6, P7 |
| `compiler/tests/run_fault_spec_test.rs` | No additions — the frozen fault data and the Amiga fault list stay untouched | Oracle only |
| The frozen spec suites and the origin suites (existing cases) | Regression oracle | Batches I/J/K/L and the close-outs |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| The origin `_impl_test.rs` files (append) | The dot-call report code/span, the `E11103` site and kind matrix, the qualified symbol/modes/params/return, the conditional and direct-context spans, the nested type-argument spans and counts, the keyed element symbol, the member widening text, the store sequence/RHS count/per-instance owning drop/target order, the bounds guard, the loan union and `E10030` sites, the single deref call and pointee move (store/return/field/array/conditional), the deferred pointer fill and guarded emptying, the literal destination form, the call counters, and the re-emission determinism pins. Only byte-identity pins may be re-pinned; spec cases never (AR-35) | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own + sema | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen + own | Base vs head over the corpus and the named phase probe sets (P1 `m360_*`/`m361_*`/`m363_*`/`m390_*`; P2 `m364_*`/`m365_*`/`m368_*`; P4 `m367_*`/`m377_*`; P6 `m375_*`/`m376_*`; P7 `m378_*`/`m379_*`); every delta classified per phase (the keys/destinations, the qualified symbols, the store shape) |
| Memory suite runs | the suite `RDC_ASAN=1` switch | The P2/P4/P5/P6/P7 cases ASan/LSan-clean; P1 runs the symbol/IR with the memory lens; P3 runs the nullable-handle ASan control |
| Profile legs | driver + toolchains | The hosted-amiga leg for `DEF-365`/`368`/`378`/`384`/`385`; the freestanding compile for `DEF-365` and the no-import `m364_control_fs` control; the freestanding exclusions for `DEF-368`/`384`/`385` (they import `std.text`/`std.mem`, `E11200`) are recorded (AR-28) |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the phase cases | `clorraine run` on the case sources (host) | Exit per the ST expectation |
| Bounds fault | The `DEF-367` out-of-range store in `conditional_projection_residue_spec_test.rs` with the inline `assert_traps` helper | `error E11211: index out of range` on the plain and ASan builds |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- The multi-file fixtures for `DEF-390` (the lib/sig modules, the func-typed global `m.hook()`,
  and the qualified-head local-shadow `m390_shadow`; the file list is the program).
- The six direct-name fixtures for `DEF-361` (local, parameter, generic-body local, expression
  lambda, block lambda, no-import baseline) and the aliased oracle sibling; the module-own shape
  is excluded (the `DEF-363` collision).
- The `str[2]` call-valued literal fixtures for `DEF-378` (read, unguarded, grid, guarded) and
  the owning nested literal for `DEF-379`, including the loop fixture `b379_loop` that re-executes
  the guard and the eager-order control.
- The `DEF-367` store fixtures (the local array, the uncalled borrow-slice function, the borrow
  parameter, the generic borrow, the constant index, compound, update, deep chain, nested generic
  member, multi-index, target order, the generic owning member/element/whole-element stores, the
  out-of-range index).
- The `DEF-384` fixtures (annotation, explicit call type argument, two nested cases, the
  duplicate-count case, the nullable-handle control) and the `DEF-386` position matrix with
  `-128`/`-129`/`u8 -5`.
- The `DEF-375` store/return/field/array/conditional fixtures, the `b375_discard`/`b375_arg`
  controls, and the `DEF-376` false/true impure-pointer fixtures; the `DEF-373` mixed-position,
  different-arm-count, and fill-form fixtures (`b373_mix`/`b373_diff`/`b373_fill`).
- The `DEF-377` conditional-rooted and `unsafe`-wrapped fixtures plus the block controls
  (`s.f.g`, `s.arr[i]`, `p.f.g` inside `unsafe { }`).
- The real compiler, C toolchain, ASan, the m68k toolchain, and the driver — no mocks.

## Verification Gates

| Gate | Command | Applies |
| ---- | ------- | ------- |
| Fast verification | `make check` | Every phase |
| Focused suite (P1–P3) | `make -C docker exec CMD="cargo test --test checker_conformance_spec_test --test checker_conformance_impl_test"` | P1, P2, P3 |
| Focused suite (P1 `DEF-390`) | `make -C docker exec CMD="cargo test --test callable_release_integrity_spec_test --test callable_release_integrity_impl_test"` | P1 |
| Focused suite (P4–P7) | `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | P4–P7 |
| ASan suite leg | `make -C docker exec CMD="RDC_ASAN=1 cargo test --test <suite>"` | P2, P4, P5, P6, P7; P3's nullable-handle control; P1's symbol/IR via the memory lens |
| Host ASan probes | `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The `DEF-372`/`373`/`375`/`376`/`379` reds (before the fix) and the accepted controls (after) |
| Strict build | `make -C docker exec CMD="target/debug/clc --profile host -o out.c prog.clo && cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out.bin out.c"` | Every accepted shape |
| Hosted-amiga leg | `clc --profile hosted-amiga` + `m68k-amigaos-gcc` | `DEF-365`, `368`, `378`, `384`, `385`; the no-import `m364_control_fs` control |
| Freestanding compile | `clc --profile freestanding` | `DEF-365` and the no-import `m364_control_fs` control; the `DEF-368`/`384`/`385` exclusions are recorded (`std` modules, `E11200`) |
| Emission differential | per-phase clc+cc over the named probe sets and the corpus | P1, P2, P4, P6, P7 (and P3 when the qualified symbol changes); every delta classified (AR-37) |
| Full acceptance | `make verify` | Close-out |

## Verification Checklist

- [x] All specification test cases (ST-*) defined with concrete input/output pairs
- [x] Every ST case traces to a requirement, spec doc, or AR entry
- [x] Specification tests written BEFORE implementation
- [x] Specification tests verified to FAIL before implementation (red phase; the controls
      verified green first — AR-3, AR-5)
- [x] All specification tests pass after implementation (green phase)
- [x] Implementation tests written for edge cases and internals
- [x] All unit / integration / E2E tests pass
- [x] No regressions in existing tests (the origin suites, the frozen spec suites, the batch-I/J/K/L
      oracles)
- [x] Test coverage meets goals
