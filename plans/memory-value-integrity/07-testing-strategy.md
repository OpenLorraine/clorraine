# Testing Strategy: Memory and Value Integrity (Batch N)

> **Document**: 07-testing-strategy.md
> **Parent**: [Index](00-index.md)
> **Baseline**: head `f6da78f`

## Testing Overview

### Coverage Goals

| Area | Target |
| ---- | ------ |
| Loan transfer | A stored copy of a view keeps every source position: the exact self-assignment, the composite conditional/array source at an element/projected destination, the deeper projection after a shorter recording, and the element-store nested fill each report `E10030` at the escaping fill and stay accepted otherwise |
| Loan sources | A runtime-index store's union reaches a constant-read copy; a temporary-derived element records its owner; a call-mediated element view consults the binding loans; each escape reports `E10030` |
| Deref move-outs | A projection of a raw pointee, a guarded take aliasing the conditional target, a guarded borrowing intrinsic, and a guarded take of a member over a raw pointer each release exactly once; the chosen path evaluates, the false path does not; ASan/LSan clean |
| Store lowering | The `Deref`-rooted member store emits (never drops); the whole-element target is pinned before the RHS; the fixed-array literal self-read runs before the clear; the constant-index self-copy is skipped; the element/raw-target RHS materializes once |
| Per-instance | The conditional by-value move and the borrowed element move report one `E10034` for an owning instance and accept a duplicable one; the generic compound division emits the `E11210` guard and traps, never SIGFPE |
| Literal destinations | The conditional-of-literals read base, the ternary-arm take literal, and the conditional array literal of places build strictly and run with the chosen values; each call once; the controls stay byte-identical |
| Controls | `DEF-427`'s view-length matrix reads 2 in every spelling; `DEF-447`'s filed literal and variants are LSan-clean; no code change |

- Test names state behavior (`should_…`), as the existing suites do.
- **The new cases go into the ORIGIN suites, additions only** (AR-30):
  - `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs`: ST-1–ST-15, ST-18–ST-21,
    and ST-23 (P1–P4, P6, `DEF-426`, `DEF-447`).
  - `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs`: ST-16–ST-17 (`DEF-410`,
    `DEF-425`, the per-instance lend family).
  - `compiler/tests/array_value_integrity_{spec,impl}_test.rs`: ST-22 (`DEF-427`, the
    fixed-array/`len` origin).
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
| ST-2 | `compositeelementstore` (`DEF-429`) | `grid[0] = c ? xs : zs`; `out2[0] = c ? xs[0] : zs[0]`; `g[0] = [xs[0], xs[1]]`; the direct conditional twin | One `E10030` at each affected fill; the copies stay accepted; the direct twin keeps its report; no TU | Red: all accepted; ASan heap-use-after-free |
| ST-3 | `deeperprojectioncopy` (`DEF-431`) | `rows[0] = xs; let s = rows[0][0]; fill(s)`; the direct `fill(rows[0][0])` control; the fill-record variant | One `E10030` at the copy's fill; the direct control keeps its report; the fill-record variant reports at its fill; no TU | Red: accepted; ASan heap-use-after-free; the direct control `E10030` (15:5) |
| ST-4 | `elementstorenestedfill` (`DEF-435`) | `g[1] = [; a.v]` plus the copy and the direct read; the literal-fill form | One `E10030` at the copy's fill; the direct read keeps its report; the store is accepted; no TU | Red: accepted; ASan heap-use-after-free; the direct read `E10030` (13:5) |

### P2 — Loan sources (`03-02`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-5 | `runtimestorecopy` (`DEF-430`) | `xs[k] = a.v` with `k = 1`, then `let s = xs[1]; fill(s)`; the `k = 0` variant; the constant-index store control; the direct runtime fill | One `E10030` at the copy's fill; the constant-index control keeps its report; the direct form is recorded per AR-8; no TU | Red: accepted; ASan heap-use-after-free; the constant-index control `E10030` (13:5) |
| ST-6 | `temporaryelementcopy` (`DEF-432`) | `let ys: str[2] = [mk().v; "EE"]; fill(ys[0])`; the direct `fill(mk().v)` twin; an in-scope read | One `E10030` at the copy's fill; the direct twin keeps its report; the in-scope read runs 0; no TU | Red: accepted; ASan heap-use-after-free; the direct twin `E10030` (12:5) |
| ST-7 | `callviewresult` (`DEF-433`) | `let s = pick(xs); fill(s)` and `fill(pick(xs), slot)`; the direct `fill(xs[0])` control; an in-scope result | One `E10030` at each affected fill; the direct control keeps its report; the in-scope result runs 0; no TU | Red: both accepted; ASan heap-use-after-free |

