# Findings Ledger: Memory and Value Integrity (Batch N)

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — each phase review appends its verdicts, fix rounds, and filed rows here.
> The batch-M ledger (`plans/residual-major-completion/05-findings.md`) is the format and method
> reference.
> **Baseline**: head `100180c` (the compiler sources are unchanged since the batch-M close-out
> `f6da78f`)

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
- `DEF-425` is **parked**, not fixed: phase 5 records the frozen-spec conflict, the pinned batch-M
  cases, and the ruling request; the close-out carries it as parked (AR-22).
- Two residual rows are filed at execution, with no code in this batch: the scalar-`len` fold
  (phase 7, from `DEF-427`) and the accepted post-store source read `at1` (phase 4, from
  `DEF-423`) (AR-18, AR-27).

## Row Ledger (empty result cells — the execution fills them)

| Row | Severity | Verdict at `f6da78f` | Phase | Post-fix expectation | Phase result |
|-----|----------|----------------------|-------|----------------------|--------------|
| `DEF-428` | 🔴 critical | Red | P1 | One `E10030` at the fill after `s = s`/`ys = ys`; no TU; the in-scope read-back runs | _(to be filled)_ |
| `DEF-429` | 🔴 critical | Red | P1 | One `E10030` per affected fill for the conditional/array-literal/runtime element and projected stores; the copies stay accepted; the constant sibling stays clean | _(to be filled)_ |
| `DEF-431` | 🔴 critical | Red | P1 | One `E10030` for `rows[0] = xs; let s = rows[0][0]`; incidental if the `429` gate greens it; the direct control keeps its report | _(to be filled)_ |
| `DEF-435` | 🟠 major | Red | P1 | One `E10030` for the constant and runtime element-store copies; incidental (constant) if the `429` gate greens it; the constant direct read (`d435b`) keeps its report and the runtime direct read (`q435runtime_direct`) is the AR-35(e) delta/residual | _(to be filled)_ |
| `DEF-430` | 🔴 critical | Red | P2 | One `E10030` for the constant read after the runtime store; the constant-index control keeps its report | _(to be filled)_ |
| `DEF-432` | 🔴 critical | Red | P2 | One `E10030` for the temporary-derived element; the direct twin keeps its report | _(to be filled)_ |
| `DEF-433` | 🔴 critical | Red | P2 | One `E10030` for the call-mediated element view (both forms); the direct fill keeps its report | _(to be filled)_ |
| `DEF-437` | 🔴 critical | Red | P3 | Each projection of a raw pointee moves once through all three emission gates; read-backs correct; ASan/LSan clean | _(to be filled)_ |
| `DEF-439` | 🟠 major | Red | P3 | The `g = true` selection runs 0; the distinct-length `g = false` selection runs 2 (it reads the stored `y`); the non-aliasing control drops its save; no double-free; ASan/LSan clean | _(to be filled)_ |
| `DEF-440` | 🔴 critical | Red | P3 | The borrowing intrinsic and borrow argument do not bind/release; the loop runs twice; ASan/LSan clean | _(to be filled)_ |
| `DEF-441` | 🔴 critical | Red | P3 | The guarded take empties only the selected member; the sibling stays owned; both selections clean; the false path does not call | _(to be filled)_ |
| `DEF-421` | 🟠 major | Red | P4 | The four `Deref`-rooted store forms emit and read back; the base/index is pinned before the RHS; the structural `.len` keeps `E10221` | _(to be filled)_ |
| `DEF-423` | 🟠 major | Red | P4 | The pinned target stores in bounds; the conditional self-store is a legal move with a correct destination read; `E11211` stays; `at1` is filed as a residual | _(to be filled)_ |
| `DEF-424` | 🟠 major | Red | P4 | The literal self-read runs before the clear; the constant-index self-copy and raw self-alias (`is_raw_self_alias`) are no-ops; the distinct-binding/place controls and following store make a lost store visible; runs 0 | _(to be filled)_ |
| `DEF-436` | 🟠 major | Red | P4 | Each pointer expression runs once (two total for the raw-target form); the pointee is read and emptied; the correct value is stored; ASan/LSan clean | _(to be filled)_ |
| `DEF-410` | 🟠 major | Red | P5 | One `E10034` per owning instance; the duplicable generic whole move runs; the concrete twin unchanged | _(to be filled)_ |
| `DEF-425` | 🟠 major | **PARKED** | P5 record | No fix: frozen BR-5 and the concrete/take twins conflict with the pinned batch-M spec cases; record the evidence and the ruling request | _(to be filled)_ |
| `DEF-426` | 🟠 major | Red | P5 | The generic and concrete guards agree through the resolved projection type; `/0` and MIN/-1 trap `E11210`; no SIGFPE | _(to be filled)_ |
| `DEF-445` | 🟠 major | Red | P6 | The conditional-of-literals read base and nested forms build strictly and run 0; each call once | _(to be filled)_ |
| `DEF-446` | 🟠 major | Red | P6 | The ternary-arm take literal builds strictly and runs 0; the `if`-condition control stays green | _(to be filled)_ |
| `DEF-448` | 🟠 major | Red | P6 | The conditional array literal of places builds strictly and reads the chosen arm | _(to be filled)_ |
| `DEF-427` | 🟠 major | Control (false positive) | P7 | The view-length matrix reads 2; the filed byte probe is accepted and ASan/LSan-clean; the exact `254`/`0` output is an impl pin; a future scalar-`len` fix needs a user ruling; no code change | _(to be filled)_ |
| `DEF-447` | 🟠 major | Control (false positive) | P7 | The filed inline read-base literal and variants are LSan-clean; no code change | _(to be filled)_ |

