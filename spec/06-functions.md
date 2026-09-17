# Chapter 06 - Functions and Parameters

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 03 (ownership), Ch 04 (expressions), Ch 12 (FFI), Ch 16 (borrow rules), Ch 17 (codegen)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.1, 8, 9
> **Ledger:** F2, F3, F7, F8, V5-V7, V9, G1-G8, L1-L5, M8, T8, F9, GF22

## 1. Declaration

A function is declared with `func`. The keyword is `func`, never `function`.
The return type is mandatory: write the result type, or `void` when nothing is
returned.

```clorraine
export func add(a: i32, b: i32): i32 {
    return a + b;
}

func logNote(msg: str): void {
    write(msg);
}
```

| Rule | Statement | Diagnostic |
|---|---|---|
| FN-1 | `export func name(params): R { ... }`; the return type is required (`void` allowed) | - |
| FN-2 | No overloading: a name is declared once per module | E11101 |
| FN-3 | No default, named, or variadic arguments; no tuples and no multiple returns (F3) | - |
| FN-4 | A function value has the exact C signature `func(T, ...): R` | E10210 |

`export` marks the function public; the default is module-private (M3).

## 2. Parameters

A parameter writes its mode first, then the name, then the type:
`borrow e: Entity`. The mode decides what the callee may do with the
argument.

| Written | Meaning | Caller |
|---|---|---|
| `e: Entity` | read-only lend (shared) | keeps ownership |
| `const e: Entity` | read-only lend; the explicit spelling | keeps ownership |
| `borrow e: Entity` | writable lend (exclusive) | keeps ownership; exclusive for the call |
| `take e: File` | ownership transfer | the argument moves into the callee |

* Scalars are copied at the call, whatever the mode (V5).
* Writing through a read-only parameter is E10024.
* A lend of a duplicable value binds a copy (V9); a write through a `borrow`
  of a duplicable type affects that copy only. A needless lend warns as
  W10001, not an error (D9).

```clorraine
struct Entity { name: String; x: i32; y: i32; }

func describe(e: Entity): i32 { return e.x; }   // read-only lend
func nudge(borrow e: Entity): void { e.x = e.x + 1; } // writable lend
func release(take f: File): void { Close(f); }        // ownership moves in
```

### 2.1 Call-site markers

Call-site markers are optional assertions (V6). `nudge(e)` and
`nudge(borrow e)` are the same call. When a marker is written it must match
the declaration; otherwise the compiler reports E10039. The declaration is
always authoritative.

```clorraine
let e = Entity { name: stringOf("ship"), x: 1, y: 2 };

nudge(e);            // the declaration says 'borrow': a writable lend
nudge(borrow e);     // same call; the marker only asserts it
```

A future compiler strict mode, `--require-explicit-markers`, may require
markers at every call site (V7). It is a compiler option, not a language
rule, so code without markers stays valid.

## 3. Returns

Returns are by value or by view type (F2). There is no `const`, `borrow`, or
`ref` return marker.

| Return kind | What happens |
|---|---|
| duplicable value | the result is copied |
| move-only value | ownership transfers to the caller |
| view type | a view derived from exactly one lend parameter |

```clorraine
func origin(): Point { return Point {}; }          // duplicable: copied
func box(take b: Buffer): Buffer { return b; }     // move-only: transferred
func head(xs: u8[], n: usize): u8[] {              // view of the one lend
    return slice(xs, 0, n);
}
```

| Rule | Statement | Diagnostic |
|---|---|---|
| FN-5 | Returning a view of a local owner | E10035 |
| FN-6 | A loan that escapes its origin (stored in a field or global) | E10033 |
| FN-7 | A borrowed parameter released or stored by the callee | E10034 |
| FN-8 | A view return that does not derive from exactly one view-like lend parameter | E10036 |

### 3.1 View returns and lifetime elision

There is no lifetime annotation syntax (D2). A view return takes its lifetime
from its parameters. The elision rules are:

