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

## Phase 2 — Element placeholder, inference, bounds

**Reviewed implementation**: `9e88cb5` plus two correction rounds (`3096378`, `802d016`).
**Lenses**: correctness (K2R), semantics (K2S), emission-integrity (F-1/F-2), memory-safety (K2M).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | K2R-1 (the unjudged substituted index kinds: previously-diagnosed programs accepted with invalid C), K2R-2 (the element member audit never fired), K2R-3 (the declined projected-argument shapes — filed `DEF-342`); 2,060-program differential |
| semantics | FAIL — 2 blockers | K2S-1 (the nested element placeholder rendered into C — a head-only regression; base built and ran), K2S-2 (= K2R-1), K2S-3..6 (the declined/keyed/chained/null families — filed); Q1-Q4 conformant otherwise (the element rule, the projected-place key, the null rule, the index codes) |
| emission-integrity | FINDINGS — 2 | F-1 (the chained index's inner judgment — head-newly-accepted, an OOB read), F-2 (the conditional argument's keying — head-newly-accepted, an undeclared symbol); 3,685-program differential |
| memory-safety | FINDINGS — 1 critical | K2M-1 (the owning-element by-value capture double-frees — the capture-move duty's substitution missed the element binding) + K2M-P1/O2/O3 (pre-existing, filed) |

### Fix rounds

| Item | Outcome |
|------|---------|
| C1 (K2R-1/K2S-2) | **Fixed**: the instance index judgment completed (nullable → `E10010`, non-indexable → one `E10209` at the base span, arrays keep `E10216`/the guard, `str`/`cstring`/`raw` via the shared `index_element`); the walk/emitter resolver duplication collapsed |
| C2 (K2S-1/K2M-O1) | **Fixed**: the nested element placeholders bind to a fixed point — the nested loop builds and runs (21) |
| C3 (K2M-1) | **Fixed**: the capture move requirements substitute with the instance bindings — one `E10401`, no TU |
| C4 (K2R-2) | **Fixed**: the projection and duplicability audits use the instance bindings — one `E10212` |
| F-1 | **Fixed**: an opaque-base index types as the base's one-step element placeholder, so a chained index's inner base records its own opaque index; the guard base resolves through the value-type resolver — the inner runtime index traps `E11211`, the constant reports `E10216`; `DEF-343` closed incidentally |
| F-2 | **Fixed**: the walk's value resolver gained the conditional arm (the emitter's both-arms-agree rule) — the conditional argument keys its instance |

### Confirmation

**CONFIRMED — 0 findings** (the emission lens, 3,913 programs): 2,398 strict-OK at head (83 CCFAIL, none head-only), 112 intended emission diffs, 26 intended rejects, 28 accepts (27 intended + one invalid probe of a pre-existing family), determinism 2,481/2,481, both alternate profiles clean; the capture, nested-loop, chained-index, string-index and conditional-argument repros all behave as designed.

### Filed rows

`DEF-339` (the borrowed element's owned-field store leak), `DEF-340` (the temp-rooted projected argument), `DEF-341` (the lifted lambda return substitution), `DEF-342` (the declined projected-argument shapes), `DEF-343` (**closed fixed** in round 2), `DEF-344` (the null-shaped argument against a sibling-fixed parameter), `DEF-345` (the substituted element in a conversion/member position — the pre-existing gap the corrected index path now reaches).
