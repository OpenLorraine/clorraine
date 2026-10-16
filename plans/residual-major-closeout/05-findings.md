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

## Phase 3 — Member typing

**Reviewed implementation**: `5491c87` plus two correction rounds (`ea46489`, `b8296d2`) and the
note commit (`4d91949`). **Lenses**: correctness (K3R/K3C2), semantics (K3S/K3C), and
emission-integrity (EM-1..EM-4).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | K3R-1 (the global `raw u8 → cstring` widening — outside the frozen table, with head-only accepts whose C fails), K3R-2 (the fixed-array instance `.ptr` flip), K3R-3 (the pin's `raw u8` half misleading); the enum rule and the typed members otherwise verified |
| semantics | FAIL — 2 blockers | K3S-1 (= K3R-1; Ch 02 §9 "and no others"), K3S-2 (= K3R-2), K3S-3 (the pin record); the view-member typing and the codes conformant |
| emission-integrity | FINDINGS — 4 | EM-1 (the argument cast missing), EM-2 (the substituted-member store cast), EM-3 (= K3R-2, critical), EM-4 (the instance use conversion unjudged) — all the same two roots; no emission delta in the corpus |

### Fix rounds

| Item | Outcome |
|------|---------|
| C1 (K3R-1/K3S-1/EM-1/EM-2) | **Fixed**: the global widening reverted; the view pointer member reads marked by expression and exempted at a cstring destination; the sanctioned cast rendered at the argument and the substituted-member store |
| C2 (K3R-2/K3S-2/EM-3) | **Fixed**: the instance pointer-member arm guarded on a dynamic extent — a fixed array keeps `E10212`, the slice twin accepted |
| C3 (EM-4) | **Fixed**: the `OpaqueMemberUse` instance duty — a generic `.ptr` at an integer destination reports one `E10200` |
| F1a (K3C-2) | **Fixed**: the exemption narrowed to the destination slots and the returns — the user-call argument, the FFI argument, and the struct-literal field report one `E10200`; the conversion case re-scoped per the ruling (the argument half now a rejection) with two new rejection cases |
| F1b (K3C-1) | **Fixed**: the struct-literal field's un-rendered-cast acceptance now reports |
| K3R-3/K3S-3 | **Recorded**: the pin carries the note (the `raw u8` half records a pre-existing const-discard shape deliberately) |

### Confirmations

| Lens | Result |
|------|--------|
| correctness (`ea46489`) | **PASS WITH MINORS — 0 blockers**: the four repros and the wider family at frozen parity (message, span, help); the sanctioned positions cast and build; the four new cases red at the round base; K3C2-1 (a record-wording nit — the conditional merge's `E10200` is a *harmonization*, not frozen parity: the frozen binary accepted it with an invalid C) and K3C2-2 (a comment nit) recorded |
| semantics (`ea46489`) | **FAIL — the scope ruling**: the member's exemption is sanctioned only at the store slots and the return (the frozen table's cstring entries are position-tagged; the stdlib's only `.ptr`-at-cstring spelling is `toCString`'s return) → the tightening round |
| parent re-check (`b8296d2`) | The per-position matrix verified: the return and the store slots accepted (the cast; the sanctioned shapes strict-build); the user-call argument and the struct-literal field each one `E10200`; the suites 108/108 plain and ASan |

### Record notes

- Recorded consequences (conformant, disclosed by the confirmation): `.len` at narrowing integer
  destinations now reports `E10200`, and `.ptr[i]` outside `unsafe` now `E10602` (per Ch 02 §9 and
  Ch 04 §9); no base-working program changed class otherwise.
- Unchanged residuals recorded: the generic return of a member read at `cstring` (`E10036`, a
  pre-existing view-return rule); `(*p).ptr`'s pre-existing shape; the view-typed struct field
  (`E10205`).

### Filed rows

`DEF-346` (the fixed array's `.ptr` and unknown view members on the concrete path — the concrete
half persists), `DEF-347` (the const pointer at mutable destinations), `DEF-348` (the raw-enum
member read), `DEF-349` (the `E10200` help's cast not host-build-clean).

## Phase 4 — Call conversions and ffi typing

**Reviewed implementation**: `5c66f88` plus four correction rounds (`dc77c85`, `53a0517`,
`8bcd9cd`, `382f9ac`). **Lenses**: correctness (K4R), semantics (K4S), emission-integrity (F1),
and memory-safety (0 findings).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers | K4R-1 (the dot-call receiver's deferred conversion was never recorded — the `DEF-242` hole stayed open on that route), K4R-2 (the null-shape record fired at every destination — duplicate reports and head-only element/field rejects), K4R-3 (record wording); the codes/positions/FFI typing otherwise verified |
| semantics | PASS WITH MINORS — 0 blockers | K4S-1 (= K4R-2), K4S-2 (a dependent `E10012` on an `E10011`-poisoned `T?` declaration); the code choices, the destination set, the deferral mechanism, the FFI result typing, and the legal instances all conformant |
| emission-integrity | FINDINGS — 1 | F1 (a view-typed FFI result took the struct-representation path — `len(Make())` dereferenced a `void *`; head-only, the base emitted a silently-wrong `0`); the three named emit classes and the 16 rejects otherwise intended, no head-only accepts, determinism 3,206/3,206 |
| memory-safety | CONFIRMED CLEAN — 0 findings | Every changed accepted shape probed (strict + ASan/LSan); the diff incidentally fixes the FFI-result temporary leaks (`h18`/`h20`) and several invalid-C shapes; one in-diff duplicate-report observation routed to the correctness lens (= K4R-2) |

### Fix rounds

| Item | Outcome |
|------|---------|
| C1 (K4R-1) | **Fixed**: the receiver duty records with the argument shape (the receiver span; the out-slot route; the previous false-positive out-slot receiver now accepts) |
| C2 (K4R-2/K4S-1) | **Fixed**: the null-shape record gates on a parameter-mentioning or unknown destination — the duplicates collapse to one report and the element/field positions match their concrete siblings |
| C3 (K4S-2) | **Fixed**: a bare `T?` destination records no duty (one `E10011`, the dependent `E10012` gone) |
| C4 (F1) | **Fixed**: a view/slice FFI result types as the ABI pointer (`raw u8`/`raw <elem>`, the `?` preserved) — the repro builds and runs the folded length, and the view destinations report one `E10200` each; the raw path now matches the plain-pointer behavior |

### Confirmations

No separate lens re-review was dispatched; the four rounds were verified by the fixer differentials
(no unintended class; no head-only accept), the parent's focused suites (124/124 plain and under
`RDC_ASAN=1`, the impl pins 43/43), `make check` exit 0, and the parent's re-run of the F1 repro
(build + run 0).

### Filed rows

`DEF-350` (the FFI view/slice result — extended with F1's head-only manifestation and the P2
spellings; F1 fixed), `DEF-351` (the inverse conversion routes — extended with the `bs01`
destination-typed shape), `DEF-352` (a raw pointer's `.ptr`/`.len` emit the arrow shape).

### Record notes

- Intended harmonizations recorded: `if (Count())` → `E10218`; `consume(u8)` from an `i32` result →
  `E10200`; the un-narrowed `File?` result → `E10010`; the `e3`/`fr17m` diagnostic-shape changes on
  programs rejected either way. The FFI arithmetic now renders the defined wrapping shape.
- Left as recorded: the out-slot conversion help keeps the conversion text; a receiver whose
  nullability differs from the out-slot type passes the kind-level judgment; the substitution's
  Param arm still drops the declaration `?` marker (worked around at the duty record).

## Phase 5 — Null destinations and capture stores

**Reviewed implementation**: `e6b9a54` plus the correction rounds (`add447c`, `c42e3e9`, `9095674`).
**Lenses**: correctness (K5R), semantics (K5S/K5C), memory-safety (F1), plus two bounded
confirmations.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | K5R-1 (the unread-local walk under-marked null-shaped children of array literals, struct fields, and conditional arms — a spurious unused-local strict failure), K5R-2 (the capture record added a second report for owning non-out-slot destinations), K5R-3 (a stale plan file list), K5R-P1 (the struct compound-initializer field — filed `DEF-355`); the null lowering, the drops, and the capture judgment otherwise verified |
| semantics | FAIL — 2 blockers | K5S-1 (the null-shaped conditional arm at an owning destination — the walk mismatch), K5S-2 (the capture duty fired on a view-widening store — a false rejection), K5S-3 (the predicate's defensive breadth); the destination set, the hosted-amiga zero, the drop order, and the capture rule otherwise conformant |
| memory-safety | CONFIRMED CLEAN — 0 memory classes | Every changed shape probed (strict + ASan/LSan); F1 (non-memory: the null-shaped lowering elided a computed closure call — exit 255) routed to the fix round; `DEF-353`/`DEF-354` filed as pre-existing |
| confirmation | FAIL then PASS | The bounded confirmation found K5C-1 (the `0`-profile pointer conditionals — an in-diff strict failure incl. the driver's `clorraine build -p hosted-amiga` internal compiler error); the scoped re-confirmation after the fix: **PASS — 0 blockers** |

### Fix rounds

| Item | Outcome |
|------|---------|
| K5S-1 | **Fixed**: the destination threads the conditional walk (a null-shaped arm is unread; the redundant zero skipped) — the `File?`/`String?` arms build and run on the host and the amiga leg |
| K5S-2/K5R-2 | **Fixed**: the capture record carries the store destination and the lend target; a duplicable or non-out-slot destination is skipped — the view-widening store accepts (run 5) and the non-out-slot stores keep one `E10034` while the out-slot move keeps one `E10401` |
| K5S-3 | **Fixed**: the null-capable predicate narrowed to the frozen nullable set (handles, `raw`, `cstring`, views/slices) |
| K5R-1 | **Fixed**: the walk's null-shaped skip extended to the conditional arms, array elements/fill, and struct fields — those shapes build and run (host + amiga) with only the harmless marker |
| F1 | **Fixed**: the lowering keys on a place/temporary; a computed null shape (a closure call) runs before the empty store — the side-effect probes exit 0, and the view-destination conditionals were repaired incidentally |
| K5C-1 | **Fixed**: on the `0` profiles a pointer destination spells its own cast (`(char *)0`/`(u8 *)0`) and the conditional's arms take the destination — the repros and the pre-existing literal twins cross-build, two view-argument conditionals were repaired, and the driver builds on all three profiles |

### Confirmations

The scoped re-confirmation: **PASS — 0 blockers** (12/12 host/HA/FS legs; the driver
`clorraine build` on `host`, `hosted-amiga`, and `freestanding`; the arm spellings as designed; the
host shapes byte-identical; determinism 17/17; the differential's only two diffs are the repaired
view-argument conditionals).

### Filed rows

`DEF-353` (a `borrow out: i32` instance loses the store), `DEF-354` (a two-root conditional capture
store reports `E10401` twice), `DEF-355` (the struct-literal compound-initializer field never
reaches the typed-null lowering).

### Record notes

- Pre-existing residuals confirmed unchanged: the concrete scalar element/field nulls (both sides);
  a computed value in a conditional arm at a `str?` view destination (the `?:` mismatch);
  `q04`'s `E10010`.
- The handle destinations keep the integer `0` on the `0` profiles (the `BPTR` ABI).
