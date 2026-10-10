# Findings: Evaluation and Emission Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Protocol**: per-phase reviews follow the `AGENTS.md` dispatch matrix; each finding gets a
> ruling (fixed in-batch, filed as a roadmap row, or dismissed with evidence) before its phase
> closes.

## Phase 1: Module-initializer integrity (DEF-117, DEF-121, DEF-122)

**Reviewers**: correctness reviewer (RV), emission-integrity auditor (EIA), memory-safety auditor
(MS). Diff reviewed: `135cfc5..655d7bd`.

### Fixed in-batch (the review's blocking findings)

| # | Sev | Finding | Ruling |
| - | --- | ------- | ------ |
| RV-001 / EIA (O1) | 🔴 | The new `\|\|` restructure initialized its result local to `false`, so a true left operand yielded `false`. | **Fixed**: the result local starts at the operator's left-decided value (`false` for `&&`, `true` for `\|\|`); both spellings are pinned. |
| RV-002 / EIA-1 / MS (O1) | 🔴 | `materialize_guarded_sub_operands` hoisted lazy-position operand stores (`?:` arms, `&&`/`\|\|` right sides) before the construct, so a skipped operand's call ran — a behavior regression for previously working programs. | **Fixed**: the merged walk stores operands under branch guards (`if (c) { … }`) and never hoists out of a lazily evaluated position; side-effect counts are pinned. |
| RV-003 / MS (O4) | 🔴 | Two materialization passes ran before the render, so a right-side bind ran before an earlier left-side operand store (left-to-right order violated for newly accepted shapes). | **Fixed**: one left-to-right walk (`materialize_init_value`) binds and stores at each sub-expression's source position; an order-counting pin covers the mixed case. |
| RV-004a / EIA-2 | 🟠 | The short-circuit restructure dropped its enclosing guard, so a skipped arm's left operand ran. | **Fixed**: the decision combines the enclosing guard with the left operand (`if ((guard && left))` / `(guard && !left)`) and the left's own binds/stores stay under the guard. |
| RV-004b / EIA-3 | 🟠 | The short-circuit left operand was not single-evaluated (its guards re-read impure operands), and the right side's operands ran unstored. | **Fixed**: the left renders through embedded guards (`guarded_value`) over stored operands; the right side is fully walked inside the decision block. |
| EIA-4 / RV-P2 (fill and scalar literal positions) | 🟠 | Fixed-array calls inside array-literal elements, struct-literal fields, and template placeholders still emitted destination-less calls (base-identical, so pre-existing, but inside the changed lane). | **Partially fixed**: the walk descends `ArrayLit`, `StructLit`, and `Template`, and the probes build and run (literal element, row literal, template argument). A struct-literal fixed-array *field* still renders an invalid compound literal (`(ClBox){ __cl_itmp0 }`; the same spelling fails for a placed field with no call at all, and the function body emits the statement-form store) — that remaining renderer gap is filed as **DEF-134**. |
| EIA-5 | 🟠 | A fill conditional's condition was stored without binding a fixed-array call inside it. | **Fixed**: the condition is walked before its store, in both the fill and scalar paths. |
| RV-P3 | 🟡 | `collect_init_slice_types` skipped written intrinsic type arguments (`sizeof(i32[])`). | **Fixed**: the `IntrinsicArg::Type` arm resolves and collects. |
| RV-P4 | 🟡 | The literal-slice fix added a follow-on `E10200` next to the fill's `E10215`, and a view-return spelling now reports `E10200` instead of the signature-level `E10036`. | **Partially accepted**: the extent replacement is scoped to fill-free literals, so a fill target keeps its single `E10215`; the view-return spelling's `E10200` is kept — the literal is not a view, the conversion report is the accurate one, and both spellings reject. Recorded as the fix's documented consequence. |
| MS (F3) | 🟡 | The `run_memory`/`run_asan` helpers were unused, so the module-initializer shapes never ran under the sanitizer even with `RDC_ASAN=1`. | **Fixed**: the Phase-1 specification cases run through `run_memory` (ordinary strict host build by default, ASan with `RDC_ASAN`). |

### Fixed as a consequence of the redesign

