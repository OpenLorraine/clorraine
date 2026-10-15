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

## Phase 1 — View owners, loans, and stores

**Reviewed implementation**: `e08ea08` plus two correction rounds. **Lenses**: correctness
(J1R/J1C) and memory-safety (F*), dispatched on the phase diff.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | J1R-1 (the six parallel cross-frame paths — a pre-existing residual, ruled option A and fixed), J1R-6 (a phase-introduced false rejection, shared the remedy), J1R-5 (the read-only boundary, pinned), J1R-2/3/4 (pre-existing, filed); the blast-radius table; the 594-program differential with only the intended deltas |
| memory-safety | FINDINGS — 5, all base-verified pre-existing, 0 new | F1/F3 (holes in the new element-loan and cross-frame rules), F2 (carrier-copy laundering), F4 (the retain depth), F5 (the fixed-array take gate); every accepted twin ASan-clean |

### Fix rounds

| Item | Outcome |
|------|---------|
| A1 — the owners-resolved cross-frame rule (the six missed spellings + the receiver slot + the conditional/array-literal walk) | **Fixed**: each spelling rejects with one report; the caller-backed carrier and alias false rejections closed |
| A2 — the read-only boundary pin | **Pinned** with a case (one `E10030`) |
| F2 — the carrier-copy element-loan transfer | **Fixed** |
| F4 — the deep member chain + the view arrays | **Fixed**; the conditional member STOP (per-branch init fills) filed as `DEF-314` |
| F5 — the fixed-array take gate | **Fixed** |
| C1 — `Param` owners in the element loans + the global destination | **Fixed** (a fix-round claim gap: the take-parameter element stores still accepted) — the repros reject with `E10035`/`E10031`/`E10030`/`E10033` per destination |
| C2 — the retain gate + the array fill | **Fixed** (the second claim gap: the view-position gate and the fill path) |
| C3 — the two-level element loan | **Fixed** (one `E10030`) |
| C4 — the mixed array-literal argument | **Fixed** (one `E10030`; the conservative twin pinned) |

### Confirmations

| Lens | Result | Detail |
|------|--------|--------|
| correctness | PASS WITH MINORS — 0 blockers | On the frozen revision (107 spec / 63 impl): the six spellings + twins pass; the 24-delta list all intended; the read-only boundary kept and pinned; J1C-2 (the mixed-conditional spelling) closed with an added case; J1C-3 (the module-view element cases carry a host-ASan red note) |
| memory-safety | CONFIRMED CLEAN — 0 in-scope; 1 pre-existing | Every repro rejects or runs clean; the twins unchanged; the hunt found one pre-existing UAF (the view-like fixed-array copy drops element loans, base byte-identical) filed as `DEF-315` |

### Filed rows

`DEF-311` (the operand/guarded root-fill gaps), `DEF-312` (the owner-granular loan conflicts),
`DEF-313` (the all-null call argument), `DEF-314` (the conditional-member retain), `DEF-315` (the
view-like copy loans).