| Rule | Statement |
|---|---|
| EL-1 | Every view-like parameter (view, handle, `raw`, `cstring`) carries its own elided lifetime |
| EL-2 | A view-returning function must derive the view from exactly one view-like lend parameter |
| EL-3 | Zero candidates, or two or more, is E10036; annotations do not exist to break the tie |
| EL-4 | A `static str` return needs no candidate: static storage has no input |
| EL-5 | A view parameter passed to another call is a reborrow limited to that call expression |
| EL-6 | Nullable views add `null` as a value; the lifetime rule is unchanged |

A `take` parameter is **not** an elision candidate: it creates no loan, so it
cannot be the source of a returned view. A view return whose only view-like lend
candidate is a `take` parameter is E10036.

The full rules and the checker live in [Ch 16](16-borrow-rules.md).

```clorraine
func pair(a: u8[], b: u8[]): u8[] {
    return a;                    // E10036: two candidates, no way to choose
}

func greeting(): static str {
    return "hello";              // EL-4: no candidate needed
}
```

## 4. Out-parameters

A callee that must report detail writes it through a `borrow` out-parameter,
usually a nullable handle (E1). The caller must initialize the slot on every
path before the call (`let f: File? = null;`); passing an uninitialized slot is
E10021, the ordinary definite-assignment rule (F9).

**Out-parameter ownership.** A `borrow T?` parameter is an in-out ownership
slot. The callee owns the incoming value on entry: normally `null`, and a
non-null incoming value must be an initialized owned handle. The callee may
release that incoming value, so E10034 does not apply to the slot. The callee
is not required to write on every path; afterwards the slot owns the resulting
handle, or stays `null`. Assigning through the slot drops the callee-owned
previous value first, then stores the new one (the O3 carve-out). Callers
normally pass `null`; passing a non-null owned value transfers it into the
slot.

```clorraine
func openInto(path: str, borrow f: File?): void {
    f = openFile(path);          // writes the handle through the borrow
}

func load(path: str): File? {
    let f: File? = null;         // the caller initializes the slot first
    openInto(path, borrow f);
    return f;                    // the callee may have filled the slot
}
```

`null` is the only empty state; narrowing needs an `== null` or `!= null` check
(use before the check is E10010).

```clorraine
let f: File? = null;
openInto(path, borrow f);
if (f != null) {
    readFile(f);
}
```

## 5. Recursion

Recursion is unrestricted (F7): direct, indirect, mutual, and generic
recursion all work. There is no static frame allocation and there is no
recursion diagnostic. The C activation stack supplies the frames, so stack
depth is a runtime concern with documented guidance; see
[Ch 11](11-memory.md).

```clorraine
func countDown(n: i32): void {
    if (n <= 0) { return; }
    countDown(n - 1);
}
```

## 6. Function values

A function value is an exact-signature C function pointer, written
`func(T, ...): R` (F3). A bare identifier names the function; there is no `&`
because `&` is address-of (GF13).

```clorraine
func addOne(x: i32): i32 { return x + 1; }

func applyTwice(x: i32, f: func(i32): i32): i32 {
    return f(f(x));
}

func demo(): i32 {
    return applyTwice(3, addOne);    // bare identifier: no '&'
}
```

### 6.1 Lambdas

Lambda syntax is frozen now; capture semantics are staged (L1, L4). A
capture-less lambda is a plain function pointer and matches `func(...)`; a
capturing lambda used where `func(...)` is required is E10400. A lambda
expression is part of the expression grammar; see [Ch 04](04-expressions.md).

```clorraine
let inc: func(i32): i32 = (x: i32) => x + 1;   // capture-less: a function pointer

// Frozen capture syntax; non-empty captures are staged (E90002)
let bump = [borrow counter] (n: i32) => { counter = counter + n; };

let bad = [take s] () => s;                     // E10403: 'take' captures are rejected
```

| Rule | Statement | Diagnostic |
|---|---|---|
| L-1 | Syntax: `(params) => expr`, `(params) => { ... }`, optional `[borrow x, ...]` capture list | - |
| L-2 | Duplicable captures copy; non-duplicable captures borrow read-only | - |
| L-3 | A lambda that moves a captured binding | E10401 |
| L-4 | Writing a captured binding needs a `[borrow x]` capture | E10402 |
| L-5 | `take` captures are rejected in v1 | E10403 |
| L-6 | A capturing closure is ephemeral: it cannot be returned, stored, or captured | E10404 |
| L-7 | Non-empty captures are staged; the semantic stage reports E90002 | E90002 |

