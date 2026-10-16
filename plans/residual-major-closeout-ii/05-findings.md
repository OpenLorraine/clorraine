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
