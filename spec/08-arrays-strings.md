# Chapter 08 - Arrays, Slices, and Strings

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 02 (types), Ch 03 (ownership), Ch 07 (structs), Ch 11 (memory), Ch 14 (diagnostics), Ch 16 (borrow rules) *(informational)*
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 4, 6.7, 7
> **Ledger:** T2-T5, T9, T12, O2, F1, GF9, GF16, S1-S5, F8, section 18

## 1. Fixed arrays

A fixed array has a compile-time extent: `T[N]`. The extent is a constant
expression, so literals and `const` values may size it. A fixed array lives
where its binding lives; it never decays to a pointer.

```clorraine
let primes: u32[4] = [2, 3, 5, 7];
const WIDTH: usize = 8;
let row: u8[WIDTH] = [; 0];
```

| Rule | Statement | Diagnostic |
|---|---|---|
| AR-1 | The extent is a compile-time constant expression | - |
| AR-2 | A fixed array does not decay to a pointer | - |
| AR-3 | Extents may repeat (`T[N][M]`); only the outermost may be omitted | - |
| AR-4 | There are no dynamic-length arrays and no jagged forms | - |
| AR-5 | An extent that is not greater than zero (`T[0]`) is rejected | E10226 |

`T[][M]` is a slice whose elements are `T[M]` arrays. An omitted inner extent
has no meaning and is rejected with E10220. Every extent must be greater than
zero: an extent that is not, such as `u8[0]`, is E10226, because C99 has no
zero-length array. A
growable sequence is the standard-library `Array<T>` (section 9), not a
language form.

## 2. Array literals

```ebnf
array_literal      = "[" , [ array_init_content ] , "]" ;
array_init_content = expression , { "," , expression }
                   | expression , { "," , expression } , ";" , expression
                   | ";" , expression ;
```

`[]` is the empty literal. `[a, b, c]` lists elements. `[a, b; fill]` lists a
prefix and fills the rest, and `[; fill]` fills every element. Elements are
typed by the expected type, not by the literal alone.

| Rule | Statement | Diagnostic |
|---|---|---|
| AL-1 | The fill forms need a fixed extent in context and a duplicable fill value, and a literal prefix longer than the extent is rejected | E10215 |
| AL-2 | Elements are typed by the expected type | - |
| AL-3 | `[]` is the empty literal | - |

```clorraine
let a: u32[4] = [2, 3, 5, 7];
let b: u8[8]  = [1, 2; 0];   // 1, 2, then six zero bytes
let c: u8[8]  = [; 0];       // all zero
let d: u8[3][2] = [[1, 2], [3, 4], [5, 6]];

let bad = [; 0];             // E10215: no fixed extent in context
let over: u8[2] = [1, 2, 3]; // E10215: three elements do not fit u8[2]
```

## 3. Slices

A slice `T[]` is a view: a pointer plus a length. It borrows storage owned
elsewhere and never owns it.

| Rule | Statement | Diagnostic |
|---|---|---|
| SL-1 | A slice is a view (pointer + length); it never owns storage | - |
| SL-2 | `len(xs)` is the element count: compile-time for a fixed array, runtime for a slice and `str` | - |
| SL-3 | Pointer arithmetic on a slice is not available in safe code | - |

`len` is an intrinsic ([grammar.ebnf.md](grammar.ebnf.md) section 7). Dot-call
works: `xs.len()` resolves to `len(xs)` (F8).

```clorraine
func total(xs: u32[]): u32 {
    let sum: u32 = 0;
    for (x in xs) {
        sum = sum + x;
    }
    return sum;
}

func tail(buf: u8[]): u8[] {
    return slice(buf, 1, len(buf));   // stdlib view into the same storage
}
```

Slices come from four sources: view parameters, view returns, the
standard-library `slice` helper, and the standard-library unsafe
slice-from-raw helper. There is no grammar form that slices from a raw pointer
(GF16); the helper takes `raw T` plus a length and returns `T[]` inside
`unsafe` (T6).

## 4. Indexing is always checked

Indexing a slice, a fixed array, or `str` is bounds-checked on every access.
There is no unchecked mode, and no profile disables the check (T12).

| Rule | Statement | Diagnostic |
|---|---|---|
| IX-1 | Indexing is bounds-checked at run time; out of range is a fault | E11211 |
| IX-2 | A constant index known out of range is a compile error | E10216 |
| IX-3 | Indexing a type that is not indexable is a compile error | E10209 |

```clorraine
func byteAt(s: str, i: usize): u8 {
    return s[i];             // E11211 at run time if i >= len(s)
}

let primes: u32[4] = [2, 3, 5, 7];
let x = primes[9];           // E10216: 9 is out of range for u32[4]
let n: u32 = 7;
let y = n[0];                // E10209: u32 is not indexable
```

The checks run in every build. Interrupt handlers and hot paths use the
explicit `raw`/`unsafe` accessors or the standard-library `unchecked` helpers
instead (T12).

## 5. Element loops

The element form of `for` walks a fixed array or a slice
([grammar.ebnf.md](grammar.ebnf.md) section 5.2).

| Rule | Statement | Diagnostic |
|---|---|---|
| LOOP-1 | `for (x in xs)` copies each duplicable element | - |
| LOOP-2 | `for (borrow e in xs)` binds an exclusive element view | - |
| LOOP-3 | Non-duplicable elements require the `borrow` form | E10040 |

An element view aliases the element: writes through `for (borrow e in xs)`
reach the array even when the element type is duplicable, because only the
view (an index or a pointer) is ever copied, not the element (V9).

```clorraine
func doubleAll(borrow xs: u32[]): void {
    for (borrow e in xs) {
        e = e * 2;           // writes through the element view
    }
}

func inspect(files: File[2]): void {
    for (f in files) { }     // E10040: File is not duplicable
}

func releaseAll(borrow files: File[2]): void {
    for (borrow f in files) { }   // exclusive per-element view
}
```

