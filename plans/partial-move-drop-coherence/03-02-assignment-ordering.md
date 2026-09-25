# Assignment Ordering: Partial-Move / Drop Coherence

> **Document**: 03-02-assignment-ordering.md
> **Parent**: [Index](00-index.md)

## Overview

This component fixes the assignment lowering so it never drops a stale copy that
aliases a value the RHS moved out. It fixes the struct-literal reassignment
form (`b = Box<String> { value: b.value };`, DEF-11). It depends on 03-01, which
zeroes the moved-out fields while the RHS is evaluated.

## Architecture

### Current Architecture

`emit_assign` (`compiler/src/codegen/function.rs:483-518`) chooses between three
paths for an owned target: a direct store when the RHS does not read the target;
a save/restore path when the RHS reads the target, which copies the old target
before the RHS and drops the copy after storing.

### Proposed Changes

For an owned target, evaluate the RHS into a temporary, release the old target,
then store the temporary. The RHS no longer needs a save-copy: an owned RHS
value is independent of the target once produced, and any field the RHS moved
out was already zeroed by 03-01.

## Implementation Details

### New/Changed Functions

```rust
fn emit_assign(&mut self, span: Span, target: &Place, value: &Value) {
    let ty = self.substitute(&target.ty);
    self.emit_line_directive(span);
    let target_c = self.place(target);
    if !self.emits_drop(&ty) {
        self.emit_guards(value);
        let lowered = self.value(value);
        self.push(&format!("    {target_c} = {lowered};"));
        return;
    }
    // Evaluate the right-hand side first, then release the old target, then
    // store. A nested move has already zeroed the fields the RHS took, so the
    // release cannot free a value the new target owns.
    let temp = self.unique_local("__cl_new");
    self.emit_guards(value);
    let lowered = self.value(value);
    self.push(&format!("    {} = {};", declarator(&ty, &temp), lowered));
    self.emit_drop(target);
    self.push(&format!("    {target_c} = {temp};"));
}
```

`value_reads_base` and the save/restore branch are removed as no longer needed.

### Integration Points

- `emit_drop` / `emit_drop_expr` are unchanged; they drop the target's remaining
  (un-moved) fields, and a moved field is already empty.
- The existing borrow test `an_assignment_reading_its_target_drops_the_old_value_after_the_rhs`
  (`compiler/tests/codegen_run_spec_test.rs:690-722`) must still pass: the RHS is
  evaluated, then the old value is released, then the new value is stored.
- `Statement::Move` (whole-place moves) keeps its own lowering
  (`compiler/src/codegen/function.rs:324-332`); this component touches only
  `Statement::Assign` to an owned target.

## Code Examples

### Example 1: Struct-literal reassignment

```clorraine
struct Box<T> { value: T; }
func demo(): void {
    let b = Box<String> { value: stringOf("x") };
    b = Box<String> { value: b.value };
}
```

With 03-01 the RHS evaluates as `tmp_field = b.value; b.value = NULL;` and
builds the literal from `tmp_field`. This component then stores the literal into
a temporary, drops `b` (whose `value` is empty), and assigns the temporary. The
handle is freed once.

### Example 2: Borrow RHS unchanged

```clorraine
handle File freedBy Close;
func Dup(borrow f: File): File { ... }
func demo(f: File): void { f = Dup(borrow f); }
```

Emits `open/dup/close/close`: the RHS result is stored in a temporary, the old
value is released, then the new value is stored.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| Owned RHS that reads the target | Evaluated before the release; correct because a produced owned value is independent of the target | AR-3 |
| Self-assignment `s = s` | The RHS is a moved place (03-01), so the old source is empty before the release | AR-13 |
| A duplicable target | The plain store path is unchanged | AR-2 |
| A view target | Not owned; does not enter this path | AR-12 |

> **Traceability:** Every error-handling strategy references the Ambiguity
> Register entry (AR #) that resolved it. See `00-ambiguity-register.md`.

## Testing Requirements

- `compiler/tests/codegen_run_spec_test.rs`: the struct-literal reassignment
  runs and frees once (ASan clean); the borrow case still emits
  `open/dup/close/close`.
- `compiler/tests/stage_spec_test.rs`: the struct-literal form's emitted C has
  no `__cl_save` for a moved-out handle and zeroes the moved field.
- The existing reassignment and drop tests still pass.
