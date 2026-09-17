# Chapter 05 - Statements and Control Flow

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 03 (ownership), Ch 04 (expressions), Ch 06 (functions), Ch 14 (diagnostics), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) section 5
> **Ledger:** B1-B7, F1, F2, F5, F9, GF4, GF9, T12

## 1. Statement forms

| Statement | Form |
|---|---|
| Local mutable binding | `let name [: T] [= expr];` |
| Local immutable binding | `const name [: T] = expr;` |
| Expression | `expr;` |
| Block | `{ statements }` |
| `if` / `else` | `if (cond) { ... } [else ...]` |
| `while` | `while (cond) { ... }` |
| `do` / `while` | `do { ... } while (cond);` |
| `for` | `for (...) { ... }` |
| `switch` | `switch (expr) { ... }` |
| `return` | `return [expr];` |
| `break` / `continue` | `break;` / `continue;` |
| `defer` | `defer { ... }` |
| `unsafe` | `unsafe { ... }` |

Braces are mandatory on every control-flow body, so the dangling-`else`
question cannot arise (B1). The language has no `goto`, no labels, and no
`loop` keyword; `for (;;)` is the infinite loop (B2, B6).

## 2. Blocks and scopes

Braces form a block, and a block is a scope. Locals live to the end of their
enclosing block and drop in reverse declaration order (O2)
[03-ownership.md](03-ownership.md). A block is also a statement where a
statement is expected. Statements inside a block run in order.

## 3. Local declarations

`let` declares a mutable binding; `const` declares an immutable binding (V1).
There is no `var`. A local infers its type and mode from its initializer (F1);
an explicit type annotation is always allowed. Globals, signatures, and fields
stay explicit.

| Declaration | Rule |
|---|---|
| `let x = expr;` | Type and mode come from `expr`; the binding is mutable |
| `let x: T = expr;` | Explicit type; the binding is mutable |
| `let x: T;` | Allowed with no value; definite assignment applies (F9) |
| `const x = expr;` | Immutable; an initializer is required |
| `const x: T = expr;` | Immutable; an initializer is required |

Assignment to a `const` binding is E10023. Writing through a read-only
parameter (unmarked or `const`) is E10024. A `const` may be read, copied if
duplicable, and lent read-only; assignment, moving out, and writable lends are
errors (V3). A local that is declared and never used warns as W10000.

```clorraine
let count = 0;                 // i32, mutable
const limit: i32 = 10;         // immutable
let total: i32;                // declared, then assigned
total = count + limit;         // definite assignment satisfied
```

## 4. Expression statements

Any expression followed by `;` is an expression statement. Because assignment
has a `void` result, chaining is E10217; a value expression is usually written
for its side effect, such as a call or an assignment.

```clorraine
step();                 // call for its effect
count = count + 1;      // assignment
count = (count = 2);    // E10217
```

Call-site markers are optional assertions that must match the declaration; a
mismatch is E10039 [03-ownership.md](03-ownership.md).

## 5. Conditions

Every condition — `if`, `while`, `do`/`while`, the middle `for` clause, and the
`?:` operator — must have type `bool`, otherwise E10218. There is no truthiness
and no implicit conversion from integers or pointers, so write the comparison
explicitly [04-expressions.md](04-expressions.md).

## 6. `if` and `else`

```clorraine
if (x > 0) {
    sign = 1;
} else if (x < 0) {
    sign = -1;
} else {
    sign = 0;
}
```

`else if` is parsed as an `else` containing another `if`; the meaning is
identical. Both branches are blocks, so braces are always present (B1).

## 7. `while` and `do`/`while`

```clorraine
while (!ready) {
    pump();
}

do {
    tick();
} while (ticks < limit);
```

`while` tests before the body; `do`/`while` tests after the body and so runs
it at least once (B3). Use `for (;;)` for an unconditional loop (B2).

## 8. Three-clause `for`

`for (init; cond; step) { ... }` runs `init` once, tests `cond` before each
iteration, and runs `step` after each iteration. Any clause may be empty, so
`for (;;)` is the infinite loop. The init clause is a `let`, a `const`, or an
expression list. There is no range syntax (B4).

```clorraine
for (let i = 0; i < len(xs); i++) {
    xs[i] = xs[i] + 1;      // bounds checked: E11211 if i is out of range
}
for (;;) {
    if (!poll()) {
        break;
    }
}
```

## 9. Element loops

`for (x in xs) { ... }` visits each element of an indexable value. The loop
variable is a copy, so the element type must be duplicable. To write through,
use `for (borrow e in xs) { ... }`, which binds an exclusive view of each
element in turn; that loan lasts for one iteration only. `borrow` is required
when the element type is not duplicable (E10040). This is the form for
mutating a slice (GF9, F1).

```clorraine
func total(xs: Entity[]): i32 {
    let sum = 0;
    for (e in xs) {         // copy of each duplicable Entity
        sum = sum + e.x;
    }
    return sum;
}

export func moveAll(borrow xs: Entity[], dx: i32): void {
    for (borrow e in xs) {  // exclusive view of each element
        e.x = e.x + dx;
    }
}
```

