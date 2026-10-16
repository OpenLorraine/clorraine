# Current State: Residual Major Closeout II (Batch L)

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The planning baseline is head `8829be6` (the batch-K close-out commit). Batches I/J/K built the
machinery each row extends; the line numbers below are that tree and the executor re-resolves
them before editing.

- **Nullable-view widening** (`compiler/src/codegen/expr.rs`, `codegen/calls.rs`,
  `codegen/function.rs`, `codegen/program.rs`): `conditional_arm` (`expr.rs:610`) renders one
  conditional arm, widening an owned handle through `widen_text_view` (`function.rs:2150`);
  `store_text` (`function.rs:2068`) already carries a null guard for the place/store spelling;
  the call argument renders a conditional through `conditional_arm` (`calls.rs:150`); the
  initializer store wraps a `String` destination through `cl_string_view(value)` (`program.rs`
  ~:1070) and renders a conditional per arm (~:1065). The call/argument/initializer positions
  lack the place-rooted null guard the batch-K phase-1 fix gave `store_text`.
- **The monomorphization walk** (`compiler/src/own/generics.rs`): `record_call` (:407) keys an
  explicit or inferred call; `record_call_instance` (:1002); `resolve_ir_call_args` (:1031) and
  `resolved_ir_value_ty` (:1063) resolve the written arguments' types — the value resolver
  follows `Member`/`Index`/`Place` through the local table but declines a local-headed qualified
  receiver, a statement temporary, and a conditional/non-local place base (:1109 requires a
  `Local` base). The emitter mirrors a subset (`codegen/expr.rs:887`, the temp-following
  `resolved_call_type_args`), so a declined base keys no instance (or keys an unrecorded one).
- **Instance substitution and element surfaces** (`compiler/src/own/instance_check.rs`,
  `compiler/src/codegen/drop.rs`, `compiler/src/types`): the per-instance duties substitute
  through `types::instance_bindings` (:94 and the sibling duties); the capture-move duty
  (`check_capture_move_requirements` :78) substitutes, so the batch-K phase-2 correction rejects
  an owning capture move per instance. The element store's overwritten-owner drop decision runs
  through `codegen/drop.rs` (`drop_expr_text` :141, `emits_drop` :237) against the *declared*
  (placeholder) type; a conversion or member use of a substituted element lacks a per-instance
  duty (`own/instance_check.rs` has the iterable, storable, out-slot, and conversion duties).
- **Ownership transfer judgments** (`compiler/src/own/expr.rs`, `own/statements.rs`,
  `own/place.rs`): the view-like store/return judgment gates on
  `is_view_like(target_ty)`/`is_view_like(value_ty)` (`own/expr.rs:1429` the escaping-store
  check and the sibling sites); the carrier-copy transfer (the `let`/assignment hand-off) is
  guarded by `!is_view_like(&target)` (`own/statements.rs:365`), so a view-like array copy
  records no element loans (`own/place.rs:1112` `move_element_loans`). The `unsafe` raw deref
  read lowers through the unary `Deref` arm (`own/expr.rs:515`) and the IR `Deref` value
  (`:963`) without the owned read's move/duplicate judgment.
- **Store emission** (`compiler/src/own/lower.rs`): a conditional base is materialized for reads
  (`mark_read_conditional_move(base)` with the read-only materialization, ~:70-72); the element
  store through a conditional/projected base is accepted and dropped (the statement vanishes; the
  place judgment does not reject the conditional base).
  `materialize_guard_literal` (:1680) binds a conditionally evaluated literal whose fields only a
  store can fill; `literal_fields_need_stores` (:4488) routes view-like array elements to
  `view_element_needs_binding` (:4534), while `literal_field_needs_stores` (:4512) returns false
  for a view-array field, so a call-filled view-array field keeps the brace-list spelling.
- **Call conversions and `ffi` typing** (`compiler/src/sema/call.rs`, `own/call_check.rs`,
  `own/instance_check.rs`): the batch-K phase-4 duty records an opaque conversion when the source
  mentions a parameter or is null-shaped (`sema/call.rs`, the `OpaqueConversion` record) and the
  instance audit reports the frozen `E10200`; an inferred parameter that resolved to the null
  shape reports the null rule (`own/call_check.rs:397`) — a null-shaped argument whose *sibling*
  fixes the parameter to a concrete non-nullable type is not judged. The inverted routes (a
  caller value at a parameter-typed destination via an explicit type argument, or the caller's
  own parameter as an explicit argument) record no duty.
