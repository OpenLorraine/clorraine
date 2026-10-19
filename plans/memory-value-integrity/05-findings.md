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

**Reviewed implementation**: `5eb62e6` (the projection and take fixes), `44a5b78` (the pins),
`1a93141` and `661f42e` (the correction rounds). **Lenses**: correctness, semantics, memory-safety,
and emission-integrity (AR-31).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers after correction | RV-001 (the alias check missed the false path, `||` right, nested arms, arm takes, and projected members) closed by the runtime flag; RV-004 (the `__cl_cond` source-binding collision) closed by `unique_local`; RV-003 (aggregate containment) filed as `DEF-456`; RV-002 (stale plan state) closed by this record |
| semantics | PASS — 0 blockers after correction | SR-001 (the cross-type pointer comparison emitted invalid C) closed by the `void *` address test; SR-002 (the skipped-take leak under nested short-circuit contexts) closed by the runtime flag; SR-003 (a plain arm move set the take flag) closed by the take-only setter; the containment residual filed |
| memory-safety | CONFIRMED — 0 in-diff after correction | The four projected moves, both guard selections, the intrinsic loop and borrow argument, the guarded member take, the leak variants, the false-returning callee, the nested and arm takes, the cast alias, and the batch-M controls are ASan/LSan-clean; the containment failure mode filed |
| emission-integrity | CONFIRMED — 0 in-diff after correction | Intended deltas only (pointer pin, move-out placement, intrinsic argument, alias flag); no-take stores byte-identical; strict builds over the corpus and the probe matrix; both cross-profile legs; F1/N1 closed, R1/R2 filed |

### Findings and rulings

- The first review found the branch-keyed alias check incomplete (RV-001/SR-002) and the
  cross-type comparison invalid C (SR-001). The correction (`1a93141`) introduced the runtime
  `__cl_take_alias` flag, the condition local, and the address-of-taken-value comparison.
- The bounded confirmation found the type gate missed a cast alias and the fixed condition name
  collided with a source binding; the second correction (`661f42e`) uses the `void *` comparison,
  `unique_local`, and the take-only flag setter.
- The deliberate boundary: the alias test is address equality, not containment; an aggregate or
  subobject containment take is filed as `DEF-456`. The fixed-array conditional destination
  (`DEF-455`), the guarded struct-literal argument/receiver field (`DEF-454`), the nested-temporary
  materialization (`DEF-457`), the named-place `||` skip (`DEF-458`), the mutated pointer re-read
  (`DEF-459`), and the self-move value loss (`DEF-460`) are filed.

### Filed rows

`DEF-454` (guarded struct-literal argument/receiver field), `DEF-455` (conditional fixed-array
destination), `DEF-456` (aggregate containment save granularity), `DEF-457` (capture lost through a
nested temporary), `DEF-458` (named-place take in a skipped `||` right), `DEF-459` (mutated pointer
place re-read), `DEF-460` (conditional self-move through a raw pointer loses the destination).

## Phase 4 — Store lowering and evaluation order

