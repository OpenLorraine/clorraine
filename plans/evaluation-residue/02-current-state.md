# Current State — Evaluation and Initializer Residue (Batch T)

Baseline `55beee0`.

## `DEF-529` — run-time-index view-store leak

Reproduction (accepted at the baseline):

```clorraine
import { String, stringOf } from std.text;

struct Box { v: String; }
struct Wrap<T> { items: T[2]; tag: String; }

func store<T>(borrow w: Wrap<T>, borrow a: Box, i: i32): void {
    w.items[i] = a.v;
}

func main(): i32 {
    let w: Wrap<str> = Wrap<str> { items: ["", ""], tag: stringOf("t") };
    let a: Box = Box { v: stringOf("ab") };
    store(w, a, 1);
    return i32(len(w.items[1])) - 2 + i32(len(a.v)) - 2;
}
```

The baseline emits in `cl_store__str`:

```c
ClString __cl_tmp1 = NULL;
...
__cl_tmp1 = (*a).v;
(*a).v = NULL;
*(&((*w).items[__cl_tmp0])) = cl_string_view(__cl_tmp1);
```

The source place is consumed into a transfer temporary even though the instance
resolves the element to `str`; the temporary owns the handle and is never
released, and the caller's `a.v` is emptied through the borrow. The caller's
`len(a.v)` then reads a null handle. The constant-index twin emits
`(*w).items[0] = cl_string_view((*a).v);` and is clean.

Code site: `compiler/src/own/expr.rs::emit_indexed_store`, the value branch
after `lower_value`: an element type that is a type parameter takes the owned
`consume_place` path because `is_view`/`is_view_like` are false for the
unresolved type. The own checker already defers the move
(`record_opaque_store` returns `deferred_owned_move`); the lowering did not.

## `DEF-531` — non-call effectful `len` operands

Baseline examples (each returns the folded extent and drops the effects):

```clorraine
let n: usize = std.mem.len(mkCond(run));        // mkCond returns a fixed array
let n: usize = std.mem.len([mkRow(), mkRow()]); // literal of fixed arrays
let n: usize = std.mem.len(a[bump()]);          // run-time index
```

The only evaluated shape is a direct call operand, lowered through
`materialize_guard_operand`. A conditional, array literal, or run-time index
falls into the structural `ValueKind::Int(extent)` fold with no evaluation.

Code site: `compiler/src/own/call.rs::lower_intrinsic_call`, the free `len`
branch: `is_pure_place` folds, `ExprKind::Call` materializes, every other shape
folds.

## `DEF-532` — module initializer own-call gap

Baseline examples:

```clorraine
let n: N = N { v: 1 };

let g: i32 = n.peek(5);       // module-bound global receiver; accepted
let h: i32 = peek(n, 5);      // written global borrow; accepted
```

The function-body twins report `E10038: cannot borrow global 'n'`. Module
initializers are checked by sema (`check_globals`) but the own stage's call
checker (`check_call_expr`) is only reached from function-body lowering.

Code site: `compiler/src/own.rs`/`compiler/src/sema.rs` module-entry walk; the
own call checker in `compiler/src/own/call_check.rs::check_call_expr`.

## Existing tests to keep green

- `compiler/tests/callable_release_integrity_spec_test.rs` — release/leak
  controls, including the owned-projection and defer cases.
- `compiler/tests/array_value_integrity_spec_test.rs` — `len` and fixed-array
  runs.
- `compiler/tests/eval_emission_integrity_spec_test.rs` — constant-index view
  store and the conditional evaluation order.
- `compiler/tests/checker_conformance_spec_test.rs` — the module-path and
  qualified-intrinsic cases from batch S.
