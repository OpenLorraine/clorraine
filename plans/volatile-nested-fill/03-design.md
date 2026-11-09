# Design: Volatile-Safe Nested Fill Copy

> **Document**: 03-design.md
> **Parent**: [Index](00-index.md)

## Problem

`emit_array_assign` computes one `compact` flag. A volatile destination makes
the flag false, but the nested fill path treats false as "no compaction" and
still copies each row with `memcpy` into the volatile object. The C front end
rejects the call because `memcpy` takes `void *`, not `volatile void *`.

The same problem reaches two other copy sites:

- a place-valued prefix row is delegated to `emit_fixed_array_assign_to`,
  whose duplicable final branch copies the whole row with `memcpy`;
- a run-time indexed whole-element target (`g[i] = [; [; 0]]`) is lowered to
  `Statement::StorePointer`, whose fixed-array branch builds an `ArrayTarget`
  from the dereferenced pointer and also copies with `memcpy`.

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
- `array_target(place, c_name)` computes it from the place.
- The `StorePointer` fixed-array branch computes it from the addressed value
  and sets it on the target it builds.

## Place Predicate

```rust
/// True when a fill target's storage is `volatile`-qualified in C.
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
                if self.struct_field_volatile(&ty, field) {
                    return true;
                }
                ty = self.field_type(&ty, field);
            }
            _ => ty = element_type(&ty),
        }
    }
    false
}
```

`struct_field_volatile` strips raw pointer layers and checks the field
declaration on the pointee struct. A `volatile raw T` field is emitted as
`volatile T *`, so a member behind it is volatile in the emitted C too; the
predicate intentionally treats it as volatile.

The predicate is a property of the syntactic place. It covers the storage a
place names directly. It does not cover indirection that loses the qualifier
before the fill (a `for borrow` loop header) or a `borrow` parameter, which the
checker already rejects for a global owner. Those are recorded as separate
limitations.

## Value Predicate for Run-Time Indexed Targets

A run-time indexed target arrives as an address-of expression whose operand is
a `Value`, not a `Place`:

```rust
/// True when a value names storage that is `volatile`-qualified in C.
fn value_storage_volatile(&self, value: &Value) -> bool {
    match &value.kind {
        ValueKind::Place(place) => self.fill_target_volatile(place),
        ValueKind::Member { base, name } => {
            self.struct_field_volatile(&base.ty, name)
                || self.value_storage_volatile(base)
        }
        ValueKind::Index { base, .. } => self.value_storage_volatile(base),
        ValueKind::Unary {
            op: UnaryOp::AddressOf,
            operand,
        } => self.value_storage_volatile(operand),
        ValueKind::Materialize { value, .. } => self.value_storage_volatile(value),
        _ => false,
    }
}
```

The `StorePointer` fixed-array branch derives the flag:

```rust
let storage_volatile = matches!(
    &pointer.kind,
    ValueKind::Unary {
        op: UnaryOp::AddressOf,
        operand,
    } if self.value_storage_volatile(operand)
);
let mut target = ArrayTarget::object(&target_c);
target.is_volatile = storage_volatile;
```

A raw pointer dereference does not add the qualifier; the pointer type does
not carry it. A run-time target through an alias pointer is a separate
limitation.

## Copy Helper