**Reviewed implementation**: `804e95e` (the store fixes), `e095444` (the pins), `5ccbc7d`,
`911795d`, and `4f9b9c1` (the correction rounds). **Lenses**: all four (AR-31; semantics owns the
place rule and the order).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers after correction | RV-001 (the self-copy recognizers over-matched runtime/updating indices and dropped stores, calls, updates, and bounds traps) closed by the stable-expression comparison and the emitted guards; RV-002 (unknown intermediate types emitted `.` for a raw-pointer field) closed; RV-003 (the skips bypassed writability/read-only/borrow checks) closed; RV-004 (duplicated comment) closed; RV-006 (a correction case passed at base) closed by the distinct-length cases; RV-005 closed by this record |
| semantics | PASS — 0 blockers after correction | SR-001 (raw projected self-copies destroyed the value / emitted invalid C) closed by the same-chain recognition and the discarded read; SR-002 (runtime/updating indices were treated as stable) closed; SR-003 (a view element target consumed its owner) closed; SR-004 (raw-field intermediate render) closed; SR-005 (prefix `unsafe` target) closed; SR-006 (the lend check was bypassed) closed; SR-007 (fixed-array and call-rooted aliases lacked the runtime guard) closed in rounds 2–3 |
| memory-safety | CONFIRMED — 0 in-diff after correction | The raw self-copy/alias crash, the fixed-array pointer alias, the call-rooted alias, and the address-of fixed-array alias are clean; the four phase rows, the bounds trap, and the batch-M/L controls are ASan/LSan-clean; the take fixed-array-parameter alias is pre-existing and filed |
| emission-integrity | CONFIRMED — 0 in-diff after correction | The target projections, the pinned index, the literal temporary, the self-copy discard, the `void *` address guards, and the per-pointer call counts are the intended shapes; the strict matrix has no new C failure; the no-take/no-self and corpus differentials are unchanged; F1/F2/F3/N1 closed; the take-parameter address-of guard warning is pre-existing and filed |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `804e95e`, `e095444` | **Applied**: the raw-deref store route covers a member/element chain with a pinned pointer and index; the whole-element target pins always; the fixed-array literal fills a zeroed temporary before the save/clear; the raw self-alias and constant self-copy are no-ops; the element/raw-target RHS runs the owning-deref materializer |
| C1 | `5ccbc7d` | **Resolved SR-001/SR-002/SR-003/SR-004/SR-005/SR-006 and RV-001/RV-002/RV-003/RV-004**: the recognizers compare stable expressions only; the no-op discards-reads the target so the guards and pointer use remain; the checks run before the skip; intermediate chain types are checked types; view targets do not consume; prefix `unsafe` is honored; a runtime address guard covers distinct raw pointers |
| C2 | `911795d` | **Resolved SR-007/F3**: the fixed-array branch guards a stable raw-deref source by the `void *` source/target addresses; a call-rooted raw-deref value takes the aliasable path; the phase distinct cases use distinct lengths |
| C3 | `4f9b9c1` | **Resolved the last memory finding**: an address-of fixed-array source (`*(&a.items)`) is a stable alias source and takes the same runtime guard |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (all four lenses on `e095444`) | **NOT CONFIRMED**: the recognizer over-match, the raw-field invalid C, the writability bypass, the raw projected self-copy crash, the view consume, and the fixed-array/call-rooted aliases |
| B2 (four lenses on `5ccbc7d`) | **NOT CONFIRMED**: the fixed-array pointer alias and the address-of fixed-array alias remained; the other findings were confirmed fixed |
| B3 (four lenses on `911795d`/`4f9b9c1`) | **CONFIRMED**: the fixed-array, call-rooted, and address-of aliases are guarded no-ops with the distinct controls moving; the bounds trap and the checks are restored; the phase closes for the four rows |

### Filed rows

`DEF-461` (a take fixed-array parameter that aliases the raw target; base-equal double free), `DEF-462`
(the raw-field target's intermediate field is not pinned before a side-effecting RHS), `DEF-463` (the
raw-store RHS pointer binding reports a false `W10000` unused-local warning), `DEF-464` (the `at1`
post-store source read is an accepted use-after-move), `DEF-465` (a duplicable fixed-array literal
self-read/fill uses the cleared destination), `DEF-466` (a raw alias element store whose RHS pointee
is the destination loses the value), `DEF-467` (an out-slot borrow raw self-assign is accepted).

### Record notes

- The design's boundary holds: `place_target` stays `Deref`-free; the raw chain walker is
  store-only. The conditional-base and structural `.len` rejections are unchanged.
- The self-copy/alias rule is: evaluate the target and value, run every index and bounds guard, then
  skip only the transfer; a runtime address guard covers distinct pointer expressions. The stable
  comparison never matches a call or an updating index.
- `at1` remains the accepted post-store source read; the phase pins the destination read only.
- The generic fixed-array parameter alias and the address-of take parameter are pre-existing
  (base wrong/C-fail) and are filed, not fixed.
## Phase 5 — Per-instance generic judgments

