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

`lower_intrinsic_call` and the bare/dot spellings distinguish three operand
shapes:

1. a pure fixed-array place: fold to the extent, no read;
2. a direct call: lower and materialize through the guard, so the call runs
   only on the reached path;
3. any other non-pure shape: an effects-only walk before the fold.

The third shape splits by the evaluation context:

- **Unconditional expression position** (`defer_move_outs == 0` and
  `!in_loop_condition`): run `evaluate_len_operand_effects` before returning the
  extent. The walk evaluates a conditional's condition and, when both arms are
  fresh storage, the selected arm; it evaluates an array literal's element
  expressions and a run-time index's base, index, and bounds guard. A caller
  place is never moved by the structural fold.
- **Conditionally evaluated position**: keep the structural fold for non-call
  shapes, because a statement here would run on a path that skips the call.
  This narrower shape is the recorded `DEF-534` residual.

A pure place still folds. A direct call still materializes. The three spellings
— qualified `std.mem.len(x)`, bare `len(x)`, and dot `x.len()` — share the same
walk. A conditional that mixes a caller place arm with a fresh arm runs only its
condition; discarding the whole conditional would move the caller's owner.

## Phase 3 — `DEF-532`: apply the global-borrow call rule to initializers

Module initializers are checked by sema but not by the own function checker.
Add a focused initializer walk to `compiler/src/own/program.rs` and call it
from `own::check` with the collected `ProgramInfo`:

- walk each `GlobalLet` initializer recursively; a `GlobalConst` initializer
  cannot call a function (the constant check reports `E10224`) so it needs no
  call walk;
- resolve the callee by the body order: a module-qualified name in that module,
  otherwise the current module, then an import binding under its original name,
  then the program-wide map; the declaring module supplies the parameter and
  type-parameter names;
- resolve the receiver's projected type from the semantic expression type (or
  project a module global's declared type through its fields) and skip the call
  when the receiver's type declares a func-typed field with the callee's name;
- for a direct call, a dot call, or a module-qualified owner, judge each
  written argument and the receiver against the declared parameter mode;
- report `E10038` once when a non-`const` global is passed to a `borrow`,
  `const`, or unmarked parameter. A module-qualified receiver, a `const` global,
  a non-place argument, and an unmarked scalar/view or unresolved generic
  parameter stay accepted to preserve the existing oracles (`DEF-533`).

This reuses the same rule and text as the function-body check without building
a synthetic function body or emitting IR. The walk is read-only.

## Review corrections

The first review round found one critical memory-safety defect and several
initializer false positives and false negatives:

- the `len` correction evaluates effects only and never moves a caller place
  arm; a mixed conditional runs only its condition;
- a discarded bare call result binds to a statement temporary, an ownerless
  projection root binds before the discard, and a fixed-array projection arm
  binds its root on the arm's path, so an owning call leaf or member root
  releases and a skipped arm never runs;
- the freshness classification treats an index of fresh storage as fresh, and
  a projection whose base is a nested conditional distributes into its arms so
  each names an lvalue and only the selected arm evaluates;
- the initializer walk resolves the callee scope-first and uses the projected
  receiver type, the semantic field table, and the declaring module;
- an explicit `borrow` on a generic dot callee reports, while an unresolved
  unmarked parameter keeps the oracle shape;
- the receiver's written type is not compared with the declared parameter type:
  the call is already type-checked, and a valid widening still borrows the
  global. The func-field guard is the only exclusion.

The residual conditionally evaluated `len` operand and the func-typed
initializer call arguments are recorded as `DEF-534` and `DEF-537`.

## Verification

- Phase 1: a spec test that reads the caller's owner after the call, plus an
  implementation pin on the C shape; a host ASan run.
- Phase 2: spec tests for the conditional, literal, and run-time-index
  operands (only the reached arm/elements/index run), the pure-place fold, and
  the direct-call reached-path controls.
- Phase 3: a rejecting spec test for the initializer global borrow (direct and
  dot) and an accepted `const` control.
- Each phase: focused suites + `make check`; close-out: `make verify`.
