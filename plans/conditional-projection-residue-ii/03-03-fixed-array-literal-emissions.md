# Fixed-Array and Literal Emissions: Conditional, Projection, and Update Residue II

> **Document**: 03-03-fixed-array-literal-emissions.md
> **Parent**: [Index](00-index.md)

## Overview

Eight rows where an accepted shape emits C that fails the strict build, or where the emitted C
mis-handles a destination. Four are field-initializer positions (a conditional value, a
conditional-root projection, a guarded view-array field, an array-literal element), two are
conditional-of-literals shapes (call-filled fields, the guarded/deferred fallback), one is the
**critical** `String[2]` conditional-root return segfault, and one is the residual element-read
base of the nested conditional-arm projection. The operative contract is Ch 17 (the hidden
fixed-array destination ABI, `spec/17-codegen.md:59-84`) plus the batch-I per-arm destination
store; the critical's store twin is already correct and is the shape to reuse.

## Design

### 1. The conditional value in a field initializer (`DEF-298`)

`Holder { row: c ? a.row : mkBox().row }` guarded emits `(ClHolder){ (c ? a.row : mkBox().row) }.row[0]`
(`int-conversion`, `missing-braces` at base and head). A fixed-array field value cannot ride a
C initializer list.

Route the conditional value through the per-arm destination store in the field-initializer
position: the literal binds into a statement-form build (the batch-I `module-initializer` and
field-store shapes), each arm fills the field on its path (the call arm passes the hidden
destination through the existing arm store), and the built struct is consumed by the
surrounding position.

### 2. The conditional root in a field initializer (`DEF-299`)

`Holder { row: (c ? a : b).row }` emits the same invalid initializer: `DEF-275`'s per-arm address
form covers the store/return positions but not the field-initializer route.

Extend the per-arm rendering to the field-initializer position: the projection's base
distributes per arm (`distribute_conditional_projection`), each arm's root binds where the arm
has no place, and the field copies from the per-arm address exactly as the store spelling does.

### 3. The guarded view-array field (`DEF-300`)

`g && len(Holder { names: [mk().v] }.names[0]) > 0` emits an incomplete `ClStr` initializer
(`-Werror=missing-field-initializers`, distinct from the fixed-array arm route).

In the guarded position, the literal's view-array field uses the array-elements initializer (the
element-wise form the unguarded spelling uses), so the emitted literal is complete and the
guarded operand evaluates on its path only.

### 4. The conditional of call-filled literals (`DEF-302`)

`ok ? Mixed { names: n1(), tag: … } : …;` (call-filled fixed-array fields) drops the call
arguments (`too few arguments`): each literal's field build must pass the hidden destination
per arm.

Route the conditional-of-literals through the guards literal-fill work: each arm's literal
builds with its field stores (the call-filled field passing the destination), and the
conditional's destination fills on the selected path. The single-literal spelling keeps its
batch-I shape.

### 5. The `String[2]` conditional-root return — critical (`DEF-303`)

`func pick(c: bool): String[2] { …; return (c ? b1 : b2).xs; }` emits
`__cl_ret = __cl_tmp1;` / `__cl_ret = __cl_tmp2;` — assigning an array to the C-adjusted array
parameter — and segfaults at run time (probe-confirmed, exit 139; the emission is byte-identical
to the phase-5 tree). The `String[2]` store twin is correct.

Bind the conditional root per arm in the fixed-array return emitter: each arm copies into the
hidden destination (`memcpy` per arm on its own path), the `Temp` arms from
`consume_conditional_projection` feed the copies, and no array assignment remains. This is the
gap between the Temp arms and `emit_fixed_array_return_arm`; the fix reuses the store twin's
copy shape.

### 6. The guarded/deferred expression-path fallback (`DEF-304`)

`g && consume((c ? b1 : b2).names)` and `g && consume(c ? mk1() : mk2())` are accepted and the
guarded expression-path fallback emits `assignment to expression with array type` / `too few
arguments`. The guards statement-path branch batch I replaced is unreachable today (0 hits
across 4593 corpus runs), so the fallback is the live route.

Route the expression-path fallback through the per-arm destination store when a constructor
reaches it: the fallback emits the same per-arm destination fills as the statement path, the
guarded operand keeps its lazy evaluation, and no constructor is rendered as a value expression.

### 7. The array-literal element gate (`DEF-305`)

`Outer { items: [Inner { row: n1() }, …] };` (an array-of-structs element with a call-filled
fixed-array field) emits `too few arguments`: `fixed_array_field_needs_stores` does not descend
`ArrayLit` elements.

Extend the gate to `ArrayLit` elements so an array element whose struct carries a call-filled
fixed-array field takes the statement-form store route.

### 8. The nested-arm read base — residual (`DEF-306`)

`(c ? (d ? b1 : b2).ys : b3.ys)[i]` emits `lvalue required as unary '&' operand` (probe
confirmed, container and host): the batch-I phase-5 correction distributed the store/return/
iterable gates through nested arms, but the element-read base still renders the nested
conditional directly.

Distribute the read base like the fixed gates: the nested arms rebuild per leaf
(`c ? (d ? b1.ys : b2.ys) : b3.ys`), each arm's root binds where needed, and the index reads the
selected arm's storage. The fixed read position's byte identity is the control.

## Integration Points

- `codegen/function.rs`: the literal field route (:569 `fixed_array_field_needs_stores` :1542),
  the per-arm destination stores.
- `codegen/guards.rs`: the deferred fill and the expression-path fallback.
- `codegen/cleanup.rs`: `emit_fixed_array_return_arm` (:751).
- `own/lower.rs`: `consume_conditional_projection` (:2181),
  `distribute_conditional_projection` (:1660), `bind_fixed_array_root` (:1590), the field
  preparation.
- `own/statements.rs`: `prepare_store` (:412)/`check_return` (:1253) — the arm gates.

## Error Handling

| Error Case | Handling Strategy | AR Ref |
|------------|-------------------|--------|
| A conditional value in a field initializer | The statement-form build; each arm fills on its path | AR-14 |
| A guarded/deferred fixed-array constructor | The per-arm destination store; the guard keeps its lazy evaluation | — |
| A `String[2]` conditional-root return | Each arm copies into the hidden destination; no array assignment | — |
| A nested-arm element read | The arm rebuild per leaf; the direct read unchanged | — |
| A constructor-free guarded operand | The existing expression path (re-emits identically) | — |
| An array-literal element without a call-filled fixed-array field | The existing literal route (byte-identical) | — |

## Testing Requirements

- Spec: the field-initializer cases (both selection paths, guarded and plain; strict build and
  run), the guarded view-array field, the conditional of call-filled literals (call counts per
  arm), the `String[2]` conditional-root return (both selections; run exit 0, ASan-clean — red at
  the planning head with exit 139), the guarded/deferred fallback cases, the array-literal
  element case, and the nested-arm read case (store/return/iterable controls strict-build and
  run green as verified in AR-3).
- Impl: the per-arm destination shapes, the hidden-destination pass, the copy shapes, the arm
  rebuild, and the re-emission determinism pins for the direct spellings.
