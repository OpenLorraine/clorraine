# Chapter 03 - Ownership, Moves, and Destructors

> **Status:** stable (v1.0)
> **Depends on:** Ch 02 (types), Ch 12 (FFI and contracts), Ch 16 (borrow rules), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.1 (params), 3.4 (handles), 3.5 (module values), 4 (types), 5 (statements), 6.3-6.4 (unsafe and postfix expressions)
> **Ledger:** [decisions.md](decisions.md) sections 1, 4, 5; plus 6 (T6, T8), 11 (X1, X2), 15 (P2-P5)

This chapter defines what owns a value, when a value moves, and when it is
released. It is the source-level contract; [Ch 16](16-borrow-rules.md) specifies
how the checker proves the borrow rules, and [Ch 17](17-codegen.md) shows the
generated cleanup ladder.

## 1. Values and resources

A **value** is a scalar, an aggregate, a view, or a handle.

| Kind | Examples | Duplicability |
|---|---|---|
| Scalar | `i32`, `u32`, `f64`, `bool` | duplicable; a copy is a new value |
| Aggregate | struct, fixed array | duplicable when every field or element is duplicable (V8) |
| View | `str`, `T[]`, `cstring` | duplicable; a copy is a cheap pointer-and-length view |
| Handle | `File`, `Buffer`, `String` | move-only; released by its `freedBy` deleter |
| Box | `Box<T>` | move-only; released by generated recursive drop glue |

A **handle** names a resource: heap memory, a library base, a file, a message
port. The library that owns the resource declares it once with
`handle T freedBy F;` (X1). `F` is a function that takes `take T` and returns
`void`; it may be an ordinary Clorraine function or an `ffi` declaration, and
there is exactly one deleter per type. An ordinary deleter is resolved
module-scoped (the handle's own module, its imports, then exactly one exported
ordinary function program-wide); a cross-module ordinary candidate must be
`export`ed, a non-function declaration shadows the fallback, and a missing,
ambiguous, wrongly typed, or non-consuming deleter is E10102. The deleter's own
`take T` parameter is excluded from the implicit scope drop, and its body must
pass that parameter to a call. Native structs never use `freedBy`; they release
their fields structurally (X2). Application code never writes `ffi` blocks
(X6); [Ch 12](12-ffi-contracts.md) owns the boundary.

The built-in `Box<T>` owns its pointee through generated drop glue rather than
a user deleter (P7a). `Box<T>` is move-only and `Box<T>?` drops null-safely; a
boxed value cannot be moved out, and a nested box, struct, or array drops
recursively ([Ch 11](11-memory.md)).

Bindings hold values. A view arrives from a parameter or a view-typed return,
never from a local view expression (O9). The retired placeholder keywords of
the draft are not part of the surface.

## 2. Ownership model

**One owner per owned value.** The owner is the local binding, `take`
parameter, field of an owned value, array element of an owned array, or
returned value that is responsible for the release. A **lend** is a temporary
borrow passed to a callee; it carries access, not responsibility, so it never
releases.

**Duplicability is derived from fields** (V8). Scalars are duplicable. A struct
is duplicable when all its fields are duplicable; handles and structs with
resource fields are move-only. There is no `copy` keyword. A duplicable value
copies; a move-only value transfers by move.

**Parameter modes** decide how the callee receives each argument. The mode is
written before the name (V5):

| Mode | Meaning | May write | Ownership |
|---|---|---|---|
| unmarked `x: T` | read-only lend | no | caller keeps |
| `const x: T` | read-only lend | no | caller keeps |
| `borrow x: T` | writable lend (exclusive) | yes | caller keeps |
| `take x: T` | ownership transfer | yes | callee owns; the caller's binding is invalidated |

Scalars are copied in every mode. Call-site markers (`take f`, `borrow f`,
`const f`) are optional assertions; if written they must agree with the
declaration, otherwise E10039, and the declaration is always authoritative (V6).

Bindings: `let` is mutable and `const` is immutable (V1). A `const` binding may
be read, copied when duplicable, and lent read-only; assigning to it is E10023,
and moving out of it or taking a writable lend of it is an error (V3).
Assigning through a read-only parameter is E10024; declare the parameter
`borrow` to write through it. A lend of a duplicable value binds a copy (V9);
a writable lend of a duplicable value warns as W10001 instead of erroring
(D9), and a `const` marker on a copy type has no effect and is likewise
tolerated as a warning (V4; no dedicated registry code in v1). The copy rule
covers value lends: a view lend (`T[]`, `str`, `cstring`) copies only the view
(pointer and length), and the storage it points into is shared, so writes
through a writable view reach the owner. The same holds for the per-element
views of `for (borrow e in xs)` ([Ch 08](08-arrays-strings.md) section 5).
Writable lends of owned temporaries are allowed; writable lends of `const`
bindings are errors (V11).

## 3. Moves

A **move** transfers ownership. It happens in these situations:

