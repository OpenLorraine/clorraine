# Current State: Borrow-View Moves and Shadowed Narrowing

> **Document**: 02-current-state.md
> **Parent**: [Index](00-index.md)

## Existing Implementation

### What Exists

The ownership stage already rejects a move out of a `borrow` parameter with
`E10034` in `move_place` (`own/place.rs:182-196`), the shared decision every
ownership transfer reaches. Loop element bindings are created by
`bind_for_in_element` (`own/loops.rs:302-367`) as `BindingKind::Local` with
`Mode::Borrow` and a `DerefTemp` base that aliases the iterable element, so
**none** of the existing parameter checks see them. The declaration path
(`check_let` at `own/statements.rs:178-215`, `check_const` at `:262-287`)
deliberately resolves the store and the ownership effect before the binding is
declared, but runs `check_value_into` after it; `check_value_into` →
`check_nonnull_expr` → `plain_binding` then resolves the initializer's name to
the just-declared shadowing binding, whose state carries no narrowing.

### Relevant Files

| File | Purpose | Changes Needed |
| ---- | ------- | -------------- |
| `compiler/src/own/place.rs` | `move_place`: the shared move decision | Reject a transfer whose binding is a borrow element view |
| `compiler/src/own/loops.rs` | `bind_for_in_element` | None; the binding's shape is the detection evidence |
| `compiler/src/own/statements.rs` | `check_let`, `check_const` | Run the initializer's null check before the declaration |
| `compiler/src/own/expr.rs` | `check_value_into`, `check_nonnull_expr` | None; the order changes at the call site |
| `compiler/tests/borrow_view_and_shadowing_spec_test.rs` | New specification suite | Create |
| `compiler/tests/borrow_view_and_shadowing_impl_test.rs` | New implementation suite | Create |
| `compiler/src/own/tests.rs` | Ownership-stage unit tests | Add view-rejection coverage |

### Code Analysis

The borrow element binding (`own/loops.rs:311-338`):

```rust
let id = self.declare(
    &decl.name,
    decl.name_span,
    element.clone(),
    BindingKind::Local,
    if decl.borrow { Mode::Borrow } else { Mode::None },
    false,
    false,
);
...
if decl.borrow {
    self.bindings[id].base = Some(ir::PlaceBase::DerefTemp {
        temp: address_temp,
        ty: element.clone(),
    });
```

The move rule that misses it (`own/place.rs:182-196`):

```rust
pub(super) fn move_place(&mut self, id: usize, projections: &[ir::Projection], span: Span) {
    self.bindings[id].used = true;
    let binding = &self.bindings[id];
    if binding.kind == BindingKind::Param
        && matches!(binding.mode, Mode::None | Mode::Const | Mode::Borrow)
    {
        // E10034: cannot release or store borrowed parameter
    }
```

The declaration order (`own/statements.rs:201-208`):

```rust
let id = self.declare_local(&decl.name, target.clone(), false, decl.name_span);
if let (Some(value), Some(prepared), Some(effect)) = (&decl.value, prepared, effect) {
    ...
    self.set_place(id, init);
    self.check_value_into(value, &target, decl.span);   // runs after the shadow
```

## Gaps Identified

### Gap 1: Moving through a borrow element view is accepted (DEF-18)

**Current Behavior:** For `xs: String[2]`, `for (borrow e in xs) { eat(e); }`
(take argument), `let s: String = e;` (owned binding), `let s: String = p.name;`
(field path), `Holder { s: e }` (aggregate field), and `return e;` (return) are
all accepted; the emitted code moves the element and nulls the array slot. The
probe on 2026-09-29 reproduced all of these.

**Required Behavior:** Each is an element move limited in v1 and reports
`E10026` with the registry's message and help (`spec/03` §3–4, `spec/08` §5).

**Fix Required:** One guard in `move_place` for a `BindingKind::Local` binding
with `Mode::Borrow`, before the state changes.

### Gap 2: A narrowed shadowing initializer reports a spurious E10010 (DEF-20)

**Current Behavior:** `let b: Buffer? = bufferNew(8); if (b != null) { let b:
Buffer = b; ... }` reports `value 'b' may be null here` at the inner initializer.
The `const` form reports the same.

**Required Behavior:** Both compile; the initializer resolves to the outer
binding, whose `!= null` narrowing holds. A genuine nullable use without a
narrowing check still reports `E10010`.

**Fix Required:** Call `check_value_into` on the initializer before
`declare_local` in `check_let` and `check_const`, matching the pre-declaration
resolution the store and effect preparation already use.

## Dependencies

### Internal Dependencies

- `move_place` as the single choke point for every move form
  (`check_argument_place` take mode, `prepare_store`, `check_return`, and the
  aggregate consume point).
- `check_nonnull_expr` / `plain_binding` as the null-narrowing check.
- The existing `E10010` and `E10026` registry rows (`spec/14`).

### External Dependencies

- None.

## Risks and Concerns

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| The new rejection also catches a legal transfer (e.g. a view of a view, or a copy-form binding) | Low | High | The guard is limited to `BindingKind::Local` + `Mode::Borrow`; guard spec cases pin the copy form, reads, lends, and writes |
| Moving `check_value_into` earlier changes another diagnostic | Low | Medium | It only checks nullability against the expected type and emits no IR; the full workspace check covers the corpus |
| The `E10026` help text is generic for a view move | Medium | Low | Accepted (AR-7): the registry row is canonical and frozen |
