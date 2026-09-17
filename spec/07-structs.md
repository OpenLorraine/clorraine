# Chapter 07 - Structs

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 03 (ownership), Ch 06 (functions), Ch 12 (FFI), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.2 and 6.6
> **Ledger:** F4, V8, O2, O4-O6, T4, T7, T10, T11, F8, G1-G8

## 1. Declaration

A struct groups named fields into one nominal type. The keyword is `struct`.

```clorraine
export struct Point {
    x: i32;
    y: i32;
}
```

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-1 | `struct Name { field: T; ... }`, optionally `export struct` | - |
| ST-2 | Fields are semicolon-terminated and declared in C-layout order; the compiler never reorders them | - |
| ST-3 | At least one field is required; C99 has no empty structs | E10206 |
| ST-4 | `volatile` is the only field qualifier; any other qualifier is a syntax error | E20000 |

`export` makes the struct visible to other modules; the default is
module-private (M3). Structs are nominal: two structs with identical fields
are still different types.

## 2. Struct literals

A struct value is written as the type name followed by named fields (F4).

```clorraine
let p = Point { x: 1, y: 2 };    // named fields, any order
let origin = Point {};           // the explicit zero value
let q = Point { y: 2, x: 1, };   // order-independent; trailing comma allowed
```

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-5 | Fields are named and order-independent | - |
| ST-6 | A missing, unknown, or duplicate field is an error | E10203 |
| ST-7 | `Point {}` is the explicit zero value (O7) | - |
| ST-8 | Trailing commas are allowed | - |
| ST-9 | There is no update syntax in v1 | - |

```clorraine
let a = Point { x: 1 };              // E10203: y is missing
let b = Point { x: 1, y: 2, x: 3 };  // E10203: x is duplicated
let c = Point { x: 1, y: 2, z: 3 };  // E10203: z is unknown
```

In statement position, `identifier {` always starts a struct literal, never a
block (grammar section 11).

```clorraine
Point { x: 1, y: 2 };    // a struct-literal expression statement
```

## 3. Duplicability

Duplicability is derived from fields (V8). There is no `copy struct` marker
and no `copy` keyword.

| Struct | Duplicable? | Why |
|---|---|---|
| every field duplicable | yes | a copy duplicates every field |
| any handle or resource field | no; move-only | handles own a resource |

```clorraine
struct Point { x: i32; y: i32; }         // duplicable: both fields duplicable
struct Label { name: String; n: u32; }   // move-only: String is an owned handle
```

Assignment and argument passing follow the type: a duplicable struct copies,
a move-only struct moves (O1).

```clorraine
let a = Point { x: 1, y: 2 };
let b = a;                               // copy; a and b stay usable

let l1 = Label { name: stringOf("hi"), n: 1 };
let l2 = l1;                             // move; l1 is no longer usable
```

## 4. Move-only structs and partial moves

A move-only struct moves as a whole, and it can also be moved field by field
at the top level (O4). Partial moves are limited to top-level fields of owned
locals and `take` parameters.

```clorraine
struct Pair { left: String; right: String; }

func takeLeft(take p: Pair): String {
    let l = p.left;      // partial move: a top-level field
    // p as a whole is now unusable ...
    return l;            // ... but p.right still drops at exit
}
```

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-10 | Whole-value use, move, or lend of a partially moved struct is an error | E10025 |
| ST-11 | Partial moves are limited to top-level fields of owned locals and `take` parameters | E10026 |
| ST-12 | Nested-path and element moves are deferred to a later version | E10026 |
| ST-13 | Remaining fields stay usable; reassigning a field re-initializes the struct | - |
| ST-14 | At a control-flow join, a field moved on any path counts as moved (conservative) | - |

```clorraine
struct Inner { s: String; }
struct Outer { inner: Inner; }

let o = Outer { inner: Inner { s: stringOf("x") } };
let s = o.inner.s;       // E10026: nested-path move
let e = xs[0];           // E10026: element move
```

After the partial move in `takeLeft`, `p.right` may still be read or moved,
and assigning `p.left` again re-initializes the whole struct (O5).

## 5. Drop order

Fields drop in declaration order (O2). A native struct drops structurally:
each field drops as its own type dictates. A handle drops through its
`freedBy` deleter (X2); native structs never declare `freedBy`.

```clorraine
struct Resource {
    first: String;     // dropped first
    second: Buffer;    // dropped second
}
```

Array elements drop first-to-last, parameters drop after body locals, and
locals drop in reverse declaration order (O2; see
[Ch 03](03-ownership.md)).