The loop variable borrows one element at a time. It never owns the element and
cannot leave the loop body.

## 6. Arrays of move-only elements

An array whose element type is move-only is itself move-only (duplicability is
derived from elements and fields; V8). At scope exit its elements drop
first-to-last, then the array storage itself (O2).

```clorraine
func pair(): String[2] {
    let a: String = stringOf("a");
    let b: String = stringOf("b");
    return [a, b];           // both handles move into the array
}
// At the caller's scope exit: element 0 drops first, then element 1 (O2).
```

## 7. Strings

Clorraine has four string-related types, each with one job.

| Type | Kind | Ownership | Notes |
|---|---|---|---|
| `String` | owned handle | move-only | one `[len][cap][bytes][NUL]` block; mutable through `borrow`; keeps the trailing NUL for FFI |
| `str` | immutable view | copies freely | pointer + length; byte-oriented |
| `cstring` | raw view | copies freely | zero-terminated pointer for the C boundary |
| `static str` | static value | copies freely | literal with static origin; widens to `str` |

Two widenings are implicit: `static str` widens to `str`, and `String` widens
to `str` (a view of the owned bytes). Widening a `String` to `str` creates a
shared loan of that `String`, live until the end of the derived view's origin;
mutating, moving, or reallocating the `String` while such a view is live is
E10031 / E10022. Every other conversion is an explicit, named call, and no
conversion allocates silently (T5).

The helpers that return a view or `cstring` over owned storage — `bytes`,
`toCString`, `slice`, and `bufferBytes` — likewise create a shared loan of that
owner for the result's origin, so the same E10031 / E10022 rules apply while
the result is live ([Ch 16](16-borrow-rules.md) section 2).

| Conversion | Direction | Allocation |
|---|---|---|
| `stringOf(v)` | value or `str` to `String` | allocates |
| `clone(s)` | `String` to an independent `String` | allocates |
| `bytes(s)` | `String` to its `u8[]` bytes | none |
| `text(v)` | formattable value to its text form | allocates for a `String` |
| `toCString(s)` | `String` or `str` to `cstring` | none |

These are standard-library functions; the library reference owns their exact
signatures. The direction is fixed by the ledger (T5).

```clorraine
let owned: String = stringOf("hello");   // one allocation
let view: str = owned;                    // implicit widening, no allocation
let copy: String = clone(owned);          // a second, independent owner
let byteView = bytes(owned);              // u8[] over the same bytes
let c = toCString(owned);                 // cstring for an FFI call
```

A `borrow` lend of a `String` permits mutation; the standard-library mutators
may reallocate the single block, so the lend is exclusive.

A struct field may hold `String`, `cstring`, or `static str`, but never a
plain `str` (T4):

```clorraine
struct Entry {
    name: String;        // owned; dropped with the struct
    tag: static str;     // literal with static origin
    path: cstring;       // raw zero-terminated pointer
}

struct Bad {
    text: str;           // E10205: view-typed field
}
```

A function that returns a view type derives it from exactly one view-like lend
parameter ([Ch 16](16-borrow-rules.md), EL-1..EL-6). `static str` returns need
no candidate:

```clorraine
func head(buf: u8[], n: usize): u8[] {
    return slice(buf, 0, n);   // the single view-like lend is buf (EL-2)
}

func greeting(): static str {
    return "hello";            // static storage needs no candidate (EL-4)
}
```

## 8. Templates

`$"..."` is the string-formatting form (S1-S5). A template with no
placeholders is a plain `str`; with placeholders it is an owned `String` (one
allocation).

| Rule | Statement | Diagnostic |
|---|---|---|
| TP-1 | Formattable types: `str`, `String`, integer types, `f32`/`f64`, `bool` | E10500 |
| TP-2 | No format specs in v1 | E10501 |
| TP-3 | `{{` and `}}` are literal braces; unbalanced braces are an error | E10503 |
| TP-4 | No nested templates; struct literals inside placeholders are allowed | - |

```clorraine
func label(name: str, n: u32): String {
    return $"name={name} n={n}";   // owned String, one allocation
}

let literal: str = $"hello";        // no placeholders: plain str
```

## 9. Standard-library containers

`Buffer` (owned bytes) and `Array<T>` (growable) are standard-library types,
not language types (ledger section 18). See [Ch 11](11-memory.md) for their
scope; the v1 library signatures are in
[appendix-stdlib.md](appendix-stdlib.md). Standard-library APIs are written
dot-call style: `x.f(y)` resolves to the free function `f(x, y)` with static
resolution and no methods (F8).

```clorraine
import { Array, arrayNew } from std.mem;   // names illustrative

let xs: Array<i32> = arrayNew<i32>();       // explicit type argument (G2)
let n = xs.len();                            // dot-call resolves to len(xs)
```

## 10. Diagnostics

| Code | Trigger |
|---|---|
| E10040 | element loop over non-duplicable elements without `borrow` |
| E10205 | view-typed struct field other than `static str` |
| E10209 | indexing a type that is not indexable |
| E10215 | array fill literal without a fixed extent in context, a fill value whose element type is not duplicable, or a literal prefix longer than the extent |
| E10216 | constant index into a fixed array is out of range |
| E10220 | inner extent omitted in a multi-dimensional type |
| E10226 | an extent that is not greater than zero (`u8[0]`) |
| E10500 | non-formattable type in a template placeholder |
| E10501 | format specifier inside a placeholder |
| E10503 | unbalanced braces in a template |
| E11211 | runtime fault: index out of range |

All message text lives in [Ch 14](14-diagnostics.md).