| # | Sev | Finding | Ruling |
| - | --- | ------- | ------ |
| RV-P1 / MS (F2) / EIA (P-3) | 🟠 | Impure operands under call arguments and inside array-literal elements evaluated multiple times (bounds/division guard read plus the rendered read), pre-existing at base. | **Fixed by the merged walk**: call arguments and literal elements are walked at their positions, so their impure operands store once. Probes cover the argument, element, and divisor spellings. |

### Filed (pre-existing classes outside the phase's rows; new rows on the roadmap)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| MS (F1) | 🟠 | Module initializers never release owned temporaries; the phase's new bindings extend the pre-existing leak class to newly runnable positions (member projection with an owned member, `String` element projection, an owned conditional arm). | DEF-130 |
| EIA-6 | 🟠 | A closure or func-typed-global callee in a module initializer emits an unresolved or empty callable (`(__cl_itmp0);`, or an undeclared plain name for a func-typed global). | DEF-131 |
| EIA (P-1) | 🟠 | A generic function call in a module initializer emits the plain module symbol; no instance is declared. | DEF-132 |
| EIA (P-2) | 🟠 | A callable-field call in a module initializer drops the receiver. | DEF-133 |
| EIA-4 (struct-literal remainder) | 🟡 | A module-initializer struct literal with a fixed-array field from a place emits an invalid compound literal; the function body emits the statement-form store. | DEF-134 |

### Verification evidence (fix round)

All reviewer reproductions re-run after the redesign: `\|\|` left-true result (exit 1), arm and `&&`/`\|\|` skip laziness (side-effect counts 0), left-to-right order (log 12), enclosing-guard skip (`f` not called), left single evaluation (one call), literal element (3), row literal (3), fill condition (4), template argument (1), call-argument operand (one call, value 7), `\|\|` skip/take (10/20 pair), plus the full specification and implementation suites and `make check`.

### Verification evidence (second fix round — slice guard and or-guard conjunct)

The scoped re-review of the redesign raised RV-R4 (the restored eager slice guard escaped lazy positions through the lowering-time materialization: a slice inside a skipped `?:` arm or `\|\|` operand aborted) and RV-R5 (the fill recursion re-emitted operand guards per literal level — behavior-neutral noise). Both are fixed: the intrinsic slice operands now store and guard inside the walk, at the evaluation site, under the enclosing guard; the lowering no longer binds them; and the fill discipline runs over already-walked values without re-running the walk.

| # | Sev | Ruling |
| - | --- | ------ |
| RV-R4 | 🔴 → fixed | The walk owns the slice-operand storage and the slice guard; the lowering-time `init_intrinsic_arg` binding is removed (it was context-blind). Reproductions: `pick() ? slice(arr,0,n).len() : 0` skipped → 0 (was abort), taken → `E11211`; `yes() \|\| (slice(...).len() > 0)` → 1; the nested spelling → 1; a skipped `slice(arr, f(), 2)` argument runs nothing. Pins: `should_guard_an_eager_slice_view`, `should_keep_a_slice_guard_lazy_under_an_enclosing_choice`. |
| RV-R5 | 🟡 → fixed | `prepare_walked_fill_value` applies the per-element discipline to walked values only, so guards and binds no longer multiply with literal depth. Pin: `should_single_evaluate_a_fill_element_operand` (one `idx()` call; the walk's guard plus the element store's guard are the only copies). |

### Verification evidence (third fix round — lazy slice operands)

The verification of the slice-guard fix raised RV-001 (a view-typed lazy part with an impure offset was cloned unwalked, so the offset ran three times — its embedded guard plus the construction twice — and a changing offset read out of bounds), RV-002 (a guarded non-scalar operand store initialized a struct with the scalar `0`), and a pin-spelling gap (the laziness pin used spellings the base already handled).