| Situation | Moves? |
|---|---|
| `let b = a;` where `a` is move-only | yes |
| Passing a value to a `take` parameter | yes |
| Returning an owned value | yes |
| Assignment of a move-only value | yes |
| A top-level field out of an owned local or `take` parameter | yes; partial (section 4) |
| Binding or copying a duplicable value | no; it copies |
| Binding a view | no; the view copies |

* After a move the source is invalid: a later read, borrow, move, or drop of it
  is E10020 (O1). The compiler omits the drop of a moved value, and zero = empty
  (O7) makes any remaining exit-path drop a no-op, so a drop never runs twice.
* Re-initializing a moved binding is allowed (O3). After
  `a = openFile("RAM:b");` the binding is live again.
* Assignment drops the live old value of the target first, then stores the new
  value (O3). This includes assignment through a `borrow T?` out-parameter: the
  callee owns the incoming slot value, so releasing it is allowed (the E10034
  carve-out in section 6).
* Move state is flow-sensitive. At a control-flow join, a value moved on any
  path counts as moved (O6, conservative).
* **Zero = empty** (O7): the all-zero bit pattern is the valid "nothing to
  release" state for every droppable type. The release routine checks this
  state before releasing, and the zeroed value is the drop flag.

```clorraine
import { File, openFile } from std.io;

func consume(take f: File): void {
}

func demo(): void {
    let f: File = openFile("RAM:a");
    consume(take f);        // ownership moves into consume; f is invalid after this
}
```

Element loops follow the same rule: `for (e in xs)` copies duplicable elements,
and non-duplicable elements require `for (borrow e in xs)`, otherwise E10040
(grammar 5.2, GF9). Moving an element out of an array is an element move, which
v1 limits (section 4).

## 4. Partial moves

A partial move takes one top-level field out of an owned local or a `take`
parameter (O4). Nested paths and element moves are E10026 in v1.

| Rule | Diagnostic |
|---|---|
| Partial moves are limited to top-level fields of owned locals and `take` parameters | E10026 |
| After a partial move, whole-value use (read, move, or lend) is an error | E10025 |
| The remaining fields stay usable | - |
| Reassigning the moved field re-initializes the struct | - |
| A field moved on any path counts as moved at a control-flow join | - (O6) |

```clorraine
import { String, stringOf } from std.text;

struct Job {
    name: String;
    status: u32;
}

func consume(take s: String): void {
}

func demo(): void {
    let job: Job = Job { name: stringOf("build"), status: 0 };
    let name: String = job.name;   // partial move of the top-level field
    job.status = 1;                // remaining fields stay usable
    // let whole: Job = job;       // E10025: the whole value is partially moved
    job.name = stringOf("run");    // reassignment re-initializes the struct
    consume(take name);
}
```

## 5. Drops and destructors

An owned value is **dropped** (released) when its owner's scope ends. Drop
order is fixed (O2):

| Scope | Order |
|---|---|
| Locals | reverse declaration order |
| Struct fields | declaration order |
| Array elements | first to last |
| Parameters | after the body's locals |

Temporaries drop at the end of the statement that created them (D5).
`defer { ... }` joins the same cleanup ladder: drops and defers run in reverse
registration order, and the block is checked at its exit context (F5). Drops run
on every `return`, `break`, and `continue` that leaves a scope (P2), and the
null-safe check (O7) makes them run exactly once (O1).

A `Box<T>` drop releases its pointee recursively: a scalar pointee is freed
directly, a struct or array pointee drops its owned fields or elements first,
and a nested `Box` pointee recurses. The generated helper is null-safe and
checks the box before touching the pointee, so a `null` box drops nothing. A
`Box<T>?` uses the same helper and is a no-op when null. An ordinary `freedBy`
deleter lowers to a null-safe call to the deleter symbol, exactly like an
`ffi` deleter (P5).

There are no drops at process exit. `panic` aborts without running the ladder
(D4, E4). A global must therefore be duplicable or `const`: a non-duplicable
global is E11106 (O10).

```clorraine
let tracing: bool = false;

func process(): void {
    tracing = true;
    defer {
        tracing = false;    // runs on return, break, or continue; not on panic
    }
    // ... work ...
}
```

## 6. Nullability in ownership

`T?` applies to handles, `raw`, `cstring`, and views (T8). On any other type it
is E10011. `T` widens into `T?`, and narrowing requires an `== null` or
`!= null` check; using the value before the check is E10010. Using `null` where
a non-nullable type is required is E10012, except where an explicit cast's own
rule applies (Ch 04 §14).

A nullable owned value drops null-safely (O7): when it is `null` there is
nothing to release, so the drop is skipped. The **out-parameter pattern**
writes a handle through a `borrow T?` parameter (T8):

```clorraine
import { File, openInto } from std.io;

func dump(path: str): void {
    let f: File? = null;        // the caller initializes the slot; uninitialized is E10021
    openInto(path, borrow f);   // the callee writes a handle or leaves it null
    if (f == null) {
        return;                 // nothing to release
    }
    // f is narrowed to File here; it drops null-safely at scope end
}
```

