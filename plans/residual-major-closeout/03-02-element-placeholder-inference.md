# Generic Element Placeholder, Inference, and Bounds: Residual Major Closeout

> **Document**: 03-02-element-placeholder-inference.md
> **Parent**: [Index](00-index.md)

## Overview

Four batch G Phase 2 rows share one root: the loop element of an opaque iterable is bound only
in the ownership stage, while the type stage types it `unknown`. The direct member read was
already made to build (the batch G work bound the ownership-stage element and made the emitter
carry the type), but the class stays live in every type-sensitive use: a member read passed to
a generic callee (`DEF-229`), a closure capture's environment field (`DEF-230`), an inferred
instance key derived from the placeholder or the null shape (`DEF-231`), and an index of a row
element with no bounds judgment (`DEF-232`). The fix boundary the roadmap names: bind the
element in the type stage (`Param(element_param(T))`), record element member reads as opaque
projections, and teach the substitution, the audit, and the key derivation the
`#element of T → arg.iterated_element()` mapping. The machinery already exists for the codegen
instance substitution (`codegen.rs:749-752`); the fix makes the type stage and the key
derivation agree with it (AR-14).

## Design

### 1. The element member read through a generic callee (`DEF-229`)

`id(v.y)` inside `each<S[2]>` infers the argument as an opaque element and keys
`cl_id__unknown` (strict build fails); the direct read `total + i32(v.y)` is a verified green
control. The type stage's `check_for_in` (`sema/stmt.rs:210-218`) binds
`iterable.iterated_element()`, which is `unknown` for a bare `Param(T)`, while the ownership
stage writes `Param("#element of T")` (`own/loops.rs:531`).

Bind the loop element in the type stage as `Param(element_param(T))` when the iterable's type
is a bare parameter: the element's checked type then names the placeholder, the member read
records its opaque projection (`sema/expr.rs:587-596`), and the inference/key derivation maps
the placeholder to the substituted argument's one-step element through the existing
`codegen.rs:749-752` mapping applied to the type stage's inference as well. A member-read
argument that is still unresolved at the declaration must resolve before the key derives — its
substituted *field* type (not the element itself) drives the key — so the phase pins the exact
key spelling too. The instance then declares and the strict build passes; the direct-read
control stays accepted.

### 2. The closure capture's environment field (`DEF-230`)

`[v] () => v.y` inside `each<S[2]>` declares the environment field `i32 v` for a `ClS`
element (`incompatible types`/`request for member 'y'`). The capture's field type comes from
the captured binding's checked type — with the element bound in the type stage (§1), the field
carries `Param("#element of T")` and substitutes to `ClS` at the instance; the lifted lambda's
environment declares `ClS v` and the member read resolves. The float/row variants the row
names are the same mapping.

### 3. The unresolved inferred type argument (`DEF-231`)

`inner(v)` inside `outer<i32[2]>` infers the element placeholder and keys
`cl_inner__unknown`; `each(x)` with `x = null` keys `cl_each__unknownQ`. Both are accepted and
fail the strict build on an implicit declaration.

Resolve before deriving an instance key:
- The element placeholder resolves through the `#element of T → arg.iterated_element()` mapping
  (§1), so `inner(v)` keys `cl_inner__i32` and instantiates.
- The null shape is not an instance-able type: a call whose inferred argument is the null shape
  reports the frozen null judgment (`E10012`, the null-shaped argument rule DEF-151 pinned)
  instead of deriving an `unknownQ` key. The exact report is pinned by the case; the design
  keeps the argument's frozen rule rather than inventing an instance.

### 4. The row-element index bounds judgment (`DEF-232`)

`row[5]` and `row[i]` inside `rowsum<i32[2][3]>` are accepted, emit no guard, and an
out-of-bounds read runs to completion (exit 1); the concrete twin reports `E10216` for the
constant and traps `E11211` at run time. `check_index` (`sema/expr.rs:604-628`) folds a
constant against a fixed-array extent and the emitter's `bounds_guard_condition`
(`codegen/guards.rs:1173`) covers fixed arrays/slices/`str` — an opaque base produces neither.

Register the index judgment for an opaque base and resolve it at the instance: a constant index
past the substituted row's extent reports `E10216` at the instance (the concrete twin's code),
and a run-time index emits the `E11211` bounds trap on the substituted row (the emitter decides
on the substituted row type through the same guard-condition path). The constant-index concrete
twin stays `E10216` (control); the run-time concrete twin's trap stays.

## Integration Points

- `sema/stmt.rs:210-218` (`check_for_in`) — the type-stage element binding.
- `own/loops.rs:531` — the ownership-stage placeholder (already written; keep the spelling
  shared through `types::element_param`).
- `codegen.rs:743-752` (`instance_subst`) — the existing mapping the type stage's inference and
  key derivation join.
- `types/keys.rs:105-120` (`view_element`, the key derivation) — the element mapping for
  instance keys.
- `own/instance_check.rs:897-944` (the iterable duty), `:946-991` (the opaque projection
  audit) — the element/member instance judgments.
- `sema/expr.rs:604-628` (`check_index`), `codegen/guards.rs:1157-1204`
  (`emit_bounds_guard`/`bounds_guard_condition`) — the deferred index judgment and trap.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A member read on an opaque element whose instance lacks the member | The existing per-instance audit reports `E10212` (concrete parity) | — |
| An inferred argument that does not determine a parameter | No key derives; the type stage's judgment governs | — |
| A null-shaped inferred argument | The frozen null-shaped argument rule reports (`E10012`), no `unknownQ` key | — |
| A constant index past the substituted extent | `E10216` at the instance (concrete parity) | — |
| A run-time index on an opaque row | The `E11211` guard emits on the substituted row | — |
| An element instance that still mentions a parameter | The existing deferral keeps the accepted path (judged at its own instance) | — |

## Testing Requirements

- Spec: the member read through a generic callee (build, run, the sum value; the direct-read
  control), the closure capture (build, run; the environment carries the struct element), the
  placeholder instance (build, run, the keyed symbol), the null-shaped argument (rejected once
  with the frozen code), and the row index (the constant case reports `E10216` at the instance;
  the run-time case traps; the concrete twins as controls).
- Impl: the emitted element declarations (`ClS v = {0};`, the borrow address temporary), the
  keyed instance symbols (the member read's substituted type key), the environment field
  declarator, the guard condition text for the substituted row, and a re-emission determinism
  pin.