**Reviewed implementation**: `2f6ad0d` (the conditional effect and the resolved division guard), `8a71022` (the pins), `22600b9` (the view effect, the mixed arms, the generic pin), `0462780` (the source-kind split), and `f09a0f8` (the out-slot source and the duplicable target). **Lenses**: all four (AR-31).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers after correction | RV-001 (the deferred effect preempted the view effect and rejected a generic view conditional) closed by the placement; RV-002 (the division pin used a concrete parameter) closed; RV-003/SR-001 (the mixed arms lost the concrete source) closed by the merge; RV-004 closed by this record |
| semantics | PASS — 0 blockers after correction | SR-001 (mixed arms) closed; SR-002 (a concrete local/take source lost its definite move) closed by the source-kind split; SR-003 (the out-slot source was treated as a lend) closed by excluding out-slots; the pre-existing return/field/deref residuals filed |
| memory-safety | CONFIRMED — 0 in-diff after correction | The `T = str` view instance is accepted; the duplicable-target take source copies; the out-slot source moves and reports; no base-clean → head-red memory defect; the maybe-target concrete-lend double-free is base-equal and filed |
| emission-integrity | CONFIRMED — 0 in-diff after correction | The generic/concrete division guards are three/three with identical text and order; the view conditional emission matches the concrete twin; the no-take/corpus differential is unchanged; the DEF-426 pin is generic and base-red |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `2f6ad0d`, `8a71022` | **Applied**: the deferred conditional effect and the resolved division guard; the pins |
| C1 | `22600b9` | **Resolved RV-001/RV-002/RV-003**: the effect runs after the view effect; only a type-parameter arm takes it; concrete arms merge; the division pin is generic |
| C2 | `0462780` | **Resolved SR-002 and the T=str view instance**: the sources split by binding kind; the maybe-target branch keeps only type-parameter sources |
| C3 | `f09a0f8` | **Resolved SR-003 and the duplicable-target regression**: out-slot sources move definitely; a duplicable destination copies |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (on `8a71022`) | **NOT CONFIRMED**: the view effect was preempted, the mixed arms lost a source, the DEF-426 pin was not generic |
| B2 (on `22600b9`) | **NOT CONFIRMED**: the concrete local/take later move was lost, the `T = str` view instance was rejected, the out-slot source was accepted, the duplicable target was rejected |
| B3 (on `f09a0f8`) | **CONFIRMED**: the two rows and the controls are clean; the phase closes |

### Filed rows

`DEF-468` (generic conditional return double-free), `DEF-469` (generic field-projection conditional arm), `DEF-470` (generic field-projected take later move), `DEF-471` (hidden generic deref-chain division), `DEF-472` (maybe-target concrete lend double-free), `DEF-473` (maybe-target take + concrete local use-after-move), `DEF-474` (generic view conditional return divergence).

### Parked row

`DEF-425` remains **PARKED** pending the user's ruling: the batch-M cases accept the generic borrow-element move, while frozen BR-5 and the concrete/take twins require `E10034`/`E10026`. No code, test, spec, or registry change was made for it; the park evidence is the existing batch-M case paths and the `d425a`/`d425b`/`d425d` probes.

### Record notes

- The deferred effect is per-instance: a lend parameter reports `E10034` at an owning instance; a local/take source moves definitely at a concrete destination; a maybe-owning destination keeps the per-instance path so a duplicable instance copies.
- The division guard resolves the operand through the instance; float division stays unguarded.
- The view effect remains ahead of the deferred effect, so a view target records its loans rather than an owning move.
## Phase 6 — Literal destinations and conditional arrays

**Reviewed implementation**: `9a44220` (the predicate recursion, the fixed-array take route, and the guard-store arm), `9a1c0b2` (the pins), `020fd4c` (the owned-field gate, the guarded take deferral, and the source order), `589bb0d` (the read-base storage-arm key), and `61c1572` (the fresh-root projection guard). **Lenses**: all four (AR-31).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 in-diff blockers after correction | RV-001 (the owned conditional field used the copy form and double-freed) closed; RV-002 (the read base moved a place arm) closed by the storage-arm key; RV-003 (the guarded take filled eagerly) closed; RV-004/RV-005 and the nested/shape gaps are base-red residuals; RV-006 closed; RV-007 closed by this record |
| semantics | PASS — 0 in-diff blockers after correction | SR-001 (the owned mixed read base moved the place) closed; SR-002 (the guarded conditional array filled before the guard) closed; SR-003 (the expression-form field order) closed, also fixing the pre-existing PRE-1 order/use; the owned all-literal read base stays green |
| memory-safety | CONFIRMED — 0 in-diff after correction | The owned conditional-field double free/UAF, the read-base place move, and the fresh-root projection leak are closed; no base-clean → head-red leak/double-free/UAF; the pre-existing call-arm leaks and the base-red no-TU shapes are filed |
| emission-integrity | CONFIRMED — 0 in-diff after correction | The hidden destinations are bound, no brace-list value survives, the guarded fill is inside the evaluated arm, the source order matches Ch 04 §3, the strict matrix has no new C failure, re-emission is byte-identical, and both profiles build |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `9a44220`, `9a1c0b2` | The literal-read recursion and read-base materialization; the fixed-array conditional take temp; the guard-store arm; the pins |
| C1 | `020fd4c` | The owned-field gate; the guarded take deferral; the source-order field loop; the read-base restriction |
| C2 | `589bb0d` | The read-base skip keyed on a caller-storage arm, restoring the owned all-literal base |
| C3 | `61c1572` | An owning fresh-root projection keeps the projection form, so the root's unselected fields are not lost |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (on `9a1c0b2`) | **NOT CONFIRMED**: the owned conditional field double-freed, the owned mixed read base moved the place, the guarded take filled eagerly, and the expression-form order was declaration order |
| B2 (on `020fd4c`) | **NOT CONFIRMED**: the owned all-literal read base regressed to a C failure; the mixed-place read-base fix was over-broad |
| B3 (on `589bb0d`/`61c1572`) | **CONFIRMED**: all named rows and the owned/guarded/order cases are green; the fresh-root projection no longer leaks; the remaining shapes are base-red or base-equal and filed |