| # | Sev | Ruling |
| - | --- | ------ |
| RV-001 | 🔴 → fixed | The materialization predicate now mirrors the slice arm: a lazily evaluated view with an impure operand needs its guard structured, so the offset stores once under the branch guard and the construction and the embedded guard share it. Repro: `no() \|\| (slice(arr, f(), 2).len() > 0)` with a changing offset → one call, ASan-clean (was three calls and an out-of-bounds read; the reviewer's exact shapes re-run). Pin: `should_store_a_lazy_slice_offset_once`. |
| RV-002 | 🟠 → fixed | The guarded operand stores pass the value's empty state (`{0}`/`NULL`) instead of the scalar `0`, so a struct- or view-typed guarded operand builds. Pin: `should_build_a_guarded_nested_slice_operand` (covers the scalar and fill positions). |
| Pin spelling | 🟡 → fixed | The laziness pin now uses the base-failing spellings (`slice(...).len` and the nested slice) alongside the offset-once shape. |

**Review budget**: three review dispatches for Phase 1 (one full, two scoped) plus this fix round; all blocking findings fixed in-batch, pre-existing classes filed (DEF-130 – DEF-137).

### Filed (second fix round)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| PA-2 / EIA (a18) | 🟠 | A `str`-valued template module initializer assigns the concatenation result to a `ClStr` without the view wrapper, so the strict build fails. | DEF-135 |
| Re-review residual | 🟠 | An impure dividend or index base is re-read by its guard (`d() / 1` calls `d()` twice; `v(s)[1]` calls `v` twice), in module initializers and function bodies alike. | DEF-136 |
| RV-R3 note | 🟡 | A written slice type argument in a function body (`sizeof(i32[])`) misses its slice typedef; the module-initializer spelling is fixed, the body spelling is not. | DEF-137 |

### Dismissed with evidence

| # | Finding | Evidence |
| - | ------- | -------- |
| EIA (P-4) | Freestanding `bool` missing. | Already the batch's DEF-90 (Phase 2); the auditor's base-identical finding confirms the row's evidence. |

**Review budget**: one scoped re-review after the fix round (correctness + emission lenses on the
redesigned walk).

## Phase 2: Strict-build emission shapes (DEF-76, 97, 80, 90, 98, 112, 120, 128)

**Reviewers**: correctness reviewer (RV), emission-integrity auditor (EIA), memory-safety
auditor (MS). Diff reviewed: `259cda6..4dfbf22`. All three ran the base-tree red checks;
the memory audit found no in-scope defect (43 host ASan probes clean).

### Fixed in-batch (the review's blocking findings)

| # | Sev | Finding | Ruling |
| - | --- | ------- | ------ |
| RV-001 / EIA (F1) | 🟠 | The DEF-98 coercion wrapped an argument that `argument_text` already converted to a C pointer, so a `cstring` literal regressed to `C(("x").ptr)` — invalid C, a working program broken. | **Fixed**: the coercion now applies to `APTR` targets only and only when the argument actually crosses as a view (a slice/`str` value, or an owned `String` widening to `str`), conjoining the pointer with the ABI cast (`(APTR)(text).ptr`). The `cstring` literal control re-runs (`n = C("x")`), and the `str`, `String`-widening, and slice spellings are pinned. |
| RV-002 | 🟠 | A store to a structural `.len` (`xs.len = 4`, `+=`, `++`) passed the checker and was silently dropped by the emission (base failed loudly with invalid C) — a wrong program that ran. | **Fixed**: the store and update paths report `E10221` ("left side of an assignment must be a place") for a structural `.len` target; the three spellings are specification cases. |
| RV-003 | 🟡 | The `__cl_tmpN` reservation arm had no committed pin. | **Fixed**: a `__cl_tmp0` parameter plus a guard temporary pins the rename (`__cl_tmp0_1`) and the temporary's own spelling. |
| RV-P1 / EIA (P3) | 🟠 | `len(xs)` and `xs.len()` on a never-otherwise-read fixed-array local still failed the strict build (`-Wunused-but-set-variable`), and `len(mkBox().arr)` still evaluated its receiver root — the residual halves of the structural-`len` fold. | **Fixed**: the fold now covers every `len` spelling at the own layer (the intrinsic argument and the dot-call receiver fold to the compile-time extent before any lowering), so the base instance disappears and the local reads nothing. Pinned by the extended member-spelling case (`xs.len`, `len(xs)`, `xs.len()`) and the module-init projection probe. |

### Deliberate test update

- `array_value_integrity_impl_test.rs` (`ai6`): the pinned spelling of the structural extent moved
  from `(i32)(2)` to `(i32)(2u)`, the folded `usize` literal's suffix. The assertion's intent (the
  outermost extent is the constant 2) is preserved; the whitespace and suffix change is the fold's.

### Filed (pre-existing classes; new rows on the roadmap)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| MS (PRE-1) | 🔴 | A source local named `__cl_owned0` collides with the ownership layer's generated temporary of the same name in the name-keyed binding lookup; the accepted program double-frees (host ASan confirms; base identical). | DEF-138 |
| MS (PRE-2) | 🟠 | A field call through a temporary receiver (`H{…}.call(5)`, `makeH().call(5)`) leaks the receiver's owned sibling fields. | DEF-139 |
| MS (PRE-3) | 🟠 | Two instances of one generic struct with different `func` signatures share the type key `fn`, so `ClHolder__fn` is redefined and the accepted program does not build. | DEF-140 |
| EIA (P8) | 🟠 | `len` of a null `String` segfaults at run time (a null pointer read through the view widening). | DEF-141 |
| EIA (P5) | 🟡 | A slice of a handle-element array built only as an inline argument (`len(slice(mkS(), 0, 2))`) misses its `ClSlice_<T>` typedef. | DEF-142 |

### Verification evidence (Phase 2)

The base tree reproduces every ST-8 – ST-16 failure and the phase's pins fail there; the head runs
all 17 specification cases and 30 implementation pins green. The emission auditor's sweep (3543
emission pairs over 1181 programs × profiles) shows every corpus diff attributable to the eight
fixes and no build regression; the memory audit's 43 ASan probes are clean for every in-scope
shape. After the fix round: `make check` green (1526 tests), `./spec/check.sh` OK, the reviewers'
reproductions re-run (the `cstring` literal builds, the `str`/`String`/slice coercions build and
run, the three `.len` writes reject with one `E10221`).

