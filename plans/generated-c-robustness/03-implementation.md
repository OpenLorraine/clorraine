# Implementation Design: Generated-C Robustness Bundle

> **Document**: 03-implementation.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R1–R6 (see [01-requirements.md](01-requirements.md))

## 1. Module-Path Precedence (#31)

### 1.1 Rule

MOD-13: `A.B.f()` is always available. MOD-16: only a **local** may shadow a
module name. A module-level or imported value named `std` therefore does not
hide `std.fmt.text`; the intrinsic (or a real declaration) fires. A lexical
`std` binding keeps the member path. The rule applies in function bodies and in
module initializers.

### 1.2 Sema

In `is_std_fmt_text_path` (`compiler/src/sema/call.rs`), remove
`!self.name_is_value("std")`. The caller already runs only when the head is not
a lexical binding (`!self.is_local(&name.segments[0])`). Keep the real
declaration gates (`qualified_decl_kind`, `qualified_ffi_signature`,
`table.global`).

### 1.3 Ownership

In `is_builtin_text_call` (`compiler/src/own/expr.rs`), remove
`|| self.value_head_shadows("std")`. `is_dot_call` already returns true for a
lexical head and false when `module_prefix_len` finds a loaded module. Remove
`value_head_shadows` (its only caller) and update the doc comments in
`is_std_fmt_text_path`, `is_builtin_text_call`, and `init_text_intrinsic_name`:
only a lexical binding shadows the module path; a module-level/imported value
does not.

### 1.4 Initializer

The initializer must decide the intrinsic **before** the callable fallback.
In `lower_global`'s `ExprKind::Call` arm, after the type-cast/intrinsic/lambda
checks and before `init_callable_value`:

```rust
if let Some(text_name) = self.init_text_intrinsic_name(module, callee) {
    let values: Vec<Value> = args
        .iter()
        .map(|arg| self.lower_global(module, &arg.value))
        .collect();
    return Value::new(
        ty,
        ValueKind::Call {
            callee: text_name,
            type_args: Vec::new(),
            args: values,
            receiver: None,
        },
    );
}
let callable = self.init_callable_value(module, callee);
```

Remove the later `let name = self.init_text_intrinsic_name(...)` line, which
becomes unreachable. Remove the module-level `std` value-head gate from
`init_text_intrinsic_name` (the initializer has no lexical bindings).

`declares_text` must return true for **any** declaration of the spelling — a
function, global, type, or `ffi` signature — mirroring sema/own, or the early
bypass would displace a real `std.fmt` declaration. The global case is pinned
by ST-15 and IT-8; a non-callable declaration is pinned by IT-10.

A real `std.fmt.text` `ffi` needs its own initializer branch before the
value-head callable fallback: `init_text_ffi_signature` resolves the exact
path or imported alias, and the call keeps the **qualified** spelling so the
emitter resolves the ffi, not a same-named function of the initializing
module. IT-9/IT-11 pin it.

A non-callable `std.fmt.text` declaration (struct/handle/enum) is rejected
with E10309 in the type stage for the qualified, bare, and imported/aliased
spellings, so no initializer emits a literal `std.fmt.text(...)`. IT-10 and
IT-12 pin the rejection.

## 2. Owned Text Argument (#30)

### 2.1 Rule

`text` reads a `String` and copies it. A named place stays a copy and remains
readable. An owned body-level temporary is materialized once and released by
the function-scope cleanup after the call. The initializer walk already binds
and releases its argument (verified with ASan), so it is unchanged.

### 2.2 Lowering

In `lower_call` (`compiler/src/own/call.rs:336`), when
`self.is_builtin_text_call(callee)`, give the one argument a synthetic
`Mode::Borrow`:

```rust
let modes: Option<Vec<Mode>> = if self.is_builtin_text_call(callee) {
    Some(vec![Mode::Borrow; args.len()])
} else if let Some(callable) = &receiver {
    ...
};
```

The existing `materialize_owned_argument(value, false)` path then:

- leaves a place-rooted argument (`let s = ...; text(s)`) as a copy;
- binds a fresh owning call/template result to a generated local and
  registers its function-scope release.

No new IR node or codegen helper is needed.

## 3. Init-Local Reservation (#16)

`next_init_local` (`compiler/src/codegen/program.rs:3673`) scans
`is_taken_symbol`, exactly as `__cl_ctmpl` does:

```rust
fn next_init_local(&mut self) -> String {
    let mut name = format!("__cl_itmp{}", self.init_locals);
    while self.is_taken_symbol(&name) {
        self.init_locals += 1;
        name = format!("__cl_itmp{}", self.init_locals);
    }
    self.init_locals += 1;
    name
}
```

An ordinary program with no `__cl_itmp` user symbol keeps the first name.

## 4. #29 Pin Supersession (AR-9)

`compiler/tests/template_placeholder_residuals_iii_impl_test.rs` IT-13 and the
`ST-15` comment in `template_placeholder_residuals_iii_spec_test.rs` currently
assert that a module-level `std` keeps the member path and no `cl_text_*`.
#31 reverses that: the module path wins. Update IT-13 to assert
`cl_text_i32((i32)(7))` and no `std.fmt.text(7)` for the module-level head, and
rename it accordingly. Update the #29 ST-15 comment (the lexical case is
unchanged) and add a note to #29 `05-findings.md` §5 item 2 and the #29 R2
wording that #31 supersedes the module-level clause.

## 5. Invariants

1. A module-level/imported `std` value does not hide `std.fmt.text` in a body
   or an initializer; a local `std` does.
2. A real `std.fmt.text` declaration wins over the intrinsic.
3. A named-place `text` argument is not moved or released by the call.
4. An owned body-level `text` temporary is released exactly once.
5. A generated init local never collides with a user symbol.
6. The #24/#28/#29 contracts stay green, except the intended IT-13 update.
