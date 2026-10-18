# Findings Ledger: Memory and Value Integrity (Batch N)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-M ledger (`plans/residual-major-completion/05-findings.md`) is the format and method
> reference.
> **Baseline**: head `f6da78f` (the batch-M close-out commit)

## How to use this ledger

- One section per phase. The phase review appends: the reviewed revisions (the fix tree and the
  red-cases baseline), the dispatched lenses, the verdict table (finding ID, severity, area,
  evidence), the fix-round table, and the confirmations.
- Findings IDs follow the phase's lens initials (`P1C`/`RV` correctness, `P1S` semantics,
  `P1M`/`SA` memory, `P1E`/`EI` emission), with the phase-prefixed confirmations (`P1F`).
- In-round fixes are recorded as fixed-and-confirmed; anything left open is filed as a new
  `DEF-` row in `plans/00-roadmap.md` with a reproduction and base/head evidence (the batch
  convention).
- The close-out section records the final verification, the row accounting, and the code record.
- The two false-positive controls (`DEF-427`, `DEF-447`) record their probe-verified state in
  phase 7; a close-out probe that shows a regression turns the row red there (AR-27, AR-28,
  AR-35(d)).

## Row Ledger (empty result cells — the execution fills them)

| Row | Severity | Verdict at `f6da78f` | Phase | Post-fix expectation | Phase result |
|-----|----------|----------------------|-------|----------------------|--------------|
| `DEF-428` | 🔴 critical | Red | P1 | One `E10030` at the fill after `s = s`/`ys = ys`; no TU; the in-scope read-back runs | _(to be filled)_ |
| `DEF-429` | 🔴 critical | Red | P1 | One `E10030` per affected fill for the conditional/array-literal element and projected stores; the copies stay accepted | _(to be filled)_ |
| `DEF-431` | 🔴 critical | Red | P1 | One `E10030` for `rows[0] = xs; let s = rows[0][0]`; the direct control keeps its report | _(to be filled)_ |
| `DEF-435` | 🟠 major | Red | P1 | One `E10030` at the element-store copy's fill, matching the direct read | _(to be filled)_ |
| `DEF-430` | 🔴 critical | Red | P2 | One `E10030` for the constant read after the runtime store; the constant-index control keeps its report | _(to be filled)_ |
| `DEF-432` | 🔴 critical | Red | P2 | One `E10030` for the temporary-derived element; the direct twin keeps its report | _(to be filled)_ |
| `DEF-433` | 🔴 critical | Red | P2 | One `E10030` for the call-mediated element view (both forms); the direct fill keeps its report | _(to be filled)_ |
| `DEF-437` | 🔴 critical | Red | P3 | Each projection of a raw pointee moves once; read-backs correct; ASan/LSan clean | _(to be filled)_ |
| `DEF-439` | 🟠 major | Red | P3 | Both guard selections run 0; no double-free of the consumed save; ASan/LSan clean | _(to be filled)_ |
| `DEF-440` | 🔴 critical | Red | P3 | The borrowing intrinsic does not bind/release; the loop runs twice; ASan/LSan clean | _(to be filled)_ |
| `DEF-441` | 🔴 critical | Red | P3 | The guarded take empties the selected member; both selections clean; the false path does not call | _(to be filled)_ |
| `DEF-421` | 🟠 major | Red | P4 | The four `Deref`-rooted store forms emit and read back; the conditional base keeps `E10221` | _(to be filled)_ |
| `DEF-423` | 🟠 major | Red | P4 | The pinned target stores in bounds; the conditional self-store is a legal move with a correct destination read; `E11211` stays | _(to be filled)_ |
| `DEF-424` | 🟠 major | Red | P4 | The literal self-read runs before the clear; the constant-index self-copy is skipped; runs 0 | _(to be filled)_ |
| `DEF-436` | 🟠 major | Red | P4 | One `cl_alt` call; the pointee is read and emptied; the correct value is stored; ASan/LSan clean | _(to be filled)_ |
| `DEF-410` | 🟠 major | Red | P5 | One `E10034` per owning instance; the duplicable instance runs; the concrete twin unchanged | _(to be filled)_ |
| `DEF-425` | 🟠 major | Red | P5 | One `E10034` per owning borrow instance; the duplicable instance runs; the take twin keeps `E10026` | _(to be filled)_ |
| `DEF-426` | 🟠 major | Red | P5 | The generic and concrete guards agree; `/0` and MIN/-1 trap `E11210`; no SIGFPE | _(to be filled)_ |
| `DEF-445` | 🟠 major | Red | P6 | The conditional-of-literals read base and nested forms build strictly and run 0; each call once | _(to be filled)_ |
| `DEF-446` | 🟠 major | Red | P6 | The ternary-arm take literal builds strictly and runs 0; the `if`-condition control stays green | _(to be filled)_ |
| `DEF-448` | 🟠 major | Red | P6 | The conditional array literal of places builds strictly and reads the chosen arm | _(to be filled)_ |
| `DEF-427` | 🟠 major | Control (false positive) | P7 | The view-length matrix reads 2; the byte double index is the documented shape; no code change | _(to be filled)_ |
| `DEF-447` | 🟠 major | Control (false positive) | P7 | The filed inline read-base literal and variants are LSan-clean; no code change | _(to be filled)_ |

