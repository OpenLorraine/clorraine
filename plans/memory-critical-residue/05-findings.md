# Findings

## Phase 1 — Carrier sources and callable expressions

**Rows**: `DEF-508`, `DEF-509`, `DEF-510`, `DEF-511`, `DEF-512`, `DEF-519`.
**Implementation**: `c435466` (carrier sources, callable callees, capture lifespan),
`00a52bf` (body-mention carrier filter), `af09e8f` (implementation pins). **Red cases**: `571660c`.
**Lenses**: correctness, semantics, memory-safety, emission-integrity.

### What changed

- `call_carrier_sources` returns every argument a returned carrier may read: each
  view-like lend parameter, each owned aggregate with handle-bearing storage, and each
  `take` argument. Every consumer iterates the set (`expr_view_roots`,
  `collect_place_copy_loans`, `frame_owned_view_owner`, `carrier_copy_escapes`).
  A candidate is kept only when the callee body mentions the parameter, so an owned
  argument the helper never reads is not treated as a source; a view carried through a
  local still names its parameter and stays a candidate (`00a52bf`).
- A member or element chain rooted at a carrier call is judged through the call's sources.
- A struct or array literal in a frame-receiving position descends its view-carrying
  fields/elements, except when the field's provenance already names a frame binding
  (the literal field judgment reports that site).
- Returning a `take` parameter no longer treats the parameter's own binding as
  frame-dying; the caller-side store still judges the argument's positions.
- `callable_call_signature` resolves a lambda literal, a conditional, a cast, and an
  indexed callable from the expression's recorded type. `check_call_expr` checks a
  non-name callable callee's arguments against that signature. `lower_call` lowers the
  callee as a value and attaches it as the receiver, so the emitter renders
  `((c ? first : second))(args)`.
- A capture's lifespan is taken from the enclosing binding it aliases, recorded before
  the lifted function's binding table is replaced. A captured local dies with the
  enclosing frame; a captured parameter or global does not.
- The conditional-take temp type resolves a `null` arm from the other arm with
  `nullable` set, so the instance declares the concrete type instead of the `i32`
  fallback.

### Red/green evidence

| Row | Red at `6a67880` | Green at `c435466` |
|-----|------------------|--------------------|
| `DEF-508` | accepted; ASan heap-use-after-free | `E10030` once, no TU; static control clean |
| `DEF-509` | accepted; ASan heap-use-after-free | `E10030` once; static pair clean |
| `DEF-510` | inline lambda accepted+UAF; conditional call emitted invalid C | both report `E10030` once; local conditional control strict-builds and runs |
| `DEF-511` | accepted; ASan heap-use-after-free | `E10030` once; parameter-backed capture control clean |
| `DEF-512` | two-argument helper accepted+UAF; identity return falsely rejected | `E10030` once at each caller store; identity control clean |
| `DEF-519` | accepted; ASan heap-use-after-free | `E10030` once; static literal control clean |

Focused legs at `00a52bf`: eval-emission spec 60/60 and impl 49/49, both green under
`RDC_ASAN=1`; borrow-generic spec 41/41 and impl 17/17. The body-mention refinement keeps
`make("AA", b)` accepted while the local-hop `let v = b.v; return Holder { names: [v] };` still
reports the frame-backed source once.

## Phase 2 — Generic conditional wrappers

**Rows**: `DEF-514`, `DEF-515`, `DEF-518`, `DEF-521`.
**Implementation**: `c435466`, pins `af09e8f`.

### What changed

- `conditional_take_needs_carrier` accepts a conditional with at least one owning place
  arm; an arm that owns nothing fills the shared carrier, and the owning arm empties its
  source in its own branch. The bare and `unsafe` spellings emit the same shape.
- `type_param_conditional_sources` and `unresolved_conditional_place_arms` strip
  `unsafe`/cast wrappers before the conditional walk.
- The array-literal element path records the type-parameter conditional duty, exactly as
  the struct-literal field path does.

### Red/green evidence

| Row | Red | Green |
|-----|-----|-------|
| `DEF-514` | double free | `ClString` carrier, per-arm `h.name = NULL`, post-call empty; clean under ASan |
| `DEF-515` | double free / invalid C | same emission as the bare twin; clean |
| `DEF-518` | accepted; double free | `E10034` once at the owning instance; duplicable control clean |
| `DEF-521` | accepted; double free | `E10034` once; `raw u8` control clean |

## Phase 3 — Moved state

**Row**: `DEF-523`.
**Implementation**: `c435466`, pin `af09e8f`.

`write_place` compares a recorded path against the written path. A strict prefix of the
write survives; the written path and its sub-paths are cleared; unrelated paths stay. A
nested write below a moved field reports `E10020` at the write and keeps the field's
moved state, so the later read reports at its own site. A whole-field write still
re-initializes the field. The generic body records the later-use duty and the instance
reports it.

Red: generic and concrete twins accepted and SEGV. Green: two `E10020` reports (write
and later read) in both twins; the whole-field re-initialization control is accepted,
runs clean, and re-emits byte-identically.

## Review

The four-lens phase review is pending. This section will record the verdicts,
corrections, and confirmations.
