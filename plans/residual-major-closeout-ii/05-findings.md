# Findings Ledger: Residual Major Closeout II (Batch L)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-K ledger (`plans/residual-major-closeout/05-findings.md`) is the format and method
> reference.

## How to use this ledger

- One section per phase. The phase review appends: the reviewed revisions (the fix tree and the
  red-cases baseline), the dispatched lenses, the verdict table (finding ID, severity, area,
  evidence), the fix-round table, and the confirmations.
- Findings IDs follow the phase's lens initials (`RV`/`P<n>R` correctness, `SA`/`M` memory,
  `EI`/`EM` emission, `S`/`PR` semantics), with the phase-prefixed confirmations (`P<n>C`).
- In-round fixes are recorded as fixed-and-confirmed; anything left open is filed as a new
  `DEF-` row in `plans/00-roadmap.md` with a reproduction and base/head evidence (the batch
  convention).
- The close-out section records the final verification, the row accounting, and the code record.
- The two pinned controls (`DEF-341`, `DEF-350`) record their probe-verified state in their
  phases; a close-out probe that shows a regression turns the row red there (AR-8).

## Phase 1 — Nullable-view arms and initializer view stores

**Reviewed fix**: _(recorded at review time — the fix commit and the red-cases baseline)_.
**Lenses**: correctness (RV), memory-safety (SA), and emission-integrity (EI), dispatched in
parallel on the phase diff.

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — The generic walk's projected-argument resolver

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the resolver unification and the keyed-symbol set are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 3 — Instance substitution and element surfaces

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the `DEF-341` control's rejection pair and the element-store drop counts are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 4 — Ownership transfer judgments

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, memory-safety, and
semantics.

### Verdicts