### Filed rows

`DEF-475` (owned mixed conditional read base), `DEF-476` (guarded duplicable conditional read base), `DEF-477` (nested conditional arms), `DEF-478` (duplicable struct conditional take in a guard), `DEF-479` (guarded nested-literal field order), `DEF-480` (owning fresh-root projection read base), `DEF-481` (call-arm read-base leaks), `DEF-482` (all-fresh guarded read base with mixed call/literal arms).

### Record notes

- The guarded literal forms are deferred on the evaluated path, so a skipped outer arm never runs the arm's calls. The expression-form field store follows the written field order; the statement form already did.
- The read-base rule is: materialize a conditional when every arm is fresh (a literal or call); a caller-storage arm keeps the projection form, and a projection of a fresh owning root also keeps it. The owned mixed and fresh-projection shapes are base-red C failures (no memory defect).
- The owned conditional field keeps the save/move/`cl_zero` route; only a non-owning field uses the per-arm copy.
- The `struct_assign_expr` source-order fix closes the pre-existing PRE-1 move-before-borrow SEGV.
- No spec/registry edit; the frozen rules (Ch 04 §3/§5, Ch 17 §5) decide the phase.
## Phase 7 — Control resolution

**Reviewed implementation**: none (fix-free; controls and pins only: `1b054b4`, `6c9c4f5`). **Lenses**: correctness and memory-safety (AR-31).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS — 0 blockers | The `DEF-427` double index is a `u8` (Ch 08 §4), so `len` of it is not the element length; the frozen view-length matrix (runtime/constant/owned/loop) is green; the filed probe is accepted and sanitizer-clean; the spec control does not assert its exit, and the impl pin records the `254`/`0` fallback with the future-ruling note. `DEF-447` and its nested/owned/grid/direct variants are LSan-clean; the inline literal binds at the function body's own scope and releases once. The three review minors (the body-scope anchor, the exact filed-function run, and the bounded ASan helper) are closed. No compiler/spec/registry edit. |
| memory-safety | CONFIRMED — 0 findings | `d427p` is 254 plain and ASan with no sanitizer output; `d447a` and the variants are 0 and LSan-clean; a deliberate leak was detected, so the ASan leg is real. |

### Findings and rulings

- **`DEF-427` closes as a false positive.** The filed `len(w.items[i][0])` indexes a `str` twice: the second index yields a `u8`, and `len` of a byte is not the element length. The expected `2` belongs to `len(w.items[i])`, which is green in every matrix spelling. The scalar-`len` fold is current output, not a frozen rule; `DEF-483` files it, and a future fix needs a user ruling.
- **`DEF-447` closes as a false positive.** The filed inline literal and every named variant are LSan-clean at the execution head; the body-scope binding is the mechanism the row described. No reproduction exists to fix.

### Filed rows

`DEF-483` (the scalar-`len` fold for a `u8` argument; current `0` fallback, future fix needs a user ruling). `DEF-427` and `DEF-447` move to Done with the control/probe evidence.

## Close-out

**Verification**: `make check` exit 0 (the frozen `spec/check.sh` reports 27 files and 110 compile-fail cases; the workspace suites pass). `make verify` exit 0, covering the full `make check` plus every hosted-amiga cross-toolchain leg: the portable corpus under `m68k-amigaos-gcc` and `vamos`, the vbcc build-only leg, and the build-only interrupt-counter and graphics programs. The focused sanitizer legs are green: residue spec 220/220 with `RDC_ASAN=1`; array-value spec 34/34 with `RDC_ASAN=1`; residue impl 223/223; array-value impl 16/16. The strict-build sweeps over the corpus, the examples, the P1–P7 probes, and the batch-M/L controls report no new C failure; every per-phase emission differential is recorded in its phase section (P3: the projected moves, the alias flag, the intrinsic argument; P4: the target projection, the pinned index, the literal temporary, the guarded aliases; P5: the guard count/text and the view parity; P6: the bound destinations, the per-arm stores, and the source order). The freestanding compile-only legs for the eligible scalar P4/P6 forms build; the ineligible forms report the recorded `E11200` reason (no libc/console surface), and `host`/`hosted-amiga` build the same forms. The specialist check was refreshed with `analyze-agents`: outcome **None** (the existing correctness, semantics, memory-safety, and emission-integrity agents cover the batch; no new specialist recurrence), 39 plans recorded in `codeops/specialist-check.json`.

