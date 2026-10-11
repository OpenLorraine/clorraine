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
