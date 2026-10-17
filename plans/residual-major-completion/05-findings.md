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
| `DEF-360` | 🟠 major | Red | P1 | One `E10200` at the dot-call written argument (call-site inference); the generic caller runs | _(to be filled)_ |
| `DEF-361` | 🟠 major | Control | P1 | Six direct-name fixtures stay green (run 0); module-own removed | _(to be filled)_ |
| `DEF-363` | 🟠 major | Red | P1 | One `E11103` per declaration/import collision (func/struct/const matrix); renamed control runs | _(to be filled)_ |
| `DEF-390` | 🟠 major | Red | P1 | Qualified call uses the module's symbol and modes/params/return; `m.hook()` control; mismatch keeps `E10200` | _(to be filled)_ |
| `DEF-364` | 🟠 major | Red | P2 | `E10200` at the else arm / `E10012` at the null, in direct let/return/field/arg contexts too; controls run | _(to be filled)_ |
| `DEF-365` | 🟠 major | Red | P2 | `cl_id__u8`, no `#`; build/run on host and m68k profiles | _(to be filled)_ |
| `DEF-368` | 🟠 major | Red | P2 | Concrete twin's `cl_string_view`; build/run 0 | _(to be filled)_ |
| `DEF-371` | 🟠 major | Red | P3 | Literal typed by the concrete parameter; build/run 0 | _(to be filled)_ |
| `DEF-384` | 🟠 major | Red | P3 | One `E10011` at each nested type-argument site and the explicit call type argument; five sites in the count case, no duplicates; nullable handle control runs | _(to be filled)_ |
| `DEF-385` | 🟠 major | Red | P3 | Qualified type path emits the unqualified instance; run 0 | _(to be filled)_ |
| `DEF-386` | 🟠 major | Red | P3 | Folded fit: `-5`/`i8 -128` accept; `u8 -5`/`i8 -129` report `E10202`; `f32`/`bool` stay `E10200` | _(to be filled)_ |
| `DEF-367` | 🔴 critical | Red | P4 | Plain/compound/update/chain/nested/multi-index/order stores emit; per-instance owning store LSan-clean; bounds trap kept | _(to be filled)_ |
| `DEF-377` | 🟠 major | Red | P4 | Five conditional-rooted forms and unsafe-wrapped conditional forms each `E10221`; raw `unsafe p.f` prefixes accepted with a store-emitting read-back; block controls accepted; `(*p).f`/`unsafe (*p).f` residual probed | _(to be filled)_ |
| `DEF-372` | 🟠 major | Red | P5 | One `E10030` at the fill; direct/member twins keep theirs | _(to be filled)_ |
| `DEF-373` | 🟠 major | Red | P5 | One `E10030` per affected fill; the mixed-position union reports both; in-scope copies accepted | _(to be filled)_ |
| `DEF-374` | 🟠 major | Control | P5 | Two `E10026` per twin at the same span | _(to be filled)_ |
| `DEF-375` | 🟠 major | Red | P6 | Store/return/field/array/conditional each call `alt` once and move the pointee; ASan/LSan-clean | _(to be filled)_ |
| `DEF-376` | 🟠 major | Red | P6 | Both selections ASan-clean; the false path does not call the impure pointer | _(to be filled)_ |
| `DEF-378` | 🟠 major | Red | P7 | Read/unguarded/nested/guarded positions build and run 0; one call each | _(to be filled)_ |
| `DEF-379` | 🟠 major | Red | P7 | Calls = 2 (single-pass) / 4 (loop); run 0; carrier fill inside the guard path; eager control run 0; ASan-clean | _(to be filled)_ |

## Phase 1 — Call identity and written-argument checks

**Reviewed fix**: _(recorded at review time — the fix commit and the red-cases baseline)_.
**Lenses**: correctness (RV), semantics (S), memory-safety (SA), and emission-integrity (EI),
dispatched in parallel on the phase diff (AR-27; the memory lens covers the symbol/IR change).

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
emission-integrity (AR-27).

### Verdicts

_(to be filled; the literal/type-argument spans and the qualified-instance emission are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 4 — Dropped store targets

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, semantics, memory-safety,
and emission-integrity (AR-27; semantics owns the place rule and the store target).

### Verdicts

_(to be filled; the `DEF-367` store shape, per-instance owning store, RHS count, bounds guard,
target order, the five plus unsafe-wrapped `DEF-377` rejection spans, and the accepted raw-prefix read-backs are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none"; the dropped stores through `(*p).f` and
`unsafe (*p).f` are known candidates for this phase — probe both and record their reproduction
and evidence before filing)_

## Phase 5 — Element and store loans

**Reviewed fix**: _(recorded at review time)_. **Lenses**: correctness, memory-safety, and
semantics (AR-27; no emission lens for P5).

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
sweep, the per-phase emission differentials)_. **Row accounting**: _(to be filled: 18 red rows
Done, 2 controls Done, the two parked majors open, the remaining minors counted by audit; every
in-execution filed row checked, including `DEF-380`'s gridel boundary and `DEF-382`'s open
status — recorded incidental vs open, AR-38)_. **Code record**: _(to be filled: the phase commits and the
final tree)_. **Residuals**: _(to be filled: any filed row or recorded limitation, including the
`(*p).f`/`unsafe (*p).f` stores if they remain)_.