## Parked and residual rows

These rows are not fixed in this batch. Their evidence is recorded at the stated phase and their
status is carried to the close-out (AR-22, AR-18, AR-27).

| Row | Status | Source | Required record |
|-----|--------|--------|-----------------|
| `DEF-425` | **PARKED** pending the user's ruling | Frozen BR-5 (`spec/16-borrow-rules.md:191`) and the concrete `E10034` (`d425b`)/take `E10026` (`d425d`) twins conflict with the pinned batch-M cases (`conditional_projection_residue_spec_test.rs:14116-14198`, `..._impl_test.rs:7365-7460`), which accept the generic body at exit 0 (`d425a`) | Phase 5, task 5.4.2: the conflict, the case paths, the probes, and the explicit ruling request; no code/spec/test edit |
| `DEF-423`-at1 | Residual use-after-move | `at1` reads `a[1]` after the conditional store; accepted and ASan-red (134, `attempting double-free`) | Phase 4 record: file a new row with the `at1` probe; ST-13 keeps the destination read |
| `DEF-427`-len | Residual scalar-`len` fold | `len(w.items[i][0])` of a `u8` emits the compiler's `0` fallback (`codegen/calls.rs:367`), not a frozen rule; the filed probe runs 254, recorded as an implementation pin | Phase 7 record: file a new row with the probe and the current-output pin; no code in this batch. A future scalar-`len` fix needs a user ruling because the residual row records the current fallback |

## Phase 1 — Loan transfer on stored copies

**Reviewed implementation**: _(recorded at review time — the fix commits and the red-cases
baseline)_.
**Lenses**: correctness (P1C), semantics (P1S), and memory-safety (P1M), dispatched in parallel on
the phase diff (AR-31).

### Verdicts

_(to be filled by the review: one row per lens with its result and evidence)_

### Findings and rulings

_(to be filled: finding ID, severity, area, ruling; include the `DEF-428` exact-self-assignment
skip (reusing `is_exact_self_assign`), the `DEF-429` gate/collector-path evidence with the
`Element(None)` wildcard record and its three-reader match, and the `DEF-431`/`DEF-435`
verification-branch outcome — incidental closure or the applied fallback; record the `DEF-435`
constant direct read and the runtime direct read per AR-35(e))_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 2 — Loan sources through runtime, temporary, and call reads

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: correctness, semantics, and
memory-safety.

### Verdicts

_(to be filled; the wildcard-runtime record, temp-owner, and binding-loan report counts/spans are
recorded here; the `k = 0` outcome and the sibling/caller-parameter controls are recorded)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: include the `DEF-430` direct-runtime-fill blast-radius outcome and the filed
residual if it stays accepted, AR-8; the `p430_sibling`/`q430sibling` acceptance is recorded)_

### Correction round (N2S-1–N2S-4, N2C-1–N2C-3)

The phase-2 lens review filed these corrections against the fix tree and its pins. This round adds
tests and design/ledger notes only; the compiler sources do not change.

