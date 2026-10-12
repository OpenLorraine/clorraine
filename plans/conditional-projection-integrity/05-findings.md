# Findings Ledger: Conditional and Projection Integrity

> **Document**: 05-findings.md
> **Parent**: [Index](00-index.md)
> **Status**: Scaffold — the phase reviews append their verdicts, fix rounds, and filed rows here
> during execution.

## Phase 1 — Conditional arm ownership

**Baseline**: `0ae53eabf119f957d4b7b38083d94ce1781cbc88`. **Lenses**: correctness (RV), memory-safety (SA), both
dispatched on the phase diff in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | 1 major, 1 minor | RV-001 (the nested projection-in-arm spelling missed the checker's move marking), RV-002 (a formatting artifact); the reviewer re-ran the suites and the red phase, reproduced the red base (plain 7/19, ASan 12/19, impl 6/7 pins) and traced all five hooks |
| memory-safety | 1 critical | SA-001 (a re-executed `take` of a conditional projection kept the transferred pointer in the caller's arm temporary, so the second execution double-freed it); every other probed shape was clean, including the store/return/defer/guarded arms and the two-field sibling releases |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-001 | 🟠 major | `own/expr.rs` (`projection_move_arm_sources`) | **Fixed**: the arm walk now descends a `Member`/`Index` arm onto its conditional root with the segment appended, so a nested spelling (`d ? (c ? b : mk()).v : z`) records every arm's projected place; the new spec case `armnestedread` pins the `E10020` read-after-store |
| RV-002 | 🟡 minor | `own/lower.rs` | **Fixed** directly: the collapsed brace on `is_pure_value` restored to the file's formatting |
| SA-001 | 🔴 critical | `own/lower.rs` (`materialize_owned_argument`) | **Fixed**: a `take` projection of a conditional now fills one shared carrier temporary per arm (`materialize_take_conditional_projection`), and the ordinary post-call move-out empties it, so a re-executed statement never frees the transferred value twice; the new spec case `armtakeloop` runs both taken arms across two iterations under ASan |

**Verification after the fix round**: spec 21/21 plain and under `RDC_ASAN=1`; impl 7/7; `make check` exit 0
(64 test binaries); the two filed reproducers (`take` in a loop, `defer` + `take` in a loop) are ASan-clean; the
store-path emission is byte-identical to the pre-fix compiler.

### Pre-existing findings (filed, not folded into the phase's verdict)

| Row | Severity | Shape |
|-----|----------|-------|
| DEF-257 | 🟠 major | a generic destination (`let s: T = (c ? a : mk<T>()).v;` instantiated owning) keeps copy semantics and is accepted-unsound at base and head (the generic gate is deliberate) |
| DEF-258 | 🟠 major | a conditional-of-projections spellings' fresh root leaks its sibling (`let s: String = c ? a.v : mkBox().v;`) |
| DEF-259 | 🟡 minor | a struct-literal fresh arm's sibling leaks (`(c ? a : Box { ... }).v`) — the literal root is never bound |
| DEF-260 | 🟠 major | a guarded `take` of a conditional projection double-frees (`g && consume((c ? a : mkBox()).v)`, the `\|\|` and fresh-arm variants alike; byte-identical at base and head) |

### Scoped re-review of the fix round

Both lenses re-ran on the fix diff (`a447db4..33070ad`). The correctness lens closed RV-001 and RV-002 and
filed three follow-ups; the memory lens closed SA-001 for every eager `take` shape and confirmed the store
and return emission is byte-identical.

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| RV-101 = SA-102 | 🟠 major | the new `armtakeloop` spec case (test integrity) | **Fixed**: the case alternated arms, so neither per-arm temporary was re-entered and the base compiler passed it; it now takes the place arm twice and the fresh arm twice, which the base compiler double-frees |
| RV-102 | 🟠 major | `own/expr.rs` (`conditional_move_sources`) | **Fixed**: an arm that is a projection over a conditional and names no single place now contributes its own arms' projected places, so the place-sibling spelling marks the moved member; the new spec case `armnestedplace` pins the `E10020` read |
| RV-103 | 🟡 minor | `own/lower.rs` (a dead per-arm temporary when the carrier exists) | **Fixed** directly: the carrier is passed into the consumption so no unused temporary is allocated |
| SA-101 | 🔴 critical | a guarded `take` of a conditional projection (pre-existing, base == head) | **Filed** as DEF-260 (outside the phase's declared scope; the guarded intrinsic read is DEF-178) |
| SA-001 | 🔴 critical | the eager `take` carrier | **Closed**: every base-failing eager shape (loop, defer copy, nested, func-typed callee, discarded call, two takes, early return/panic) is ASan-clean at head |

**Targeted confirmation**: dispatched once on the follow-up fix diff (`33070ad..3af940b`) and returned
**PASS** (2026-10-12 02:03): the rewritten loop case double-frees on the pre-carrier base under both ASan
and a plain build; the place-sibling program is accepted at the base and rejected `E10020` at head; the
dead per-arm temporary is gone and the carrier is filled by both arms with the post-call move-out intact;
spec 22/22 plain and under `RDC_ASAN=1`, impl 7/7, `make check` exit 0.

## Phase 2 — Guarded receivers and loans

**Baseline**: `e6f8848c8561c161dcb39c317b3a4bef804ec973` (phase 1's final commit tree at `576b424`). **Lenses**:
correctness (RV), memory-safety (SA), semantics (PR), all dispatched on the phase diff in parallel.

### Verdicts

| Lens | Result | Evidence |
|------|--------|----------|
| correctness | no blockers; 1 minor in scope, 1 minor pre-existing | RV-201 (the widened gate also fired for a bare all-ownerless conditional), RV-202 (a fixed-array `.len` folds and never evaluates its receiver); all 39 spec and 11 impl cases verified, the red phase reproduced at the baseline (plain 37/2, ASan 32/7) |
| memory-safety | 1 critical regression; 2 major pre-existing | SA-201 (the guarded struct-literal receiver double-freed in a **loop condition**: the loop-condition walk gave the literal's field call a second owner), SA-202 (a call-wrapped conditional view escapes the loan), SA-203 (a conditional-of-literals receiver in a loop condition double-frees at base and head); every other probed shape clean |
| semantics | no findings | The shared-loan reading verified against Ch 03 §7 and Ch 16 §3 across 18 probes: named/conditional parity for same-member and sibling writes, whole-owner writes, moves, take/borrow parameters, nested chains, element arms, and the arm-less spelling; the corrected `armplace` control judged forced by the frozen rule, not a weakening |

### Findings and rulings

| Finding | Severity | Area | Ruling |
|---------|----------|------|--------|
| SA-201 | 🔴 critical | `own/loops.rs` (`materialize_owned_calls`) | **Fixed**: the materialize skip now covers a struct-literal inner, so the literal's temporary is the single owner of its fields and the loop releases them once per iteration; the new spec case `guardliteralloop` pins the shape (clean at base, double-free at the pre-fix head) |
| RV-201 | 🟡 minor | `own/lower.rs` (`conditional_root_needs_arm_fill`) | **Fixed** directly: the predicate now routes a projection chain per arm whenever its root's arms need a fill, but a *bare* conditional only when an arm borrows a place — restoring the whole-value route for the bare all-ownerless spelling |
| RV-202 | 🟡 minor | `own/call.rs`, `own/lower.rs` (pre-existing) | **Filed** as DEF-263 (spec-silent; needs a ruling on the folded receiver's evaluation) |
| SA-202 | 🟠 major | `own/place.rs` (`view_provenance`; pre-existing) | **Filed** as DEF-261 |
| SA-203 | 🟠 major | `own/loops.rs` (pre-existing) | **Filed** as DEF-262 |

**Verification after the fix round**: spec 40/40 plain and under `RDC_ASAN=1` (the new loop
regression included); impl 11/11; `make check` exit 0; the critical reproducers (the exact loop
condition, a `for`/`do-while` pair, and the `||` twin) are ASan-clean.

### Re-review of the fix round

Pending — dispatched on the fix diff.

## Phase 3 — View provenance and elision

_(to be filled by the Phase 3 review: correctness + memory-safety + semantics verdicts, the fix
round, filed rows)_

## Phase 4 — Statement-path discards

_(to be filled by the Phase 4 review: correctness + memory-safety + emission-integrity verdicts,
the fix round, filed rows)_

## Phase 5 — Emission spellings

_(to be filled by the Phase 5 review: correctness + emission-integrity + semantics verdicts, the
fix round, filed rows)_

## Close-out

_(to be filled by the close-out: the final verification, the roadmap sync, and any late filings)_
