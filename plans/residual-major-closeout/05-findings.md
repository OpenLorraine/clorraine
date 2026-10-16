# Findings Ledger: Residual Major Closeout

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-J ledger (`plans/conditional-projection-residue-ii/05-findings.md`) is the format
> and method reference.

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

## Phase 1 — Initializer call resolution, guards, and nullable view conditionals

**Reviewed fix**: _(recorded at review time — the fix commit and the red-cases baseline)_.
**Lenses**: correctness (RV), emission-integrity (EI), and memory-safety (SA), dispatched in
parallel on the phase diff.

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — Generic element placeholder, inference, and bounds

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics,
emission-integrity, and memory-safety.

### Verdicts

_(to be filled; the element binding's ownership effects and the guard emission are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 3 — Member typing: enum value form and view members

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
emission-integrity.

### Verdicts

_(to be filled; the `DEF-233` rejection's spans and the `.ptr` conversion shape are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 4 — Call conversions and `ffi` result typing

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
emission-integrity.

### Verdicts

_(to be filled; the duty's per-instance counts and the legal-instance controls are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 5 — Null-shaped destinations and capture slot stores

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, memory-safety, and
semantics.

### Verdicts

_(to be filled; the `DEF-245` stop-condition outcome and the `DEF-251` report are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 6 — Standalone emission shapes

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, emission-integrity, and
semantics.

### Verdicts

_(to be filled; the `DEF-189` host/m68k legs and the `DEF-211` byte-identity controls are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Close-out (Phase 7)

**Verification**: _(to be filled — `make check`, `make verify`, the focused origin suites plain
and under `RDC_ASAN=1`, the baseline suite counts)_

**Rows**: _(to be filled — the 20 rows accounted for: 19 Done with per-row evidence;
`DEF-209` recorded as incidentally fixed with its control pins; the parked rows untouched)_

**Code**: _(to be filled — the phase commits and the specification-first loop per phase)_

**Plan**: _(marked Complete in `00-index.md` at close)_

## Phase 1 — Initializer call resolution, guards, and view conditionals

**Reviewed implementation**: `f39c268` plus one correction (`b8e6029`). **Lenses**: correctness (K1R),
emission-integrity (the 27-delta audit), and memory-safety (P1-P4).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | K1R-1 (the receiver-blind inference mis-keys a receiver-bound generic dot call — the emitted C calls the wrong specialization — and adds two spurious instances in existing programs), K1R-2 (the argument destination — pre-existing, filed `DEF-336`), K1R-3 (the baseline placeholder), K1R-4 (the differential undercount: 22 measured vs 19 claimed); 2,026-inline-program differential with no status or diagnostic change |
| emission-integrity | FINDINGS — 1 minor | F-1 (a dead mis-keyed specialization — the same K1R-1 root; the audit's own re-check confirms the correction removed it); otherwise 0 head-only strict failures (77 base → 71 head), 27 intended deltas over 3,663 programs, re-emission 3,663/3,663 identical, both alternate profiles clean, ASan clean |
| memory-safety | CONFIRMED CLEAN — 0 new classes | 85 probes: 51 head-fixed, 30 controls clean, 0 base-clean-defective; four pre-existing NULL-deref siblings (P1-P4) filed |

### Fix round

| Item | Outcome |
|------|---------|
| C1 (K1R-1/F-1) | **Fixed**: the inference is receiver-inclusive at both sites (the dot receiver as argument position zero, skipping callable values) — the probe keys `cl_mergeBox__str_i32`, the two spurious-instance programs revert to the base symbol set, and `d02` builds and runs; a spec case and an impl pin added |
| K1R-3/K1R-4 | **Recorded**: the phase-1 baseline (`d949f7f`); the index status; the wider 27-delta accounting (this section) |
| K1R-2 | **Filed**: `DEF-336` |

### Confirmations

The correction's re-check: the audit (post-review context) confirms the dead instance is gone from
both F-1 programs and `d02` builds and runs at `b8e6029`; the `st35` definition-order swap is
accepted (the same symbol set, run-identical, deterministic); the suites are 45/45 plain and under
`RDC_ASAN=1`, the impl pins 26/26, `make check` exit 0.

### Filed rows

`DEF-335` (the nullable-view conditional call and place arms), `DEF-336` (the argument),
`DEF-337` (the initializer member projection), `DEF-338` (the local-headed qualified receiver's
initializer dot call).
