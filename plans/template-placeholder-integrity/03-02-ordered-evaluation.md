# Ordered Placeholder Evaluation

> **Document**: 03-02-ordered-evaluation.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R2–R6, R8 (see [01-requirements.md](01-requirements.md))

## 1. Contract

A template is strictly left-to-right (`spec/04-expressions.md` §3). Every
placeholder expression is evaluated at its own source position, before any
later placeholder's effects can change or release its source. A
`String`/`str` placeholder's bytes are copied at that position, so a later
`take` sibling cannot invalidate them. A direct call, intrinsic, projection
over a fresh root, view conditional, and owning conditional are each
evaluated once.

The plan realizes this contract in the two lowering paths:

- the function-body path (`compiler/src/own/lower.rs`), and
- the module-initializer path (`compiler/src/codegen/program.rs`).

## 2. Placeholder Classes

| Class | Types | Handling |
|-------|-------|----------|
| String-like | `str`, `static str`, `String`, `String?`, `str?`, a type parameter, an opaque/unknown type | Copy to an owned `String` at the placeholder position |
| Scalar | integer types, `f32`, `f64`, `bool` | Keep the direct part; add a temporary only when ordering requires it |
| Other | Anything sema accepts through a generic path but does not classify | Keep the existing codegen fallback |

A type parameter is string-like because its instantiation may be `String` or
`str`. The copy uses the concrete type's converter at emission time, so the
same generic body serves every instantiation.

## 3. Body Lowering

`compiler/src/own/lower.rs:754` currently maps each placeholder with
`materialize_owned_argument(value, false)` and embeds the result. Replace that
mapping with the following ordered walk.

### 3.1 Pre-scan

Before lowering any piece, compute:

```text
effectful[i] = expression_has_effects(placeholder_i)   // false for literals
last_effect  = the largest i with effectful[i], or None
```

`expression_has_effects` is the existing AST predicate at
`compiler/src/own/call.rs:1463`. It treats a call, an intrinsic call, an
assignment, and an update as effects, recursively. The function becomes
`pub(super)` so the sibling lowering module can use it. It is conservative:
materializing an extra pure operand is always safe; missing an effect is not.

### 3.2 Per Placeholder

For placeholder `i` with source expression `inner`:

1. `let value = self.lower_value(inner);`
2. `let value = self.materialize_owned_argument(value, false);`
   This preserves the existing binding of an owned call result, so the scope
   cleanup releases the source exactly once. Then
   `let value = self.bind_member_receiver(value);` binds a fresh projection
   root (`mkBox().name`, `spawn().v`) so the view or field reads through a
   stable temporary instead of re-evaluating the root call. Both helpers
   respect the deferred position (`bind_member_receiver` has its own guarded
   path).
3. If `self.speculative_lowering`, keep `value` unchanged. A speculative pass
   has no statement position and its result is not emitted.
4. If the placeholder is **string-like**:
   1. If the value is a **direct fresh evaluation** (`Call`, `ClosureCall`,
      `Intrinsic`) or a **view-typed impure value**, evaluate the source at its
      position (`materialize_at_position`, §3.3/§3.4). This evaluates a `str`
      call, a view conditional, and a view projection over a fresh call once;
      a `String` call was already bound in step 2 and an owning projection was
      bound in step 2.
   2. `let copy = internal_text_copy(value);` — a `ValueKind::Call` with the
      reserved callee `$text`, one argument, no type arguments, no receiver,
      and type `String` (§4).
   3. `let copy = self.materialize_copy_at_position(copy);` (§3.3).
   4. Use `ValuePiece::Placeholder { value: copy, owned: true }` as the
      placeholder value. The `owned` flag is the provenance mark that lets
      the emitter adopt the copy instead of copying it again (§5).
5. Else if the placeholder is **scalar**:
   1. If `effectful[i] || last_effect.is_some_and(|last| i < last)`,
      materialize the scalar at its position (`materialize_scalar_at_position`,
      §3.3).
   2. Use the resulting value.
6. Else keep `value` unchanged (the existing codegen fallback).

A pure scalar after the last effect stays inline: nothing later in the
template can reorder it. This keeps `benchmarks/strings/program.clo:13`
(`$"token-{i}-{i * u32(3)}"`) byte-identical in shape (AR-9).