A `borrow T?` out-parameter is an **in-out ownership slot**: the callee owns the
incoming value on entry and may release it, so E10034 does not apply to
releasing or replacing that incoming value. The exemption is limited to the
slot's own value: storing a different lend parameter's value or view in the
slot is still E10034, because that lend is call-scoped and the caller can
reassign its backing after the call. The callee is not required to write on
every path; afterwards the slot owns the resulting handle (or stays `null`).
Assignment through the slot drops the callee-owned previous value first, then
stores the new one (the O3 carve-out). [Ch 06](06-functions.md) section 4
states the full rule.

Definite assignment (F9) applies to the caller's slot too: the caller must
assign it on every path before the call, so write `let f: File? = null;` first.
Passing an uninitialized slot to a `borrow T?` out-parameter is E10021.

## 7. Borrows of owned values

This section summarizes the borrow contract for owners. The checker and its
borrow model are specified in [Ch 16](16-borrow-rules.md); borrows never
release a value and never change ownership.

| Rule | Diagnostic |
|---|---|
| A lend may not outlive its owner | E10030 |
| No move of an owner while a loan of it is live | E10022 |
| No mutation of an owner while a shared lend is live | E10031 |
| A writable lend is unique; a second conflicting loan is an error | E10032 |
| Lends do not escape their origin (a field, a global, or a local's return) | E10033 |
| A borrowed parameter is call-scoped; the callee may not release or store it (except the `borrow T?` slot's own incoming value, which the callee owns) | E10034 |
| A view returned from a local is an error | E10035 |
| A view return must derive from exactly one view-like lend parameter | E10036 |
| A global may not be borrowed | E10038 |
| A writable view may not derive from a read-only borrow | E10041 |
| A writable lend of a duplicable value is a warning, not an error | W10001 |

The shape is many readers XOR one writer. Field borrows are allowed and tracked
conservatively as whole-struct loans (O8). **Reborrowing views** (O12): a
read-only view derived from a lend is a shared reborrow, and a writable view
derived from a writable lend is an exclusive reborrow limited to the
expression. This is what makes `borrow bufferBytes(buf)` valid when `buf` is
itself a parameter. Returned views obey lifetime elision (EL-1..EL-6); a view
of a local is rejected because it would outlive its owner.

## 8. The raw escape hatch

`raw T` is an unchecked copy pointer, nullable as `raw T?` (T6). Dereference,
indexing, pointer arithmetic, pointer casts, and address-of require `unsafe`,
written as a block `unsafe { ... }` or a prefix operator `unsafe p.next` (T6,
GF4). Declaring, passing, storing, and comparing raw pointers, and
integer-to-`raw` value casts, are allowed outside `unsafe` (T6).

Ownership never flows through `raw` automatically. Converting an owned handle to
or from `raw` is an explicit, audited step. An uncontracted extern call requires
`unsafe` and may not receive owned or borrowed values (T6, X4). `unsafe` is a
greppable audit marker, not a provenance guarantee (T6): the rule that a `raw`
pointer may not launder a lend out of its scope is an audited convention and an
explicit review requirement, not a compiler check. It is not enforced by the
compiler; [Ch 12](12-ffi-contracts.md) keeps the boundary in reviewed wrappers.

## 9. Rule index

The draft rule identifiers become an index for diagnostics and tests (ledger
section 1). They are restated here in the current vocabulary.

| Rule | Statement | Diagnostic |
|---|---|---|
| OW-1 | Every owned value has one owner; every binding, parameter, and field has a mode | - |
| OW-2 | Duplicability is derived from fields; there is no `copy` marker | - |
| OW-3 | Lends and views carry access, never release | - |
| OW-4 | No move of an owner while a loan of it is live | E10022 |
| OW-5 | No read, borrow, move, or drop of a moved value | E10020 |
| OW-6 | A move copies then zeroes the source; joins are conservative | - (codegen) |
| OW-7 | A lend may not outlive its owner | E10030 |
| OW-8 | No move or writable lend while a shared lend is live | E10031 |
| OW-9 | A writable lend is unique | E10032 |
| OW-10 | Lends do not escape their origin | E10033 |
| OW-11 | Drops run at scope exit, locals in reverse declaration order | - (codegen) |
| OW-12 | Drops are null-safe and run exactly once | - (codegen) |
| OW-13 | Temporaries drop at the statement end | - (codegen) |
| OW-14 | No drops at process exit; globals may not be move-only owners | E11106 |
| OW-15 | A `take` parameter transfers ownership into the callee | - |
| OW-16 | A borrowed parameter is call-scoped; the callee must not release or store it, except the `borrow T?` slot's own incoming value, which the callee owns | E10034 |
| OW-17 | A returned view derives from a parameter (elision) | E10035 / E10036 |
| OW-18 | v1 aggregates carry no view-typed fields except `static str` | - (Ch 02) |
| OW-19 | A global must be duplicable or `const`; a global may not be borrowed | E11106 / E10038 |
