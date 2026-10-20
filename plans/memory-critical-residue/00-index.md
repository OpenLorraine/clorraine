# Memory Critical Residue (Batch Q)

> **Feature**: Close the eleven remaining open critical defects left by the memory/value,
> critical-closeout, and memory-critical-completion batches. The scope is `DEF-508`..`DEF-512`,
> `DEF-514`, `DEF-515`, `DEF-518`, `DEF-519`, `DEF-521`, and `DEF-523` from
> `plans/00-roadmap.md`.
> **Status**: In progress (2026-10-20)
> **Created**: 2026-10-20
> **Planning baseline**: `6a67880` (the batch-P close-out and push).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

Batch P closed its fifteen named criticals, but the phase reviews filed eleven base-identical
sibling shapes that remain open. Each is an accepted program that reads freed storage,
double-frees, or emits invalid C. They fall into four families:

1. a **carrier source** the call-boundary walk cannot name: a projected member of a
   carrier-returning call, a call with several lend sources, a `take` or owned-aggregate source,
   and a carrier call nested in a returned struct literal;
2. a **callable expression** the call check and source walks do not resolve: an inline lambda
   callee and a conditional over func values, plus the capture-backed out-slot write such a
   lambda can perform;
3. a **generic conditional wrapper** the per-instance walk does not descend: the `take` of a
   type-parameter member, an `unsafe`/cast wrapper, and an owned array-literal element;
4. a **nested write** that clears the moved state of the whole field instead of only the written
   path.

None is a regression from batch P; each is byte-identical at the batch-P base and head, except
the `id(take h)` identity return, which batch P's carrier-return fix now falsely rejects. Batch Q
fixes that false rejection together with the escape it was meant to catch.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | The row-derived requirements and acceptance criteria |
| [02-current-state.md](02-current-state.md) | The reproductions, observed behavior, and code sites |
| [03-design.md](03-design.md) | The fix approach per phase |
| [05-findings.md](05-findings.md) | The per-phase records |
| [99-execution-plan.md](99-execution-plan.md) | The ordered task list and progress |

## Scope decisions

- The frozen `spec/` chapters are the oracle. Every rejecting row reports the rule its concrete
  twin already reports; no new diagnostic, registry entry, or `spec/` edit.
- A rejected program reports the concrete twin's existing code (`E10020`, `E10030`, `E10033`,
  `E10034`, or `E10035`), once per site, and writes no translation unit.
- Accepted controls keep their behavior, build as strict C, and run ASan/LSan clean.
- The red cases are specification tests in the suites that already own the neighboring shapes.
  Each case must fail at `6a67880` before its fix lands.
- The batch does not change the `DEF-425` deferral, the `DEF-506`/`DEF-507`/`DEF-522`
  conservative rulings, or reopen a closed row.