### 3.3 Materialization Helpers

The lowering already has the two required positions; the plan adds named
wrappers so the policy is explicit and testable.

| Helper | Eager position | Deferred position |
|--------|----------------|-------------------|
| `materialize_at_position` | `materialize_value` (`lower.rs:3438`) | `materialize_guard_operand` (`lower.rs:2646`) |
| `materialize_scalar_at_position` | `materialize_value` | A forced `Materialize` temporary with no release (the scalar owns nothing) |
| `materialize_adopted_copy` | A `Statement::Adopt` at the source position | A forced `Materialize` with `adopting: true` and no release |

"Eager" means **only** `self.defer_move_outs == 0`: the current block is a
statement position (a statement, a loop-condition block, or a lambda/defer
body). Every `?:` arm and every `&&`/`||` right operand sets
`defer_move_outs > 0`, so it uses the deferred fills even when the enclosing
conditional is itself eager. `eager_conditional_arm` is **not** a statement
position: it only tells an arm's field store that its fields are emitted
eagerly, and a `Statement::Assign` emitted while lowering the arm would land
above the `?:` and run on the skipped path. The deferred fill places the copy
inside the chosen branch.

The forced deferred temporary is:

```rust
let ty = self.materialized_ty(&value);
let temp = self.new_temp(ty.clone());
Value::new(
    ty,
    ValueKind::Materialize {
        temp: Some(temp),
        destination: None,
        value: Box::new(value),
        move_outs: Vec::new(),
        value_move_outs: Vec::new(),
        adopting: true, // copy only
    },
)
```

An eager copy uses `Statement::Adopt`: the store `target = value;` does **not**
release the previous target value, because the part builder adopts the copy.
A plain `Statement::Assign` would pre-drop the previous copy on a loop
re-entry and free a value the builder already freed; the eager
`Materialize` fill would run after a later sibling's hoisted mutation and
copy the wrong bytes. The `Adopt` store is emitted at the piece's source
position, so both problems are avoided.

It deliberately does **not** call `mark_destination_temp` and does **not**
register a pending release: the copy is adopted by the part, exactly like the
existing adopting carrier in `consume_place_into`
(`compiler/src/own/lower.rs:3496`). A re-executed guard must not drop the
previous value: the builder already freed it. The guard emitter already skips
the re-fill drop for `adopting` carriers (`compiler/src/codegen/guards.rs`).

### 3.4 Source Materialization Predicate

Only a shape whose evaluation is a **fresh result or a view over an impure
root** is materialized in front of the copy:

```text
match &value.kind {
    Call { .. } | ClosureCall { .. } | Intrinsic { .. } => true,
    _ if is_view_like(&value.ty) && !is_pure_value(&value) => true,
    _ => false,
}
```

- A direct `str` call is evaluated once; a `String` call was bound by step 2
  (a `Place`) and is not a `Call` any more.
- A view projection over a fresh call (`get().name` where `name` is `static
  str`/`str`) is impure; materializing the **member value** into a `str`
  temporary evaluates the root once. The temporary owns nothing, so its drop
  is a no-op. This fixes the current double evaluation of `get()`.
- A projection over a place is pure and stays a read; an owning member
  (`mkH().name` with a `String` field) is bound by `bind_member_receiver` and
  reads once.
- A view-typed conditional owns nothing; materializing it is safe even when
  one arm borrows a place. This fixes `$"{(c ? v : mk())}"`.
- A raw dereference of a pure pointer is a borrowed read and stays inline; an
  impure view dereference is materialized as a view temporary (no ownership).
- A `String`-typed conditional or a `String` projection over a conditional is
  owning, not a view, so it is not materialized by this predicate.
  `materialize_owned_argument` (and `bind_member_receiver` for a projection)
  binds it as an owned value; the base behavior already evaluates its call
  once, and the copy path reads the bound value. It never registers a second
  release for a borrowed arm.

The old failure mode is avoided: `is_pure_value` returns `false` for a
`Materialize` (`compiler/src/own/lower.rs:7129`), but the predicate above
requires a **view type** for the impure-projection case and never wraps an
owning `Materialize`, an owning conditional, or an owned projection. A view
temporary's release is a no-op, so a second registration cannot double-free a
String.

