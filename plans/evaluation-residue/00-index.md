# Evaluation and Initializer Residue (Batch T)

> **Feature**: Close the three majors left by the batch-R and batch-S reviews:
> a run-time-index view-store leak (`DEF-529`), effectful non-call fixed-array
> `len` operands (`DEF-531`), and the module-initializer own-call gap
> (`DEF-532`).
> **Status**: In progress (2026-10-20)
> **Created**: 2026-10-20
> **Planning baseline**: `55beee0` (the batch-S close-out).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

The three rows share one theme: the checker knows a value's shape, but the
lowering commits to a transfer or a read before the instance or the evaluated
path decides the shape.

1. A generic run-time-index store whose element is a type parameter consumes
   an owned source place into a transfer temporary. At a `str` instance the
   store is a view, so the temporary is never released and the caller's field
   is emptied.
2. A free `len` of a non-call effectful fixed-array operand folds the extent
   without evaluating the operand: a conditional arm call, an array-literal
   element call, or a run-time index read is dropped.
3. Module initializers run the type checker but not the ownership call checker,
   so a global passed to a `borrow` parameter is accepted there while the
   function-body twin reports `E10038`.

`DEF-533` (the module-qualified global-borrow oracle conflict) and `DEF-530`
(two return sites report one escape) stay out of this batch: the first needs a
user ruling, the second is a minor precision issue.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | Row-derived acceptance criteria |
| [02-current-state.md](02-current-state.md) | Reproductions and code sites |
| [03-design.md](03-design.md) | Per-phase approach |
| [05-findings.md](05-findings.md) | Phase records |
| [99-execution-plan.md](99-execution-plan.md) | Ordered tasks |
