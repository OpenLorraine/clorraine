# Testing Strategy: Memory and Value Integrity (Batch N)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)
> **Baseline**: head `f6da78f`

## Testing Overview

### Coverage Goals

| Area | Target |
| ---- | ------ |
| Loan transfer | A stored copy of a view keeps every source position: the exact self-assignment, the composite conditional/array source at an element/projected/runtime destination (with the constant-sibling acceptance), the wildcard `Element(None)` record and its three-reader match (the non-constant branch keeps the `E10031` union), the deeper projection after a shorter recording, and the element-store nested fill each report `E10030` at the escaping fill and stay accepted otherwise; `DEF-431`/`DEF-435` close incidental if the `429` gate alone greens them |
| Loan sources | A runtime store's wildcard `Element(None)` position reaches a constant-read copy (the sibling read stays accepted); a temporary-derived element records its lowered owner; a call-mediated element view consults the binding loans (both copy and direct-call forms); each escape reports `E10030` |
| Deref move-outs | A projection of a raw pointee (through all three emission gates), a guarded take aliasing the conditional target, a guarded borrowing intrinsic or borrow argument, and a guarded take of a projected member over a raw pointer each release exactly once; the chosen path evaluates, the false path does not; the sibling member stays owned; ASan/LSan clean |
| Store lowering | The `Deref`-rooted member store emits (never drops) and pins its base/index before the RHS; the whole-element target is pinned before the RHS; the fixed-array literal self-read runs before the clear; the constant-index self-copy and raw self-alias are no-ops; the element/raw-target RHS materializes once per pointer expression |
| Per-instance | The conditional by-value move reports one `E10034` for an owning instance and accepts a separate duplicable generic whole move; the generic compound division emits the `E11210` guard through the resolved projection type and traps, never SIGFPE; `DEF-425` is parked with its evidence |
| Literal destinations | The conditional-of-literals read base (including the mixed place/literal form), the ternary-arm take literal, and the conditional array literal of places build strictly and run with the chosen values; each call once; the controls stay byte-identical |
| Controls | `DEF-427`'s view-length matrix reads 2 in every spelling; its filed byte probe is accepted and ASan/LSan-clean (the exact `254`/`0` output is an implementation pin); a future scalar-`len` fix needs a user ruling; `DEF-447`'s filed literal and variants are LSan-clean; no code change |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the ORIGIN suites, additions only** (AR-30):
  - `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs`: ST-1–ST-15, ST-17–ST-20,
    and ST-22 (P1–P4, P6, `DEF-426`, `DEF-447`).
  - `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs`: ST-16 (`DEF-410`, the
    per-instance lend family). `DEF-425` has no new case; its park evidence is the batch-M case
    paths.
  - `compiler/tests/array_value_integrity_{spec,impl}_test.rs`: ST-21 (`DEF-427`, the
    fixed-array/`len` origin; its `run_memory` helper is added with the control).
  - **No new suite pair is created.** A case with no home needs a register entry and user
    approval.
- The committed origin cases and the frozen spec suites (`stage_spec_test.rs`,
  `compile_fail_spec_test.rs`, `codegen_run_spec_test.rs`, `run_fault_spec_test.rs`,
  `determinism_spec_test.rs`, `corpus_spec_test.rs`, the batch-I–M spec suites) stay green as the
  regression oracle. **Only `_impl_test.rs` byte-identity pins may be re-pinned**, and only when
  the per-phase differential proves the delta intended; **spec cases are never edited** (AR-30).
- Integration is the corpus run on the host and the profile compile legs; E2E is the `clorraine`
  driver run over the spec cases.

## 🚨 Specification Test Cases (MANDATORY — NON-NEGOTIABLE)

> Derived exclusively from the phase designs (`03-01`–`03-07`), the frozen specification
> (Ch 04/07/08/16/17, `spec/14` registry), and this plan's Ambiguity Register. IMMUTABLE ORACLE:
> do not modify an expectation to match the implementation.