### 3.5 Literals

A template nested in a struct or array literal can emit a copy statement at
its own field's position, before the C literal is built. If an earlier field
or element has effects, it must run first. The literal lowering pins a
duplicable operand (the existing update pin), an owning **call** (with an
adopting `Statement::Adopt` so a loop re-entry does not free the field's
handle), and a fixed array (the existing element-wise store). An owning
**aggregate** field (`StructLit`/`ArrayLit`) stays inline: its structured
store has no no-pre-drop form, and pinning it would emit an invalid aggregate
assignment. The scalar/call order is pinned by ST-39/ST-40; the owning
aggregate order is the recorded residual (AR-21). The rule uses the
`contains_template_placeholder` AST predicate.

## 4. Internal Copy Call

### 4.1 IR Shape

```rust
Value::new(
    Ty::handle("String"),
    ValueKind::Call {
        callee: "$text".to_owned(),
        type_args: Vec::new(),
        args: vec![source],
        receiver: None,
    },
)
```

`$text` is an internal sentinel, not a user-callable name. A Clorraine
identifier cannot contain `$`, so no user function can collide.

### 4.2 Emission

`Emitter::render_call` (`compiler/src/codegen/expr.rs:1160`) gains an early
arm:

```rust
if callee == "$text" {
    return self.text_call(args.first());
}
```

`text_call` (`compiler/src/codegen/expr.rs:1556`) already dispatches on the
argument's substituted type:

| Source type | Generated copy |
|-------------|----------------|
| `String` | `cl_string_copy(value)` |
| `str`, `static str` | `cl_text_str((value).ptr, (value).len)` |
| `i8`/`i16`/`i32`/`isize` | `cl_text_i32((i32)(value))` |
| `u8`/`u16`/`u32`/`usize` | `cl_text_u32((u32)(value))` |
| `bool` | `cl_text_bool((int)(value))` |
| `f32`/`f64` | `cl_text_f64((double)(value))` |

The rendered C never contains `$text`. The existing shim converters and their
profiles are reused; no runtime edit is required.

## 5. Template Emission

`ValuePiece::Placeholder` gains an `owned` provenance flag, so the emitter can
distinguish a copy made at the source position from a raw value that must
still be copied defensively:

```rust
pub enum ValuePiece {
    Literal(String),
    Placeholder { value: Value, owned: bool },
}
```

Every `ValuePiece` walk is updated to the struct pattern: generic substitution,
the guard walks, `collect_materialized_temps`, the discard/needs-statement
walks, the raw-read and place walks, the own-stage instance/loop/arm walks, and
the two template builders. The flag is preserved by the generic substitution
walk and by the module-initializer walk. A producer that does not run the
ordered lowering (for example the discarded-effects template at
`compiler/src/own/lower.rs:2257`) leaves `owned: false`.

`compiler/src/codegen/expr.rs:1620` (`template`) then renders:

```text
TyKind::Handle("String") if owned   => cl_str_part_owned(<value>)
TyKind::Handle("String")            => cl_str_part_owned(cl_string_copy(<value>))
TyKind::Str | TyKind::StaticStr     => cl_str_part_owned(cl_text_str(...))
```

The `owned` arm adopts the one copy the ordering stage made. The raw String
arm keeps the old defensive copy, so a producer that bypasses the ordering
stage is still memory-safe. The `str` arm is likewise the defensive path; the
supported lowering always supplies an owned String. The scalar and `_`
fallback arms are unchanged.

The part count and literal byte parts are unchanged. A placeholder-free
template still renders `cl_str(...)` and allocates nothing.

## 6. Guarded Positions

The deferred path reuses the existing guarded-fill machinery:

- The source (when impure) becomes `materialize_guard_operand`, which the
  guard emitter renders as a comma fill at the arm/operand's evaluation point.
- The copy becomes an adopting `Materialize`. `guard_expressions` walks a
  `Template`'s pieces in order and emits each placeholder's fills before the
  concat, so an earlier copy fill precedes a later `take` fill.
- The emitted expression keeps the short-circuit or arm guard. A skipped path
  evaluates neither the copy nor the source.
