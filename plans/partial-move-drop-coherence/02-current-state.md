# Current State: Partial-Move / Drop Coherence

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The IR models a move explicitly. `Statement::Move` copies a source place into a
target and then zeroes the source (`compiler/src/ir.rs:288-294`), and codegen
realizes the zeroing in `emit_move_zero` (`compiler/src/codegen/function.rs:553-561`):
a handle becomes `NULL`, an aggregate is `cl_zero(&place)`. The ownership stage
emits these for a whole-place move (`emit_store`,
`compiler/src/own/statements.rs:179-189`) and for a `take` argument
(`take_move_out_places`, `compiler/src/own/call.rs:302-322`). A temporary
mechanism already exists (`new_temp` in `compiler/src/own.rs`,
`ir::PlaceBase::Temp` in `compiler/src/ir/place.rs`, `materialize_value` in
`compiler/src/own/lower.rs:524-534`), so a lowering helper can materialize an
owned place into a temporary.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/expr.rs` | `ExprKind::StructLiteral` check (`:133-137`) | Mark a consumed non-duplicable, non-type-parameter field moved |
| `compiler/src/own/lower.rs` | `lower_value`, `StructLit` arm (`:158-164`) | Consume an owned place field as a move |
| `compiler/src/own/place.rs` | `move_target`/`move_place` (`:170-244`) | Reused unchanged; enforces `E10020`/`E10026`/`E10034` |
| `compiler/src/codegen/function.rs` | `emit_assign` (`:483-518`), `emit_move_zero` (`:553-561`) | Unchanged |
| `compiler/tests/stage_spec_test.rs` | Emitted-C oracle | Add source-zero cases |
| `compiler/tests/codegen_run_spec_test.rs` | Compile-and-run oracle | Add free-once cases |
| `compiler/src/own/tests.rs` | Ownership diagnostics | Add `E10020`/`E10026` cases |

### Code Analysis

A struct literal's field is lowered as a plain value, so a non-duplicable place
field is copied without zeroing (`compiler/src/own/lower.rs:158-170`), and the
ownership stage's literal check only recurses (`compiler/src/own/expr.rs:133-137`).
The assignment emitter then sees a RHS that reads the target and takes the
save/restore path (`compiler/src/codegen/function.rs:503-517`):

```rust
let save = self.unique_local("__cl_save");
self.push("    {");
self.push(&format!("        {} = {};", declarator(&ty, &save), target_c));
self.emit_guards(value);
let lowered = self.value(value);
self.push(&format!("        {target_c} = {lowered};"));
self.emit_drop_expr(&save, &ty);
```

The save path is the amplifier that produces the double free, but it is not the
root: once the literal field is consumed into a temporary, the RHS no longer
reads the target, `value_reads_base` is false
(`compiler/src/codegen/function.rs:620-623`), and the existing fast path
(`:494-501`) already drops the empty old target and stores correctly. **The fix
is therefore an ownership mark plus the lowering change; the assignment emitter
is untouched.**

## Gaps Identified

### Gap 1: An owned place nested in a literal is copied, not moved (DEF-11)

**Current Behavior:** `b = Box<String> { value: b.value };` emits
`__cl_save = b; b = (ClBox__String){ b.value }; drop(__cl_save);` — the saved
handle aliases `b.value` and is freed, then `b.value` is freed again.
**Required Behavior:** the RHS moves `b.value` out (zeroing it), and the old
target is released without aliasing the new value; the handle is freed once.
**Fix Required:** consume an owned place nested in an aggregate literal as a
move (AR-1, AR-3).

### Gap 2: The ownership flow does not record a consumed literal field as moved

**Current Behavior:** `compiler/src/own/expr.rs:133-137` only recurses, so
`p = Pair { first: p.first, second: p.first }` compiles with no `E10020` and
`Box { value: o.inner.s }` compiles with no `E10026`, contrary to
`spec/03-ownership.md:123-134`.
**Required Behavior:** the consumed field is marked moved, so the second read is
`E10020` and a deeper path is `E10026`.
**Fix Required:** mark the consumed non-duplicable, non-type-parameter field
through `move_target` (AR-14, AR-15).

## Dependencies

### Internal Dependencies

- `Statement::Move` / `emit_move_zero` (existing) — the copy-plus-zero mechanism.
- `TypeTable::duplicable` — identifies an owned (non-duplicable) place.
- `move_target`/`move_place` (existing) — the flow-state and partial-move rules.
- The paused `generic-field-integrity` plan's Phase 4 depends on this plan.

### External Dependencies

- None. ASan is available in the SDK container for the test oracle (AR-4).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| A consumed field is not marked moved, so a second read is silently emptied | Medium | High | Mark the field in the same check that lowers it; add `E10020`/`E10026` regressions |
| A generic body is lowered once with `Param` types | Medium | Medium | Reuse the `mentions_type_param` guard; state the generic-body boundary (AR-11, PF-007) |
| A guarded/short-circuit field is mis-moved | Low | Medium | Scope it out; only a place field is consumed (AR-15, PF-011) |
| Non-move emitted C changes | Low | High | The assignment emitter is untouched; the determinism/corpus suites pin the fast path |
