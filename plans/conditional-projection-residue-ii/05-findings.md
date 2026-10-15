# Findings Ledger: Conditional, Projection, and Update Residue II

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-I ledger (`plans/conditional-projection-residue/05-findings.md`) is the format and
> method reference.

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

## Phase 1 — View owners, loans, and stores

**Reviewed fix**: _(recorded at review time — the fix commit and the red-cases baseline)_.
**Lenses**: correctness (RV) and memory-safety (SA), dispatched in parallel on the phase diff.

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — Guarded, discarded, and carrier roots

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness and memory-safety.

### Verdicts

_(to be filled)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 3 — Fixed-array and literal emissions

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, emission-integrity, and
memory-safety.

### Verdicts

_(to be filled)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 4 — Loop conditions, update order, and store lowering

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, emission-integrity,
semantics, and memory-safety.

### Verdicts

_(to be filled; the `DEF-294` toolchain outcome — fixed or control — is recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 5 — Diagnostics and marks

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness and semantics.

### Verdicts

_(to be filled; the one-report boundary and its pinned counts are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Close-out (Phase 6)

**Verification**: _(to be filled — `make check`, `make verify`, the focused suites plain and
under `RDC_ASAN=1`, the baseline suite counts)_

**Rows**: _(to be filled — the 27 rows accounted for: Done with per-row evidence; the landed
partial-fix controls; the parked rows untouched)_

**Code**: _(to be filled — the phase commits and the specification-first loop per phase)_

**Plan**: _(marked Complete in `00-index.md` at close)_
