# Current State: Partial-Move / Drop Coherence

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The IR models a move explicitly. `Statement::Move` copies a source place into a
target and then zeroes the source; `Statement::MoveOut` zeroes a place whose
value moved into a call (`compiler/src/ir.rs:288-305`). Codegen realizes the
zeroing in `emit_move_zero` (`compiler/src/codegen/function.rs:551-561`): a
handle becomes `NULL`, an aggregate is `cl_zero(&place)`. The ownership stage
emits these statements for a whole-place move (`emit_store`,
`compiler/src/own/statements.rs:179-189`) and for a `take` argument through
`take_move_out_places` (`compiler/src/own/call.rs:302-322`).

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/lower.rs` | `lower_value`, including the `StructLit`/`ArrayLit` arms (`:158-170`) | Consume an owned place field/element as a move |
| `compiler/src/own/call.rs` | `take_move_out_places` (`:302-322`) | Also move a nested `Member`/`Index` argument |
| `compiler/src/own/statements.rs` | `emit_store` (`:179-189`) | Reuse the shared consume helper |
| `compiler/src/codegen/function.rs` | `emit_assign` (`:483-518`) | Evaluate the RHS, release the old target, then store |
| `compiler/tests/codegen_run_spec_test.rs` | Compile-and-run oracle | Add free-once cases |
| `compiler/tests/stage_spec_test.rs` | Emitted-C oracle | Add source-zero cases |

### Code Analysis

A struct literal's field is lowered as a plain value, so a non-duplicable place
field is copied without zeroing (`compiler/src/own/lower.rs:158-170`), and
`take_move_out_places` matches only a whole `ValueKind::Place`, not a
`ValueKind::Member` (`compiler/src/own/call.rs:317`). The assignment emitter then
takes a save-copy when the RHS reads the target and drops that copy after
storing (`compiler/src/codegen/function.rs:503-517`):

```rust
let save = self.unique_local("__cl_save");
self.push("    {");
self.push(&format!("        {} = {};", declarator(&ty, &save), target_c));
self.emit_guards(value);
let lowered = self.value(value);
self.push(&format!("        {target_c} = {lowered};"));
self.emit_drop_expr(&save, &ty);
```

## Gaps Identified

### Gap 1: An owned place nested in a literal is copied, not moved (DEF-11)

**Current Behavior:** `b = Box<String> { value: b.value };` emits
`__cl_save = b; b = (ClBox__String){ b.value }; drop(__cl_save);` — the saved
handle aliases `b.value` and is freed, then `b.value` is freed again.
**Required Behavior:** the RHS moves `b.value` out (zeroing it), and the old
target is released without aliasing the new value; the handle is freed once.
**Fix Required:** consume an owned place nested in an aggregate literal as a
move (AR-1, AR-3).

### Gap 2: The assignment save-copy can free a moved-out value

**Current Behavior:** the save path copies the old target before the RHS and
drops the copy afterward, which is wrong when the RHS moved a field out.
**Required Behavior:** evaluate the RHS, release the old target, then store.
**Fix Required:** reorder `emit_assign` and drop the save copy (AR-3).

### Gap 3: A `take` argument that is a field projection is not emptied

**Current Behavior:** `take_move_out_places` matches only a whole place, so
`consume(b.value)` would not zero `b.value`.
**Required Behavior:** a value-consuming `take` argument that is a non-duplicable
place (including a field) moves and zeroes the source.
**Fix Required:** route the `take` argument through the shared consume helper
(AR-1).

## Dependencies

### Internal Dependencies

- `Statement::Move` / `emit_move_zero` (existing) — the copy-plus-zero mechanism.
- `TypeTable::duplicable` — identifies an owned (non-duplicable) place.
- The paused `generic-field-integrity` plan's Phase 4 depends on this plan.

### External Dependencies

- None. ASan is available in the SDK container for the test oracle (AR-4).

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The assignment reorder changes emitted C for unrelated programs | Medium | High | The order for a non-reading RHS is unchanged (drop then store); the reading-RHS order is already drop-after-RHS in the existing test; the determinism suite pins byte identity |
| A move is missed at a value-consuming site | Medium | Medium | Route every owned-place consumption through one helper; grep the sites in review |
| An owned RHS value depends on the target after evaluation | Low | High | An owned RHS never holds a reference to the target; a view assigned to an owned target is a type error (AR-12) |
| Moving a field twice | Low | Medium | The ownership stage already reports use-after-move `E10020`; the lowering moves only the first read (AR-14) |