**Row accounting**: all **22/22 active rows are Done** — the 20 red rows are fixed with the per-phase spec/impl/ASan evidence and the two false-positive controls (`DEF-427`, `DEF-447`) are Done with their control and probe evidence. `DEF-425` is **Deferred (parked)** pending the user's ruling: the pinned batch-M cases accept the generic borrow-element move while frozen BR-5 and the concrete/take twins require `E10034`/`E10026`. The other parked majors (`DEF-167`, `DEF-171`) stay open. The rows filed during execution (`DEF-449`..`DEF-483`, 35 rows) are present in the roadmap with a reproduction, base evidence, and a fix direction; `DEF-481` is the pre-existing call-arm read-base leak class and `DEF-483` the scalar-`len` current-output fold. The roadmap now holds 472 `DEF-` tracker rows: 332 Done, 139 open, and 1 Deferred; the next batch is the 139-row open backlog. The batch accounting is explicit in the roadmap's `## Batch N accounting` section and the execution plan's close-out tasks.

**Code record**: phases 1–7 and the close-out are committed on `main` (phase 1 `70ef134`..`1efa342`; phase 2 `f1ab264`..`8ca7f06`; phase 3 `99a4c7e`..`909e0a9`; phase 4 `87040fb`..`06409cc`; phase 5 `4a113c3`..`a88e3dd`; phase 6 `98ace8c`..`796fcf5`; phase 7 `1b054b4`..`3795224`; the close-out commit is the final tree). The working tree is clean apart from the untracked `.opencode/` tooling catalog; no `spec/` oracle, registry, or generated C was edited, and no new crate or build script exists.

**Residuals**: the phase-1/2 incidental closures: `DEF-430` (the call-result binding) and `DEF-431`/`DEF-435` (the phase-1 gate) closed without code of their own. The phase-4 `at1` source read is `DEF-464`; the phase-7 scalar-`len` fold is `DEF-483`; the phase-6 read-base/nested/guarded residuals are `DEF-475`..`DEF-482`; the phase-2/3 residuals are `DEF-449`..`DEF-460`; the phase-4/5 residuals are `DEF-461`..`DEF-474`. No residual was silently fixed or reopened.

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

## Phase 2 — Loan sources through runtime, temporary, and call reads

