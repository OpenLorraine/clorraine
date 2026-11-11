# Lowering and Runtime: Zero State, Discard Cleanup, Aggregate Adopt

> **Document**: 03-02-lowering-and-runtime.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R3, R4, R5, R6, R9 (see [01-requirements.md](01-requirements.md))

## 1. Zero-State String Handle

### 1.1 Rule

`spec/03-ownership.md` O7: the all-zero bit pattern is the valid
"nothing to release" state for every droppable type. `spec/07-structs.md`
STR-7: `Point {}` is the explicit zero value. A `String` field of a zero value
is therefore the empty String, not an invalid pointer. The runtime read
accessors must agree with `FreeString`, which is already null-safe.

### 1.2 Runtime Change

| Helper | NULL behavior |
|--------|---------------|
| `cl_string_ptr` | return `NULL` (an empty view's pointer) |
| `cl_string_len` | return `0` |
| `cl_string_copy` | return `cl_string_new(0)` |

Apply the same change to `compiler/runtime/shim_host.c` (`:276`, `:282`,
`:305`) and `compiler/runtime/shim_hosted_amiga.c` (`:893`, `:899`, `:922`).
`cl_string_append` already skips a zero-length copy, so
`cl_string_new(0)` stays valid. `cl_string_from` is only called with a valid
view or a null-safe copy.

The generated `cl_string_view` (`compiler/src/codegen/program.rs:221`) needs no
change: it reads the two now-safe accessors and produces `{NULL, 0}` for the
zero state, which is the language's empty view.

### 1.3 Consequences

- `$"{h.name}"` copies the empty String and prints empty.
- `println(h.name)` widens through `cl_string_view` and writes zero bytes.
- `len(h.name)` reads the view length (0).
- `clone(h.name)` allocates an empty String.

The plan tests those four paths. `bytes(h.name)` returns an empty slice and
`toCString(h.name)` returns the empty cstring (NULL pointer); they are not part
of R3 and are not claimed as tested. A NULL cstring is not a printable C string,
so callers must check it as the empty state (existing cstring behavior).

A zero-state `String` widened to `String?` compares equal to `null`, because
both use the all-zero bit pattern; `clone(h.name)` returns a fresh, non-null
empty String because it allocates. That identity is the existing O7/TY-8
behavior and this plan does not change it; the runtime accessors only make the
read safe (AR-3).

## 2. Discarded Template and For-Step Cleanup

### 2.1 Rule

An expression statement whose result owns storage must release it at the end of
the statement. A `for` step is evaluated once per iteration, so its cleanup
must be emitted inside the step block on both the fall-through and the
`continue` edge (AR-4, AR-9, AR-18).

### 2.2 Discarded Template Binding

`compiler/src/own/lower.rs:1984` (`lower_discarded`) currently binds only a
direct `Call`/`ClosureCall` result. Extend the condition to include a
`Template` value:

```rust
if matches!(
    &value.kind,
    ValueKind::Call { .. } | ValueKind::ClosureCall { .. } | ValueKind::Template(_)
) && self.discard_binds_owner(&value.ty)
{
    self.bind_discarded_owner(value);
    return;
}
```

`bind_discarded_owner` allocates a statement temporary, registers a pending
release, and stores the value. A placeholder-free template is a `str` and owns
nothing, so `discard_binds_owner` is false and the existing path is unchanged.

### 2.3 For-Step Release Floor

`compiler/src/own/loops.rs` checks each step expression at `:332` (the
speculative pass) and `:394` (the reporting pass). Both loops get the same
release-floor discipline as `check_stmt`
(`compiler/src/own/statements.rs:159`):

```rust
for step in &decl.step {
    let release_floor = self.pending_releases.len();
    self.check_expr_statement(step);
    self.drain_pending_releases(release_floor);
}
```

A drained release emits a `Statement::ReleaseTemp` in the step block. The
block already runs on the fall-through and every `continue` edge, so each
iteration releases its own result. A step that carries a template or a call is
already a non-assignment statement, so the loop already uses the flat
label/goto form; the drain does not change that shape. The step is not on a
scored hot path.

## 3. Owning Aggregate Literal Order

### 3.1 Rule

Struct literal fields evaluate in source order
(`spec/04-expressions.md` §3). When a later field contains a template, the
template's copy statement is emitted while that field is lowered, before the
outer C initializer. Every earlier field must therefore be pinned at its own
position, including an owning aggregate. A pinned owning aggregate needs a
store that does not release a previous temporary value, because the outer
literal adopts it (AR-5, AR-6, AR-19).

### 3.2 Pin Change

`compiler/src/own/lower.rs:2830` (`pin_literal_operand`) currently:

| Operand | Current | New |
|---------|---------|-----|
| Duplicable or update | `pin_update_value` | unchanged |
| Fixed array | `materialize_value` | unchanged |
| Owning `Call`/`ClosureCall` | `materialize_adopted_operand` | unchanged |
| Owning `StructLit` (non-fixed) | inline | `materialize_adopted_operand` |
| `ArrayLit` | inline | unchanged (fixed arrays use the store path) |
| Everything else | `materialize_value` | unchanged |

The aggregate ordering only matters when the literal is pinned at all, which
happens when any field/element contains a template or an update
(`contains_template_placeholder`, `contains_update`).

### 3.3 Adopted Aggregate Store

`compiler/src/codegen/function.rs:701` (`Statement::Adopt`) currently renders
`target = value;`. Add a `StructLit` branch before the plain store:

```rust
Statement::Adopt { target, value } => {
    self.emit_line_directive(span);
    let ty = self.substitute(&target.ty);
    if let ValueKind::StructLit { name, fields } = &value.kind {
        if self.struct_literal_needs_stores(name, fields) {
            let target_c = self.place(target);
            self.push(&format!("    cl_zero(&{target_c});"));
            self.emit_struct_literal_fields(&target_c, &ty, fields);
            return;
        }
    }
    self.emit_guards(value);
    let target_c = self.place(target);
    let value_c = self.value(value);
    self.push(&format!("    {target_c} = {value_c};"));
}
```

- The temporary is zeroed before the fieldwise fill, because the fieldwise core
  pre-drops a fixed-array field's elements and an owned conditional's saved
  value. The temporary is a fresh alias whose previous values the outer literal
  already adopted, so zeroing loses nothing and makes each pre-drop a no-op on
  loop re-entry. This is the no-drop difference from
  `emit_struct_literal_store` (`compiler/src/codegen/function.rs:3041`), which
  drops the target before filling it.
- `emit_struct_literal_fields` (`:3078`) already handles a fixed-array field
  through the element-wise core, an owned conditional through the move-assign
  path, a nested literal through recursion, and every other field through a
  plain assignment with the source move.
- A `StructLit` that needs no stores renders as a valid C compound literal and
  keeps the plain assignment.
- An `ArrayLit` is never routed here: a fixed array keeps the existing
  `materialize_value` store path, and the pin leaves a non-fixed array literal
  inline.

The outer literal then reads the temporary as a place. The struct assignment
copies the adopted handles into the field; the temporary is not released. On a
loop re-entry, the temporary is zeroed and refilled without a live pre-drop, and
the previous iteration's outer literal has already dropped its own field, so no
double free is possible (AR-24).

### 3.4 Examples

```clorraine
struct Inner { a: String; }
struct P { inner: Inner; b: String; }

// mark(1) in mkString runs before mark(2) in the later placeholder: seq=12.
let p: P = P { inner: Inner { a: mkString() }, b: $"{mark(2)}" };

// A fixed-array field inside the aggregate takes the fieldwise store:
struct InnerA { a: String[2]; }
struct PA { inner: InnerA; b: String; }
let q: PA = PA { inner: InnerA { a: mkPair() }, b: $"{mark(2)}" };
```

## 4. Ownership Invariants

1. The zero-state String is an empty value; no accessor dereferences NULL.
2. A discarded owned template result has exactly one release per statement or
   per loop iteration.
3. An adopted aggregate's temporary is a zeroed alias before each fill; it is
   never released, and each fieldwise pre-drop sees the empty state.
4. The outer literal's field owns the adopted handles and releases them once
   through its normal scope/store cleanup.
5. The #26 ordered-lowering invariants (one copy per string-like placeholder,
   adopting part, module retained copy) are unchanged.

