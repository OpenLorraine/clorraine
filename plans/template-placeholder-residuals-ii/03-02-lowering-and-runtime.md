# Lowering and Runtime: loops, aggregate order, panic

> **Document**: 03-02-lowering-and-runtime.md
> **Parent**: [Index](00-index.md)
> **Governing requirements**: R6–R9, R11 (see [01-requirements.md](01-requirements.md))

## 1. For-Step Owning Assignment

### 1.1 Rule

A `for` step runs after the body and after every `continue`. An assignment to
an owning target must release the old value before the store (O3), exactly as
an ordinary statement does. The compact C `for` header can carry only an
assignment expression, so a step whose target may own keeps the flat
label/`goto` loop and the ordinary store path.

### 1.2 Implementation

`compiler/src/own/loops.rs:999` (`for_step_is_structured`) changes its
`Statement::Assign` arm:

```rust
Statement::Assign { target, value } => {
    !self.owns_drop(&target.ty)
        && !self.maybe_owns(&target.ty)
        && !value_needs_guards(value)
        && !matches!(value.kind, ValueKind::ArrayLit { .. })
}
```

A non-owning assignment keeps the compact header. An owning target (a
`String`, a `Box`, a struct with owning fields, or a parameter that may own at
an instance) falls back to the flat form. No new statement or emitter is
needed: the flat form already prints the pre-drop from
`compiler/src/codegen/function.rs:1721` (`emit_assign`).

### 1.3 Examples

```clorraine
for (let i: i32 = 0; i < 3; s = $"{i}", i = i + 1) { }  // flat; old s freed
for (let i: i32 = 0; i < 3; n = n + 1, i = i + 1) { }   // compact header kept
```

## 2. For-Init Effects

### 2.1 Rule

A `for` init expression list is evaluated once, before the first condition
test. Each expression is a statement: a call, template, conditional, or
increment must run its effect, and an owned result must be released before the
loop head. A template, owning call, or increment there previously did not run
(or leaked).

### 2.2 Implementation

`compiler/src/own/loops.rs:280` (`check_for_c`, `ForInit::Expr`) replaces
`self.eval(expression)` with the ordinary statement path:

```rust
for expression in expressions {
    let release_floor = self.pending_releases.len();
    self.check_expr_statement(expression);
    self.drain_pending_releases(release_floor);
}
```

`check_expr_statement` emits the init statement into the preheader block. The
loop shape is set on the head branch, so the preheader statements print before
the `for` header and the head keeps its compact form. A discarded owned result
binds a statement temporary that the drain releases once.

## 3. Aggregate Initializer Order

### 3.1 Rule

Struct and array fields evaluate in source order
(`spec/04-expressions.md` §3). The compiler emits a C initializer list in
declaration order and switches to fieldwise stores when the written order
differs (`compiler/src/codegen/guards.rs:1413`,
`struct_literal_order_differs`). When the orders agree, the supported-toolchain contract assumes the two
supported C compilers for the frozen profiles — GCC and vbcc — evaluate the
initializer list in the written order. GCC execution is confirmed by the
corpus and benchmark programs; vbcc is build-only in this repository
(`bench-shape`), so its execution is part of the documented contract rather
than a test claim. The codegen comments state the assumption and a host regression test
pins the same-order (`12`) and differing-order (`21`) results.

### 3.2 Implementation

- Update the rustdoc on `struct_literal_order_differs`
  (`compiler/src/codegen/guards.rs:1413`) and `struct_literal_needs_stores`
  (`compiler/src/codegen/function.rs:3003`) to state the assumption, the
  differing-order fallback, and the test that pins it.
- Add two specification cases:
  - a same-order literal (`P { a: mark(1), b: mark(2) }`) prints `12`;
  - a differing-order literal (`P { b: mark(2), a: mark(1) }`) prints `21`
    (the fieldwise store path).
- No emission-wide change: every aggregate would otherwise grow statement
  temporaries for no observable supported-toolchain difference, and a scored
  benchmark or C-size regression would be the only result (AR-12).

### 3.3 Scope Note

This resolves the issue's second authorized option ("document the
declaration-order assumption in the codegen contract"). A port to a C
compiler that evaluates initializer lists in another order is out of scope;
such a port must revisit this contract.

## 4. `cl_panic` Zero-Length Message

### 4.1 Rule

A null view or zero-state `String` is a NULL pointer with length zero. The
panic prints `panic: ` and a newline and must not pass a NULL buffer to
`fwrite`, even with a zero count.

### 4.2 Implementation

In `compiler/runtime/shim_host.c:56` and
`compiler/runtime/shim_hosted_amiga.c:560`:

```c
/* host shim: stderr; Amiga shim: stdout; each keeps its own stream. */
fwrite("panic: ", 1, 7, stream);
if (len > 0) {
    fwrite(message, 1, len, stream);
}
fwrite("\n", 1, 1, stream);
```

The freestanding shim (`compiler/runtime/shim_freestanding.c:240`) already
copies byte by byte and reads the message only while `len > 0`; no change.
The host runtime case is exercised by the spec suite. The Amiga body will be
pinned by the shim-parity implementation test (IT-11) and compiled by
`make verify`; no hosted-amiga runtime probe is claimed.

## 5. Ownership Invariants

1. A for-step owning assignment releases the old value exactly once per
   iteration on the fall-through and every `continue` edge.
2. A for-init expression runs exactly once and its discarded owner releases
   before the first condition test.
3. A same-order aggregate evaluates its fields in source order on the
   supported toolchains; a differing order uses the fieldwise store.
4. A zero-length panic never calls `fwrite` with a NULL buffer.
5. The #26/#27 ordered-lowering and zero-state contracts are unchanged.

## 6. Affected Code

| File | Function | Change |
|------|----------|--------|
| `compiler/src/own/loops.rs` | `check_for_c`, `for_step_is_structured` | Init statement path; owning-assignment flat form |
| `compiler/src/codegen/guards.rs` | `struct_literal_order_differs` | Contract comment |
| `compiler/src/codegen/function.rs` | `struct_literal_needs_stores` | Contract comment |
| `compiler/runtime/shim_host.c` | `cl_panic` | Zero-length guard |
| `compiler/runtime/shim_hosted_amiga.c` | `cl_panic` | Zero-length guard |

## 7. Failure Handling

- If the flat for-step form changes a non-owning step's emitted shape, the
  shape predicate is narrowed to the owning target only (AR-10).
- If a for-init expression runs twice or releases twice, the drain is moved
  after the whole list (AR-11).
- If GCC or a future supported toolchain evaluates a same-order initializer
  out of order, the contract reopens and the fieldwise store becomes mandatory
  for multi-field literals (AR-12).