**Reviewed implementation**: `2443d91` (the implementation), `9430b3c` (the pins), `938beaa` (the
corrections). **Lenses**: correctness (N2C), semantics (N2S), memory-safety.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | N2C-1 (the owner-subject phrase changed for a pre-existing report), N2C-2 (the `check_assign` threading unpinned — a mutation kept the suites green), N2C-3 (the conservative container union unpinned); P1 (a call-mediated view stored in a global) filed as `DEF-453` |
| semantics | FAIL — 1 blocker | N2S-1 (a call-mediated view over an inline temporary-derived array is still accepted — pre-existing and filed as `DEF-451`; the phase's claim narrowed to the pinned non-place forms); N2S-2..N2S-4 minors; PRE-1 (the generated-owner lifetime extension vs the frozen D5 rule) filed as `DEF-452` for a ruling |
| memory-safety | CONFIRMED — 0 in-diff | The red shapes reject with no TU; the accepted controls are ASan/LSan-clean with byte-identical emission; two pre-existing critical UAFs filed (`DEF-450`, `DEF-451`) |

### Fix rounds

| Round | Outcome |
|-------|---------|
| C1 (`938beaa`) | **Resolved**: the phase claim narrowed (the non-place copy/direct forms are covered; a materialized inline temporary array is the filed `DEF-451` residual); the conservative container-call union pinned with a no-loan control; the two-site take-report parity pinned; the owner-subject phrase pinned per route; the assignment/fill/nested call-result cases and the `check_assign` impl pins added (the `None` mutation now fails them) |

### Confirmations

| Lens | Result |
|------|--------|
| all three | **CLEAN**: N2S-1 scoped and filed (`DEF-451`), N2S-2/3/4 and N2C-1/2/3 closed; the base discrimination, the mutation check, the suites (204/204 + 204/204 plain and ASan), and `make check` green; no compiler source changed since `9430b3c` |

### Filed rows

`DEF-450` (a cross-frame out-slot escape through a borrowed aggregate argument), `DEF-451` (a
call-mediated view of an inline temporary view-array), `DEF-452` (the generated-owner lifetime vs
D5 — a ruling request), `DEF-453` (a call-mediated view stored in a global dangles).

### Record notes

- `DEF-430` is an **incidental closure**: the phase-1 fixed-extent wildcard already reports the
  k=0/k=1 runtime-store copies and the direct runtime reads; the constant sibling stays accepted.
- The deliberate conservative readings: a call-mediated view over a container reports the whole
  binding union (the modular call model has no positional precision); the container sibling's
  rejection is pinned with a no-loan control.
- The two-site take report matches the base direct twin's parity and is pinned.
- The generated-owner lifetime extension (`__cl_owned0` at body scope) contradicts the frozen D5
  temporary rule; `DEF-452` requests the ruling (extend D5 for generated owners, or reject the
  next-statement read).

## Phase 3 — Raw-deref move-outs

**Reviewed implementation**: `5eb62e6` (the projection materializer, the emission gates, the
intrinsic/non-take rule, and the projected take move-out), `44a5b78` (the pins), `1a93141` (the
runtime alias flag), `661f42e` (the take-only flag, the unique condition local, and the `void *`
comparison). **Lenses**: correctness (N3C), semantics (N3S), memory-safety, and emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers after correction | RV-001 (the branch-keyed alias check missed the false-returning callee, `||` right, nested arm, materialize arm, and projected-member takes) closed; RV-004 (a source binding named `__cl_cond` was shadowed) closed; RV-003 (aggregate containment) filed as `DEF-456`; RV-002 (stale Progress and task state) closed by this record |
| semantics | PASS — 0 blockers after correction | SR-001 (unrelated-type take emitted `if ((&(v)) == p)`, invalid C under `-Werror`) closed by the `void *` address test; SR-002 (a skipped `&&` take under `||`/`?:`/`!`/comparison zeroed the save and leaked) closed by the runtime flag; SR-003 (a plain arm source move set the flag and leaked the save) closed by the take-only setter |
| memory-safety | CONFIRMED — 0 in-diff after correction | The four projected moves, the alias/non-alias/false selections, the intrinsic loop and borrow argument, the guarded member take, the six leak variants, the false-returning callee, the nested and arm takes, the cast alias, and the batch-M `m375_*`/`m376_*` controls are ASan/LSan-clean; the containment failure-mode change is filed |
| emission-integrity | CONFIRMED — 0 in-diff after correction | The P3 differential classes are the intended pointer pin, move-out placement, intrinsic argument shape, and alias flag; no-take conditional stores are byte-identical to `44a5b78`; 121 probes strict-build with no C failure; the repository differential over the run corpus is only `d439a`/`d439b`; both cross-profile legs link; F1/N1 closed, R1/R2 filed |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `5eb62e6`, `44a5b78` | **Applied**: `pin_deref_projection` walks Member/Index to the Deref root; `value_moves_pointee` covers the three emission gates and the guarded move-out; `deref_rooted` covers the take move-out and the intrinsic/non-take rules; the projected member is recorded instead of the root; the conditional store adds the first alias check |
| C1 | `1a93141` | **Resolved SR-001/SR-002**: the runtime `__cl_take_alias` flag is set in the take's own move-out comma sequence; the condition evaluates once into a local; the save is zeroed before the branches and re-checked after the arms; the address is the taken value's address, not the pointer operand |
| C2 | `661f42e` | **Resolved NU-1/SR-003/N1**: only take move-outs set the flag; the condition local uses `unique_local`; both compared addresses cast to `void *` with no nominal-type gate, so a cast alias is recognized and an unrelated object compares unequal |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (all four lenses on `44a5b78`) | **NOT CONFIRMED**: SR-001 cross-type invalid C; SR-002 skipped-take leak; RV-001 false-path and arm takes; RV-004 condition-name collision (found in B2); F2/R1/R2 base-equal residuals |
| B2 (four lenses on `6f6546b`) | **NOT CONFIRMED**: the cast-alias regression (the type gate dropped the cast case) and the `__cl_cond` collision; the original leak and false-path findings confirmed fixed |
| B3 (four lenses on `661f42e`) | **CONFIRMED**: all correction repros clean; the take-only flag and `void *` comparison verified; the containment failure-mode change accepted as a filed residual (`DEF-456`); the phase closes for the four rows |

### Filed rows

`DEF-454` (a guarded struct-literal call argument or receiver copies a raw-deref owning field
instead of moving it; base-equal, ASan double-free on the argument), `DEF-455` (a conditional
fixed-array destination has no take capture; base-equal double-free), `DEF-456` (the alias test is
address equality, not containment: a non-first-member take releases the whole save and double-frees,
while a first-member take into a whole-struct destination zeroes the whole save and leaks the
sibling — base-red, a failure-mode change at offset zero), `DEF-457` (the capture does not survive a
nested temporary materialization, `y439nestedarg`), `DEF-458` (a named-place take in a skipped `||`
right zeroes the save unconditionally), `DEF-459` (a pure pointer place mutated by the callee is
re-read by the move-out/alias test), `DEF-460` (a conditional self-move through a raw pointer loses
the destination).

### Record notes

- The alias test is deliberately an address equality over the taken value, not containment: the
  design's `take_ran && &pointee == &target` is emitted as the flag plus the `void *` comparison.
  The containment cases are filed in `DEF-456`; the aggregate/offset-zero sub-case changes from a
  double free to a sibling leak, which the reviewers accepted as a residual on a base-red shape.
- The plain-arm flag regression (SR-003) is the only setter that the take-only split needed: the
  statement and guarded take move-out loops call `emit_take_alias`; `emit_arm_source_moves` never
  sets the flag.
- The condition local is created only when a qualifying pointee exists, so a no-take conditional
  store emits the original direct condition. The batch-M conditional-store pins stay green.
- The frozen-spec reading: Ch 04 §5 (the chosen path only), Ch 16 §2 (a read does not move), and
  Ch 17 §5 (copy plus zero, one release per owner) decide the four rows; no spec or registry edit
  was needed.

## Phase 4 — Store lowering and evaluation order

**Reviewed implementation**: `804e95e` (the raw store route, the unconditional index pin, the
fixed-array literal temporary, the self-copy skip, and the element/raw-target materializer),
`e095444` (the pins), `5ccbc7d` (the stable-expression recognizers, the guarded no-op, the check
ordering, the intermediate types, the view/prefix handling, and the runtime address guard),
`911795d` (the fixed-array and call-rooted guards, the distinct-length cases), and `4f9b9c1` (the
address-of fixed-array source). **Lenses**: correctness (N4C), semantics (N4S), memory-safety, and
emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers after correction | RV-001 (different runtime/updating indices were treated as stable: stores, calls, updates, and bounds traps were dropped) closed; RV-002 (a raw-pointer field rendered `.` through an unknown intermediate type) closed; RV-003 (the skips bypassed const/read-only/borrow checks) closed; RV-004 (duplicated comment) closed; RV-006 (a correction case passed at base) closed by distinct lengths |
| semantics | PASS — 0 blockers after correction | SR-001 (raw projected self-copies destroyed the value and two fixed spellings failed the strict build) closed; SR-002 (runtime/updating index self-copies were skipped) closed; SR-003 (a raw view-element store consumed its owner) closed; SR-004 (intermediate raw-field rendering) closed; SR-005 (prefix `unsafe` target) closed; SR-006 (borrow-parameter self-copy bypassed E10034) closed; SR-007 (fixed-array and call-rooted aliases) closed in C2/C3 |
| memory-safety | CONFIRMED — 0 in-diff after correction | The raw member/element self-copy crash, the fixed-array pointer alias, the call-rooted alias, and the address-of fixed-array alias are ASan/LSan-clean; the four rows, the bounds trap, and the batch-M/L controls are green; the take fixed-array-parameter alias is pre-existing (base 255) and filed |
| emission-integrity | CONFIRMED — 0 in-diff after correction | The target projection, the pinned index, the literal temporary, the discarded self-read, the `void *` alias guards, and the one/two-call counts are intended; the strict matrix has no new C failure; the no-take/corpus differential is unchanged; the take-parameter address-of clear warning is pre-existing and filed |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `804e95e`, `e095444` | **Applied**: the store route and pins; the literal temporary; the self-copy skip; the materializer |
| C1 | `5ccbc7d` | **Resolved the first review**: stable-expression self-copies; discarded read for the no-op; checks before the skip; intermediate types; view no-consume; prefix `unsafe`; runtime alias guard; fixed-array conditional self-arm |
| C2 | `911795d` | **Resolved SR-007/F3**: the fixed-array runtime address guard; the call-rooted aliasable path; the distinct-length cases |
| C3 | `4f9b9c1` | **Resolved the address-of fixed-array alias** |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (on `e095444`) | **NOT CONFIRMED**: the recognizer over-match, the raw-field invalid C, the writability bypass, the raw self-copy crash, the view consume, and the fixed-array/call-rooted aliases |
| B2 (on `5ccbc7d`) | **NOT CONFIRMED**: the fixed-array pointer alias and the address-of fixed-array alias |
| B3 (on `911795d`/`4f9b9c1`) | **CONFIRMED**: all phase rows and the correction findings are clean; the phase closes |

### Filed rows

`DEF-461` (take fixed-array parameter alias), `DEF-462` (raw-field target RHS order), `DEF-463`
(raw-store RHS unused-local warning), `DEF-464` (`at1` post-store source read), `DEF-465`
(duplicable fixed-array literal self-read/fill), `DEF-466` (raw alias element store value loss),
`DEF-467` (out-slot borrow raw self-assign).

### Record notes

- The bounds and place rules are unchanged: a runtime index still pins and traps; the conditional
  base and structural `.len` still reject with one `E10221`.
- A self-copy is a no-op transfer, not a no-op statement: the target/value evaluate, the index
  guards run, and the selected storage is read (the discard read keeps the pointer used).
- The address guards compare through `void *`, so a fixed-array parameter's C-adjusted address
  compiles and a cast or address-of alias is recognized.
- The `at1` source read stays accepted; the phase pins only the destination read.
- Pre-existing residuals: the take fixed-array parameter alias (base wrong/C-fail), the raw-field
  RHS order (host-correct but unspecified), and the raw-store unused-local warning (base has the
  bare form).

## Phase 5 — Per-instance generic judgments

**Reviewed implementation**: `2f6ad0d` (the deferred conditional effect and the resolved division guard),
`8a71022` (the pins), `22600b9` (the view effect, the mixed-arm merge, the generic division pin),
`0462780` (the source-kind split), and `f09a0f8` (the out-slot source and the duplicable target).
**Lenses**: correctness (N5C), semantics (N5S), memory-safety, and emission-integrity.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers after correction | The deferred effect preempted the view effect (RV-001); the division pin used a concrete parameter (RV-002); the mixed arms lost the concrete source (RV-003/SR-001); all closed |
| semantics | PASS — 0 blockers after correction | The mixed arms report both sources; a concrete local/take source moves definitely (E10020); an out-slot source moves; a view target records loans |
| memory-safety | CONFIRMED — 0 in-diff | The `T = str` view instance, the duplicable target, and the out-slot rejection are clean; no base-clean → head-red defect |
| emission-integrity | CONFIRMED — 0 in-diff | The generic/concrete division guards are identical; the view conditional emission matches the concrete twin; the corpus/no-take differential is unchanged |

### Fix rounds

| Round | Commits | Outcome |
|-------|---------|---------|
| I1 | `2f6ad0d`, `8a71022` | The deferred conditional effect and the resolved guard; the pins |
| C1 | `22600b9` | The view effect runs first; only a type-parameter arm defers; the concrete arms merge; the division pin is generic |
| C2 | `0462780` | The sources split by binding kind; the maybe-target branch keeps only type-parameter sources |
| C3 | `f09a0f8` | The out-slot source moves; a duplicable destination copies |

### Confirmations

| Round | Result |
|-------|--------|
| B1 (on `8a71022`) | **NOT CONFIRMED**: view preemption, mixed-arm loss, non-generic pin |
| B2 (on `22600b9`) | **NOT CONFIRMED**: local/take move lost, `T = str` rejected, out-slot accepted, duplicable target rejected |
| B3 (on `f09a0f8`) | **CONFIRMED** |

### Filed rows

`DEF-468` (generic conditional return), `DEF-469` (field-projection arm), `DEF-470` (field-projected take later move), `DEF-471` (hidden deref-chain division), `DEF-472` (maybe-target concrete lend), `DEF-473` (maybe-target take + concrete local), `DEF-474` (generic view conditional return divergence).

### Record notes

- `DEF-425` stays parked with the batch-M evidence and the ruling request; no oracle edit.
- The simple generic take later move is fixed incidentally by the phase (base accepted; head reports `E10020`); it is not a residual.
- The duplicable-instance later use stays accepted because the maybe-owning destination keeps every source on the per-instance path.
