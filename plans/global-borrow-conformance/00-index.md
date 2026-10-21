# Global Borrow and Move Conformance (Batch U)

> **Status**: Done (2026-10-21)
> **Created**: 2026-10-21
> **Baseline**: `513fd90` (the batch-T close-out).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

Two deferred rows are oracle-versus-frozen-rule conflicts, and one row is the
initializer-call rule's last gap:

1. `DEF-533`: the frozen BR-9 says a global may not be borrowed, while several
   Rust spec oracles accept initializer and module-qualified receiver borrows.
2. `DEF-537`: a module initializer call through a func-typed global or field
   skips the global-borrow rule.
3. `DEF-425`: a generic body accepts an owning element move through a borrow
   parameter while the concrete/take twins reject it.

## Rulings

**User ruling (2026-10-21, delegated: "choose the best possible options").**
The frozen specification wins over a conflicting Rust spec pin.

- `DEF-533`: apply BR-9 uniformly. Remove the module-qualified receiver
  carve-out in the body and initializer, and check a view builder's non-`const`
  global source. Retarget the conflicting spec oracles to `const`, local, or
  call-result sources; do not edit `spec/`.
- `DEF-425`: apply frozen BR-5/STR-11/12. Reject the generic element move
  through a borrow parameter, and retarget the pinned acceptance cases to the
  concrete/take rejection.

## Close-out

All three rows are Done. The frozen specification wins over the conflicting
Rust pins. `make check` and `make verify` pass at the batch head. The review
corrections added the generic element call-argument and multi-extent take
judgments; the aliased view-builder bypass is filed as `DEF-538`.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | Row-derived acceptance criteria |
| [02-current-state.md](02-current-state.md) | Reproductions and code sites |
| [03-design.md](03-design.md) | Per-phase approach |
| [05-findings.md](05-findings.md) | Phase records |
| [99-execution-plan.md](99-execution-plan.md) | Ordered tasks |
