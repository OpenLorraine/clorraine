# Current State: Slice and Temporary Integrity

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The compiler lowers source to a typed ownership IR (`compiler/src/own/`), then emits C99
(`compiler/src/codegen/`). Two existing mechanisms matter throughout this plan:

- **The deferred `Materialize` carrier.** `ValueKind::Materialize { temp, value, move_outs }`
  (`compiler/src/ir.rs`) names a compiler temporary whose fill and move-outs are emitted by the
  guard emitter (`compiler/src/codegen/guards.rs:111-201`) at the point where the value is
  evaluated. It is how guarded operands stay lazy and how a guarded take keeps its root call on the
  evaluated path.
- **The prepare/store discipline.** The store path (`prepare_store`, `compiler/src/own/statements.rs:320-347`)
  prepares a projection before deriving its place and emits through `PreparedStore`
  (`Assign`, `AssignThenMoveOut`, `Move`, `compiler/src/own/statements.rs:351-359`); the literal
  store path (`emit_struct_literal_fields`, `compiler/src/codegen/function.rs:1516-1560`) stores
  fields in source order and calls `emit_guards(field)` before each store.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/types/mod.rs` | Type model; `iterated_element` is row-aware (`:375-386`), `array` folds nested fixed extents (`:262-289`) | None expected beyond a view-element helper if the design needs one |
| `compiler/src/types/keys.rs` | `type_key` (`:28-59`), `slice_base_element` (`:68-74`), `slice_element_key` (`:77-79`) | Key the view by the row element, not the unwrapped base |
| `compiler/src/own/call.rs` | `slice` result typing (`:294-318`), `element_of` (`:400-405`), `take_move_out_places` (`:349-383`) | Use the row element; keep move-out derivation |
| `compiler/src/own/lower.rs` | Index lowering + recovery gate (`:38-68`, `:683-693`), eager move-out branch (`:112-145`), guard operand (`:705-738`), base settlement (`:749-788`), index chains (`:791-852`), `consume_place` (`:924-952`), `value_place` (`:960-999`), root binds (`:1349-1434`), intrinsic/receiver binds (`:1447-1486`, `:1554-1653`) | The main ownership-side changes |
| `compiler/src/own/statements.rs` | `prepare_store`/`emit_prepared_store` (`:320-359`) | Reused shape; no change expected |
| `compiler/src/codegen/types.rs` | `c_type` slice spelling (`:97-103`), `collect_slice_types` (`:336-355`) | Row element registration; `zero_view_helper` key (preflight PF-011) |
| `compiler/src/codegen/program.rs` | `ClSlice_<key>` typedefs (`:144-165`) and their zero helpers | Pointer-to-row field type |
| `compiler/src/codegen/calls.rs` | `row_slice_types` (`:52-67`), `slice_source` (`:79-119`) | The view stores the row pointer; the cast dance goes |
| `compiler/src/codegen/guards.rs` | Deferred fill and move-out emission (`:111-201`) | Reused by the consume preparation |
| `compiler/src/codegen/function.rs` | `Move`/`StorePointer` emission (`:405-518`), literal stores (`:1516-1560`) | Element-store drop spelling; literal store predicate |
| `compiler/src/codegen/drop.rs` | Struct drop recursion and the field append (`:86`) | Parenthesize the appended base |

### Code Analysis

The seven gaps below are the filed defects. Root-cause references were verified in the current
working tree (2026-10-08).

## Gaps Identified

### Gap 1: DEF-70 — multi-extent slice views are flat-typed

**Current Behavior:** `slice`'s result type is built with `element_of` (`compiler/src/own/call.rs:400-405`),
which returns the *immediate* element of the source array and drops later extents; the element-type
recovery that could repair an unknown access type is deliberately gated to fixed single-extent
sources (`compiler/src/own/lower.rs:683-693`). The key and typedef flatten to the base element
(`compiler/src/types/keys.rs:53,77-79`; `compiler/src/codegen/types.rs:101,345-347`), so a view of
`T[M]` rows is emitted as `ClSlice_T { T *ptr; usize len; }` while the builder casts a
pointer-to-row (`compiler/src/codegen/calls.rs:57-67,84-119`). A row move copies one scalar while
the caller still owns the row (host ASan double-free); a nested access mis-types and the strict
build fails at the access (the exact diagnostic depends on the element spelling).

**Required Behavior:** R1 — the view element is the row; keys, typedefs, the builder, and access
typing all agree.

**Fix Required:** type the slice result from the source's row (`iterated_element`), carry the row
element through the key/typedef/registration paths (including `zero_view_helper` and the builder
spellings), use the pointer-to-array declarator for the view's `ptr` field, and drop the
now-redundant builder casts. Add the `slice<T>` disagreement check as a direct comparison in the
intrinsic lowering (AR-9; preflight PF-002).

### Gap 2: DEF-72 — a literal-field index call is never emitted

**Current Behavior:** `Index` lowering wraps a side-effecting index in a deferred `Materialize`
(`compiler/src/own/lower.rs:48-50`, `:705-738`). `consume_place` prepares element projections
(`:930-933`), but `materialize_index` treats an already-deferred index as done (`:831-837`), so the
fill is never emitted; `value_place` maps it to `Projection::ElementTemp` (`:990`) and
`Statement::Move` renders the place without calling `emit_guards` (`compiler/src/codegen/function.rs:405-449`).
The zero-initialized temp reads slot 0 and `pick()` never runs.

**Required Behavior:** R2 — the index call runs once and selects the intended slot.

**Fix Required:** carry the pending fill on the consumed value and emit it through the guard
emitter at the consuming point, and empty the deferred temporary at every consumer — including the
indexed-assignment adoption (03-02; preflight PF-006).

### Gap 3: DEF-73 — element-projection consume is eager during literal lowering

**Current Behavior:** `consume_place` settles a deferred base eagerly (`compiler/src/own/lower.rs:771-785`)
and emits the `make()` statement while the literal is being lowered, before the literal statement
that renders the earlier field `f()` — so a later field's base call runs before an earlier field's
inline value (`spec/04-expressions.md:51-66` is violated; trace 21 instead of 12).

**Required Behavior:** R3 — strict source order.

**Fix Required:** defer the consume preparation to the consuming point (03-02).

### Gap 4: DEF-74 — member-rooted consume leaks the siblings

**Current Behavior:** `consume_place` prepares only `Index` values (`compiler/src/own/lower.rs:930-936`);
a `Member` over a call falls through to `value_place`, which returns `None` for a call base
(`:975-997`), so the selected field is read inline and the call result's other owning fields are
never released. The deliberate doc comment (`:920-923`) records that the member form kept its
inline read to preserve field order after the earlier field-order fix.

**Required Behavior:** R4 — release the siblings without breaking R3.

**Fix Required:** extend the consume preparation to member roots and route it through the deferred
path (03-02).

### Gap 5: DEF-71 — non-direct-call take/consume roots leak

**Current Behavior:** (a) A call with its own `take` argument takes the eager branch
(`compiler/src/own/lower.rs:135-145`): `materialize_value` registers no release (`:895-905`), so a
later `sink(mkfrom(mka()).a)` moves `.a` out while `.b` is never released. (b) A take argument whose
root is a `Conditional` is not matched by the guarded root bind, which handles only
`Call`/`ClosureCall` (`:1424-1431`), so neither arm's result gets a release.

**Required Behavior:** R5 — release the remaining fields on every path.

**Fix Required:** bind the eager temporary through the destination-temp materializer and emit the
emptying adoption (`AssignThenMoveOut`) for registered destination temporaries; bind a conditional
root as a whole (03-03; preflight PF-003).

### Gap 6: DEF-89 — an owned member receiver is not bound

**Current Behavior:** Only intrinsic arguments (`compiler/src/own/lower.rs:1447-1486`) and dot-`len`
receivers (`compiler/src/own/call.rs:278-279`) route through the owned binding; a plain `Member`
read (`compiler/src/own/lower.rs:34-37`) leaves `spawn()` inline, so the `Array` buffer leaks
(`return i32(spawn().count);`, 20 bytes).

**Required Behavior:** R6 — bind and release the receiver like dot-`len`.

**Fix Required:** bind an owned member receiver at the read positions that take the value
(cast/intrinsic/argument/return reads), explicitly excluding consume preparation, guarded
positions, and loop-condition lowering (03-03; preflight PF-004).

### Gap 7: DEF-75 — invalid element-store drop spelling

**Current Behavior:** The owned branch of `Statement::StorePointer` releases the replaced element
with `emit_drop_expr(&format!("*({pointer_c})"), ...)` (`compiler/src/codegen/function.rs:509`);
`emit_struct_drop` appends fields as `{c_name}.{field}` (`compiler/src/codegen/drop.rs:86`), which
renders `*(&(out[i])).a` — parsed as `*((&(out[i])).a)` — and the strict host build rejects it.

**Required Behavior:** R7 — the target drop reads `(*(&(out[i]))).a`.

**Fix Required:** parenthesize the base at the field-append site (03-04).

## Dependencies

### Internal Dependencies

- The ownership/IR stage (`own/lower.rs`, `own/call.rs`) must keep the IR invariants the emitter
  assumes: a `Materialize`'s fill runs where the value is evaluated; a move empties exactly once;
  a pinned index survives on the place.
- The emission-integrity auditor's strict `-Werror` unit builds are the enforcement for the
  row-typed view spelling: a forgotten row type becomes a compile error, not silent arithmetic.
- The host ASan probe harnesses in the existing suites (`run_memory`/`assert_no_asan` in
  `compiler/tests/def_backlog_closeout_spec_test.rs`, `emit_host`/`run_memory_relaxed` in its
  impl suite) are the memory oracle.

### External Dependencies

None. The workspace stays std-only; no new crate, no new host tool.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Row-typed view keys change monomorphization identity for multi-extent slice users | Medium | Medium | Programs whose behavior depends on the view element are wrong today; the rest are retyped without behavior change; the emission-integrity audit traces every diff to R1 (preflight PF-010) |
| Two row representations (`T[][M]` and a row-element `T[]`) register different typedefs | Medium | Medium | `iterated_element` yields the same row for both; a unit test pins the shared key, and the registration path skips parameter-typed rows |
| A deferred consume fill or move-out runs twice (or not at all) on a branch | Medium | High | Phase 2 spec cases pin exact-once release under host ASan; the guard emitter is the single fill point |
| A consume temporary is not emptied at a consumer (indexed store) | Medium | High | Consumer-side emptiness is specified; a loop-form host-ASan probe pins exact-once release (preflight PF-006) |
| Receiver binding changes a consuming, guarded, or loop position | Medium | High | Read-position placement with explicit exclusions; the probe matrix covers adoption, guarded literal, and loop-condition forms (preflight PF-004) |
| Binding an owned receiver changes an adopting position and double-releases | Low | High | Extend only the read positions; the existing `value_may_lend`, `projects_from_place`, and pending-release guards stay in force (03-03) |
| The flat-name implementation pins fail after the row-typed change | High | Low | The pins are migrated in the same phase (AR-3); the specification tests are untouched |
