# Findings Ledger: Residual Major Completion (Batch M)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-L ledger (`plans/residual-major-closeout-ii/05-findings.md`) is the format and method
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
- The two pinned controls (`DEF-361`, `DEF-374`) record their probe-verified state in their
  phases; a close-out probe that shows a regression turns the row red there (AR-31, AR-32).

## Row Ledger (empty result cells — the execution fills them)

| Row | Severity | Verdict at `86d6f51` | Phase | Post-fix expectation | Phase result |
|-----|----------|----------------------|-------|----------------------|--------------|
| `DEF-360` | 🟠 major | Red | P1 | One `E10200` at the dot-call written argument; matching calls run | _(to be filled)_ |
| `DEF-361` | 🟠 major | Control | P1 | Direct-name shadow shapes stay green (run 0) | _(to be filled)_ |
| `DEF-363` | 🟠 major | Red | P1 | One `E11103` per declaration/import collision; renamed control runs | _(to be filled)_ |
| `DEF-390` | 🟠 major | Red | P1 | Qualified call runs the module's function; mismatch keeps `E10200` | _(to be filled)_ |
| `DEF-364` | 🟠 major | Red | P2 | `E10200` at the else arm / `E10012` at the null; controls run | _(to be filled)_ |
| `DEF-365` | 🟠 major | Red | P2 | `cl_id__u8`, no `#`; build/run on host and m68k profiles | _(to be filled)_ |
| `DEF-368` | 🟠 major | Red | P2 | Concrete twin's `cl_string_view`; build/run 0 | _(to be filled)_ |
| `DEF-371` | 🟠 major | Red | P3 | Literal typed by the concrete parameter; build/run 0 | _(to be filled)_ |
| `DEF-384` | 🟠 major | Red | P3 | One `E10011` at `Array<i32?>`; nullable handle control runs | _(to be filled)_ |
| `DEF-385` | 🟠 major | Red | P3 | Qualified type path emits the unqualified instance; run 0 | _(to be filled)_ |
| `DEF-386` | 🟠 major | Red | P3 | `i16`/`i8` negated literals build/run; out-of-range `E10202` | _(to be filled)_ |
| `DEF-367` | 🔴 critical | Red | P4 | Store and RHS emitted once; calls = 4; run 0; bounds trap kept; ASan-clean | _(to be filled)_ |
| `DEF-377` | 🟠 major | Red | P4 | One `E10221` per conditional-base store; read control accepted | _(to be filled)_ |
| `DEF-372` | 🟠 major | Red | P5 | One `E10030` at the fill; direct/member twins keep theirs | _(to be filled)_ |
| `DEF-373` | 🟠 major | Red | P5 | One `E10030` per copy fill; copies used in-scope accepted | _(to be filled)_ |
| `DEF-374` | 🟠 major | Control | P5 | Two `E10026` per twin at the same span | _(to be filled)_ |
| `DEF-375` | 🟠 major | Red | P6 | `alt` once; run 12; ASan/LSan-clean | _(to be filled)_ |
| `DEF-376` | 🟠 major | Red | P6 | Both selections ASan-clean (0/1) | _(to be filled)_ |
| `DEF-378` | 🟠 major | Red | P7 | Three shapes build and run 0; one call each | _(to be filled)_ |
| `DEF-379` | 🟠 major | Red | P7 | Calls = 2; run 0; ASan-clean | _(to be filled)_ |

## Phase 1 — Call identity and written-argument checks

**Reviewed fix**: _(recorded at review time — the fix commit and the red-cases baseline)_.
**Lenses**: correctness (RV), semantics (S), and emission-integrity (EI), dispatched in parallel
on the phase diff.

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling; include the `DEF-361` control evidence and
the `DEF-390` symbol-set blast radius)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — Substitution edges I: conditional and element keys

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the conditional report counts/spans and the keyed-symbol set are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 3 — Substitution edges II: context typing

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
emission-integrity.

### Verdicts

_(to be filled; the literal/type-argument spans and the qualified-instance emission are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 4 — Dropped store targets

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the `DEF-367` store shape, RHS count, bounds guard, and `DEF-377` rejection spans
are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 5 — Element and store loans

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, memory-safety, and
semantics.

### Verdicts

_(to be filled; the loan matrix, the `E10030` counts/spans, and the `DEF-374` control parity are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 6 — Deref and take channels

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the single-call emission and the guard-path emptying are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 7 — Literal destinations and guards

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity.

### Verdicts

_(to be filled; the destination form, the call counts, and the guard-path evaluation are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Close-out

**Verification**: _(to be filled: `make check`, `make verify`, the focused ASan legs, the strict
sweep)_. **Row accounting**: _(to be filled: 18 Done, 2 controls, the parked rows, the minors)_.
**Code record**: _(to be filled: the phase commits and the final tree)_.
**Residuals**: _(to be filled: any filed row or recorded limitation)_.
