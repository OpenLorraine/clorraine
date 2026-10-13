# Conditional and Projection Integrity Implementation Plan

> **Feature**: Close 14 defect rows in the conditional, projection, and release machinery: the
> guarded intrinsic argument of an owned conditional projection double-free, the nested conditional
> arm's owned-store double-free, the returned view whose elision candidate differs from its actual
> derivation (all three criticals), the mixed-conditional consuming-position leak, the `str[N]`
> loan gap, the conditional place-arm loan gap, the guarded struct-literal and dot-`len` receiver
> leaks, the conditional-return spelling, the postfix sequence point, the discarded closure-call and
> conditional-call releases, the owned fixed-array projection emission, and the discarded `.len()`
> statement emission
> **Status**: Complete
> **Created**: 2026-10-12 00:55
> **Implements**: DEF-176, DEF-177, DEF-178, DEF-179, DEF-180, DEF-181, DEF-182, DEF-183, DEF-184,
> DEF-185, DEF-186, DEF-187, DEF-188, DEF-256
> **CodeOps Artifact Schema**: 1

## Overview

This is batch H of the user-mandated defect program: the conditional/projection ownership and
release cluster that the callable-release-integrity reviews filed (plus one row the batch's own
scoping probes found). The defects share two roots. First, **conditional arms are not always moved
out per arm**: a projection of a conditional in a consuming position copies the select instead of
moving the chosen arm's member, and a guarded intrinsic argument materializes into a temporary that
aliases a place arm — three shapes double-free or read freed storage. Second, **the guarded binder
and the elision postlude have routing gaps**: a struct-literal receiver, a dot-`len` receiver, and a
conditional place arm do not reach the guards and loans that the named spellings already get, and a
returned view is accepted under an elision candidate that is not its actual source.

## Scope

The batch closes the 13 filed rows of the conditional/projection cluster (`DEF-176` – `DEF-188`,
three criticals, two majors, eight minors) and `DEF-256` (a discarded `.len()` statement emits an
undefined `len(...)` call, found by the batch's scoping probes). Every fix is conformance to the
frozen specification; no chapter or registry text changes, and the batch's ambiguity register
records the delegated resolutions.

## Documents

| Document | Purpose |
|----------|---------|
| [00-ambiguity-register.md](00-ambiguity-register.md) | Resolved decisions (scope, behavior, diagnostics, verification) |
| [01-requirements.md](01-requirements.md) | The functional requirements with the frozen references |
| [02-current-state.md](02-current-state.md) | Verified present behavior at the batch baseline and the code map |
| [03-01-conditional-arm-ownership.md](03-01-conditional-arm-ownership.md) | Phase 1 design: per-arm move-outs and the guarded intrinsic argument |
| [03-02-guarded-receivers-and-loans.md](03-02-guarded-receivers-and-loans.md) | Phase 2 design: struct-literal receivers, the place-arm loan, the dot-`len` receiver |
| [03-03-view-provenance-and-elision.md](03-03-view-provenance-and-elision.md) | Phase 3 design: the `str[N]` loan and the derived-candidate judgment |
| [03-04-statement-path-discards.md](03-04-statement-path-discards.md) | Phase 4 design: discarded closures, conditional calls, and intrinsic dot-calls |
| [03-05-emission-spellings.md](03-05-emission-spellings.md) | Phase 5 design: the conditional return, the postfix pin, the fixed-array address |
| [05-findings.md](05-findings.md) | The findings ledger (phase reviews append here) |
| [07-testing-strategy.md](07-testing-strategy.md) | The two suites, the case conventions, and the acceptance |
| [99-execution-plan.md](99-execution-plan.md) | The task-by-task execution plan |

## Quick reference

- **Baseline tree**: `5452bce` (the def-backlog-closeout close-out head; this plan's creation).
- **Verify**: `make check` per phase; `make verify` at close-out.
- **Focused suites**: `compiler/tests/conditional_projection_integrity_spec_test.rs` and
  `compiler/tests/conditional_projection_integrity_impl_test.rs`.
- **Frozen references**: Ch 03 §3/§5/§7 (moves, drops, borrows), Ch 04 §3/§8/§11 (evaluation order,
  member access and dot-call, intrinsics), Ch 06 §3.1 (view returns and lifetime elision), Ch 16 §4
  (EL-1..EL-6, BR-6/BR-7), Ch 17 (typed emission); the registry rows `E10022`, `E10030`, `E10031`,
  `E10036`.

## Specialist Agents

| Specialist | Needed? | Evidence |
|------------|---------|----------|
| correctness-reviewer | yes — every phase | The batch changes ownership, borrows, evaluation order, and emission |
| semantics-reviewer | yes — phases 2, 3, 5 | Loan recording (`E10031`), the elision rules (EL-2/EL-3), and evaluation order (Ch 04 §3) are language rules |
| memory-safety-auditor (read-only) | yes — ownership phases (1, 2, 3, 4) | Every phase adds or moves a release path; the auditor probes the shapes under host AddressSanitizer |
| emission-integrity-auditor (read-only) | yes — phases 4, 5 | IR/codegen changes (the discard lowering, the conditional and postfix spellings, the fixed-array address) need the base/head differential |

No specialist gap: the project's agents cover every surface the batch touches; no new specialist
type is needed.

## Related files

- `compiler/src/own/lower.rs` — the guarded binder, the intrinsic-argument binder, the conditional
  arm helpers, and the discard lowering.
- `compiler/src/own/statements.rs` — the statement path (`emit_expr_statement`), the provenance
  suppression of elision.
- `compiler/src/own/place.rs` — the elision pre/postlude and the loan bookkeeping.
- `compiler/src/borrow.rs` — `is_elided_view`, `is_lend_candidate`, `is_view_like`.
- `compiler/src/codegen/expr.rs`, `calls.rs`, `cleanup.rs` — the conditional, postfix, intrinsic,
  and address spellings.
- `spec/06-functions.md` §3.1, `spec/16-borrow-rules.md` §4, `spec/03-ownership.md` §3–§7,
  `spec/04-expressions.md` §3/§8/§11.
