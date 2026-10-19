# Memory and Borrow Critical Closeout (Batch O)

> **Feature**: Close the seven open critical defects left by the memory/value batches, plus the
> generated-owner lifetime ruling. The scope is `DEF-449`, `DEF-450`, `DEF-451`, `DEF-452`,
> `DEF-453`, `DEF-459`, `DEF-468`, and `DEF-469` from `plans/00-roadmap.md`.
> **Status**: Planning
> **Created**: 2026-10-19
> **Planning baseline**: `d19a467` (the batch-N close-out commit).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

The memory/value batches closed their named rows but left seven critical residuals, each with an
accepted program that reads freed storage or double-frees. They share one property: the value flows
through a **conditional carrier**, a **borrowed aggregate argument**, a **call-mediated view**, or
a **take alias** before it escapes. The generated-owner lifetime (`DEF-452`) is the one ruling that
decides whether the same-statement read fixes stay conformant.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | The row-derived requirements and acceptance criteria |
| [02-current-state.md](02-current-state.md) | The filed reproductions, observed behavior, and code sites |
| [03-design.md](03-design.md) | The fix approach per phase |
| [05-findings.md](05-findings.md) | The per-phase records |
| [99-execution-plan.md](99-execution-plan.md) | The ordered task list and progress |

## Scope decisions

- The frozen `spec/` chapters are the oracle. A row whose observed behavior contradicts them is
  fixed to the chapter; a Rust test pin that codified the contradiction is corrected with the
  ruling recorded in `05-findings.md`.
- `DEF-452` follows Ch 03 D5 (`spec/03-ownership.md:169`): a source temporary drops at the end of
  its statement. The generated owner is an implementation detail of the same temporary, so a view
  that outlives the statement is rejected; a same-statement read stays accepted. The phase-2 pin
  that codified the extension is corrected under the ruling.
- No new diagnostic is introduced. Each row reports through the existing rule its concrete twin
  uses (`E10030`, `E10033`, or `E10034`).
- All fixes are conformance fixes: no `spec/` chapter or registry edit.