### P1 — Loan transfer on stored copies (`03-01`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-1 | `selfassignloans` (`DEF-428`) | `s = s` and `ys = ys` over `xs[0] = a.v`, then `fill`; the in-scope read-back; the direct `fill(xs[0])` control | One `E10030` at each fill after the self-assignment; no TU; the in-scope read-back runs; the direct control keeps its report | Red: accepted; ASan heap-use-after-free (the sanitizer is the oracle); the direct control reports `E10030` (13:5) |
| ST-2 | `compositeelementstore` (`DEF-429`) | `grid[0] = c ? xs : zs`; `out2[0] = c ? xs[0] : zs[0]`; `g[0] = [xs[0], xs[1]]`; the runtime `grid[k] = c ? xs : zs` with the direct fill and the copy; the direct conditional fill; the constant sibling `ys[0] = a.v; let s = ys[1]; fill(s)` | One `E10030` at each affected fill (including the runtime forms and the direct conditional fill); the copies stay accepted; the sibling is accepted and clean; no TU | Red: all accepted; ASan heap-use-after-free; the direct conditional fill is red at head and a control after the fix; the sibling is clean |
| ST-3 | `deeperprojectioncopy` (`DEF-431`) | `rows[0] = xs; let s = rows[0][0]; fill(s)`; the direct `fill(rows[0][0])` control; the deeper read the prefix fallback must cover | One `E10030` at the copy's fill; the direct control keeps its report; no TU. Incidental closure allowed if the `429` gate greens it | Red: accepted; ASan heap-use-after-free; the direct control `E10030` (15:5) |
| ST-4 | `elementstorenestedfill` (`DEF-435`) | The constant `g[1] = [; a.v]` copy and its direct read (`d435b`); the runtime `g[k] = [; a.v]` copy and its later constant read; the direct runtime read (`q435runtime_direct`); the literal-fill form | One `E10030` at the constant copy's fill (incidental if the `429` gate greens it) and one at the runtime copy's fill; the constant direct read (`d435b`) reports `E10030`; the runtime direct read (`q435runtime_direct`) is accepted and ASan-red at the head and is recorded per AR-35(e) as an intended delta or a filed residual; the store is accepted; no TU | Red: accepted; ASan heap-use-after-free; the constant direct read `E10030` (13:5) |

### P2 — Loan sources (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-5 | `runtimestorecopy` (`DEF-430`) | `xs[k] = a.v` with `k = 1`, then `let s = xs[1]; fill(s)`; the `k = 0` variant; the constant-index store control; the constant sibling `ys[0] = a.v; let s = ys[1]; fill(s)`; the direct runtime fill | One `E10030` at the copy's fill; the `k = 0` variant keeps the conservative report; the constant-index control keeps its report; the sibling is accepted and clean; the direct form is recorded per AR-8; no TU | Red: accepted; ASan heap-use-after-free; the constant-index control `E10030` (13:5) |
| ST-6 | `temporaryelementcopy` (`DEF-432`) | `let ys: str[2] = [mk().v; "EE"]; fill(ys[0])`; the direct `fill(mk().v)` twin; an in-scope read | One `E10030` at the copy's fill; the direct twin keeps its report; the in-scope read runs 0; no TU; the impl pin names the `__cl_owned0`/temp owner | Red: accepted; ASan heap-use-after-free; the direct twin `E10030` (12:5) |
| ST-7 | `callviewresult` (`DEF-433`) | `let s = pick(xs); fill(s)` and `fill(pick(xs), slot)`; the direct `fill(xs[0])` control; a caller-parameter view; an in-scope result | One `E10030` at each affected fill; the direct control keeps its report; the caller-parameter view stays accepted; the in-scope result runs 0; no TU | Red: both accepted; ASan heap-use-after-free |