| Item | Evidence | Decision and pin |
|------|----------|------------------|
| N2S-1 (semantics blocker) — inline temporary view-array through a call | `let s: str = pick([mk().v; "EE"], 0); fill(s, borrow slot)` (and the direct and element-store forms) is accepted at base `f1ab264` and at the reviewed head `c479e67`; the read is an ASan
heap-use-after-free in both. The lowering materializes the inline array to a generated local (`__cl_owned1`), so `view_owners`/`lowered_*` stop at that place and never name `__cl_owned0`; `frame_owned_view_owner` then sees no positions and a static provenance. | Filed as `DEF-451` (pre-existing; the fix direction is recorded in the roadmap). **No lowering change in this phase.** The design claim is narrowed in `03-02-loan-sources.md`: the binding-loan fallback covers the non-place copy/direct forms pinned by ST-7; a place carrying a materialized inline temporary array is the residual. |
| N2S-2 / N2C-3 — conservative container-call union | `xs[0] = a.v; fill(pick(xs, 1), slot)` and `ys: str[2] = pickArr(xs); fill(ys[1], slot)` accept and run ASan-clean at base; head rejects `E10030` naming the loaned sibling `a` (not `xs`). | **Intended conservative boundary**: the modular call model has no positional precision, so a call result carries the container's loan union; Ch 16 §9's soundness stance rejects when the checker cannot prove the read safe. Pinned by both spellings plus a no-loan container control that runs 0. |
| N2S-3 — take-materialized array-element parity | `consume(take ys[0], slot)` with `ys = [mk().v; ...]` reports two `E10030` at head (the callee take-parameter store at `9:54` and the caller temporary at `13:5`); the direct twin `consume(take mk().v, slot)` reports the same two at base. The array spelling reported only the callee site at base. | **Contractual parity**: the element copy now records the lowered owner, so both spellings carry the same two-site judgment. Pinned as a two-site assertion for both spellings. |
| N2S-4 / N2C-1 — owner-subject phrase | The pre-existing element-store shape `ys[0] = mk().v; fill(ys[0], slot)` reported `a compiler-generated owner` at base and now reports `a statement temporary` at head; the direct `fill(mk().v, slot)` value route reported `a statement temporary` at both. | **Contractual phrase**: `frame_view_owner` maps a generated owner to the statement-temporary phrase, matching the direct twin. The recorded-position case is the discriminator, the value-route case is the control, and both are pinned with the internal `__cl_` spelling and the old phrase absent. |
| N2C-2 — unpinned `check_assign` and spelling breadth | A mutation at `expr.rs:1303` passing `None` instead of the lowered value kept the suites green while `let ys: str[2] = ["AA","BB"]; ys = [mk().v; "EE"]; fill(ys[0], slot)` regressed to acceptance. | Added the assignment spec case, the array-fill and nested-call spellings, and the `check_assign`/nested impl pins. All fail at base `f1ab264`; the mutation now fails the assignment pin while the declaration pin stays green. |

**Round verification**: the six new test functions fail at base `f1ab264` (an accepted program, one
report instead of two, or the old phrase) and pass at head; the controls (the no-loan container,
the direct take twin, the value-route phrase, and the in-scope assignment read) pass at both. The
compiler sources are untouched, so this round changes no emission. The wider probe set it pins
records these base/head differences, all intended: two container-union reject rows
(`containerunionpick`, `containerunionarray`), three temporary-copy reject rows (`assignedcopy`,
`assignedfill`, `assignednested`), one extra take-materialized report site (`takesiteselement`),
and one owner-phrase normalization (`ownerphraserecorded`). The accepted controls re-emit
byte-identically, the emission over accepted programs shows no delta, and there is no head-only
strict failure.

## Phase 3 — Raw-deref move-outs

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: correctness, semantics,
memory-safety, and emission-integrity (AR-31).

### Verdicts

_(to be filled; the per-pointer-expression call count, pointer identity, projected-member
`MoveOutValue` placement, alias-condition text, and guarded-path evidence are recorded here; the
`DEF-439` false-path expectation is corrected to "the false arm stores `y`", with the distinct-length
re-run (`q439alias_false`, `v = "vv"`, `y = "yyyy"`, run 2))_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: the projection-materializer and the three emission gates, the projected-member
`MoveOutValue`, the runtime alias condition, or "none")_

## Phase 4 — Store lowering and evaluation order

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: all four (AR-31; semantics
owns the place rule and the order).

### Verdicts

_(to be filled: the `Deref`-rooted store shape and pinned base/index, the pin, the
RHS-before-clear, the self-copy/self-alias skip (`is_raw_self_alias`, with the distinct-value
controls), and the per-pointer-expression call count are recorded here; the `at1` residual row is
filed)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: new `DEF-` rows, or "none")_

## Phase 5 — Per-instance generic judgments

