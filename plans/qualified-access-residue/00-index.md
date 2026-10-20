# Qualified Access Residue (Batch S)

> **Feature**: Close the four qualified-access residue rows left by batches M and R:
> `DEF-397`, `DEF-398`, `DEF-399`, and `DEF-400`.
> **Status**: Done (2026-10-20)
> **Created**: 2026-10-20
> **Planning baseline**: `5830521` (the batch-R close-out).
> **Head**: `0fac18e`; 26/26 tasks; `make check` and `make verify` pass.
> **Layout**: flat (`plans/<feature>/99-execution-plan.md`).

## Why this batch exists

Batch R fixed the qualified *function call* route (`DEF-396`) and the generic
fixed-array iterable (`DEF-407`). Four sibling shapes remain:

1. a generic dot call still skips the direct call's inference (`E10301`),
   recursion-limit, callable-bound (`E10303`), and ephemeral-binding checks;
2. a qualified function value is typed but emitted as a raw C path
   (`p.funcvalue.helper`), so strict C reports an undeclared module;
3. a qualified compiler intrinsic (`std.mem.len`, `std.mem.rawSlice`,
   `std.mem.rawStr`) or a cross-module `ffi` function is rejected `E10219`
   although qualified access is available;
4. a module path whose head is an imported value is treated as a dot call, so
   the module function is shadowed by the value.

All four are checker/emitter residue around the same qualified identity. The batch
extends the existing direct-call and module-path routes; it adds no diagnostic,
registry entry, or `spec/` edit.

## Documents

| Document | Purpose |
|----------|---------|
| [01-requirements.md](01-requirements.md) | Row-derived acceptance criteria |
| [02-current-state.md](02-current-state.md) | Reproductions and code sites |
| [03-design.md](03-design.md) | Per-phase approach |
| [05-findings.md](05-findings.md) | Phase records |
| [99-execution-plan.md](99-execution-plan.md) | Ordered tasks and progress |

## Scope decisions

- A dot call reports the direct twin's existing codes: `E10301` for an
  uninferable parameter, `E10305` for recursive instantiation, `E10303` for a
  callable-bound violation, and the existing ephemeral-binding rule.
- A qualified function value resolves to the emitted C symbol; it is not
  rejected, because the type stage already accepts the value and the emitter has
  the module identity.
- Qualified access of a compiler intrinsic or an `ffi` declaration is available
  even though neither is `export`ed; an import still requires the export rule.
- A module path wins over an imported value head; a lexical local still shadows
  it.
- Every red case is a specification test in the suite that owns the neighboring
  shape, and it must fail at `5830521`.

## Close-out

`DEF-397`, `DEF-398`, `DEF-399`, and `DEF-400` are Done. The batch head is
`0fac18e`; `make check` and `make verify` pass (all legs).

The completed work is recorded in [05-findings.md](05-findings.md), including
the three review rounds and the per-phase rulings. The follow-up rows are:

| Row | Disposition |
|-----|-------------|
| `DEF-531` | Backlog — non-call effectful fixed-array `len` operands and the bare/dot call spellings still fold without evaluation |
| `DEF-532` | Backlog — module initializers skip the own call check |
| `DEF-533` | Deferred — the module-qualified global-borrow oracle conflicts with BR-9 and needs a user ruling |

No `spec/` file, diagnostic registry entry, or generated C was edited.