### P3 — Raw-deref move-outs (`03-03`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-8 | `rawderefprojection` (`DEF-437`) | The store (`let v: String = (*p).s`), the return, the struct-literal field, and the take argument; the conditional-arm control | Each moves the pointee once; the read-backs are correct; ASan/LSan clean; the conditional arm stays green | Red: accepted; abort 134 + ASan heap-use-after-free |
| ST-9 | `guardedtargetalias` (`DEF-439`) | `v = (g && eat(take (unsafe *p))) ? x : y` with `p = &v`; `g = true`/`false`; a pure-pointer variant | Both selections run 0; ASan/LSan clean; the false path leaves v intact; the taken path does not free the consumed save | Red: `g = true` abort 134 + ASan double-free; `g = false` runs 0 (control) |
| ST-10 | `guardedintrinsicderef` (`DEF-440`) | `while (i < 2 && len(*alt(p,q)) == 2)`; the `i32(len(...))` spelling; the impure call counter | Two iterations, no double free; `alt` once per condition evaluation; ASan/LSan clean | Red: abort 134 + ASan double-free |
| ST-11 | `guardedmembertake` (`DEF-441`) | `g && (eat(take (*alt(p,q)).name) == 1)`; both selections; the other member read-back | The taken member is emptied; the false path does not call; the other member stays owned; ASan/LSan clean | Red: abort 134 + ASan heap-use-after-free |

### P4 — Store lowering and evaluation order (`03-04`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-12 | `derefrootedstore` (`DEF-421`) | `(*p).f`, `(*p).f.x`, `(*p).arr[i].x`, `unsafe (*p).f`; the block controls; the conditional-base rejection | Each store emits and reads back the value (run 0); the conditional base keeps one `E10221`; the block controls stay green; hosted-amiga compiles the accepted forms | Red: accepted; the store absent; runs 249/249/251; the `unsafe` spelling newly accepted and dropped |
| ST-13 | `elementtargetorder` (`DEF-423`) | `a[idx] = mkrow()` (mkrow sets idx = 5); `a[idx] = set_idx() ? a[idx] : Elem {...}` with a destination read-back (and the no-read `at2` form); the out-of-range store; the member-route control | The stores land in the pinned in-bounds element; the conditional self-store is a legal move with a correct destination read; the out-of-range store traps `E11211` on the plain and ASan builds; the member control unchanged | Red: SEGV 139 / double-free 134; ASan stack-buffer-overflow / double-free; the member-route equivalent reads the emptied source, so the case reads the destination (AR-18) |
| ST-14 | `fixedarrayselfread` (`DEF-424`) | `a[i].names = [stringOf(a[i].names[0]), ...]`; `a[i].inner = *(&a[i].inner)`; the generic constant-index self-copy; the concrete constant-index control | The self-read stores run 0 with their values; the self-copy runs 0 with the value intact; the concrete `E10026` unchanged; ASan/LSan clean | Red: SEGV 139 on all three; the concrete control `E10026` |
| ST-15 | `derefrhistore` (`DEF-436`) | `xs[i] = *alt(p,q)`; `unsafe *alt(rp,rq) = *alt(p,q)`; the named-store control; the duplicable control | `alt` is called once; the stored value is correct; ASan/LSan clean; the controls stay green | Red: abort 134 + ASan double-free (element); the raw-target probe stores the wrong value (100, calls = 3) |