- **Member dispatch** (`compiler/src/sema/expr.rs`, `own/expr.rs`, `codegen/function.rs`):
  `member_type` types struct fields, enum members, raw fields, and opaque projections; every
  other base falls to `Ty::unknown()` (`sema/expr.rs:748`, the fallthrough of the dispatch at
  :692), which silently accepts an
  unknown member on a fixed array or a view. The view `.ptr`/`.len` members are typed (batch K
  phase 3), and the member conversion is scoped to the store slots and returns
  (`codegen/function.rs` the pointer-member conversion; the `raw u8` destination lacks the cast).
  The `raw E` member read routes through the raw-field arm rather than the enum rule.
- **Module/global identity** (`compiler/src/codegen/expr.rs`, `compiler/src/own/call.rs`,
  `compiler/src/load/`): the batch-K phase-6 corrections thread the func-global identity through
  the plain, qualified, and field spellings (`codegen/expr.rs` `global_binding_path` :481,
  `own/call.rs` `func_global_callee_path` :454, `function_in_scope` :486); the self-qualified and
  plain spellings resolve (controls). An **aliased import** of a func-typed global emits the alias
  as the symbol (the import's source name is not threaded), and a qualified value-head path to a
  *struct global's field* (`q.holder.g`) is rejected `E10219`. The checker types a bare
  func-global call through the program-wide simple-name map, not the scope's binding.
- **Const folder** (`compiler/src/consts/mod.rs`): `fold_intrinsic` (:495) folds `sizeof`
  through `table.size_of(&resolved)` (:511), which computes the *unpadded* declared layout for a
  struct (`S { a: i32; b: u8 }` → 5) and a fixed view size (`str` → 8 on host); a value position
  is emitted as a C `sizeof(ClS)` (the green control), while an extent position uses the folded
  number (`u8 pad[5]`).

### Relevant Files

| File | Purpose | Changes Needed |
|------|---------|----------------|
| `compiler/src/codegen/expr.rs` | `conditional_arm`, the callee/global resolution | The null-guarded arm widening; the func-global identity at the value-head paths |
| `compiler/src/codegen/calls.rs`, `codegen/function.rs` | Arguments, `store_text`, `widen_text_view`, the member conversion | The per-arm argument guard; the mutable-destination member cast |
| `compiler/src/codegen/program.rs` | The initializer store arms, the init-owner pass | The place-rooted guard for a member projection |
| `compiler/src/own/generics.rs` | The monomorphization walk and its value resolver | One shared temp-following projected-argument resolver |
| `compiler/src/own/instance_check.rs` | The per-instance duties and audits | The element conversion/member duty; the element-store drop; the out-slot instance |
| `compiler/src/codegen/drop.rs` | The drop decision | The substituted element store's overwritten owner |
| `compiler/src/own/expr.rs` | Loans, moves, the raw deref read | The view-like copy's loan judgment; the owned raw read judgment |
| `compiler/src/own/statements.rs`, `own/place.rs` | The copy transfer, the element loans | The view-like container's copy transfer (`!is_view_like(&target)`) |
| `compiler/src/own/lower.rs` | Store lowering, the guard-literal materialization | The per-arm conditional-base store; the view-array guard literal |
| `compiler/src/sema/call.rs`, `own/call_check.rs` | Call typing, the null rule, conversion duties | The sibling-fixed null argument; the inverted duty routes |
| `compiler/src/sema/expr.rs`, `own/expr.rs` | The member dispatch | The unknown-member rejection on concrete bases; the raw `E` member |
| `compiler/src/own/call.rs` | The callee identity | The aliased import's binding |
| `compiler/src/consts/mod.rs` | The intrinsic folds | The extent-position `sizeof` deferral |
| The origin suites (`module_initializer_integrity_*`, `checker_conformance_*`, `conditional_projection_residue_*`, `callable_release_integrity_*`) | Regression oracles | Additions only (AR-9) |

## Gaps Identified

Each gap names the row, the observed behavior at the head, the required behavior, and the site.
The roadmap rows carry the reproductions; AR-3 carries the probe results; this table is the site
map.

### P1 — Nullable-view arms and initializer view stores

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-335` | `(c ? cl_string_view(maybe()) : cl_string_view(...))` dereferences NULL on the null selection (run 139); the local/parameter place arms run 139 | Each arm widens with the destination's null guard (the zero view on a null handle) | `codegen/expr.rs:610` (`conditional_arm`), `codegen/function.rs:2150` (the widening), `codegen/program.rs:~1065` |
| `DEF-336` | `cl_string_view(__cl_owned0)` as an argument dereferences NULL (run 139) | The argument's per-arm widening carries the null guard | `codegen/calls.rs:150` |
| `DEF-337` | `cl_string_view(__cl_itmp1.name)` on a null handle (run 139, module-level) | The initializer store's member projection guards the nullable place | `codegen/program.rs` the store arm (~:1052-1077), `function.rs:2068` (`store_text`'s guard shape) |

### P2 — The walk's projected-argument resolver

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-338` | `let g: i32 = b.pick(7);` emits an unkeyed/incorrect instance (`cl_pick__Box` type error) | The local-headed qualified receiver resolves like the body's IR walk | `own/generics.rs:1031`/`:1063` (the resolver), `codegen/expr.rs` (the emitter's mirror) |
| `DEF-340` | `keep(v.name)` keys `cl_keep__String` in the emitter while the walk records nothing | One shared temp-following resolver | `own/generics.rs:1063-1119` (the `Local`-base requirement), `codegen/expr.rs:887` |
| `DEF-342` | `id(c ? a.y : b.y)` keys no instance (`cl_id__i32` implicit) | The resolver follows conditional and non-local place bases | same as `DEF-340` |

