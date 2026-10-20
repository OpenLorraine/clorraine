# Design

Each fix extends an existing route. No new diagnostic, registry entry, or `spec/` chapter.

## Phase 1 — Opaque member store (`DEF-525`)

### The duty

A store into a projection whose destination type is still `Unknown` has no ownership judgment
at the declaration: the instance decides whether the member is a view or an owner. The checker
records `ir::RequiresOpaqueStore` for each such store, carrying:

- the generic function and the root binding's declared type (`T`);
- the projection path (field and element steps);
- the source value's declared type;
- the frame-dying backing subject, when the source reads one (`a`, or a statement temporary);
- the store span, and the return span once the root is returned.

The record is field on `FunctionChecker` and is discharged by a per-instance audit. Only stores
whose destination is a projection (`!projections.is_empty()`) and whose recorded destination
type is unknown or mentions a parameter are recorded; a concrete store keeps its ordinary path.

### The instance judgment

For each recorded instance, substitute the root and project the path with
`project_place_path`. Skip when either the destination or the source still contains a parameter
or is unknown. Then:

1. `implicit_conversion(source, destination)` false reports `E10200` at the store — the
   concrete twin's rule. This is the view source at an owned destination, and the owned source
   at an incompatible owned destination.
2. A view-like destination that received a widened owner or another view is accepted. If the
   root was returned and the recorded backing dies with the frame, report `E10030` at the return
   with the recorded subject. A caller-backed or static source stays accepted.

One duty reports once — the first offending instance — so the body site is not repeated.

### Recording the escape

The return check finds the returned expression's root binding (an exact place or an `unsafe`
wrapper) and sets `escaped` and the return span on every recorded duty for that binding. The
concrete twin's report lands at the return, so the audit uses that span for `E10030`.

### Emission

A `Statement::Move` whose recorded target type was unknown but whose resolved instance target
is view-like becomes a view store: render the destination and `store_text(source, resolved
target)` (which widens an owned `String` through `cl_string_view`), and do not empty the source.
The owned-instance path keeps the existing move.

## Phase 2 — Type and closure surface (`DEF-527`, `DEF-528`)

### `DEF-527` — slice of `func`

`declarator`'s `Raw` arm asks `pointer_to_array_declarator` first and otherwise appends `*` to
`c_type(inner)`. For a function inner type that spelling is `ret (*)(params) *name`, which is
not a declarator. Add the function case: a pointer to a function type is
`ret (**name)(params)`, built through the existing `func_pointer_declarator` with the name
spelled `*name`. The slice typedef then uses `declarator(&Ty::raw(element), "ptr", table)`
instead of the manual `c_type(element) + " *ptr"`, so every pointer shape shares one rule.

### `DEF-528` — a conditional of capturing closures

`conditional_result` returns `None` for two closure kinds because the site id is part of the
kind. A conditional of two distinct capturing closures has no common C value: each closure has
its own environment struct. The conditional therefore rejects with `E10404` at its span and
returns `Ty::unknown()`, so the enclosing storage position does not repeat the report. A
capture-less pair keeps the ordinary `Func` conversion (the kinds are equal). No emitter change
is needed because no translation unit is written.

## Phase 3 — Capture write summary (`DEF-526`)

`mark_written_captures` currently marks every written write-borrow local frame-backed. Add a
body summary: walk the lambda body outside nested lambdas for whole-name assignments to the
capture. A write is *surviving* when its right side is a static string, a module path, a
captured binding whose enclosing backing survives the frame
(`capture_source_dies_with_frame` is false), or a conditional/cast/unsafe wrapper whose every
arm survives. Any other right side — a lambda local, a call result, a temporary, an unlisted
shape — is not surviving.

If the local's current provenance already survives and every write is surviving, keep a
surviving provenance (the source's backing) and do not set `capture_written`. Otherwise keep
the conservative rule. The batch-Q `capturedestination` case (a local-owner view written into
the local and then escaped) still rejects because its source does not survive the frame.

## Phase 4 — Generic dot call (`DEF-395`, `DEF-401`, `DEF-406`)

`check_generic_dot_call` already computes the callee's bindings. It returns the substituted
return type instead of `()`, and `check_dot_call` returns that value. This fixes `DEF-395`
(the emitted temporary is `String`, not `ClT`) and `DEF-406` (the callee's `T` no longer
collides with the caller's `T` when the return is judged at the caller's instance).

Thread the call's explicit type arguments into `check_dot_call` and
`check_generic_dot_call`. A non-empty list is resolved, its count is judged against the
declaration (`E10300`), and it seeds the bindings before the receiver and argument matching.
An inferred call keeps the current matching.

## Phase 5 — Qualified call route (`DEF-396`, `DEF-407`)

`check_call` resolves a `Qualified` callee whose head is not a lexical local through the type
table's module function (`table.function(module, last)`). A generic callee routes through
`check_generic_call` with the written type arguments; a concrete one routes through
`check_args_direct` with its modes. This gives the qualified spelling the direct twin's arity,
type-argument count, conversions, and substituted return type. `DEF-407`'s loop then sees
`i32[2]`, so its element is `i32`.

The existing value-headed dot-call branch stays after this route: a lexical local still
shadows, and a module path that names a function wins over an imported value of the same head.
`DEF-399` (qualified intrinsics/ffi) stays out of scope.

## Phase 6 — Initializer verification (`DEF-402`)

Add the self-qualified initializer case to `checker_conformance_spec_test.rs`. It is green at
the baseline; if the phase-5 route keeps it green, close the row as incidentally fixed and
record that no code change was required. Otherwise file the remaining gap back to the roadmap.
