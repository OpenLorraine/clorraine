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
