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
