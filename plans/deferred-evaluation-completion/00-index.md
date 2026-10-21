# Deferred Evaluation Completion (Batch V)

> **Status**: Done (2026-10-21), with the pre-existing caller-side view
> invalidation rule filed as `DEF-540`.
> **Created**: 2026-10-21
> **Baseline**: `55b6419` (the batch-U close-out).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

Two batch-T residuals remained on the evaluation path:

1. `DEF-534`: a fixed-array `len` in a conditionally evaluated position kept
   the structural fold but dropped the operand's effects.
2. `DEF-535`: a conditional of borrowed places was rejected at a view-instance
   opaque store while the concrete view twin accepted it.

The batch also carries the review corrections found while fixing them:
deferred growable-container releases, conditional loop and for-step tests,
fresh-arm view stores, nested conditional conditions, and the arm diagnostic
text.

## Decisions

- The frozen specification is unchanged. The `spec/` oracle stays authoritative.
- The `len` fix carries the operand's effects in a new `DiscardThen` IR value and
  runs them where the value is evaluated; the structural extent stays a
  compile-time constant.
- The opaque-store fix records each conditional arm and judges the destination
  per instance: a view destination widens every arm, an owning destination
  keeps the concrete arm judgment.
- A definite re-initialization is tracked in the flow state and joined by
  intersection, so a write under a branch does not hide the other path's use.
- The caller-side rule that invalidates a stored view when its backing is later
  reassigned is a separate pre-existing defect, filed as `DEF-540`; it is not
  claimed fixed here.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | Row-derived acceptance criteria |
| [02-current-state.md](02-current-state.md) | Reproductions and code sites |
| [03-design.md](03-design.md) | Per-area approach |
| [05-findings.md](05-findings.md) | Phases and review records |
| [99-execution-plan.md](99-execution-plan.md) | Ordered tasks |
