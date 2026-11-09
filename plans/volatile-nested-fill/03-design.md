# Design: Volatile-Safe Nested Fill Copy

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## Problem

`emit_array_assign` computes one `compact` flag. A volatile destination makes
the flag false, but the nested fill path treats false as "no compaction" and
still copies each row with `memcpy` into the volatile object. The C front end
rejects the call because `memcpy` takes `void *`, not `volatile void *`.

The same problem reaches the prefix-row store: a place-valued prefix row is
delegated to `emit_fixed_array_assign_to`, whose duplicable final branch also
copies the whole row with `memcpy`.

The copy needs a second signal: whether the destination storage is volatile.
That signal belongs to the rendered array target, because the same target is
threaded through the literal, fixed-array, and conditional paths.

## Volatility Signal

Add one field to `ArrayTarget`:

```rust
pub(super) struct ArrayTarget {
    /// The C expression that names the array.
    text: String,
    /// The C expression that is the array's address.
    pub(super) address: String,
    /// The C expression for the array's byte count.
    pub(super) bytes: String,
    /// True when the C storage is `volatile`-qualified.
    is_volatile: bool,
}
```

- `ArrayTarget::object` and `ArrayTarget::parameter` default it to `false`.
  A local, a compiler temporary, a hidden return slot, and an adjusted array
  parameter never carry the qualifier.
- `ArrayTarget::element` propagates the flag to the row sub-target.
- `array_target(place, c_name)` computes it from the place:

```rust
let is_volatile = self.fill_target_volatile(place);
```

## Predicate

```rust
/// True when a fill target's storage is `volatile`-qualified in C.
///
/// Volatility is declared on a module-level `let` binding or on a struct
/// field. A `memcpy` into either discards the qualifier, so the nested fill
/// copy writes the row through the volatile lvalues instead. A field reached
/// through a raw pointer is tested through the pointee struct.
fn fill_target_volatile(&self, place: &Place) -> bool {
    if let PlaceBase::Global(id) = &place.base {
        if self.globals.get(id).is_some_and(|global| global.volatile) {
            return true;
        }
    }
    let mut ty = self.substitute(&self.place_base_type(&place.base));
    for projection in &place.projections {
        match projection {
            ir::Projection::Field(field) => {
                // `.len` is the structural length only on a fixed array; on
                // a struct it is an ordinary field with its own qualifier.
                if field == "len"
                    && matches!(
                        &ty.kind,
                        TyKind::Array { extents, .. }
                            if extents.iter().all(Option::is_some)
                    )
                {
                    ty = Ty::usize_ty();
                    continue;
                }
                let mut pointee = &ty;
                while let TyKind::Raw(inner) = &pointee.kind {
                    pointee = inner;
                }
                if let TyKind::Struct(name, _) = &pointee.kind {
                    if self.structs.get(name).is_some_and(|decl| {
                        decl.fields.iter().any(|declared| {
                            &declared.name == field && declared.volatile
                        })
                    }) {
                        return true;
                    }
                }
                ty = self.field_type(&ty, field);
            }
            _ => ty = super::render::element_type(&ty),
        }
    }
    false
}
```

The predicate walks the same projections as `place()`, so the field
declaration it tests is the one the C spelling resolves. A field reached
through a `raw` pointer strips the pointer layers before the struct lookup;
`field_type` already resolves the next projection through the pointer.

The predicate is a property of the syntactic place. It covers the storage a
place names directly. It does not cover indirection that loses the qualifier
before the fill (a `for borrow` loop header) or a `borrow` parameter, which the
checker already rejects for a global owner. Those are recorded as separate
limitations.

## Copy Helper

```rust
/// Copies a fixed array one leaf at a time through the destination lvalues.
///
/// A `volatile` destination cannot be passed to `memcpy`: the call discards
/// the qualifier. Writing every deepest element keeps each store a volatile
/// access, in row-major order, and copies the same bytes for a scalar or
/// struct leaf.
fn emit_volatile_array_copy(&mut self, target: &ArrayTarget, ty: &Ty, source: &str) {
    let TyKind::Array { element, extents } = &ty.kind else {
        return;
    };
    let extent = extents.first().and_then(|size| *size).unwrap_or(0);
    if extents.len() <= 1 {
        for index in 0..extent {
            self.push(&format!(
                "    {}[{index}] = {source}[{index}];",
                target.text
            ));
        }
        return;
    }
    let inner = Ty::array((**element).clone(), extents[1..].to_vec());
    for index in 0..extent {
        let sub = target.element(index);
        self.emit_volatile_array_copy(&sub, &inner, &format!("{source}[{index}]"));
    }
}
```

The base case is a scalar or a struct. Both are assignable C values; a struct
assignment through a volatile lvalue is valid C and performs the member
stores. Only arrays recurse.

## Copy Site 1: Fill Rows

The nested fill branch keeps the row temporary and changes only the copy:

```rust
for index in prefix..extent {
    let sub = target.element(index);
    if target.is_volatile {
        self.emit_volatile_array_copy(&sub, element_ty, &row);
    } else {
        self.push(&format!(
            "    memcpy({}, &({row}), {});",
            sub.address, sub.bytes
        ));
    }
}
```

The compact `memset` branch also tests the flag, defensively:

