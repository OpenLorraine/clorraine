# Opaque Store and Call Residue (Batch R)

> **Feature**: Close the last open critical (`DEF-525`) and the closely related opaque
> subscript/store and generic-call residue rows: `DEF-525`, `DEF-526`, `DEF-527`, `DEF-528`,
> `DEF-395`, `DEF-396`, `DEF-401`, `DEF-406`, `DEF-407`, and the incidental `DEF-402`.
> **Status**: In progress (2026-10-20)
> **Created**: 2026-10-20
> **Planning baseline**: `766e9b9` (the batch-Q close-out and push).
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

Batch Q closed every open critical it named, but its reviews filed four new rows. One is a
critical use-after-free that also emits invalid C: a local view stored into a type-parameter
member escapes through a returned `take` carrier. The other three are a slice-of-`func` typedef
that strict C rejects, a misleading conversion report on a conditional of capturing closures,
and a precision loss in the capture-destination rule.

The same investigation exposes a family of instance-substitution gaps in generic and qualified
calls: a generic dot call keeps the declared return type (`T`), so the caller emits `ClT`; an
explicit type argument on a dot call is dropped; a qualified call bypasses the direct-call
checker, so its return type and type arguments stay unsubstituted and its arity is unchecked.
These are all the same root: the call's chosen instance is not threaded into the checked result.

The batch fixes those roots together. `DEF-402` (a self-qualified generic initializer) no longer
reproduces at the baseline and is closed with a verification test if the qualified-call route
keeps it green.

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
- A rejected program reports the concrete twin's existing code (`E10030`, `E10200`, `E10404`,
  or the call-check codes `E10300`/`E10301`/`E10308`) once per site and writes no translation
  unit.
- Accepted controls keep their behavior, build as strict C, and run ASan/LSan clean.
- The red cases are specification tests in the suites that already own the neighboring shapes.
  Each case must fail at `766e9b9` before its fix lands.
- The batch does not reopen a closed row, change the `DEF-425` deferral, or take the
  `DEF-506`/`DEF-507`/`DEF-522` conservative rulings.
- A conditional of two distinct capturing closures is rejected with `E10404` (closures are
  ephemeral), not `E10200` (a conversion that does not exist). The two closures have distinct
  C environment types, so no single callable value can carry either arm.
- `DEF-402` is closed only if its verification case is green at the batch head without a
  dedicated code change; otherwise it stays open and the phase records why.