- A loop condition evaluates a source through `materialize_owned_argument`
  (which already handles per-iteration ownership) and the copy through
  `materialize_value`; a short-circuit operand of the same test uses the
  deferred fills instead. The copy temporary is not released: that
  iteration's concat adopts it. The next iteration overwrites a dangling
  pointer without dropping it. The established
  `materialize_loop_condition_operand` helper is not needed for the copy
  because the copy owns no scope storage.

`?:` arms and short-circuit operands always use the deferred fills above,
even when the enclosing conditional is itself eager. `eager_conditional_arm`
is not a statement position: a statement emitted while lowering the arm lands
above the `?:` and runs on the skipped path. The deferred fill runs only in
the chosen branch, and an untaken arm evaluates no placeholder (ST-25).

## 7. Module Initializers

`compiler/src/codegen/program.rs` lowers initializers separately, so it gets
its own ordered walk in `materialize_init_value`'s Template arm
(`:3381`). The path has no shared `String` owner: a module `let` must be
duplicable, so a `String` cannot live in a global and the copy-before-move
cases are unreachable. Only once-evaluation and copy order within the
initializer are required.

The Template arm already holds the lowered `ValuePiece::Placeholder` values
before it walks them, so the effect flags come from `init_operand_is_pure`
over those raw values, exactly as the struct-literal arm computes
`later_effect` at `compiler/src/codegen/program.rs:3307-3313`. No AST access
and no cross-module predicate are needed here; `expression_has_effects` stays
a sibling-only helper for the body lowering.

For placeholder `i`, with `effectful[i] = !init_operand_is_pure(&raw_value)`
and `last_effect` computed before the walk:

1. `let walked = self.materialize_init_value(inner, guard);`
2. String-like:
   1. If `!init_operand_is_pure(&walked)`, store the source into an init local
      with `store_impure_init_operand`
      (`compiler/src/codegen/program.rs:3410`). This stores a `str` call once;
      a `String` call is bound here too. **Save `init_retain_owners` and set
      it to `false` while storing the source**, because the source is not the
      view's backing: an owned String source must be released at the end of
      the init function. Restore the flag afterwards. (The base leaves an
      owned source call unbound and leaks it; this plan fixes that as part of
      the ordered walk.)
   2. Build the `$text` copy call.
   3. Save `init_retain_owners`, set it to `true`, call
      `materialize_init_local` with the empty String state, restore the flag.
      The copy local is not added to `init_owners`; the part adopts the copy.
      Note that `materialize_init_local` still records the local in
      `init_call_locals` (`compiler/src/codegen/program.rs:2300-2305`); that
      set is only consulted for projections and move-outs of owning roots, so
      a whole-value String read is safe. The impl test pins that the init
      function contains no `FreeString` of the copy local.
   4. Use `Placeholder { value: copy_local, owned: true }`.
3. Scalar:
   1. If `effectful[i] || i < last_effect`, store the value into an init local
      (`store_impure_init_operand` for an impure value, or
      `materialize_init_local` for a pure one). A scalar owns nothing, so the
      local is not an owner.
   2. Use `Placeholder { value, owned: false }`.
4. Other: `Placeholder { value: walked, owned: false }`.

The template result binding and `init_retain_owners` behavior at
`compiler/src/codegen/program.rs:3398` are unchanged.

## 8. Ownership Invariants

1. **One copy per string-like placeholder.** The copy is created by `$text`
   and adopted by the part. No second `cl_string_copy` wraps it.
2. **One release per owned source.** `materialize_owned_argument` binds an
   owned call result as before; the scope cleanup frees it. The copy has no
   scope release. The part builder frees the copy after appending.
3. **No release of a copy temporary.** Eager copies use `Statement::Adopt`
   (the target owns nothing and the previous value is not released); deferred
   copies use `adopting: true`; init copies use `init_retain_owners = true`.
4. **No double free on a re-executed guard.** An adopting fill skips the
   previous-value drop; the builder already freed the previous copy.
5. **No leak on a skipped path.** A deferred fill runs only where the
   operand is evaluated; a path that skips it creates no copy.
6. **No change to the null rule.** A nullable placeholder is rejected before
   lowering (03-01), so `cl_string_copy` and `cl_text_str` only see non-null
   values at runtime.

