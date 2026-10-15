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

## Phase 2 — Guarded, discarded, and carrier roots

**Reviewed implementation**: `fb9732c` plus two correction rounds. **Lenses**: correctness (J2R/J2C)
and memory-safety (F + the memory `J2C-1`), dispatched on the phase diff.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 2 blockers (J2R-1 the guards-only discard swallowed struct-literal effects; J2R-2 the out-of-frame carrier store turned a leak into a UAF) + J2R-3 (the owning-arm gate delta) + J2R-4 (a test-name nit) | The blast-radius table; test integrity; 1,392/1,401 byte-identical with 9 intended diffs; `make check` green |
| memory-safety | FINDINGS — 2 (Finding 1 the kept-view deferred lend UAF [in-diff, causal-revert-proven]; Finding 2 the mixed view/place discard leak) + A1 (= J2R-1) + A2 (`DEF-295` incidentally fixed by the residual gate) + A3/A4 pre-existing records | The probe set with base comparison; no base-clean program defective |

### Fix rounds

| Item | Outcome |
|------|---------|
| J2R-1 + memory A1 | **Fixed**: the discard materialization descends struct fields and array elements; every impure leaf binds its own statement temporary; the struct-literal discard's calls run with the right counts |
| J2R-2 | **Fixed**: the carrier store of a fresh/unnameable backing into an out-of-frame destination rejects (`E10033`); the static twin accepted |
| J2R-3 | **Fixed**: the gate requires non-owning arms; the owning arm keeps its base path (its leak filed as `DEF-317`) |
| Memory Finding 1 (C2) | **Fixed**: the kept-view deferred lend re-binds the root to a scope owner inside its branch |
| Memory Finding 2 (C4b) | **Fixed**: a view-position root over fresh storage binds in the mixed discard |
| Memory `J2C-1` (distinct from the correctness `J2C-1`) | **Fixed**: `view_position_ownerless` gained the `Conditional` arm (both arms judged) plus the arm-fill acceptance — the six element-conditional repros (`s3`, `r2_condpos`, `t3`, `s5`, `h5`, `h7`) are clean |

### Confirmations

| Lens | Result | Detail |
|------|--------|--------|
| correctness | PASS WITH MINORS — 0 blockers | The J2R corrections verified (the effects, the rejection, the gate narrowing); its `J2C-1` = the plan's 2.2.2 evidence line superseded (recorded); `J2C-2` accounting nit |
| memory-safety | FINDINGS — 1 (the element-conditional `J2C-1`) | The first confirmation cleared C1/C3/C4a/C4b and caught C2's incompleteness; the follow-up fixed it, and the parent re-ran the six repros (clean) with the suites at 116/116 plain and ASan, impl 68/68, `make check` exit 0 |

### Filed rows

`DEF-316` (the duplicable member argument), `DEF-317` (the owning-literal arm), `DEF-318` (the
module-initializer sibling), `DEF-319` (the direct literal return), `DEF-320` (the nested carrier),
`DEF-321` (the view-typed `Index` receiver); `DEF-288` extended with the whole-carrier global-store
hole. `DEF-295` is incidentally fixed by this phase's residual gate — phase 4 re-measures and can
close it early.

## Phase 3 — Fixed-array and literal emissions