_(to be filled; the loan matrix and the raw-read move shapes are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 5 — Store emission: conditional bases and guarded view literals

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, memory-safety, and
emission-integrity.

### Verdicts

_(to be filled; the AR-6 rejection outcome and the guard-literal laziness are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 6 — Call conversions and `ffi` typing

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the duty counts, the `DEF-344` count ruling, and the `DEF-350` control table are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 7 — Member typing on concrete paths

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
emission-integrity.

### Verdicts

_(to be filled; the rejection spans and the AR-7 cast-or-duty outcome are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 8 — Module/global identity and the const folder

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
emission-integrity.

### Verdicts

_(to be filled; the identity matrix, the deferred extent spelling, and the checker typing are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Close-out (Phase 9)

**Verification**: _(to be filled — `make check`, `make verify`, the focused origin suites plain
and under `RDC_ASAN=1`, the baseline suite counts)_

**Rows**: _(to be filled — the 23 rows accounted for: 21 Done with per-row evidence;
`DEF-341`/`DEF-350` recorded as incidentally fixed with their control pins; the parked rows
untouched)_

**Code**: _(to be filled — the phase commits and the specification-first loop per phase)_

**Plan**: _(marked Complete in `00-index.md` at close)_

## Phase 1 — Nullable-view arms

**Reviewed implementation**: `3eedbe3`/`9c276a2` plus three correction rounds (`fa93c38`, `b31ebaf`,
`297a1ca`). **Lenses**: correctness (L1R), memory-safety (the dropped operand guards), and
emission-integrity (F1), plus a bounded confirmation and the parent's re-run.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | L1R-1 (a typed-nullable *else* arm: the binding fired but the render guard tested the conditional's resolved type, so the store widened unguarded — a NULL deref), L1R-4 (a fresh call-rooted member arm: the guard re-evaluated the call — a side effect twice and a leak), L1R-2/L1R-3 (the body/lambda and module-initializer spellings of the same family), L1R-5 (the R1 overstatement); the init-local retention verified correct |
| memory-safety | FINDINGS — 2 | Finding 1 (the arm's guard source was built with the plain rendering — its inline operand guards and materialized fills were dropped: a base-clean call index never ran, an OOB read, an FPE), finding 2 (the pre-existing module/both-place argument route — partly closed by C4, incl. the body twin) |
| emission-integrity | FINDINGS — 1 | F1 (= L1R-4: the call-rooted member re-evaluation), P1 (= L1R-3); the 28-delta set otherwise intended, no acceptance change, 0 head-only strict failures, determinism 3,483/3,483 |
| confirmation | FAIL then PASS | The bounded confirmation found F-L1-1 (the flat-store operand guard emitted twice — a side-effecting index ran twice: `x6` head 21 vs base 0); after the fix the parent's six-probe re-run: `x6`/`v09`/`v10` 0, `h22` 0 (one call), the `E11211`/`E11210` traps — PASS |

### Fix rounds

| Item | Outcome |
|------|---------|
| C1 (L1R-4) | **Fixed**: `reads_stable_storage` gates the guard; `retain_conditional_projection_roots` binds each arm's fresh projection root under its own guard (the move-out after the store) |
| C2 (L1R-1) | **Fixed**: the shared `Value::selects_null` predicate (the literal, any nested arm, or a null-capable arm type) used by the binding and the propagation — the local is marked nullable and the guard fires |
| C3 (L1R-2) | **Fixed**: the body/lambda owner-nullness sites use the shared predicate |
| C4 (L1R-3) | **Fixed**: a conditional argument at a view parameter routes through the per-arm path; the walk binds a fresh nullable argument; the body both-place twin closed too |
| L4 (`b31ebaf`) | **Fixed** (memory finding 1): the guard's source is built through `guarded_parts`, hoisting its operand guards ahead of the null test |
| L5 (`297a1ca`) | **Fixed** (F-L1-1): the plain/inline split (`widening_with_guards`) — the flat stores keep the statement guards, the arm/argument sites use the inline form |

### Confirmations

The final state: the six probes (`x6`, `v09`, `v10`, `h22`, `h6c`, `h6f`) as designed; the flat
emissions byte-identical to the pre-L4 head; the arm operand-guard text byte-identical to the
pre-phase base; the spec suite 62/62 plain and under `RDC_ASAN=1`; the impl pins 42/42; `make check`
exit 0; no acceptance change; 0 head-only strict failures.

### Filed rows

None — every phase-1 finding was fixed in the correction rounds (the family's pre-existing
spellings closed with them).

## Phase 2 — The generic walk's projected-argument resolver

**Reviewed implementation**: `3d54079` plus two correction rounds (`cfeffd3`, `7b3cc68`). **Lenses**:
correctness (L2R/L2R-R1), semantics (L2S), memory-safety (the take move-out), and emission-integrity,
plus a bounded re-confirmation.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | L2R-1 (an import alias overwrote a same-named own binding in the walk's scope map — a base-green call emitted an undeclared symbol), L2R-2 (the body walks received the scope map without a shadowing check — a base-green program rejected `E10043`), L2R-3 (the direct-name shadow — filed `DEF-361`); the re-confirmation then found L2R-R1 (the expression-bodied lambda arm still inherited the map) → fixed and parent-verified |
| semantics | FAIL — 1 blocker | L2S-1 (the missing lexical-scope overlay — the same family as L2R-2), L2S-2 (= L2R-1); Q2/Q3 conformant (the place resolver mirrors the emitter; the conditional rule matches); Q4 confirmed `DEF-360` as a checker gap; PRE-1/2/3 filed (`DEF-363`/`DEF-364`; `DEF-347` extended) |
| memory-safety | FINDINGS — 1 | A `take` argument of an opaque member type never moves out at the instance — an ASan double free on shapes this phase newly keys (the Local-rooted twin reproduces byte-identically at the base: root pre-existing, reachability introduced); filed `DEF-362` and folded into phase 4 (AR-18); the three reds, eight controls, seven fixes, and the do-not-fix set otherwise clean |
| emission-integrity | FINDINGS — 2 | Both MINOR and both the same roots as L2R-1/L2R-2 (a dead specialization from the collision; a dead specialization from the body shadow) — already fixed by the correction; the 11 intended deltas are addition-only keyed instances; instance integrity, determinism, and the cross profiles clean |

### Fix rounds

| Item | Outcome |
|------|---------|
| C1 (L2R-1/L2S-2) | **Fixed**: `visible_bindings` skips an import whose local name is already bound — the own binding wins, mirroring the read resolution |
| C2 (L2R-2/L2S-1) | **Fixed**: only the initializer walks carry the module scope; the function bodies keep the base decline |
| R1 (L2R-R1) | **Fixed**: one empty body scope hoisted for both lambda body forms (the expression arm previously inherited the map) |

### Confirmations

The bounded re-confirmation: **FAIL — 1 blocker** (L2R-R1 only; the seven fixes, eight controls,
the differential, and the suites all confirmed). After the one-branch fix, the parent's probe
re-run: the expression-lambda shape keys `cl_pick__arr2_i32` only (exit 0); the suites 139/139 plain
and under `RDC_ASAN=1`, the impl pins 58/58, `make check` exit 0; the differential shows the same
seven intended fixes and no unintended accept/reject.

### Filed rows

`DEF-360` (the dot-call written-argument checker gap), `DEF-361` (the direct-name shadow),
`DEF-362` (the opaque-member take move-out — **folded into phase 4**, AR-18), `DEF-363` (the
declaration/import collision not reported), `DEF-364` (the opaque conditional judgments), `DEF-365`
(the element-of-element key leak), `DEF-366` (the owning row field double free), plus the `DEF-347`
extension (the newly keyed view-pointer const-discard).

## Phase 3 — Instance substitution and element surfaces

**Reviewed implementation**: `a825044` plus three correction rounds (`308711e`, `f6778e4`,
`9104572`). **Lenses**: correctness (L3R), semantics (L3S), memory-safety (F-1/F-2),
emission-integrity (F1), plus a bounded re-confirmation.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | No phase findings: the drop shape/timing matched the concrete twin, the conversion duties reported once with twin parity, the AR-19 ruling confirmed; L3R-1 (a runtime-indexed member store silently dropped — pre-existing) filed as `DEF-367`; L3R-2 (a record nit) noted |
| semantics | FAIL — 1 blocker | L3S-1 (the element store's drop ran before an aliasing right-hand side reading through the array root — a head-only SEGV), L3S-2 (an AR-19 citation nit — fixed), L3S-3 (the design doc — aligned); Q1-Q4 otherwise conformant; P-2 → `DEF-367` extended, P-3 → `DEF-370`, P-4 → `DEF-371` |
| memory-safety | FINDINGS — 2 | F-1 (the store of a place value into an opaque destination lowered as a copy — a second store double-freed where the base leaked; the same root as the emission F1), F-2 (an owned fixed-array member of a borrowed element moved out without the partial-move judgment — a newly runnable SEGV; root pre-existing); the clean-path coverage (loop, early exit, break, both conditional arms, lambda, slice, nested generics, self-read, panic) complete |
| emission-integrity | FINDINGS — 1 | F1 (the overwrite drop freed a moved-from element source — CRITICAL, head-only; P4 the same root at the base); P2 → `DEF-368`, P3 → `DEF-367` extended; the 5 intended emit diffs and the 9 rejects otherwise confirmed, with twin parity and determinism |
| re-confirmation | FAIL then PASS | The bounded re-confirmation confirmed items 1/2/4/5 and the parameter-root spellings but found L3S-4 (the save guard missed local containers — a head-only SEGV); after the fix the parent's six-probe re-run: `r2e`/`r2d`/`min_alias`/`ac_alias_concrete`/`r2a`/`r2b` all exit 0, ASan/LSan clean |

### Fix rounds

| Item | Outcome |
|------|---------|
| F1 (`308711e`) | **Fixed**: an unknown destination type stores as a move (the checker route), the move and move-out sources resolve through `resolved_place_ty`, and an unknown take-argument type is maybe-owning — the element place empties exactly like the concrete spelling (`m11`/`m09`/`v5`/`v5b` clean) |
| L3S-1 (`f6778e4`) | **Fixed**: one recursive `value_reads_place` walk and a caller-storage test; an element-target store defers its release to the save path when the right-hand side reads caller storage (the generic `min_alias` and its pre-existing concrete root `ac_alias_concrete` both clean) |
| L3S-4 (`9104572`) | **Fixed**: the named-binding predicate matches `Local` as well as `Param`/`Global`, so a local-container element store saves too (`r2e`/`r2d` clean; one pre-existing pin re-pinned to the save path, recorded) |

### Filed rows

`DEF-367` (the runtime-indexed member store silently dropped — **extended** twice: the borrow-parameter variant, the P-2 local/slice spellings), `DEF-368` (the generic element member `str` conversion), `DEF-369` (the per-instance partial-move judgment — **folded into phase 4**, AR-20, with the `m09` parity evidence), `DEF-370` (the spurious `W10001` on a move-only array lend), `DEF-371` (a literal argument at a generic callee's concrete parameter).

### Record notes

- The widened deferral saves for any named-binding read from an element or raw-pointer target (one local
  each, always safe); the differential's 23 emit diffs are all behavior-checked base-vs-head identical.
- The re-pinned shape assertion (`call_value_integrity_impl_test`): the runtime-index store now takes
  the save path; its run is clean.
- `rawself_use`'s pre-existing raw self-move failure is unchanged (the `DEF-327`/raw family).

**Phase-3 addendum (AR-21):** the F1 round's accepted `m09` shape (`eat(v.name)` on a borrowed
element, emptied after the call) was re-pinned during phase 4 to the parity oracle: the generic
instance now reports the concrete twin's one `E10026`. The other phase-3 findings stand.

## Phase 4 — Ownership transfer judgments

**Reviewed implementation**: `07f5f94` plus one large correction round (`d1328ca`). **Lenses**:
correctness (L4R), semantics (L4S), and memory-safety (M1-M5).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | No phase-introduced defect: the duty, the carrier, the loan transfer, and the deref store verified across their consumers; the AR-21 re-pin substantively correct; L4R-1 (the `eb8ba1f` message) and L4R-2 (a stale comment, fixed) minor; the pre-existing gaps L4R-3..L4R-6 mapped (L3 → the correction; L4 → `DEF-374`; L5 → `DEF-373` extended; L6 → the corrected take + the left guarded take) |
| semantics | FAIL — 4 blockers | L4S-1 (the duty missed the maybe-owning destinations and the conditional-arm sources), L4S-2 (a projected-source copy transferred no loans), L4S-3 (the guarded conditional take), L4S-4 (the deref read emptied only at the store position); L4S-5 (the phase-introduced rejection's code was `E10020` where the concrete twin reports `E10025`); the covered spellings and the AR-21 re-pin conformant |
| memory-safety | FINDINGS — 5 | All pre-existing at the base, no base-clean program defective: M1 (the conditional-source copy — `DEF-373`), M2 (= L4S-3, fixed), M3 (the borrow-element conditional take — `DEF-374`, fixed by the correction's arm sweep), M4 (the deref operand materialization's declaration-time gate — `DEF-375`, persists), M5 (= L4S-4, fixed); `DEF-366` incidentally fixed by the duty |

### Fix round

| Item | Outcome |
|------|---------|
| C1 (L4S-1) | **Fixed**: the duty records in the shared maybe-owning mark (so the return/binding/assignment destinations deposit it) and the take path sweeps a conditional's arms — the shapes report the concrete twins' `E10026` counts; the duplicable instances stay accepted |
| C2 (L4S-2) | **Fixed**: `move_element_loans` translates a projected source's positions (gated on a constant path and a non-single-view destination — the single-view control keeps exactly one `E10031`) |
| C3 (L4S-3) | **Fixed**: the unresolved-conditional route fires in the deferred context too, the guard renderers resolve the carried value, and the guarded move-outs gate on the resolved type and emptiness — the guarded shape emits the concrete twin's carrier |
| C4 (L4S-4) | **Fixed**: the direct return routes a deref source through the block-local emptying, a new consumed-source statement carries the take and field positions, and an impure pointer is pinned once |
| C5 (L4S-5) | **Fixed**: the duty reports the partial-move code (`E10025`) for a whole use conflicting with top-level field paths (the field-only predicate keeps the element paths' `E10020` and the hardening pins green) |

### Confirmations

The parent's probe re-run: `ct14` (M3) now rejects one `E10026`; `vc03` (M1's conditional
source) and `ds19` (M4) persist, so `DEF-373`/`DEF-375` stand; the guarded deref take residual filed
(`DEF-376`); the residue suites 156/156 plain and under `RDC_ASAN=1`; the impl pins 93/93;
`make check` exit 0.

### Filed rows

`DEF-372` (the element-read store — from the spec authoring), `DEF-373` (the conditional/array-literal
copy sources — **extended** twice), `DEF-374` (the borrow-element conditional take — closed in-phase by
C1 but kept as a row? no — verified fixed and left open for the close-out measurement), `DEF-375`
(the deref operand materialization), `DEF-376` (the guarded deref take), plus the `DEF-366`
incidental-fix annotation.

### Record notes

- AR-21 (the phase-3 `m09` oracle re-pin) is substantively correct per the correctness lens; the
  expected disposition item at close-out.
- The `q6` conditional take reports two `E10026` (one per arm), matching its concrete twin's count.
- Untouched residuals recorded: a runtime-index projected source transfers no loans (`DEF-373`'s
  family); the guarded deref take keeps the inline call (`DEF-376`).

## Phase 5 — Store emission: conditional bases and guarded view literals

**Reviewed implementation**: `fa2e927` plus three correction rounds (`4089f1a`, `6c34d6d`,
`ed84325`). **Lenses**: correctness (L5R), emission-integrity, and memory-safety, plus two bounded
confirmations.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | L5R-1 (the `Index` recursion newly rejected the `unsafe` base spelling that the base accepted and ran — the sibling `Member` arm's transparency was not shared), L5R-2 (the member-store conditional base silently dropped — filed `DEF-377`; the emission lens's R2 is the same class), L5R-3 (the new comment overstated the recursion); the pinned spellings, the controls, and the AR-21-disposition otherwise verified |
| emission-integrity | CONFIRMED — 0 findings | The 15 diffs and 6 rejects exactly the intended classes; 0 head-only strict failures; prototypes unchanged; run deltas only the two intended value fixes plus pre-existing nondeterministic probes; the R1-R5 register filed (`DEF-378`/`379`; R2 = `DEF-377`) |
| memory-safety | FINDINGS — 2 in scope | The base-clean→defective nested-literal take double-free (the changed predicate's inner temp copied without a move-out) and the fill leak (the deferred path stored views of unbound fresh roots); P1 (`DEF-381`) and P3 (`DEF-380`) filed; then the confirmation found the **arm-comma escape** (a `?:` third operand cannot carry the field-wise arm's comma sequence) → fixed; the final re-confirmation **PASS WITH MINORS — 0 blockers** |

### Fix rounds

| Item | Outcome |
|------|---------|
| L5R-1 (`4089f1a`) | **Fixed**: the `Index` arm accepts an `Unsafe` base like the sibling `Member` arm; the recursion comment reworded to what it covers |
| C-a/C-b (`6c34d6d`) | **Fixed**: a struct-literal arm field whose owned value reads a fresh materialized root stores field by field (the inner temp empties in the same sequence); a fill whose evaluation must happen once is pinned through the update-value channel — the leaks close and the per-element value residual is recorded (`DEF-326` extended) |
| the escape (`ed84325`) | **Fixed**: each arm of the conditional move expression is parenthesized, so the field-wise comma sequence cannot escape — the two-field arm, the skipped-arm evaluation, and `DEF-381`'s remaining leak all close; `gvnestt7`'s invalid C was repaired incidentally; the two-field oracle case and a pin added (five needles re-pointed, recorded) |

### Confirmations

| Step | Result |
|------|--------|
| The memory confirmation (`6c34d6d`) | FAIL — the arm-comma escape (a leak and a skipped-arm evaluation on the corrected nested-literal take; the single-field oracle missed it) |
| The scoped re-confirmation (`ed84325`) | **PASS WITH MINORS — 0 blockers**: every escape repro clean with the right values; the differential parenthesization-only (33 paren-only / 25 same / 0 accept-reject); the fill family sanitizer-clean; the phase's original shapes unchanged; the minors are the pre-existing `DEF-379` (the by-value nested spelling) and the recorded fill value residual |

### Filed rows

`DEF-377` (the member-store conditional base), `DEF-378` (the hidden destination in the read
positions), `DEF-379` (the by-value nested literal's skipped-arm call), `DEF-380` (the multi-extent
view-array field), and `DEF-381` (**closed as incidentally fixed** — the double free by `6c34d6d`
and the escape by `ed84325`); `DEF-326` extended with the view-fill evaluation shape.

### Record notes

- The parenthesization breadth (every arm; 114 behavior-identical emission diffs) is kept: it also
  protects the guard-carrying arm spellings; narrowing it was considered and declined.
- The C-b fill's per-element value re-evaluation is a recorded residual (`DEF-326`), sanitizer-clean.
- `DEF-379`'s nested by-value spelling is base-identical and stays filed.

## Phase 6 — Call conversions and ffi typing

**Reviewed implementation**: `d9b6ae7` plus three correction rounds (`39bf027`, `5635204`,
`d5102ee`). **Lenses**: correctness (L6R), semantics (L6S), memory-safety, and emission-integrity,
plus two bounded confirmations.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | L6R-1 (the destination-side duty over-rejected context-typed literals in every position), L6R-2 (a duplicate report on a borrow argument whose slot is nullable), L6R-3 (the docs), L6R-4 (the record counts); the sibling-fixed counts, the four route rejections, and the `bs02` coverage otherwise verified |
| semantics | FAIL — 2 blockers | L6S-1 (`sibling_fixed` treated a parameter-typed resolution as concrete — the legal nullable instance and the un-instantiated body were rejected, and the concrete instance's report doubled), L6S-2 (= L6R-1); L6S-3/L6S-4 filed (`DEF-382`/`DEF-383`); Q3 (the generic-call route) and Q4 (`bs02`) conformant |
| memory-safety | CONFIRMED CLEAN — 0 | 5,811 programs swept; only the two rows' 13 rejects; 0 emission diffs; no leak/UAF/DF; the accepted-shape battery ASan/LSan green; the do-not-fix set identical |
| emission-integrity | FINDINGS — 2 | Both = L6R-1/L6R-2 (the audit pinned the pre-correction tree and noted the in-flight fix); 0 emission diffs everywhere, 0 dead instances, determinism 3,952/3,952, the profiles unchanged; P1/P2 filed (`DEF-384`/`DEF-385`) |
| confirmations | FAIL then PASS | The first confirmation found L6S-5 (the literal rule admitted every scalar destination — float↔int, int→bool, out-of-range) and L6S-6 (the borrow-slot skip lost the non-nullable instances); after the narrowing round the re-confirmation: **PASS WITH MINORS — 0 blockers** (L6S-7 filed) |

### Fix rounds

| Item | Outcome |
|------|---------|
| C1 (`39bf027`) | **Fixed**: the record carries a numeric-literal marker and a scalar destination supplies the literal's type through the frozen widenings (a non-scalar destination keeps its `E10200`); the borrow-slot record is skipped for the nullable duplicate; the docs updated |
| L6S-1 (`5635204`) | **Fixed**: the sibling-fixed null trigger requires a concrete resolution (`!unknown`, `!nullable`, no parameter) — the legal nullable instance and the un-instantiated body accept, the concrete instance reports once |
| the narrowing (`d5102ee`) | **Fixed**: the record carries the literal's kind and value and the audit applies the twin's rule (int → an integer scalar must fit, else `E10202`; int → float/bool → `E10200`; float → f32/f64 only); the borrow-slot record is kept with the nullable-only deferral — the non-nullable instances keep their report |

### Filed rows

`DEF-382` (the argument-route literal loss — remains open), `DEF-383` (the inferred null literal at
a nullable instance — remains open), `DEF-384` (the nullable scalar type argument), `DEF-385` (the
qualified type path), `DEF-386` (the negated literal at concrete positions), plus the `DEF-360`
extension (the dot-call borrow-slot spelling).

### Record notes

- L6R-4 corrected: six spec tests landed (the record said five); the phase's head-only rejection set
  settled at 9 (the two literal probes of the L6R-1 class left it once fixed).
- L6S-7's caveat: the generic deferred judgment follows the fit rule while the concrete negated-literal
  twin is non-conformant (`DEF-386`).
- The `assert_no_asan` helper asserts only "AddressSanitizer" (a leak-only report would slip) —
  pre-existing, noted by the memory lens (its battery covered the phase's shapes).
- The non-nullable borrow instance's report span differs from its twin by the `borrow ` prefix
  (disclosed; the code and count match).