## Phase 1 — Loan transfer on stored copies

**Reviewed implementation**: _(recorded at review time — the fix commits and the red-cases
baseline)_.
**Lenses**: correctness (P1C), semantics (P1S), and memory-safety (P1M), dispatched in parallel on
the phase diff (AR-31).

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling; include the `DEF-428` snapshot order and the
`DEF-429`/`DEF-435` collector-path evidence)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — Loan sources through runtime, temporary, and call reads

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
memory-safety.

### Verdicts

_(to be filled; the runtime-union, temp-owner, and binding-loan report counts/spans are recorded
here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: include the `DEF-430` direct-runtime-fill blast-radius outcome, AR-8)_

## Phase 3 — Raw-deref move-outs

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: correctness, semantics,
memory-safety, and emission-integrity (AR-31).

### Verdicts

_(to be filled; the single-call, pointer-identity, `MoveOutValue` placement, and guarded-path
evidence are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: the projection-materializer and value-channel extensions, or "none")_

## Phase 4 — Store lowering and evaluation order

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: all four (AR-31; semantics
owns the place rule and the order).

### Verdicts

_(to be filled; the `Deref`-rooted store shape, the pin, the RHS-before-clear, the self-copy skip,
and the single `cl_alt` call are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 5 — Per-instance generic judgments

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: all four (AR-31; the
rejections are memory/ownership; the guard is emission).

### Verdicts

_(to be filled; the `E10034` records/spans, the duplicable acceptances, and the guard parity are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 6 — Literal destinations and conditional arrays

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: all four (AR-31).

### Verdicts

_(to be filled; the destination form, call counts, per-arm stores, and byte-identity controls are
recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled)_

## Phase 7 — Control resolution

**Reviewed implementation**: none (fix-free). **Lenses**: correctness and memory-safety (AR-31).

### Verdicts

_(to be filled: the `DEF-427` view-length matrix and the `DEF-447` LSan matrix at the
execution head; green or reclassified per AR-35(d))_

### Findings and rulings

_(to be filled: the false-positive closure evidence, or the reclassification ruling)_

### Filed rows

_(to be filled: the two rows close as false positives; any new red shape becomes a filed row and
re-enters the register)_

## Close-out

**Verification**: _(to be filled: `make check`, `make verify`, the focused ASan legs, the strict
sweep, the per-phase emission differentials)_. **Row accounting**: _(to be filled: 21 red rows
Done, 2 false-positive controls Done, the two parked majors open, the next-batch count recomputed;
every in-execution filed row checked — AR-36)_. **Code record**: _(to be filled: the phase commits
and the final tree)_. **Residuals**: _(to be filled: any filed row or recorded limitation,
including the `DEF-430` direct-runtime-fill outcome and any `DEF-427` scalar-len note)_.
