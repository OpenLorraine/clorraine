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

**Targeted confirmation**: dispatched once on the follow-up fix diff; recorded in the phase progress at
task 1.3.2.

## Phase 2 — Guarded receivers and loans

_(to be filled by the Phase 2 review: correctness + memory-safety + semantics verdicts, the fix
round, filed rows)_

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