```rust
/// Copies a fixed array one leaf at a time through the destination lvalues.
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
stores. Only arrays recurse. The copy preserves typed values; struct padding
after an assignment is unspecified, as it is for any C assignment.

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

The wrapper is used only by the nested fill element loop:

```rust
fn emit_prefix_row_assign_to(&mut self, target: &ArrayTarget, ty: &Ty, value: &Value) {
    if target.is_volatile && !self.needs_drop(ty) {
        if stable_storage_value(value) {
            self.emit_guards(value);
            let value_c = self.store_text(value, ty);
            if value_c == target.text {
                return;
            }
            self.emit_volatile_array_copy(target, ty, &value_c);
            return;
        }
        let row = self.unique_local("__cl_fillrow");
        self.push(&format!(
            "    {} = {};",
            declarator(ty, &row, &self.sema.table),
            self.empty_state(ty)
        ));
        let row_target = ArrayTarget::object(&row);
        self.emit_fixed_array_assign_to(&row_target, ty, value, false, false);
        self.emit_volatile_array_copy(target, ty, &row);
        return;
    }
    self.emit_fixed_array_assign_to(target, ty, value, false, false);
}
```

`stable_storage_value` is true for a place, a temporary, a literal scalar, a
member or element of a stable value with a stable index, a pure unary/binary
expression, a cast, or an address-of of one, and for a materialization that
records a temp or destination. A call, a conditional, a call-returning member,
or a call-valued index is not stable: appending an element index to its
rendered text would re-run the effect once per leaf. The temporary path
evaluates it exactly once with the shared core, then copies from the
temporary.

A constant or arithmetic index (`h[0]`, `h[k - 1]`) is therefore stable and
copies a volatile source row leaf by leaf. Rejecting literal scalars would
route the row through a temporary and copy from the volatile source with
`memcpy`, which strict C rejects.

The wrapper deliberately does not change `emit_fixed_array_assign_to`'s shared
final branch: that branch is also reached by a whole-array assignment
(`g = other`), which is not a fill. Changing it would partially fix a separate
invalid-C class without its own tests.

### Why Not Change the Shared Core

The shared final branch is also reached by a whole-array assignment of a
non-owning place (`g = other`), which is not a fill. Changing it would alter
and partially fix a separate invalid-C class without its own tests and review.
The wrapper keeps this fix at the nested fill copy sites. Whole-array place
copies remain `memcpy` and are pinned as an accepted limit.

## Copy Site 3: Run-Time Indexed Elements

The `StorePointer` fixed-array branch builds the target and sets the derived
flag before delegating to `emit_fixed_array_assign_to`. The nested fill inside
that call then copies leaf by leaf. The pointer expression is rendered once and
its guards run once; only the element reads repeat.

## Example

For the reported shape:

```c
i32 __cl_fillrow_1[9] = {0};
__cl_fillrow_1[0] = 0;
...
cl_probe_NestedFill_g[0][0] = __cl_fillrow_1[0];
...
cl_probe_NestedFill_g[7][8] = __cl_fillrow_1[8];
```

For a run-time indexed element:

```c
(*(&(cl_probe_Index_g[__cl_tmp0])))[0][0] = __cl_fillrow_1[0];
```

For an impure prefix row, the call fills one row temporary once, then the
leaf copy reads it.

## Safety Argument

| Property | Argument |
|---|---|
| Same values | The copy reads the same row temporary or stable place that `memcpy` read; each leaf is one assignment of the same typed value |
| Same order | Leaves are written row-major; the source is ordinary storage and the destination is volatile, so the volatile stores stay ordered |
| Volatile semantics | Every destination access is a volatile lvalue access. No access is elided, merged, or reordered across the sequence |
| Evaluate once | An impure fill or prefix value is evaluated once into a temporary; the leaf copy only reads it. A stable place is re-read without an effect |
| No trap change | The fill, prefix, or element value is evaluated before the copy; no guard, bounds check, or division moves |
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
| A volatile array used as a copy source into a row temporary | the row-temporary `memcpy` reads the source; separate pre-existing class |
| Struct-literal prefix element with a fixed-array field | unchanged struct-field store; separate known limitation |
| `for borrow` alias over volatile storage (global or field) | the loop header discards the qualifier before the fill; separate limitation |
| `borrow` parameter bound to a volatile field | the call argument discards the qualifier; the callee's fill is not reached through a volatile type |
| `borrow` parameter bound to a volatile global | the checker rejects the borrow (`E10038`) |
| Owning explicit row copy (no fill) | the shared core may `memcpy` into a volatile field; a move-only fill is rejected (`E10215`) and a module-level `volatile let` requires a duplicable type (`E11106`) |

## Scope Check

The change adds one predicate, one value predicate, one copy helper, one
wrapper for the prefix-row path, one branch in the fill path, one flag in the
run-time element store, and one field. It does not change the shared
fixed-array store core, the module initializer, the runtime shim, the standard
library, or the ABI.