### P5 — Per-instance generic judgments (`03-05`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-16 | `genericconditionalstore` (`DEF-410`) | `pick<T>(c, h) { let r: String? = c ? h : null; }` at `T = String`; the duplicable instance; the concrete twin; the simple generic whole move | One `E10034` at the store for the owning instance; no TU; the duplicable instance runs; the concrete twin keeps its report; the simple generic move keeps its report | Red: accepted, abort 134 + ASan double-free; the concrete twin `E10034` (6:5); the simple move `E10034` (6:5) |
| ST-17 | `genericelementborrow` (`DEF-425`) | `copy<T>(borrow w: T, i, j) { w.items[i].name = w.items[j].name; }` at owning `T`; the duplicable instance; the concrete borrow/take twins | One `E10034` at the RHS element read for the owning instance; the duplicable instance runs; the concrete twin keeps `E10034`; the take twin keeps `E10026` | Red: accepted, run 0; the concrete `E10034` (9:23); the take `E10026` (9:23) |
| ST-18 | `genericdivisionguard` (`DEF-426`) | The generic and concrete `/=`, `%=` member and element twins; `/0`; `INT_MIN / -1`; `INT_MIN % -1` | The same guards in both twins; each fault traps `E11210` on the plain and ASan builds; no SIGFPE; the value results unchanged | Red: the generic emits 0 guards (concrete 3); `/0` SIGFPE 136 |