```rust
if compact && !target.is_volatile && nested_zero {
    ...
}
```

`compact` is already false for every volatile target the emitter can name, so
this guard documents the invariant and keeps the C valid if a future caller
threads a different compact decision. The row temporary's own fill still
recurses with `ArrayTarget::object`, so the temporary itself takes the
ordinary internal copy.

## Copy Site 2: Prefix Rows

A place-valued prefix row is delegated to `emit_fixed_array_assign_to`, whose
duplicable final branch copies the whole row with `memcpy`. That branch is
shared with whole-array assignments (`g = other`), so changing it in place
would silently expand the fix beyond the nested fill. The nested fill prefix
loop instead calls a dedicated wrapper:

```rust
/// Stores one explicit prefix row of a nested fill.
///
/// A volatile row destination copies leaf by leaf; every other value and
/// target keeps the shared fixed-array store core. The wrapper covers the
/// duplicable place form the prefix loop reaches. A conditional, call, or
/// owning value falls back to the core, whose volatile-copy classes are
/// recorded as separate limitations.
fn emit_prefix_row_assign_to(&mut self, target: &ArrayTarget, ty: &Ty, value: &Value) {
    let simple = !self.needs_drop(ty)
        && !matches!(
            value.kind,
            ValueKind::Conditional { .. }
                | ValueKind::Call { .. }
                | ValueKind::ClosureCall { .. }
        );
    if target.is_volatile && simple {
        self.emit_guards(value);
        let value_c = self.store_text(value, ty);
        if value_c == target.text {
            return;
        }
        self.emit_volatile_array_copy(target, ty, &value_c);
        return;
    }
    self.emit_fixed_array_assign_to(target, ty, value, false, false);
}
```

The nested fill element loop calls it in place of
`emit_fixed_array_assign_to`:

```rust
_ => self.emit_prefix_row_assign_to(&sub, element_ty, value),
```

The wrapper's guard combination matches the shared core's final branch:
`emit_guards`, `store_text`, the self-assignment return, and the copy. A
literal prefix row is stored element by element before this point and is
already valid.

### Why Not Change the Shared Core

The shared final branch is also reached by a whole-array assignment of a
non-owning place (`g = other`), which is not a fill. Changing it would alter
and partially fix a separate invalid-C class without its own tests and review.
The wrapper keeps this fix at the two nested fill copy sites. Whole-array
place copies remain `memcpy` and are pinned as an accepted limit.

## Example

For the reported shape:

```c
i32 __cl_fillrow_1[9] = {0};
__cl_fillrow_1[0] = 0;
...
cl_probe_NestedFill_g[0][0] = __cl_fillrow_1[0];
cl_probe_NestedFill_g[0][1] = __cl_fillrow_1[1];
...
cl_probe_NestedFill_g[7][8] = __cl_fillrow_1[8];
```

For the prefix place row:

```c
cl_probe_PrefixPlace_g[0][0] = row[0];
cl_probe_PrefixPlace_g[0][1] = row[1];
...
cl_probe_PrefixPlace_g[1][3] = row[3];
```

For a 3D volatile array, the helper unrolls three index steps per leaf.

## Safety Argument

| Property | Argument |
|---|---|
| Same values | The copy reads the same row temporary or prefix place that `memcpy` read; each leaf is one assignment of the same typed value |
| Same order | Leaves are written row-major; the source is ordinary storage and the destination is volatile, so the volatile stores stay ordered |
| Volatile semantics | Every destination access is a volatile lvalue access. No access is elided, merged, or reordered across the sequence |
| No trap change | The fill or prefix value is already evaluated before the copy; no guard, bounds check, or division moves |
| No drop change | The copy is value-identical to the existing copy; ownership and drop decisions are unchanged |
| C validity | No in-scope nested fill copy passes a volatile destination to `memcpy`; struct and scalar assignments through volatile lvalues are valid C99 |
| No non-volatile change | `is_volatile` defaults to false; every non-volatile target keeps the existing spelling |

## Exclusions

| Shape | Emission |
|---|---|
| Local, temporary, parameter, return slot | unchanged `memcpy` / `memset` |
| Non-volatile struct field | unchanged `memcpy` |
| Volatile one-extent scalar fill | unchanged element stores (the `compact` flag is false) |
| Volatile full-prefix zero fill | unchanged empty emission |
| Whole-array move or assignment (`g = other`) | unchanged shared store core; separate known limitation, pinned by IP-V12 |
| Struct-literal prefix element with a fixed-array field | unchanged struct-field store; separate known limitation |
| Conditional, call, or closure-call prefix row | the wrapper falls back to the shared core, which may `memcpy`; separate known limitation |
| `for borrow` alias over volatile storage (global or field) | the loop header discards the qualifier before the fill; separate limitation |
| `borrow` parameter bound to a volatile field | the call argument discards the qualifier; the callee's fill is not reached through a volatile type |
| `borrow` parameter bound to a volatile global | the checker rejects the borrow (`E10038`) |
| Owning explicit row copy (no fill) | the shared core may `memcpy` into a volatile field; a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`) |

## Scope Check

The change adds one predicate, one copy helper, one wrapper for the prefix-row
path, one branch in the fill path, and one field. It does not change the
shared fixed-array store core, the module initializer, the runtime shim, the
standard library, or the ABI.