### P3 — Raw-deref move-outs (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-8 | `rawderefprojection` (`DEF-437`) | The store (`let v: String = (*p).s`), the return, the struct-literal field, and the take argument; the conditional-arm control | Each moves the pointee once through the materializer and all three emission gates; the read-backs are correct; ASan/LSan clean; the conditional arm stays green | Red: accepted; abort 134 + ASan double-free on each isolated form |
| ST-9 | `guardedtargetalias` (`DEF-439`) | `v = (g && eat(take (unsafe *p))) ? x : y` with `p = &v`; `g = true`; `g = false` (aliasing) with distinct lengths (`v = "vv"`, `y = "yyyy"`); the non-aliasing control (`p = &w`) | The `g = true` selection runs 0; the distinct-length `g = false` selection runs 2 (it reads the stored `y`); the non-aliasing control drops the save; ASan/LSan clean; the false arm drops the save normally and stores `y`; the taken path does not free the consumed save; the emitted alias condition is pinned | Red: `g = true` abort 134 + ASan double-free; `g = false` runs 2 (it reads the stored `y`) and the non-aliasing form runs 0 (controls) |
| ST-10 | `guardedintrinsicderef` (`DEF-440`) | `while (i < 2 && len(*alt(p,q)) == 2)`; the `i32(len(...))` spelling; `read((*alt(p,q)).s)`; the impure call counter | Two iterations, no double free; `alt` once per evaluation; the borrow argument is a read; ASan/LSan clean | Red: abort 134 + ASan double-free (intrinsic and borrow argument) |
| ST-11 | `guardedmembertake` (`DEF-441`) | `g && (eat(take (*alt(p,q)).name) == 1)`; both selections; the two-owned-member sibling read; the impure call counter | The taken member is emptied, the sibling stays owned and readable; the false path does not call; the pointer runs once; ASan/LSan clean | Red: abort 134 + ASan double-free; the root-wide empty loses the sibling |

### P4 — Store lowering and evaluation order (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-12 | `derefrootedstore` (`DEF-421`) | `(*p).f`, `(*p).f.x`, `(*p).arr[i].x`, `unsafe (*p).f`; `(*p).arr[idx].y = bump()` with a call counter; the block controls; the conditional-base rejection; the structural `.len` target | Each store emits and reads back the value (run 0); the indexed form stores at the pre-call index and calls `bump` once; the conditional base keeps one `E10221`; the structural `.len` target keeps its `E10221`; the block controls stay green; hosted-amiga compiles the accepted forms | Red: accepted; the store absent; runs 249/249/251; the `unsafe` spelling newly accepted and dropped; `q423bump` exits 249 |
| ST-13 | `elementtargetorder` (`DEF-423`) | `a[idx] = mkrow()` (mkrow sets idx = 5); `a[idx] = set_idx() ? a[idx] : Elem {...}` with a destination read-back (and the no-read `at2` form); the out-of-range store; the member-route control; the `at1` source read as a residual | The stores land in the pinned in-bounds element; the conditional self-store is a legal move with a correct destination read; the out-of-range store traps `E11211` on the plain and ASan builds; the member control unchanged; `at1` is recorded as an accepted use-after-move residual, not fixed | Red: SEGV 139 / double-free 134; ASan stack-buffer-overflow / double-free; the member-route equivalent reads the emptied source, so the case reads the destination (AR-18) |
| ST-14 | `fixedarrayselfread` (`DEF-424`) | `a[i].names = [stringOf(a[i].names[0]), ...]`; the two-element self-read; `a[i].inner = *(&a[i].inner)` and `x = *(&x)`; the distinct-binding control `x = *(&y)` and the distinct-place control `a[0].inner = *(&a[1].inner)`; a following distinct-value store through each self-alias place; the generic constant-index self-copy; the concrete constant-index control | The self-read stores run 0 with their values; the self-copy and raw self-alias are no-ops with the value intact; the distinct-binding/place controls read the other value back (an over-broad no-op leaves the old value and fails); the following distinct-value store lands; the concrete `E10026` unchanged; ASan/LSan clean; the RHS temporaries are zeroed and moved out once | Red: SEGV 139 on all self-read/self-alias forms; the concrete control `E10026` |
| ST-15 | `derefrhistore` (`DEF-436`) | `xs[i] = *alt(p,q)`; `unsafe *alt(rp,rq) = *alt(p,q)`; the named-store control; the duplicable control | Each pointer expression is called once (one for the element form, two total for the raw-target form); the stored value is correct; ASan/LSan clean; the controls stay green | Red: abort 134 + ASan double-free (element); the raw-target probe stores the wrong value (100, calls = 3) |