## 5. What Is Not Changed

- The `String` helper signatures and the `ClStrPart` contract.
- The #26 `Statement::Adopt` call path and its guard emission.
- The `for` header spelling for a step without a pending release.
- `cl_string_append`, `FreeString`, and the string block layout.
- `spec/`, `stdlib/`, and the emitted C ABI.

## 6. Affected Code

| File | Function | Change |
|------|----------|--------|
| `compiler/runtime/shim_host.c` | `cl_string_ptr`, `cl_string_len`, `cl_string_copy` | Null-safe zero state |
| `compiler/runtime/shim_hosted_amiga.c` | the same three | Null-safe zero state |
| `compiler/src/own/lower.rs` | `lower_discarded` | Bind an owned template result |
| `compiler/src/own/lower.rs` | `pin_literal_operand` | Adopt an owning non-fixed `StructLit` |
| `compiler/src/own/loops.rs` | `check_for_c` step loops | Release floor and drain per expression |
| `compiler/src/codegen/function.rs` | `emit_statement`, `Adopt` | No-pre-drop structured store |

## 7. Failure Handling

- If a zero-state path still crashes through a helper not covered by the three
  accessors, the preflight reopen trigger fires and the plan adds that helper
  or narrows the claim (AR-3).
- If the adopted aggregate emits invalid C for a supported literal shape, the
  pin falls back to the inline form for that shape and records it, but only
  with a user ruling because the order residual remains (AR-5).
- A benchmark regression above 1% reopens AR-22: first try one inlined NULL
  test in the generated `cl_string_view` while keeping all three accessors
  safe; if the metric is still above 1%, stop and seek a ruling. Removing an
  accessor check is not an acceptable trade-off because `len` and `clone` call
  the accessors directly.
