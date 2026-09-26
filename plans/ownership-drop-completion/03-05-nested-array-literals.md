# Nested Array-Literal Emission: Ownership and Drop Model Completion

> **Document**: 03-05-nested-array-literals.md
> **Parent**: [Index](00-index.md)

## Overview

A multi-dimensional fixed array is initialized with a nested literal
(`u8[3][2] = [[1, 2], [3, 4], [5, 6]]`, `spec/08-arrays-strings.md:58`). The
emitter currently assigns a brace-enclosed initializer to an array element
(`d[0] = { 1, 2 };`), which is not C. This component makes the array-literal
assignment recurse per dimension so the emitted C is valid. It resolves PF-003
and is required before multi-dimensional drop (R5) can be tested.

## Architecture

### Current Architecture

`emit_array_assign` (`codegen/function.rs:521-549`) writes
`target[index] = <element value>` for each element, and an array-literal value
renders as `{ ... }` (`codegen/expr.rs:556-565`). For a nested literal, each
element is itself an array, so a C array element is assigned a brace list, which
C rejects. A type-check-only regression test exists
(`sema/regressions.rs:34-45`), but no build test, so the defect is latent.

### Proposed Changes

Make `emit_array_assign` recurse over the target's extents: when an element value
is an array literal and the element type is a fixed array, emit its sub-elements
against the indexed sub-target. A single-dimension array already works and is
unchanged.

## Implementation Details

```rust
fn emit_array_assign(&mut self, target_c: &str, ty: &Ty, elements: &[Value], fill: &Option<Box<Value>>) {
    let TyKind::Array { element, extents } = &ty.kind else { return };
    let element_ty = (**element).clone();
    for (index, value) in elements.iter().enumerate() {
        let sub = format!("{target_c}[{index}]");
        match &value.kind {
            ValueKind::ArrayLit { elements, fill }
                if element_ty.is_fixed_array() =>
            {
                self.emit_array_assign(&sub, &element_ty, elements, fill)
            }
            _ => {
                self.emit_guards(value);
                let lowered = self.value(value);
                self.push(&format!("    {sub} = {lowered};"));
            }
        }
    }
    if elements.is_empty() {
        if let Some(fill) = fill {
            let extent = extents.first().and_then(|size| *size).unwrap_or(0);
            self.emit_guards(fill);
            let lowered = self.value(fill);
            for index in 0..extent {
                self.push(&format!("    {target_c}[{index}] = {lowered};"));
            }
        }
    }
}
```

The entry point in `emit_statement` (`codegen/function.rs:311-323`) passes the
target expression string instead of a `Place`, so the recursion can append one
index per dimension. For a move-only element the value is already moved by the
R4 ownership/lowering change; this component only fixes the C shape.

## Code Examples

```clorraine
let d: u8[3][2] = [[1, 2], [3, 4], [5, 6]];
// emitted:
//   d[0][0] = 1u; d[0][1] = 2u;
//   d[1][0] = 3u; d[1][1] = 4u;
//   d[2][0] = 5u; d[2][1] = 6u;
```

## Error Handling

| Error Case | Handling Strategy | AR Ref |
| ---------- | ----------------- | ------ |
| A fill in a nested literal | each row expands to per-element assignments | AR-10 |
| A one-dimensional literal | unchanged path | AR-6 |

## Testing Requirements

- A checker test that `u8[3][2] = [[1, 2], [3, 4], [5, 6]]` is accepted (exists).
- A build-and-run oracle that the emitted C compiles and reads `d[2][1] == 6`.
- An emitter test for the per-element shape.