## 7. `comptime func`

`comptime func` syntax is frozen; the semantics are staged (GF22). The parser
accepts the declaration, and the semantic stage reports E90001 until
`comptime` evaluation lands.

```clorraine
comptime func tableSize(n: i32): i32 {
    return n * 4;
}
```

## 8. Generic functions

A function writes its type parameters after the name: `<T, F: func(T): R>`.
Explicit type arguments are allowed, and inference is arguments-only (G1, G2).
The only bound is callable (G6).

```clorraine
func identity<T>(x: T): T {
    return x;
}

func apply<T, F: func(T): T>(x: T, f: F): T {
    return f(x);
}

let a = identity<i32>(5);      // explicit type arguments
let b = identity(5);           // arguments-only inference: T = i32
```

| Rule | Statement | Diagnostic |
|---|---|---|
| G-1 | The only bound is `F: func(T, ...): R`; a violating argument is an error | E10303 |
| G-2 | Inference is arguments-only; a constructor with no value argument needs explicit type arguments (`vecNew<i32>()`) | E10301 |
| G-3 | The wrong number of type arguments | E10300 |
| G-4 | A value of a callable-bound parameter is callable-only: call or pass, never store or return | E10304 |
| G-5 | `export` on a generic declaration is an error; write a concrete wrapper | E10302 |
| G-6 | Instantiation is monomorphized with hash-deduped symbols | - (Ch 17) |
| G-7 | Instantiation recursion has a cap | E10305 |
| G-8 | A generic aggregate cannot use an ephemeral type argument | E10306 |

Monomorphization is a codegen concern; see [Ch 17](17-codegen.md).

```clorraine
// E10302: a generic declaration cannot be exported
export func identity<T>(x: T): T { return x; }

// E10304: the callable-bound value cannot be returned
func keep<T, F: func(T): T>(x: T, f: F): F {
    return f;
}
```

A generic body records the copy obligations it needs (V9) instead of declaring
bounds; the obligation is discharged at each instantiation, and an
instantiation error names the instantiation site, not the generic declaration.
Instantiation recursion is capped (G-7, E10305), and a compiler option can
report instantiation size and count for tuning (G5).

## 9. Entry points

An executable has exactly one entry function, `func main(): i32` (M8). A
missing, duplicated, or mistyped `main` is E11104. A library build instead
exports a generated init entry that the host calls.

```clorraine
func main(): i32 {
    return 0;
}
```

## 10. FFI and intrinsic declarations

An `ffi` block declares an external function with the same signature
vocabulary as a Clorraine function: ordinary modes and ordinary types, with
nullability in the type (X3). Blocks are module-scoped and are not exported
(M9); application code calls libraries through wrappers (X6). See
[Ch 12](12-ffi-contracts.md).

```clorraine
ffi libc {
    func write(fd: i32, buf: raw u8, n: usize): isize;
}
```

An `intrinsic func` declaration is a compiler-provided signature. It has no
body and is restricted to compiler-known names (GF23; grammar section 3.6). It
is the only declaration form that may bind a reserved built-in name
([Ch 01](01-lexical.md), LEX-9); an ordinary `func` that uses a reserved name is
E20006.

```clorraine
intrinsic func readVolatile16(addr: raw u16): u16;
```

## 11. Diagnostics

| Code | Meaning |
|---|---|
| E10024 | Write through a read-only parameter |
| E10033 | A loan escapes its origin |
| E10034 | A borrowed parameter released or stored |
| E10035 | A view of a local owner is returned |
| E10036 | A view return does not derive from exactly one lend parameter |
| E10039 | A call-site marker contradicts the declaration |
| E10210 | Function type mismatch |
| E10300-E10306 | Generic function diagnostics |
| E10400-E10404 | Lambda diagnostics |
| E11104 | Entry-point problems |
| E20006 | An ordinary declaration of a reserved built-in name |
| E90001 | `comptime` semantics are staged |
| E90002 | Lambda captures are staged |
| W10001 | Needless read-only borrow of a duplicable value |
