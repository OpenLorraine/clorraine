# Memory and Borrow Critical Completion (Batch P)

> **Feature**: Close the fifteen remaining open critical defects left by the memory/value and
> critical-closeout batches. The scope is `DEF-485`..`DEF-489`, `DEF-493`, `DEF-495`..`DEF-499`,
> and `DEF-501`..`DEF-504` from `plans/00-roadmap.md`.
> **Status**: Done (2026-10-20)
> **Created**: 2026-10-20
> **Planning baseline**: `efc68f9` (the batch-O close-out commit).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

Batch O closed its eight named rows but filed twenty-two base-identical sibling shapes. Fifteen of
them are critical: each is an accepted program that reads freed storage or double-frees under the
host sanitizer. They fall into three families:

1. a **loan-carrying copy** whose destination is not a plain local binding (a `const`, a global, a
   call-returned carrier, a mixed aggregate, a conditional shadow, or a loop alias), so the
   destination never records the element loans the source carries;
2. a **call-mediated view** whose source the escape walks cannot name (a returned view, an inline
   call-result carrier, a conditional call arm, a closure or wrapper call, or a dot receiver);
3. a **generic conditional source** that the per-instance audit does not see (a take, a raw field,
   a struct-literal field or projected assignment, or a projection over a nested conditional).

None is a regression from batch O; each is byte-identical at the batch-O base and head.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | The row-derived requirements and acceptance criteria |
| [02-current-state.md](02-current-state.md) | The reproductions, observed behavior, and code sites |
| [03-design.md](03-design.md) | The fix approach per phase |
| [05-findings.md](05-findings.md) | The per-phase records |
| [99-execution-plan.md](99-execution-plan.md) | The ordered task list and progress |

## Scope decisions

- The frozen `spec/` chapters are the oracle. Every row is fixed to the rule its concrete twin
  already reports; no new diagnostic, registry entry, or `spec/` edit.
- A rejected program reports exactly the concrete twin's existing code (`E10030`, `E10031`,
  `E10033`, `E10034`, or `E10035`), once per site, and writes no translation unit.
- Accepted controls keep their emitted C byte-identical and run ASan/LSan clean.
- The red cases are specification tests in the suites that already own the neighboring shapes.
  Each case must fail at `efc68f9` before its fix lands.
- The batch does not change the `DEF-425` deferral, and it does not reopen a closed row.