**Review budget**: three review dispatches for Phase 2 (correctness, emission-integrity, memory)
plus the scoped re-review of this fix round; every blocking finding fixed in-batch with pins.

### Verification evidence (scoped re-review of the fix round)

Both lenses confirmed the round's items resolved (the cstring literal builds; the slice/`str`/`String`
coercions build and run; the `.len` writes reject; the `len(xs)`/`xs.len()`/projection folds build;
the corpus diff is exactly the intended set, and the strict builds are now failure-free for every
accepted corpus unit — 0 per profile, down from 1). The re-review raised two adjacent
silent-wrong-value/invalid-C shapes with base-identical evidence:

| # | Sev | Ruling |
| - | --- | ------ |
| Value-position structural-`.len` update | 🟠 → fixed | `let y: usize = xs.len++;` emitted `y = (3u)++;`. The value-position update check now routes the structural-`.len` target to the same not-a-place report. |
| Address-of a structural `.len` | 🟠 → fixed | `unsafe { let p = &xs.len; }` (inferred) emitted `p = &(3u);`; the address-of path now reports the same not-a-place error. |
| Read-only-parameter precedence | 🟡 → fixed | A write to `.len` through a read-only parameter now keeps its `E10024` (the mode report owns it), exactly as for any other write. |
| A func-typed field named `len` | 🟠 → fixed | `h.len(5)` was silently lowered as the builtin intrinsic (emitting `0`, dropping the call); the intrinsic dispatch now defers to a callable field of that name. Pinned with a sibling field proving the call value. |

The remaining report-only classes (a module-init field dot-call yielding 0; a `str`/slice at a
`cstring` parameter) are filed as DEF-143 – DEF-144; the rarer emission gaps (an extern func value,
a module-global callable field, a lambda capture of a fixed array, a `ClEnv_0` redefinition) are
recorded here with base-identical evidence for a future triage.

**Final Phase 2 verification**: 18 specification cases, 31 implementation pins, `./spec/check.sh`
OK, full `make check` green (1528 tests).

## Phase 3: Value typing and checker conformance (DEF-99, 111, 108, 107, 118, 129)

**Reviewers**: correctness reviewer (RV) and semantics reviewer (SR), per the phase's lens.
Diff reviewed: `9d7dc60..5364d2f`. Both ran the base-tree red checks (every ST-17 – ST-25 case is
red on `9d7dc60`; the new pins fail there) and confirmed the `E10043`/`E10212` spec edits match the
approved rows.