### P3 — Instance substitution and element surfaces

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-339` | `v.name = stringOf(...)` through a borrow element leaks the overwritten owner (11 B/store) | The store's drop decision uses the substituted destination type | `codegen/drop.rs` (`drop_expr_text` :141, `emits_drop` :237), the element store path |
| `DEF-345` | `e04…e12` are accepted and fail the strict build (element in a conversion/member position) | A per-instance judgment mirrors the concrete siblings | `own/instance_check.rs` (the duty pattern), `sema`/`codegen` conversion sites |
| `DEF-353` | `wrap<i32>(borrow out, 7)` drops the write (run 249) | The duplicable out-slot instance keeps its store | the out-slot lowering (the by-value/borrow decision), `own/instance_check.rs` |
| `DEF-341` | **Control**: both spellings reject one `E10401` | Keep the rejection pair and the concrete parity | `own/instance_check.rs:78` (the substituted capture-move duty) |

### P4 — Ownership transfer judgments

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-315` | `let ys: str[2] = xs;` transfers no element loans; `fill(ys[0], borrow slot)` reads freed storage (ASan heap-use-after-free) | A view-like copy covers the copy transfer/judgment like the single-view twin | `own/statements.rs:365` (the copy transfer's `!is_view_like(&target)` guard), `own/place.rs:1112` (`move_element_loans`) |
| `DEF-329` | `let v: String = unsafe *q;` creates a second owner (ASan double free) | The owned raw deref read routes through the ordinary read's move/duplicate judgment | `own/expr.rs` the deref read (:515, the IR `:963`), the owned read judgment |

### P5 — Store emission: conditional bases and guarded view literals

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-322` | `(c ? b1 : b2).ys[i] = 7;` emits no store and drops the RHS (run 251); the scalar twin `E10221` | The frozen `E10221` rejection (the scalar twin's rule; the route is parked, AR-6/PF-001) | the assignment-place judgment (`sema/ops.rs`, the scalar twin's site), `own/lower.rs` the store lowering |
| `DEF-323` | the `take` spelling fails the strict build (`too few arguments`); the plain spelling runs the skipped arm's call (run 10) | The guard-literal materialization covers view-array elements | `own/lower.rs:4488`/`:4512` (`literal_fields_need_stores`), `:1680` (`materialize_guard_literal`) |

### P6 — Call conversions and `ffi` typing

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-344` | `pair(1, x)` passes `void *` where `i32` is declared (strict build fails); `pair(x, 1)` one `E10012`, `pair(x, x)` two | The inferred parameter fixed by a sibling judges the null-shaped argument | `own/call_check.rs:397` (the null rule), the inference path |
| `DEF-351` | `pick<cstring>`'s `return 5`, `consume2<cstring>(x)` at `T=str`, `sink<U>(a)`, `bs01` fail the strict build | The inverted conversion routes record the duty and report the frozen `E10200` | `sema/call.rs` (the duty record), `own/instance_check.rs` (the audit) |
| `DEF-350` | **Control**: view/slice destinations reject `E10200`; `len`/index/cstring build; `.ptr`/`.len` residuals = `DEF-352` | Keep the fixed spellings pinned | `sema/call.rs` the ABI-pointer typing (the batch-K F1 fix) |

### P7 — Member typing on concrete paths

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-346` | `a.ptr` on `u8[4]`; `s.foo` on `str` — accepted, strict build fails | The unknown member reports the frozen `E10212` | `sema/expr.rs:748` (the `Ty::unknown` fallthrough; dispatch at :692), `own/expr.rs` the mirror |
| `DEF-347` | `let p: raw u8 = s.ptr;`, the return, the struct field — accepted, strict build fails (`discarded-qualifiers`) | The sanctioned member cast at the mutable destination (or the duty, AR-7) | `codegen/function.rs` the pointer-member conversion, the store/return slots |
| `DEF-348` | `i32(p.A)` on `raw E` — accepted, strict build fails | The raw-enum member routes through the enum rule on both paths | `sema/expr.rs` the enum/raw member arms, `own/instance_check.rs` |

### P8 — Module/global identity and the const folder

| Row | Observed | Required | Site |
|-----|----------|----------|------|
| `DEF-356` | the aliased import emits `cl_def356holder_bg` (undeclared); `q.holder.g(5)` is rejected `E10219`; the self-qualified/plain controls run 0 | The aliased and qualified value-head paths carry the binding identity | `codegen/expr.rs` (`global_binding_path`), `own/call.rs` (`func_global_callee_path`), `load/` |
| `DEF-357` | the extent folds (`u8 pad[5]`, `u8 buf[8]`) disagree with the host layout (run 253/248); the value folds run 0 | The extent position defers to the C type size (AR-13) | `consts/mod.rs` (`fold_intrinsic` :495), the extent consumer |
| `DEF-359` | `f("x")` against an imported `func(i32)` global is accepted while a same-named `func(str)` exists (strict build fails) | The checker types the callee through the scope's binding | `sema/call.rs` the bare-callee typing |

## Dependencies

### Internal Dependencies

- The origin suites (`module_initializer_integrity_{spec,impl}_test.rs`,
  `checker_conformance_{spec,impl}_test.rs`, `conditional_projection_residue_{spec,impl}_test.rs`,
  `callable_release_integrity_{spec,impl}_test.rs`) are the regression oracles; the new cases
  append to them (AR-9).
- The batch-I/J/K suites and the pinned oracles gate the phases that touch reports, emissions,
  and the ownership surface.
- `make check` (the container) and `make verify`'s Amiga legs gate every phase's green state.

### External Dependencies

- None — the workspace stays std-only.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| The `DEF-322` per-arm route would add an acceptance the frozen place list does not define | Resolved (preflight PF-001) | Med | The route is parked; the phase reports the frozen `E10221` (AR-6) |
| The `DEF-347` destination cast is ruled a new conversion | Low | Med | The AR-7 fallback is the frozen conversion duty; the sanctioned spelling (`ctl347`) is pinned as the control |
| The `DEF-357` deferred extent spelling breaks an alternate profile | Low | Med | The freestanding/hosted-amiga legs run; the AR-13 stop condition parks the row otherwise |
| The `DEF-339` drop decision changes release counts for accepted element stores | Med | High | The memory lens probes the element store forms (plain, borrow, owning, loop) on both paths; the origin suites re-run |
| The `DEF-335`/`336`/`337` guard widening alters byte-identical view emissions | Med | Med | The emission lens diffs base/head; the non-null controls must stay byte-identical |
| The `DEF-315` loan transfer changes accepted copy spellings | Med | High | The correctness + memory lenses and the pinned `conditional_projection_residue` oracle cover the copy/loan matrix |
| The `DEF-356` identity threading redirects an unrelated callee | Low | High | The `k18` control matrix (fnself, amb, pdiff, pq4, pimp) re-runs; the emission lens verifies the symbol set |