### P5 — Per-instance generic judgments (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-16 | `genericconditionalstore` (`DEF-410`) | `pick<T>(c, h) { let r: String? = c ? h : null; }` at `T = String`; the separate duplicable generic whole move (`let r: T = h;` at `T = P`, the `q410dup` struct); the concrete twin; the simple generic whole move at `T = String` | One `E10034` at the store for the owning instance; no TU; the duplicable generic whole move runs; the concrete twin keeps its report; the simple generic move keeps its report; no report at the definition | Red: accepted, abort 134 + ASan double-free; the concrete twin `E10034` (6:5); the simple move `E10034` (6:5); `q410dup` runs 0 clean |
| ST-17 | `genericdivisionguard` (`DEF-426`) | The generic and concrete `/=`, `%=` member and element twins; `/0`; `INT_MIN / -1`; `INT_MIN % -1` | The same guards in both twins (resolved through `resolved_place_ty`/`resolved_value_ty`); each fault traps `E11210` on the plain and ASan builds; no SIGFPE; the value results unchanged | Red: the generic emits 0 guards (concrete 3); `/0` SIGFPE 136 |

`DEF-425` is parked; its park evidence is the existing batch-M cases and the `d425a`/`d425b`/
`d425d` probes, recorded in phase 5 and the close-out (AR-22).