### P6 — Literal destinations (`03-06`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-19 | `conditionalliteralread` (`DEF-445`) | The read base `(ok ? Inner { row: n1() } : Inner { row: n2() }).row[0][0]`; the nested field; the nested element; the concrete brace-list control | All build strictly and run 0; each call once; the concrete control byte-identical | Red: accepted; strict build fails `too few arguments` |
| ST-20 | `ternarytakeliteral` (`DEF-446`) | `g ? consumeO(ok ? Outer {...n1()...} : Outer {...n2()...}) : 0`; both selections; the `if`-condition control | The ternary form builds strictly and runs 0; each call once; the `if`-condition control stays green | Red: strict build fails `too few arguments`; the `if` form runs 0 |
| ST-21 | `conditionalarraytake` (`DEF-448`) | `consume(take (ok ? [a, b] : [c, d]))` with `str[2]` places; both selections; the unconditional `[a, b]` control; the `String[2]`/`Box[2]` controls | The conditional form builds strictly and reads the chosen arm's values (run 0); the controls stay green | Red: strict build fails `expected expression before '{'`; the other-type controls run 0 |

### P7 — Control resolution (`03-07`)

| # | Case (row) | Input / Scenario | Expected Output / Behavior | Red evidence / controls |
|---|-----------|------------------|----------------------------|-------------------------|
| ST-22 | `viewlengthcontrol` (`DEF-427`) | The view-length matrix `len(w.items[i])` (concrete, generic, borrow, loop alias) and `len(w.items[i].name)`; the byte-index variant `len(w.items[i][0])` | The view-length readings are 2 at every spelling and run 0; the byte variant asserts `len(...) == 0` (the `u8` read; the probe's `-2` returns 254); no memory error | **Green control**: the view matrix runs 0; the filed double index runs 254, ASan 254 |
| ST-23 | `inlineownedliteral` (`DEF-447`) | The filed `G { rows: [n1(), n1()], tag: stringOf("x") }.rows[0][0][0]`; the call-row/nested-field, view-copy, and `String`-field variants | All build strictly and run 0; ASan/LSan clean; the body-scope read-base binding is pinned | **Green control**: run 0, LSan-clean at the head |

## Test Categories

### Specification Tests

| Test File | ST Cases Covered | Component |
| --------- | ---------------- | --------- |
| `compiler/tests/conditional_projection_residue_{spec,impl}_test.rs` (append) | ST-1–ST-15, ST-18–ST-21, ST-23; the `E11211`/`E11210` traps with the inline `assert_traps` helper | P1–P4, P6, `DEF-426`, `DEF-447` |
| `compiler/tests/borrow_generic_integrity_{spec,impl}_test.rs` (append) | ST-16–ST-17 | P5 (`DEF-410`, `425`) |
| `compiler/tests/array_value_integrity_{spec,impl}_test.rs` (append) | ST-22 | P7 (`DEF-427`) |
| `compiler/tests/checker_conformance_{spec,impl}_test.rs`, `callable_release_integrity_{spec,impl}_test.rs`, `module_initializer_integrity_{spec,impl}_test.rs`, `closure_integrity_{spec,impl}_test.rs`, and the other batch-I–M origin suites | No additions — regression oracles (AR-30) | Regression |
| `compiler/tests/run_fault_spec_test.rs` | No additions — the frozen fault data and the Amiga fault list stay untouched | Oracle only |
| The frozen spec suites and the origin suites (existing cases) | Regression oracle | Batches I–M |

### Implementation Tests

| Test File | Description | Priority |
| --------- | ----------- | -------- |
| The origin `_impl_test.rs` files (append) | The loan record paths and report code/count/span; the runtime union and temp-owner records; the binding-loan fallback; the single `cl_alt` call and `MoveOutValue` placement; the store target projection and pin; the self-copy skip; the `E10034` records and duplicable acceptance; the division guard text/count; the literal destination and per-arm store; the call counters; the controls' emitted `.len` and literal binding; and the re-emission determinism pins. Only byte-identity pins may be re-pinned; spec cases never (AR-30) | High |

### Integration Tests

| Test | Components | Description |
| ---- | ---------- | ----------- |
| Corpus strict build | codegen + own + sema | Every corpus/staged/example program builds under `-Werror` on the pinned container |
| Emission differential | codegen + own | Base/head over the corpus and the named phase probe sets (P1/P2 loan shapes; P3 deref/take; P4 stores; P5 generic/concrete pairs; P6 literals; P7 controls); every delta classified |
| Memory suite runs | the suite `RDC_ASAN=1` switch | P1–P6 red/accepted shapes and the P7 `DEF-447` control |
| Profile legs | driver + toolchains | The accepted store forms and the literal forms on hosted-amiga (compile-only); the `String`-importing probes are recorded as freestanding-ineligible (`E11200`) |

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
  the composite conditional/array sources, the deeper-projection fill, and the element-store
  literal.
- The P2 fixtures: the runtime-index store reader, the `mk()` temporary element, and the
  `pick(xs)` call-mediated view (both forms).
- The P3 fixtures: the raw-pointee projection (store/return/field/take), the guarded take aliasing
  the conditional target (both selections), the guarded intrinsic loop (with the call counter),
  and the guarded member take (both selections).
- The P4 fixtures: the `Deref`-rooted stores (four forms plus the conditional-base rejection), the
  fixed-array call RHS and the conditional-order fixtures (`at1`/`at2` with the destination
  read-back), the `au1`/`ak1`/`ak2` self-reads and
  the generic constant-index self-copy, and the impure `alt` stores.
- The P5 fixtures: the generic conditional store, the generic borrow element move, and the
  generic/concrete division pairs with the fault values.
- The P6 fixtures: the `str[2]` call-valued literals (read base, nested field/element, ternary
  arm, `if`-condition control), the conditional array of `str[2]` places, and the `String`/`Box`
  controls.
- The P7 fixtures: the `str[2]`/`String[2]` view-length matrix (concrete, generic, borrow, loop
  alias, member), and the filed inline read-base literal plus its variants.
- The real compiler, C toolchain, ASan, the m68k toolchain, and the driver — no mocks.

## Verification Gates

| Gate | Command | Applies |
| ---- | ------- | ------- |
| Fast verification | `make check` | Every phase |
| Focused suite (P1–P4, P6, P5-426, P7-447) | `make -C docker exec CMD="cargo test --test conditional_projection_residue_spec_test --test conditional_projection_residue_impl_test"` | P1–P4, P6, `DEF-426`, `DEF-447` |
| Focused suite (P5-410/425) | `make -C docker exec CMD="cargo test --test borrow_generic_integrity_spec_test --test borrow_generic_integrity_impl_test"` | P5 |
| Focused suite (P7-427) | `make -C docker exec CMD="cargo test --test array_value_integrity_spec_test --test array_value_integrity_impl_test"` | P7 |
| ASan suite leg | `make -C docker exec CMD="RDC_ASAN=1 cargo test --test <suite>"` | P1–P6 and P7-447 |
| Host ASan probes | `cc -std=c99 -O0 -g -fsanitize=address -no-pie` + `ASAN_OPTIONS=detect_leaks=1:symbolize=0 timeout 30` | The red shapes (before the fix) and the accepted controls (after) |
| Strict build | `clc --profile host -o out.c prog.clo && cc -std=c99 -O0 -g -Wall -Wextra -Werror -o out.bin out.c` | Every accepted shape |
| Trap case | the inline `assert_traps` helper | `DEF-423` (`E11211`), `DEF-426` (`E11210`) |
| Hosted-amiga leg | `clc --profile hosted-amiga` + `m68k-amigaos-gcc` | The P4 store forms; the P6 literal forms (compile-only) |
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
- [ ] The controls (`DEF-427`, `DEF-447`) are green and pin the closure evidence
- [ ] Test coverage meets goals