**Reviewed implementation**: _(recorded at review time)_. **Lenses**: all four (AR-31; the
rejections are memory/ownership; the guard is emission).

### Verdicts

_(to be filled: the `E10034` records/spans, the duplicable generic whole-move acceptance, the
guard parity, and the `DEF-425` park evidence with the ruling request are recorded here)_

### Findings and rulings

_(to be filled)_

### Filed rows

_(to be filled: the `DEF-425` park record and the ruling request; no new fix row)_

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
execution head; the filed byte probe's acceptance and sanitizer-cleanliness, with the exact
`254`/`0` output in the implementation pin; green or reclassified per AR-35(d))_

### Findings and rulings

_(to be filled: the false-positive closure evidence, or the reclassification ruling; the
scalar-`len` implementation pin is a current-output record, not a frozen rule)_

### Filed rows

_(to be filled: the two rows close as false positives; the scalar-`len` fold is filed as a residual
row with no code; any new red shape becomes a filed row and re-enters the register)_

## Close-out

**Verification**: _(to be filled: `make check`, `make verify`, the focused ASan legs, the strict
sweep, the per-phase emission differentials, and the freestanding `E11200` record)_.
**Row accounting**: _(to be filled: 20 red rows Done, 2 false-positive controls Done, `DEF-425`
parked with its ruling request, the two other parked majors open, the two recorded residuals filed,
the next-batch count recomputed; every in-execution filed row checked — AR-36)_. **Code record**:
_(to be filled: the phase commits and the final tree)_. **Residuals**: _(to be filled: the
`DEF-430` direct-runtime-fill outcome, the `DEF-435` runtime-direct-read outcome, the `at1`
source-read row, and the `DEF-427` scalar-`len` row)_.

## Phase 1 — Loan transfer on stored copies

**Reviewed implementation**: `bd77905` (the implementation), `64e739a` (the pins), `d2338e8` (the
corrections). **Lenses**: correctness (N1C), semantics (N1S), memory-safety.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | N1C-1 (the `Element(None)` wildcard recorded the whole binding union, so safe runtime-store programs — a later clean overwrite then a read — were falsely rejected: base accepted + ASan/LSan-clean, head `E10030`); N1C-2 (the wildcard's negative boundary unpinned) |
| semantics | PASS WITH MINORS — 0 blockers | N1S-1 (the same over-report, a conservative reading per Ch 16 §9 but a new rejection of a safe program), N1S-2 (the AR-35(e) intended deltas recorded but unpinned) |
| memory-safety | CONFIRMED — 0 in-diff | The 48 red shapes reject with no TU; the 33 accepted controls are ASan/LSan-clean with byte-identical emission; one pre-existing critical UAF surfaced (a conditional carrier copy of view-holding struct places) and was filed as `DEF-449` |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`d2338e8`) | **Fixed**: a runtime store expands its wildcard over the destination's declared fixed extent into concrete element paths (the wildcard only for an open extent), so a later clean store clears its slot and a safe read is not rejected; added the clean-overwrite and different-prefix acceptance pins, the direct-runtime intended-delta pins, and the `position_*` unit tests; corrected the incidental-closure wording and scoped the design's no-new-rejection sentence to the copy forms |

### Confirmations

| Lens | Result |
|------|--------|
| correctness | **PASS WITH MINORS — 0 blockers**: N1C-1 closed for the reported shapes (the clean overwrite/direct twins accept at both indices), N1C-2 and N1S-2 closed; the sub-row candidate-granularity conservatism (a runtime row's loan union is not cleared by a constant store below it) recorded as a deliberate Ch 16 §9 reading |

### Filed rows

`DEF-449` (the pre-existing conditional carrier copy of view-holding struct places loses its element
loans — a critical UAF; fixed in neither the base nor the phase).

### Record notes

- `DEF-431` closes incidentally under the `DEF-429` gate (the deeper-projection copy); `DEF-435`'s
  constant half closes under the same gate and its runtime half under the `Element(None)` wildcard
  record — recorded in task 1.2.1.
- The deliberate conservative readings: an open extent keeps the wildcard; a clean store at the same
  runtime index does not clear the candidates (no index identity); a constant store below a
  runtime-written row does not clear the row's union. All are within Ch 16 §9 (`:338-339`).
- **Phase-2 note**: `DEF-430` (`xs[k] = a.v; let s = xs[1]`) already rejects under the phase-1
  wildcard record (the base at the phase-2 start accepts nothing here) — the phase-2 spec author must
  re-probe it and reclassify as an incidental closure or a control, not a red case.