**Reviewed implementation**: `6666a4d`/`6e86ffa` plus one correction round. **Lenses**: correctness
(J3R) and emission-integrity, dispatched on the phase diff.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | PASS WITH MINORS — 0 blockers | J3R-1 (a leftover `// BISECT` marker), J3R-2 (a stale doc line), J3R-3 (the red ref imprecision: the committed `47306d2` does not compile without the two-line `place.rs` widening), J3R-4 (the phase-2 build gap: `cfb4c1a`/`47306d2` were committed without that widening — a staging miss, repaired in `6666a4d`), J3R-5/J3R-6 (pre-existing, filed); 3,434-program strict differential with no unintended delta |
| emission-integrity | FINDINGS — 2 (Finding 1 the kept view binding's statement-scoped root — a head-only UAF on the `o1_owning_arm` shape that leaked at base; Finding 2 the guarded element-conditional double free) + Finding 3 (= J3R-1) | 4,348-program differential (46 deltas, all in the intended classes); strict builds: no head-only failure; determinism; both profiles |

### Fix round

| Item | Outcome |
|------|---------|
| Finding 1 (critical) | **Fixed**: `own_view_arm`'s literal branch binds the ownerless view positions through the scope-owner path, and the conditional-of-literals store branch drops the `!owning` gate — the root and per-arm owners live in the binding's scope (`keptviewtag` case) |
| Finding 2 (major) | **Fixed**: a guarded element conditional of a drop-needing element type moves per arm into the destination (`guardelemcond` cases) |
| J3R-1/J3R-2 | **Fixed**: the marker and the stale doc line removed |
| J3R-3/J3R-4 | **Recorded**: the baseline line names `47306d2` + the visibility hunk; the phase-2 build gap is noted here |
| J3R-5/J3R-6 + the guarded spellings | **Filed**: `DEF-322` (the silently dropped element store through a conditional base), `DEF-323` (the guarded call-filled view-array literals), `DEF-324` (the guarded literal spellings keeping invalid C) |

### Confirmation

The emission re-probe: **CONFIRMED — 0 findings** — the 55-delta set is all intended, no accept/reject
change, 2,433/2,433 re-emissions byte-identical, both alternate profiles parity, the strict-build
inventory unchanged (39 pre-existing failures, none added).

## Phase 4 — Loop conditions, update order, and store lowering

**Reviewed implementation**: `0666f70`/`e2ab4b6` plus five correction rounds (`7697ecd`, `240c83b`,
`1749655`, `0947475`, `5f2a78b`). **Lenses**: correctness (J4R), emission-integrity, and
memory-safety (F/R/FINAL), dispatched on the phase diff.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 4 blockers | J4R-1 (the receiver settle consumes a place arm = the memory F1), J4R-2 (the compound raw store re-lowers its target), J4R-3 (the owning pointee leak = the memory F2), J4R-4 (the nullable check skipped — an unintended acceptance); J4R-5 (the member-over-deref family) and J4R-6 (the deferred fill) pre-existing and filed; the corrected case's discrimination verified by mutations |
| emission-integrity | no accept/reject deltas; no head-only strict failures on either toolchain; the run oracle independently reproduced the F1 regression (a02, p30, p3, looprecv) | the differential over 3,886 programs; the delta classes; determinism |
| memory-safety | FINDINGS — 2 (F1 the receiver place-arm consumption: SEGV/double-free; F2 the raw-store pointee leak) | the probe set with the base comparison |

### Correction rounds

| Round | Item | Outcome |
|-------|------|---------|
| 1 (`7697ecd`) | F1/J4R-1 + F2/J4R-3 + the pointer warning | **Fixed**: a borrowed-place receiver keeps the place a read (the loop deferred spelling, the eager per-arm temporary with the reading designator); the raw store spells the pointee address so the drop decision sees the pointee type; the pointer evaluates once (`W10000` gone) |
| 2 (`240c83b`) | J4R-2 + J4R-4 | **Fixed**: the compound raw store reads and stores through one address (the shared-target pattern); the raw store requires a non-null pointer (one `E10010`, matching the read twin) |
| 3 (`1749655`) | the raw self-store/move-in hazards (found while characterizing `rawown`) | **Fixed**: the store orders release → clear → store like a named destination and consumes an owned value like a named destination; the two shared-store shape pins re-pinned (recorded) |
| 4 (`0947475`) | F3 (the re-probe's critical: the impure raw-store pointer evaluated four times) | **Fixed**: the non-trivial pointer binds once for the plain path; the compound path re-checked consistent |
| 5 (`5f2a78b`) | FINAL-1 (the owned raw-deref read used as a store value duplicated the owner) | **Fixed**: the source pointee empties through its own address like a named source; FINAL-2 (`*q = *q` then a later owner read) ruled the documented `unsafe` boundary (AR-18) |

### Confirmations

| Probe | Result |
|-------|--------|
| the memory re-probe of round 1 | F1/F2 clean (7 + 4 repros); F3 critical found; determinism identical; the do-not-fix set intact |
| the final re-probe of rounds 2–3 | F3 fixed (single evaluation in the emitted C); the named-destination parity; FINAL-1/FINAL-2 found; the container strict 26/26 and the host strict 24/26 (the two are DEF-327) |
| the parent's re-run of round 5 | `rawdistinct_read` clean; `rawself` clean; `rawmove_use` one `E10020`; `rawself_use` the AR-18 boundary (accepted, SEGV on the emptied owner's later read); the suites 138/138 plain and ASan, the pins 79/79 |

### Filed rows

`DEF-325` (the call fill per element), `DEF-326` (the deferred-arm fill per element), `DEF-327` (the
host sequence-point false positive for the place-borrowing loop receiver — the read-only receiver
carrier noted as the follow-up), `DEF-328` (the member-over-deref family), `DEF-329` (the owned raw
deref read creates a second owner), `DEF-330` (the spurious unused-local warning on a raw source
pointer). `DEF-294` closes per AR-10 (the spec case asserts its environment's leg — the container —
and the host false positive for the place-borrowing spelling is `DEF-327`); `DEF-295` closed
incidentally (the phase-2 residual gate; pinned as the green control `loopmember`).

## Phase 5 — Diagnostics and marks

**Reviewed implementation**: `9539e1f`/`3d91dc8` plus one correction round (`dece1c1`). **Lenses**:
correctness (J5R) and semantics (J5S), plus a bounded semantics confirmation (J5C).

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | FAIL — 1 blocker | J5R-1 (the withheld consequence on a proxy predicate: a suppressed callee report turned a base rejection into an accepted heap-use-after-free; causally revert-proven), J5R-2 (the phrase missed three sibling subject sites), J5R-3 (the mismatch consequence — closed as a boundary by the semantics review), J5R-4 (= `DEF-332`); the dedupe and the mark removal correct in every path probed; 2,137-program differential with no unintended corpus delta |
| semantics | FAIL — 1 blocker | J5S-1 (the gate resolves the callee by written name only: a callable value named like a lender lost its `E10033` — an FN-6/OW-10 violation), J5S-2 (the suppressed-callee acceptance correct per EL-4 but unpinned; the predicate wording inaccurate), J5S-3 (= `DEF-332`); Q1 (the one-report dedupe) and Q4 (the iterable mark) conformant; Q5 closed J5R-3 as a boundary |

### Fix round

| Item | Outcome |
|------|---------|
| C1 (J5R-1/J5S-2) | **Fixed**: the return judgment's `Composite` arm reports the array literal's local component (`E10035`) and suppresses the count postlude — the cause reports and the caller's withholding is sound; the p17/p18/p19b/p20 probes switch from one `E10036` to one `E10035` (same single report, at the cause) |
| C2 (J5S-1) | **Fixed**: `callee_elides_no_single_parameter` mirrors the call precedence (a callable field, a call-returned func, a func-typed binding, a module-global func value takes no withholding) — the callable spellings reject one `E10033` |
| C3 (J5S-2) | **Fixed**: the predicate renamed to the structural condition; the static-callee acceptance pinned as a spec case |
| C4 (J5R-2) | **Fixed**: one shared subject/display helper across the report and the three sibling sites |
| C5 (J5S-3/`DEF-332`) | **Fixed**: the deferred copies' generated ids are tracked (the spelling heuristic removed) — the user binding keeps its name and the deferred phrase holds |

### Confirmation

**PASS WITH MINORS — 0 blockers**: the `E10035` switch confirmed (a view derived from a local owner is `E10035` — `spec/16` BR-6, `spec/06` FN-5, `spec/03` OW-17; the `E10036` was the count approximation); the composite arm covers every component kind (two locals, local+param+static, local+generated, two params, param/static and global/static controls); the precedence confirmed (the callable spellings reject; the module lender controls keep theirs; the `DEF-331` boundary stands); the id tracking confirmed (per-lambda isolation; the take/restore nested correctly); the wording matches the condition (J5C-2 corrected inline). J5C-1 (a rolled-back id re-taken by a later binding renders that binding's name — the site is always rejected, only the subject is wrong) filed as `DEF-334`.

### Filed rows

`DEF-331` (the mismatch consequence — closed as a boundary: an independent cause, not a consequence), `DEF-332`
(fixed), `DEF-333` (incidentally fixed by C1 — the parent re-measured the shape rejecting `E10035`),
`DEF-334` (the rolled-back id subject).

## Close-out

**Scope**: the 27 follow-on rows of the closed `conditional-projection-residue` batch (the open
`DEF-281`–`DEF-310` subset), severity-first: the only open critical (`DEF-303`) plus its machinery
siblings. Five phases; 33 tasks.

**Result**: all 27 rows fixed — 25 implemented and 2 incidental (`DEF-295` via the batch-J phase-2
residual gate; `DEF-294` closed per AR-10 with the host false-positive residue filed as `DEF-327`).
The previously closed `DEF-278`/`289`/`296`/`301` were re-verified as controls. The six older parked
rows (`DEF-167`, `169`, `171`, `190`, `191`, `263`) stayed out of scope pending user rulings (AR-2).

**Filed rows**: `DEF-311`–`DEF-334` (24). Closed among them: `DEF-331` (a boundary, per the phase-5
semantics review), `DEF-332` (fixed in the phase-5 correction), `DEF-333` (incidentally fixed by the
same correction). The remaining 21 stay in Backlog for the following batches.

**Review record**: every phase ran the specification-first loop (spec cases red → implement → green
→ pins with base-fail evidence), then the phase lenses — correctness and memory-safety throughout,
plus emission-integrity (phases 2–4) and semantics (phase 5) — with correction rounds and targeted
confirmations. Across the batch: 12 correction rounds and 8 confirmations. Each phase section above
carries its verdicts, fix rounds, and confirmations.

**Test surface**: the batch reused the `conditional_projection_residue_{spec,impl}_test.rs` pair,
grown 88 → 145 spec tests and 52 → 85 impl pins, additions-only throughout (the one red-phase case
arithmetic corrected per an explicit ruling, recorded in the register).

**Verification**: `make check` exit 0; `make verify` all legs passed (the hosted-amiga
cross-toolchain legs and the runtime-fault programs); the residue spec suite 145/145 plain and under
`RDC_ASAN=1`; the impl pins 85/85; `cargo build --workspace` no new warnings.
