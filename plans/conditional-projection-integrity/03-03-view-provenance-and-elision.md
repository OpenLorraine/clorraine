# Phase 3: View Provenance and Elision

> **Document**: 03-03-view-provenance-and-elision.md
> **Parent**: [Index](00-index.md)

## Overview

Two defects where a view's provenance is not carried through: a `str[N]` binding of view elements
records no loan (DEF-177, major), and a returned view is judged by the candidate count instead of
its actual derivation (DEF-187, critical). The invariant: a view's borrows are the borrows of the
value it derives from, and the elision judgment compares that derivation against the view-like lend
candidates.

## Current Arc

- `elision_prelude` (`own/place.rs:1189`) counts the view-like lend parameters
  (`is_lend_candidate`, `borrow.rs:98` — `str`, `cstring`, `static str`, arrays, `raw`, handles,
  `Array`; a user struct is not a candidate). `elision_postlude` (`own/place.rs:1206`) reports
  `E10036` when the count is not one; `suppress_elision` (set at `own/statements.rs:1050-1058` for
  local- and static-derived returns) overrides.
- The **actual derivation** is not compared against the candidate: in
  `func pick(x: str, h: Box): str { return h.v; }` the count is one (`x`), the derivation is `h`
  (not a candidate), and the function is accepted — a caller passing a temporary `Box` reads freed
  storage. The zero-candidate twin (`func f(h: Box): str { return h.v; }`) already reports
  `E10036`, so the intended judgment is the derivation, not the count.
- A `str[N]` binding of view elements (`let vs: str[1] = [h.v];`) records no loan; the single-view
  twin (`let v: str = h.v;`) records one and rejects the move with `E10022`. The temporary-element
  binding (`let vs: str[1] = [mk().v];`) leaks 11 bytes; the *return* spelling of the same
  derivation already reports `E10030` (the batch E pins `arrtempescape`, `tempviewret`). The
  binding's target is the single-view binding's pinned behavior instead: adopt the backing for the
  binding's scope (accepted, released once) — a return has no scope to own the backing, a binding
  has one.

## Proposed Changes

### 1. Record the binding loan and judge element derivations — `compiler/src/own/place.rs`

- When a binding target is view-like (`is_elided_view`/`is_view_like`: a `str`/`cstring`, a slice,
  or a fixed array repeating a view), record the same loan on the source that the single-view
  binding records — for every provenance the target's elements derive from (a place field, a
  parameter, a call result).
- For a temporary-derived element, adopt the element's backing as an owner scoped to the binding
  (the single-view binding's pinned behavior), so it is released exactly once and the view never
  dangles or leaks; a place-derived element keeps the loan recorded above, so moving its backing
  while the binding is live reports `E10022`.

### 2. Tie the elision judgment to the actual derivation — `compiler/src/own/`

- Carry the returned value's derivation (the parameter it reads, or none) from the return walk to
  the elision postlude — the same provenance record the local/static suppression already uses.
- In `elision_postlude`, report `E10036` unless the derivation names the single view-like lend
  candidate (or the return is `static`-derived, EL-4): the count rule stays as the zero/two-or-more
  judgment, and the new comparison closes the "one candidate, wrong derivation" gap at the
  declaration span.
- Keep the existing pinned behaviors: `return x` (the candidate) accepted; the zero-candidate twin
  `E10036`; the fresh-temporary spelling `E10030`/`E10036` as already pinned.

## Tests

| Case | Shape | Expectation |
|------|-------|-------------|
| `viewarrayloan` | DEF-177's `[h.v]` binding + the move of `h`; the single-view control | one `E10022` (parity); the control unchanged |
| `viewarraytemp` | DEF-177's `[mk().v]` binding | accepted with the backing adopted for the binding's scope; ASan-clean (release once, no leak, no dangling); red at the baseline under `RDC_ASAN=1` |
| `viewarraycontrol` | a static-string array binding; a parameter-derived array binding | accepted; byte-identical where unchanged |
| `elisionderive` | DEF-187's `pick(x: str, h: Box)` + the `f(h: Box)` twin + the `return x` control | one `E10036` at the declaration for both wrong-derivation shapes; the control accepted and byte-identical |
| `elisiontemp` | the caller-temporary spelling (`return pick("zz", Box{...})`) | rejected by the declaration report; no sanitizer read |

## Acceptance

- The `str[N]` judgments match the single-view twin on every spelling: the place-derived loan
  spelling rejects the move with the twin's code (`E10022`), and the temporary-derived spelling
  adopts and releases the backing exactly as the twin does.
- The derivation judgment reports once at the declaration; the caller-temporary use-after-free
  disappears because the program is rejected.
- Accepted controls keep their emitted C byte-identical; the semantics lens confirms the reading
  against EL-2/EL-3 and BR-7.