## 9. Generic Instantiation

The internal copy call's argument keeps the generic spelling; its result type
is `String`. At emission, `text_call` substitutes the concrete type:

- `String` and `str` instantiate to the copy converters;
- scalars instantiate to the text converters and allocate one `String`.

The part adopts the resulting `String` in every case. The generic scalar
allocation is the accepted cost recorded in AR-18; no concrete scalar template
pays it. A generic String placeholder with a sibling `take` therefore copies
before the move (ST-13). A nullable generic instance is rejected or accepted
by the instance-level duty in [03-01 §3a](03-01-nullable-placeholders.md)
before this copy runs.

## 10. What Is Not Changed

- `ValueKind` gains no variant; the hidden call reuses `ValueKind::Call`.
  `ValuePiece::Placeholder` gains the `owned` provenance flag.
- The runtime shims gain no functions.
- The `ClStrPart` layout, kind codes, and builder contract are unchanged.
- The scalar part spelling, the float `%g` temporary, and the explicit
  `text(...)` conversion path are unchanged.
- The module-initializer nullable check and result binding are unchanged.
- `spec/`, `stdlib/`, and the example programs are unchanged.

## 11. Affected Code

| File | Function | Change |
|------|----------|--------|
| `compiler/src/ir.rs` | `ValuePiece`, `Statement::Adopt`, re-export | Add the `owned` flag and the adopting store |
| `compiler/src/ir/mono.rs` | `RequiresTemplatePlaceholder` | Per-instance formattability/nullability duty |
| `compiler/src/own/expr.rs` | `eval`, `Template` arm | Add the concrete nullable check, record the generic duty, and compute its display name |
| `compiler/src/own/instance_check.rs` | new check | Report E10500/E10010 for an invalid instance, deduped per site |
| `compiler/src/own.rs` | duty field, dedupe set, transfer | Carry the new duty to `Monomorphization` |
| `compiler/src/sema/expr.rs` | `formattable` | Expose the predicate; reject an opaque placeholder |
| `compiler/src/sema/mod.rs` | re-export | Expose `formattable` to the instance check |
| `compiler/src/own/lower.rs` | `lower_value`, `Template` arm | Ordered placeholder walk; literal operand pins |
| `compiler/src/own/lower.rs` | new helpers | Materialize at position / scalar / adopted copy; literal template predicate |
| `compiler/src/own/call.rs` | `expression_has_effects` | Visibility only (`pub(super)`) |
| `compiler/src/codegen/expr.rs` | `render_call` | `$text` early arm |
| `compiler/src/codegen/expr.rs` | `template` | Adopt only the `owned` part; copy the raw fallbacks |
| `compiler/src/codegen/function.rs` | `emit_statement` | `Statement::Adopt` store with guards and no pre-drop |
| `compiler/src/codegen/control.rs` | `for_step_text` | No adopting store is structured; the `_` fallback applies |
| `compiler/src/codegen/program.rs` | `materialize_init_value`, Template arm | Ordered initializer walk |
| `compiler/src/own/{generics,instance_check,loops}.rs` | statement walks | Carry `Adopt` like an assigned value |
| All `ValuePiece` walk sites (29 matches) | various | Carry the `owned` flag unchanged |

## 12. Failure Handling

- A lowering path that cannot name a concrete type (an unresolved generic)
  still produces the internal copy with type `String`; the instantiation
  supplies the converter. If an instantiation never resolves, the existing
  sema/instance checks reject the program before emission.
- A `ValuePiece` producer that bypasses the ordered lowering leaves
  `owned: false`; the emitter copies defensively, preserving the old
  behavior. The only such producer is the discarded-effects template, which
  is not emitted. A future producer that emits a template must run the
  ordered lowering and set `owned: true`; a late copy of a moved source is
  never accepted as a substitute.
- If the guard emitter cannot order a nested fill for a guarded case, the
  affected task is marked blocked and the plan's AR-11 reopen trigger fires;
  the fallback is to document the guarded residual and file a focused issue,
  but only with a user ruling because it leaves a known unsafe path.
- A benchmark regression above 1% reopens AR-9 and AR-16: first try the
  effect-position rule on more cases; if that is insufficient, stop and
  present the measurement before widening the change.