### P6 — Literal destinations (`03-06`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-18 | `conditionalliteralread` (`DEF-445`) | The read base `(ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]`; the mixed `(ok ? place : Inner { row: n2() }).row[0][0]`; the mixed nested field and nested element; the concrete brace-list control | All build strictly and run 0 (OR rule: any arm needing stores selects the store form); each call once; the concrete control byte-identical | Red: accepted; strict build fails `too few arguments`; the mixed form fails the same way |
| ST-19 | `ternarytakeliteral` (`DEF-446`) | `g ? consumeO(ok ? Outer {...n1()...} : Outer {...n2()...}) : 0`; both selections; the `if`-condition control | The ternary form builds strictly and runs 0; each call once; the `if`-condition control stays green | Red: strict build fails `too few arguments`; the `if` form runs 0 |
| ST-20 | `conditionalarraytake` (`DEF-448`) | `consume(take (ok ? [a, b] : [c, d]))` with `str[2]` places; both selections; the unconditional `[a, b]` control; the `String[2]`/`Box[2]` controls | The conditional form builds strictly and reads the chosen arm's values (run 0); the controls stay green | Red: strict build fails `expected expression before '{'`; the other-type controls run 0 |

### P7 — Control resolution (`03-07`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-21 | `viewlengthcontrol` (`DEF-427`) | The frozen view-length matrix `len(w.items[i])` (concrete, generic, borrow, loop alias) and `len(w.items[i].name)`; the filed byte-index probe `len(w.items[i][0])` as an accepted run | The view-length readings are 2 at every spelling and run 0; the filed probe is accepted, builds strictly, and is ASan/LSan-clean (its exit value is not asserted); the exact `254`/`0` output is asserted only in the implementation pin; a future scalar-`len` fix needs a user ruling | **Green control**: the view matrix runs 0; the filed double index is accepted and ASan/LSan-clean; the exact output is an impl pin |
| ST-22 | `inlineownedliteral` (`DEF-447`) | The filed `G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0][0]`; the call-row/nested-field, view-copy, and `String`-field variants | All build strictly and run 0; ASan/LSan clean; the body-scope read-base binding is pinned | **Green control**: run 0, LSan-clean at the head |

## Test Categories

### Specification Tests

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (append) | ST-1–ST-15, ST-17–ST-20, ST-22; the `E11211`/`E11210` traps with the inline `assert_traps` helper | P1–P4, P6, `DEF-426`, `DEF-447` |
| `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs` (append) | ST-16 | P5 (`DEF-410`) |
| `compiler/tests/array_value_integrity_{spec,impl}_test.rs` (append) | ST-21; add the suite's `run_memory` helper | P7 (`DEF-427`) |
| `compiler/tests/checker_conformance_{spec,impl}_test.rs`, `callable_release_integrity_{spec,impl}_test.rs`, `module_initializer_integrity_{spec,impl}_test.rs`, `closure_integrity_{spec,impl}_test.rs`, and the other batch-I–M origin suites | No additions — regression oracles (AR-30) | Regression |
| `compiler/tests/run_fault_spec_test.rs` | No additions — the frozen fault data and the Amiga fault list stay untouched | Oracle only |
| The frozen spec suites and the origin suites (existing cases) | Regression oracle | Batches I–M |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| The origin `_impl_test.rs` files (append) | The loan record paths and report code/count/span; the `Element(None)` wildcard record and its three-reader match (including the non-constant branch's union for `E10031`); the temp-owner records (including `__cl_owned0`); the binding-loan fallback (copy and direct-call forms); the per-pointer-expression call count and projected-member `MoveOutValue` placement; the runtime alias condition and the false arm's store of `y`; the store target projection and pinned base/index; the self-copy/self-alias skip through `is_raw_self_alias` (including a distinct-place refusal); the exact `254`/`0` scalar-`len` output; the `E10034` records and duplicable generic whole-move acceptance at `T = P`; the division guard text/count through the resolved projection type; the literal destination and per-arm store (including the mixed read base); the call counters; the controls' emitted scalar `.len` and literal binding; and the re-emission determinism pins. Only byte-identity pins may be re-pinned; spec cases never (AR-30) | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own + sema | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen + own | Base/head over the corpus and the named phase probe sets (P1/P2 loan shapes; P3 deref/take; P4 stores; P5 generic/concrete pairs; P6 literals; P7 controls); every delta classified |
| Memory suite runs | the suite `RDC_ASAN=1` switch | P1–P6 red/accepted shapes, the P7 `DEF-447` control, and the P7-427 control after the array-value `run_memory` helper lands |
| Profile legs | driver + toolchains | The accepted store forms and the literal forms on hosted-amiga (compile-only); the `String`-importing probes are recorded as freestanding-ineligible (`E11200`), with the per-case reason and the eligible scalar forms compiled freestanding at the P4/P6 records and 8.1.1 |

### End-to-End Tests

| Scenario | Steps | Expected Result |
| -------- | ----- | --------------- |
| Driver run of the phase cases | `clorraine run` on the case sources (host) | Exit per the ST expectation |
| Bounds fault | The `DEF-423` out-of-range store with the inline `assert_traps` helper | `error E11211: index out of range` on the plain and ASan builds |
| Division fault | The `DEF-426` `/0` and MIN/-1 cases with the inline `assert_traps` helper | `error E11210: integer division by zero` on the plain and ASan builds |
| Close-out acceptance | `make verify` | All legs pass (host, hosted-amiga, freestanding) |

## Test Data

### Fixtures Needed

- The P1 fixtures: the `Box`/`str[2]`/`str[2][2]` loaned elements, the exact self-assignment pair,
  the composite conditional/array sources (constant and runtime), the constant sibling, the
  deeper-projection fill, and the element-store literal.
- The P2 fixtures: the runtime-index store reader (with the wildcard and sibling controls), the
  `mk()` temporary element (`__cl_owned0`), and the `pick(xs)` call-mediated view (both forms plus
  a caller-parameter view).
- The P3 fixtures: the raw-pointee projection (store/return/field/take), the guarded take aliasing
  the conditional target (both selections plus the non-aliasing control), the guarded intrinsic
  loop and borrow argument (with the call counter), and the guarded member take (both selections
  plus the two-owned-member sibling).
- The P4 fixtures: the `Deref`-rooted stores (four forms, the indexed `bump` form, and the
  conditional-base rejection), the fixed-array call RHS and the conditional-order fixtures
  (`at1`/`at2` with the destination read-back and the `at1` residual), the
  `au1`/`ak1`/`ak2`/`x = *(&x)` self-reads, the distinct-binding/place and following distinct-value
  controls, the generic constant-index self-copy, and the impure `alt` stores.
- The P5 fixtures: the generic conditional store, the separate duplicable generic whole move at
  `T = P` (`q410dup`), and the generic/concrete division pairs with the fault values. `DEF-425` uses
  the existing batch-M fixtures as park evidence.
- The P6 fixtures: the `str[2]` call-valued literals (read base, mixed place/literal, nested
  field/element, ternary arm, `if`-condition control), the conditional array of `str[2]` places,
  and the `String`/`Box` controls.
- The P7 fixtures: the `str[2]`/`String[2]` view-length matrix (concrete, generic, borrow, loop
  alias, member), the filed byte-index probe (accepted and sanitizer-clean; exact output in the
  impl pin), and the filed inline read-base literal plus its variants.
- The real compiler, C toolchain, ASan, the m68k toolchain, and the driver — no mocks.

## Verification Gates

| Gate | Command | Applies |
| ---- | ------- | ------- |
| Fast verification | `make check` | Every phase |
| Focused suite (P1–P4, P6, P5-426, P7-447) | `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | P1–P4, P6, `DEF-426`, `DEF-447` |
| Focused suite (P5-410) | `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test --test borrow_generic_integrity_impl_test"` | P5 (`DEF-410`) |
| Focused suite (P7-427) | `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test --test array_value_integrity_impl_test"` | P7 (`DEF-427`; add the `run_memory` helper first) |
| ASan suite leg | `make -C docker exec CMD="RDC_ASAN=1 cargo test --test <suite>"` | P1–P6, P7-447, and P7-427 after the helper |
| Host ASan probes | `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The red shapes (before the fix) and the accepted controls (after) |
| Strict build | `clc --profile host -o out.c prog.clo && cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out.bin out.c` | Every accepted shape |
| Trap case | the inline `assert_traps` helper | `DEF-423` (`E11211`), `DEF-426` (`E11210`) |
| Hosted-amiga leg | `clc --profile hosted-amiga` + `m68k-amigaos-gcc` | The P4 store forms; the P6 literal forms (compile-only) |
| Freestanding record | `clc --profile freestanding` + the per-case reason capture | The eligible scalar P4/P6 forms compile; each `E11200` probe records its ineligibility reason (P4/P6 records and 8.1.1) |
| Emission differential | per-phase clc+cc over the named probe sets and the corpus | P3, P4, P5, P6 (and P1/P2 when a loan record changes an accepted emission) |
| Full acceptance | `make verify` | Close-out |

## Verification Checklist

- [ ] All specification test cases (ST-*) defined with concrete input/output pairs
- [ ] Every ST case traces to a requirement, spec doc, or AR entry
- [ ] Specification tests written BEFORE implementation
- [ ] Specification tests verified to FAIL before implementation (the controls verified green
      first — AR-27, AR-28)
- [ ] All specification tests pass after implementation (green phase)
- [ ] Implementation tests written for edge cases and internals
- [ ] All unit / integration / E2E tests pass
- [ ] No regressions in existing tests (the origin suites, the frozen spec suites, the batch-I–M
      oracles)
- [ ] The controls (`DEF-427`, `DEF-447`) pin the closure evidence: the view-length matrix and the
      filed literal are green; the filed byte probe is accepted and sanitizer-clean, with the exact
      output in the implementation pin
- [ ] Test coverage meets goals