## 6. Fields and views

A field may hold `String`, `cstring`, or `static str`. A plain `str` field is
banned (T4, the view-field ban).

```clorraine
struct Bad { part: str; }     // E10205: a view-typed field

struct Good {
    name: String;             // owned handle
    label: static str;        // literal with static origin
    cname: cstring;           // FFI string pointer
}
```

`static str` widens to `str` on read, so a literal field stays usable where a
`str` is expected (T3). A field of a pointer-like type may also be declared
with the `T?` suffix (T8).

## 7. Volatile fields and overlay types

`volatile` is a field qualifier. A struct with at least one `volatile` field
is an **overlay type** (T7, T11): it describes memory the program does not
own, such as memory-mapped registers.

```clorraine
struct CustomRegs {
    dmacon: volatile u16;
    cop1lc: volatile u32;
}
```

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-15 | A `volatile` field makes the struct an overlay type | - |
| ST-16 | An overlay value cannot be a local, copied, dropped, borrowed, or passed by value | E10207 |
| ST-17 | Overlay access goes through a `raw` pointer inside `unsafe` | E10207 |
| ST-18 | `volatile` outside a struct field or a module-level `let` | E10208 |

An overlay type has no drop and no copy, so it can never be reached as a
value. Access it through a `raw` pointer inside `unsafe`:

```clorraine
func enableCopper(): void {
    let regs = raw CustomRegs(0x00DFF000);
    unsafe {
        regs.dmacon = regs.dmacon | 0x0080;
    }
}
```

```clorraine
let r: CustomRegs;                       // E10207: no overlay locals
let v = CustomRegs { dmacon: 0 };        // E10207: no overlay values

// `volatile` is only for a struct field or a module-level `let`; on a local
// or a parameter it is E10208.
const BASE: u32 = 0x00DFF000;            // a plain const carries no qualifier
```

Raw fields are allowed in ordinary structs; using them for pointer operations
still requires `unsafe` (T11).

## 8. No methods and no inheritance

Structs have no methods and no inheritance. Behavior lives in free functions,
reached with dot-call (F8): `x.f(y)` resolves to the free function `f(x, y)`.
Resolution is static; there are no receivers and no vtables. See
[Ch 06](06-functions.md).

```clorraine
struct Point { x: i32; y: i32; }

func moveBy(borrow p: Point, dx: i32): void {
    p.x = p.x + dx;
}

let p = Point { x: 1, y: 2 };
p.moveBy(3);             // resolves to moveBy(p, 3)
```

## 9. Generic structs

A struct may declare type parameters with `<T>` (G1).

```clorraine
struct Pair<T> {
    first: T;
    second: T;
}

let p = Pair<i32> { first: 1, second: 2 };   // explicit type arguments
```

Instantiation is monomorphized (G3); the type arguments are part of the type,
and each storable instantiation produces one concrete struct with a
hash-deduped symbol. See [Ch 17](17-codegen.md).

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-19 | The wrong number of type arguments | E10300 |
| ST-20 | Type arguments that cannot be inferred; write them explicitly | E10301 |
| ST-21 | `export` on a generic struct | E10302 |
| ST-22 | A type argument that violates a declared callable bound | E10303 |
| ST-23 | A callable-only type parameter stored in a field | E10304 |
| ST-24 | Instantiation recursion beyond the cap | E10305 |
| ST-25 | A generic aggregate instantiated with an ephemeral type argument | E10306 |

## 10. Self-referential structs

A struct may refer to itself only through a `raw` pointer (T10, X1). A direct
or view-typed self-reference would be infinite or dangling, so linked data
uses the C-style node pattern:

```clorraine
struct Node {
    value: i32;
    next: raw Node?;
}
```

Tagged data is modeled the same way: a C-style struct with a tag field, all
branches present and zero-safe, decoded with a C-exact `switch`. The language
has no union type and no pattern-matching construct (T10).

```clorraine
struct Shape {
    kind: u8;        // tag: 0 circle, 1 rectangle
    x: i32;
    y: i32;
    r: i32;          // zero when unused
}
```

## 11. Diagnostics

| Code | Meaning |
|---|---|
| E10025 | Whole-value use of a partially moved struct |
| E10026 | Partial move outside a top-level field, or element/nested move |
| E10203 | Struct literal field problems (missing, unknown, duplicate) |
| E10205 | View-typed field other than `static str` |
| E10206 | Struct declares no fields |
| E10207 | Overlay (volatile) type used as a value |
| E10208 | `volatile` in the wrong place |
| E10300-E10306 | Generic struct diagnostics |