`continue` in an element loop advances to the next element; the per-element
view ends and is created again.

## 10. `switch`

`switch (expr) { case V: ... default: ... }` is C-exact (B5). A label does not
end a case body, so control falls through into the next label unless a `break`
exits the switch. There is no `fallthrough` keyword and no exhaustiveness
requirement. `default` may appear anywhere among the labels, as in C.

```clorraine
switch (state) {
    case 0:
        start();
        // falls through
    case 1:
        run();
        break;
    default:
        stop();
}
```

Falling through into the next label produces the warning W10002. A `break`
inside a `switch` exits the `switch`, not an enclosing loop.

## 11. `return`

`return expr;` returns by value (a copy for duplicable types, an ownership
transfer for move-only types) or as a view type (slices, `str`). There is no
`const`/`borrow`/`ref` return marker (F2). `return;` is allowed only in a
`void` function. A view return must derive from exactly one lend parameter
(EL-1 to EL-6) [lineage.md](lineage.md). Code after an unconditional `return`,
`break`, `continue`, or `panic` cannot be reached and warns as W10003.

```clorraine
func half(buf: Entity[]): Entity[] {
    return slice(buf, 0, len(buf) / 2);   // the view derives from buf
}
```

## 12. `break` and `continue`

`break` exits the innermost loop or `switch`; `continue` starts the next
iteration of the innermost loop. There are no labels and no `goto` (B6). Both
are exit edges for the cleanup ladder (section 15).

## 13. `defer`

`defer { ... }` runs when the enclosing scope exits. It is code motion, not a
closure: the block is checked in the context of the exit path, so narrowing
done before the exit is visible inside it (F5, GF15). Defers join the cleanup
ladder, which runs drops and defers in reverse registration order. They run on
`return`, `break`, and `continue`, but not on `panic` or an aborted runtime
fault.

```clorraine
func update(p: raw u8, n: usize): void {
    lock();
    defer { unlock(); }     // runs on any normal exit from this scope
    unsafe { *p = 0; }
}
```

## 14. `unsafe` blocks

`unsafe { ... }` is a statement block that permits the raw operations: raw
dereference, address-of, pointer arithmetic, pointer casts, and volatile
access (GF4, T6). It is a greppable audit marker, not a scope with special
ownership: values still drop at the block's end.

| Operation | Rule |
|---|---|
| Raw dereference | E10600 outside `unsafe` |
| Address-of | E10601 outside `unsafe` |
| Pointer arithmetic or raw indexing | E10602 outside `unsafe` |
| Pointer-to-pointer cast | E10603 outside `unsafe` |
| Volatile intrinsic or overlay access | E10604 outside `unsafe` |

```clorraine
unsafe {
    p.value = 1;
    writeVolatile16(addr, 0x0080);
}
```

Declaring, passing, storing, and comparing raw pointers, and integer-to-`raw`
value casts, are allowed outside `unsafe`; only the operations above require
it [04-expressions.md](04-expressions.md).

## 15. Exit edges and the cleanup ladder

Every exit edge from a scope — the normal end, `return`, `break`, and
`continue` — runs the cleanup ladder for that scope (P2). The ladder is
single-exit: drops and `defer` blocks run in reverse registration order, and
drops are null-safe, so a moved value is a no-op (O1, O2)
[17-codegen.md](17-codegen.md). `panic` and the runtime faults (E11210,
E11211, E11212, E11213) abort, so they do not run the ladder (E4).

| Exit | Cleanup ladder |
|---|---|
| End of scope | yes |
| `return` | yes |
| `break` | yes |
| `continue` | yes |
| `panic` or a runtime fault | no |

## 16. Definite assignment

Reading a local before it is assigned on every path is E10021. There is no
zero-fill and no unspecified value, so a `let x: T;` must be assigned before
any read. `borrow T?` out-parameters are exempt, because passing the binding
is how it is filled (F9).

```clorraine
let f: File?;
openInto(path, borrow f);   // exempt: the out-parameter fills f
if (f != null) {
    read(f);                // the narrowing check makes the read safe
}
```

## 17. Rule summary

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-1 | Braces are mandatory; no dangling `else` | - |
| ST-2 | Conditions must be `bool`; no truthiness | - |
| ST-3 | `let` is mutable, `const` is immutable | E10023 |
| ST-4 | A read-only parameter cannot be written | E10024 |
| ST-5 | No read before definite assignment | E10021 |
| ST-6 | The assignment result is `void` | E10217 |
| ST-7 | `for (;;)` is the infinite loop; no range syntax | - |
| ST-8 | Element loops copy; `borrow` gives exclusive views | E10040 |
| ST-9 | `switch` falls through | W10002 |
| ST-10 | No `goto` and no labels | - |
| ST-11 | `defer` runs at scope exit, not on `panic` | - |
| ST-12 | The cleanup ladder runs on every normal exit edge | - |
| ST-13 | Raw operations require `unsafe` | E10600-E10604 |
| ST-14 | Call-site markers must match the declaration | E10039 |
| ST-15 | Indexing is always checked | E11211 |
| ST-16 | A local declared and never used warns | W10000 |
| ST-17 | A statement that cannot be reached warns | W10003 |
