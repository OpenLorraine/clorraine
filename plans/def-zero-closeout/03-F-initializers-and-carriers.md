# Batch F — Initializers and Carriers

> **Document**: 03-F-initializers-and-carriers.md
> **Parent**: [Index](00-index.md)
> **Rows**: 7

## Scope

The rows listed below are fixed together because they share an owning stage and regression helpers. Each row keeps its own ST case and its own acceptance evidence.

## Per-Row Specification

### DEF-210 — An initializer `take` argument that projects a call-result array element leaks its sibling (minor)

- **Observed:** 🟡 minor, base-equal (no regression; the batch's release pass was made not to free the consumed element), surfaced by the batch F Phase 3 memory audit and correctness review: `consume(two()[0])` with `func two(): String[2]` and `func consume(take xs: String)` frees element 0 in the callee while element 1 stays unreleased, where the body twin reports `E10026`; the proper fix is the initializer call-check parity (the DEF-203 family) or a move-out of the projected element.
- **Fix:** Match the concrete twin and the frozen specification; remove the invalid C or wrong value.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-314 — A module-view binding of a conditional member leaks (minor)

- **Observed:** 🟡 minor, pre-existing (base == head), found by the batch-J phase-1 fix round of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md) (the F4 STOP): `let g: str = true ? mk().v : mk2().v;` leaks 11 B; the retain path covers the direct and deep member chains and the view arrays, but a conditional member needs per-branch init fills.
- **Fix:** add the per-branch fills.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-316 — A duplicable member argument under a guard leaks its fresh root (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical), found by the batch-J phase-2 correctness review of [conditional-projection-residue-ii](conditional-projection-residue-ii/00-index.md) (p284_dup): a duplicable member argument under a guard leaks 11 B.
- **Fix:** the deferred root bind covers the duplicable member argument.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-317 — The owning-literal carrier arm leaks its fresh view root residual (minor)

- **Observed:** 🟡 minor, pre-existing (base == head; the phase-2 gate was narrowed away from it), found by the batch-J phase-2 correctness review (p292_owning): `H { names: [mk().v], tag: String }` leaks 22 B (the loop twin 66 B).
- **Fix:** bind the view position's fresh root for the owning arm too.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-318 — A module-initializer carrier's residual sibling leaks (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical), found by the batch-J phase-2 correctness review (p292_modinit): the module-initializer spelling of the carrier store leaks the residual sibling (11 B).
- **Fix:** retain the root in the module initializer for the literal spellings.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-319 — A direct literal carrier return is accepted and leaks (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical), found by the batch-J phase-2 correctness review (p292_return): `return Holder { names: [mk().v] };` is accepted and leaks (the module-destination rule does not judge the return).
- **Fix:** judge the return's literal carrier per the destination rules.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.

### DEF-321 — A view-typed `Index` receiver leaks (minor)

- **Observed:** 🟡 minor, pre-existing (base == head, byte-identical), found by the batch-J phase-2 correctness review (p293_index): a view-typed `Index` receiver over a fresh root leaks 22 B.
- **Fix:** extend the residual gate to the index root.
- **Acceptance:** the row's reproduction no longer reproduces. The frozen spec
  and concrete twin hold; the emitted C builds under `-Werror`; the row's
  memory case is ASAN/LSan clean; every round-trip pin stays byte-identical
  where the owning suite records one.
