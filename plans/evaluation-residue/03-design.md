# Design — Evaluation and Initializer Residue (Batch T)

## Phase 1 — `DEF-529`: keep a place source at a run-time opaque store

The own checker already defers the move for an opaque store:
`record_opaque_store` returns `deferred_owned_move` when the source is an owned
place, so the generic body does not consume it. The lowering must match.

In `compiler/src/own/expr.rs::emit_indexed_store`, the owned-value branch
currently calls `consume_place(lowered)`. Add a case before it:

- when the element type is unknown or contains a type parameter, and
- the lowered value projects from a place,

keep the place. The instance emitter already resolves the `StorePointer`
pointer type:

- an owned instance takes the `needs_drop` branch, which stores the value and
  empties the source (`emit_arm_source_moves`);
- a view instance takes the widening branch, which converts the source place
  to a view and leaves it in place.

This removes the transfer temporary and preserves the caller's owner. A call
result or literal source keeps its existing ownership binding: the deferred
place rule is limited to a source that projects from a place.

## Phase 2 — `DEF-531`: evaluate non-call effectful `len` operands

`lower_intrinsic_call` distinguishes three operand shapes:

1. a pure fixed-array place: fold to the extent, no read;
2. a direct call: lower and materialize through the guard, so the call runs
   only on the reached path;
3. any other non-pure shape: currently folds and drops the effects.

Split the third shape by the evaluation context:

- **Unconditional expression position** (`speculative_lowering` false): lower
  the operand with `lower_discarded` before returning the extent. The discard
  path already handles a fixed-array conditional per selected arm, an array
  literal's element expressions, and a run-time index read. The extent is
  structural, so the folded value is unchanged.
- **Conditionally evaluated position** (`speculative_lowering` true): keep the
  structural fold for non-call shapes. A statement here would run on a path
  that skips the call; the guard materialization only knows the direct-call
  and owned-call forms. This narrower shape is recorded separately if a probe
  shows it accepted with dropped effects.

A pure place still folds. A direct call still materializes. The runtime
index case's base and index are pinned by `lower_discarded`'s target lowering.

## Phase 3 — `DEF-532`: apply the global-borrow call rule to initializers

Module initializers are checked by sema but not by the own function checker.
Add a focused initializer walk to `compiler/src/own/program.rs` and call it
from `own::check` with the collected `ProgramInfo`:

- walk each `GlobalLet`/`GlobalConst` initializer recursively;
- for a direct call, resolve the user/`ffi` signature (module path first, then
  the current module, then the unique simple-name map);
- for a dot call, resolve the last segment as the free function and use the
  receiver as parameter zero;
- for each non-`take` parameter whose declared mode is `borrow`, `const`, or
  unmarked, if the argument or receiver resolves to a non-`const` module
  global, report `E10038` once at the written place;
- the module-qualified receiver shape accepted by the existing oracle stays
  accepted (`DEF-533`); a bare module-level or imported receiver reports;
- a `const` global and a non-place argument stay accepted.

This reuses the same rule and text as the function-body check without building
a synthetic function body or emitting IR. The walk is read-only.

## Verification

- Phase 1: a spec test that reads the caller's owner after the call, plus an
  implementation pin on the C shape; a host ASan run.
- Phase 2: spec tests for the conditional, literal, and run-time-index
  operands (only the reached arm/elements/index run), the pure-place fold, and
  the direct-call reached-path controls.
- Phase 3: a rejecting spec test for the initializer global borrow (direct and
  dot) and an accepted `const` control.
- Each phase: focused suites + `make check`; close-out: `make verify`.