### Fixed in-batch (the review's blocking finding)

| # | Sev | Finding | Ruling |
| - | --- | ------- | ------ |
| RV-001 / SR-F1 | 🟠 | The new `E10043` guard rejected ANY non-array iterable, including a bare type parameter: `func each<T>(xs: T) { for (v in xs) { … } }` instantiated at an `i32[3]` or `i32[]` builds and runs on the base and was rejected at the declaration on head. AR-B8's rationale ("nothing working regresses") covers only the concrete non-iterable shapes (the `str` loop). | **Fixed** per the reviewer's recommended option (a): the guard rejects concrete non-iterable kinds and lets `Param`/`Unknown` iterables keep their accepted path (the instance decides). Reproduced: the generic loops run (exit 18) and the `str` loop still reports `E10043`. The control case now pins the generic instantiation. |

### Verdicts (no further findings)

- **DEF-107 (SR-1, confirmed)**: the frozen operator table covers scalars/enums/`raw`/handles; `str` and
  `static str` pairs report one `E10222` at the left operand, `!=` included; the null pair stays exempt
  in both orders; `cstring` keeps its pointer comparison per the AR-B7 user ruling (the reviewer's
  wording note on the table is recorded, no change).
- **DEF-108 (SR-4, confirmed)**: `void * x = NULL` (freestanding: `0`), the pairing reads the local
  against the view's pointer, and every listed spelling builds and runs.
- **DEF-129 (SR-3, confirmed)**: `E10212` for a bare-`Param` receiver, one report per callee site;
  bounded fields, free-function dot-calls, and `len` are unaffected; the `base.span` -> `callee.span`
  change is unobservable (no test asserts span ends).
- **DEF-99/111 (SR-5, confirmed)**: real per-instance lengths (7 vs base 3; count 2 vs base 0),
  nested and call-rooted projections resolve, concrete emissions stay byte-identical; the
  conditional-arm residual is filed (DEF-150).

### Deliberate test update

- `sema/tests.rs`: the unit test that asserted view equality now asserts the `E10222` rejection and
  keeps the null pairing (`views_have_no_equality_but_pair_with_null`). It cannot pass on the base.

### Filed (pre-existing classes; new rows on the roadmap)

| # | Sev | Finding | Filed as |
| - | --- | ------- | -------- |
| RV (P2) | 🟡 | A value-form member read on a bare param is accepted (`let y = x.foo;`). | DEF-146 |
| RV (P3) | 🟡 | A free-function dot-call arity mismatch is unchecked. | DEF-147 |
| SR (F3) | 🟡 | An unknown member callee on a `raw`/plain-struct receiver is accepted. | DEF-148 |
| SR (F4) | 🟡 | Assigning a view through the null-valued local is accepted. | DEF-149 |
| SR (F5) | 🟡 | A conditional-wrapped projection folds `len` to zero. | DEF-150 |
| RV (P1) | 🟡 | Uses of a null-inferred local as a typed value emit invalid C. | DEF-151 |
| SR (F1 residual) | 🟠 | A generic `T` iterable instantiated at a struct-element array emits invalid C. | DEF-145 |

### Verification evidence (Phase 3)

`./spec/check.sh` OK (27 files, 109 compile-fail tests, the `E10043` golden green in the
compile-fail suite); ST-17 - ST-25 green (27 cases); 34 implementation pins green; full
`make check` green (1541 tests) after the fix round. The reviewers' reproductions re-run: the
generic array/slice loops build and run, the `str` loop reports `E10043`.

**Scoped re-verification (no findings)**: the guard fix was verified independently — the generic
loops build and run, every concrete-kind rejection from the reviewed head is preserved, the
DEF-145 residual is byte-identical to the pre-phase base (now also covering a scalar instance, an
enum instance, and an unknown-local iterable), nested and generic-body positions report once per
site, and the extended control fails without the fix (red on `5364d2f`; also red on the base for
the rejection assertion).

**Review budget**: two review dispatches for Phase 3 (correctness, semantics) plus the scoped
re-verification of this fix round; every blocking finding fixed in-batch with a regression pin.