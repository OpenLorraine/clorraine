# Chapter 02 - Type System

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 03 (ownership), Ch 04 (expressions), Ch 06 (functions), Ch 07 (structs), Ch 08 (arrays and strings), Ch 09 (enums), Ch 12 (FFI contracts), Ch 14 (diagnostics)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) section 4
> **Ledger:** T1-T12, V5, V8, B7, B8, GF3 ([decisions.md](decisions.md))

This chapter defines the types a Clorraine value can have. It fixes their
widths, their nullability, their copy behavior, and the conversions the
language allows. Ownership of those values is [Ch 03](03-ownership.md);
expressions and casts are [Ch 04](04-expressions.md).

## 1. The type universe

Every type is one of the forms below. `void` is special: it is a return type
only and never describes a value.

| Form | Example | Chapter |
|---|---|---|
| Scalar | `i32`, `u8`, `f64`, `bool`, `usize` | section 2 |
| Named | `Point`, `Direction`, `File`, `String` | section 4 |
| View | `str`, `cstring`, `static str`, `T[]` | section 6 |
| Raw pointer | `raw T`, `raw T?` | section 7 |
| Fixed array | `T[N]` | [Ch 08](08-arrays-strings.md) |
| Function | `func(T): R` | [Ch 06](06-functions.md) |

`value_type` may carry the nullability suffix `?`; `type` is `void` or a
`value_type`. A value type may also be a generic reference such as
`Array<i32>`. A function type is `func(T): R`, an exact-signature C function
pointer (F3); a signature mismatch is E10210 ([Ch 06](06-functions.md)).

## 2. Scalar types

Scalars have fixed widths. There is no plain `int` and no 64-bit fixed-width
integer in v1.

| Type | Meaning | Notes |
|---|---|---|
| `i8` `i16` `i32` | signed two's-complement integer | widths 8, 16, 32 |
| `u8` `u16` `u32` | unsigned integer | widths 8, 16, 32 |
| `usize` `isize` | pointer-sized integer | 32-bit on m68k, 64-bit on ARM64 |
| `f32` `f64` | IEEE 754 binary floating point | 32-bit and 64-bit |
| `bool` | logical value | not an integer (section 3) |

`usize` and `isize` are the portable choices for sizes, indices, and addresses.
Code that must track a memory layout uses the fixed-width types instead.

```clorraine
let delta: i16 = -5;
let size: usize = 4096;      // pointer-sized
let ratio: f64 = 0.5;
```

`void` is written as the return type of a function that returns nothing. It is
not a parameter type, a field type, or a variable type.

```clorraine
func reset(): void { }
```

## 3. `bool` is not an integer

`bool` holds `true` or `false`. It has no truthiness: an integer or a pointer
is never accepted where a condition is required. This keeps conditions
explicit and matches the defined-operator rule (B7).

| Rule | Statement | Diagnostic |
|---|---|---|
| TY-1 | `bool` is not an integer; it converts to or from one only through an explicit cast | E10200 |
| TY-2 | A condition must have type `bool` | - ([Ch 04](04-expressions.md)) |
| TY-3 | `bool` has no arithmetic or bitwise operators | - ([Ch 04](04-expressions.md)) |

```clorraine
let n: u32 = 3;
if (n) { }                 // rejected: n is not bool
if (n != 0) { }            // correct

let flag: bool = true;
let bit: u8 = u8(flag);    // explicit: yields 0 or 1
let back: bool = bool(n);  // explicit: false only for 0
```

Conditions occur in `if`, `while`, `do`/`while`, the `for` condition, `?:`,
`&&`, `||`, `!`, and `assert` ([Ch 04](04-expressions.md)).

## 4. Named types

A named type is a reference to a declaration: a struct, an enum, a handle, or
the standard-library `String`. Named types may take explicit type arguments.

| Named type | Kind | Where |
|---|---|---|
| Struct | aggregate of named fields | [Ch 07](07-structs.md) |
| Enum | integer constants with a backing type | [Ch 09](09-enums-unions.md) |
| Handle | opaque owned resource (`File`, `Buffer`) | [Ch 12](12-ffi-contracts.md) |
| `String` | owned string handle | [Ch 08](08-arrays-strings.md) |

```clorraine
struct Point { x: i32; y: i32; }
enum Direction : u8 { UP, DOWN }
handle File freedBy Close;
```

An enum's backing type must be an integer type, otherwise E10211
([Ch 09](09-enums-unions.md)). A handle is move-only and releases through its
`freedBy` deleter ([Ch 03](03-ownership.md)). `String` is the standard-library
owned handle for text.

An **overlay type** is a struct with at least one `volatile` field. It
describes memory the program does not own, such as memory-mapped registers. An
overlay type never exists as a value: it is reached only through a `raw`
pointer inside `unsafe` (T7, T11). Using an overlay type by value is E10207;
`volatile` in any other position is E10208. See [Ch 07](07-structs.md)
section 7.

## 5. Nullability

`T?` adds the single empty state `null`. It applies only to pointer-like types:
handles, `raw`, `cstring`, and views. Scalars, `bool`, enums, and structs
cannot be nullable.

| Rule | Statement | Diagnostic |
|---|---|---|
| TY-4 | `T?` is allowed only on handles, `raw`, `cstring`, and views | E10011 |
| TY-5 | `T` widens implicitly to `T?` | - |
| TY-6 | Using a nullable value without a narrowing check is an error | E10010 |
| TY-7 | `null` used where a non-nullable type is required is an error | E10012 |

Narrowing is written as an explicit comparison. After `x != null` succeeds, the
value is known to be non-null inside the guarded block; after `x == null`, the
non-null branch applies.

```clorraine
import { File, openInto, openFile } from std.io;

let f: File? = openFile("RAM:a");   // openFile returns File?; it may fail
if (f != null) {
    let g: File = f;                // narrowed to File (TY-6)
    let h: File? = g;               // File widens to File? (TY-5)
}
// f.name                           // E10010: used before the check

let bad: i32? = null;               // E10011: scalars cannot be nullable
```

`null` is the **only** empty state. There are no zero literals for handles and
no `T{}` empty values (T8). The out-parameter pattern uses `borrow T?`:

```clorraine
func dump(path: str): void {
    let f: File? = null;
    openInto(path, borrow f);   // the callee writes a handle or leaves it null
    if (f == null) {
        return;
    }
}
```

There is no `?.` or `??` operator; nullable code uses ordinary `if` and `switch`
([Ch 04](04-expressions.md), [Ch 05](05-statements.md)).

## 6. Views

A view points at storage owned elsewhere. It copies freely, because a copy is
just a pointer plus a length (or a handle to static bytes). Views never own and
never release.

| View | Refers to | Mutability |
|---|---|---|
| `str` | a byte sequence | immutable |
| `cstring` | a zero-terminated C byte pointer | immutable |
| `static str` | bytes with static origin | immutable |
| `T[]` | a slice of an array or buffer | read-only unless derived from a `borrow` |

`static str` widens to `str` implicitly. `String` widens to `str` as a view of
its bytes ([Ch 08](08-arrays-strings.md)). A slice is created from a view
parameter, a view return, or a standard-library helper; there is no local view
expression (O9).

| Rule | Statement | Diagnostic |
|---|---|---|
| TY-8 | A function returning a view must derive it from exactly one view-like lend parameter | E10036 |
| TY-9 | A `static str` return needs no candidate parameter | - |
| TY-10 | A view-typed struct field is banned except `static str` | E10205 |

```clorraine
func head(buf: u8[], n: usize): u8[] {
    return slice(buf, 0, n);   // derives from the single view-like lend
}

func greeting(): static str {
    return "hello";            // static storage needs no parameter
}

struct Bad { part: str; }      // E10205: a view-typed field
struct Good { tag: static str; }
```

Lifetime elision (EL-1..EL-6) is specified in [Ch 16](16-borrow-rules.md).

## 7. Raw pointers

`raw T` is an unchecked copy pointer (T6). It is duplicable and is nullable as
`raw T?`. The type is the escape hatch below the memory-safety boundary: some
operations on it require `unsafe`, and the rest are safe.

| Operation | Requires `unsafe` | Diagnostic |
|---|---|---|
| Declare, pass, store, compare a `raw` pointer | no | - |
| Integer-to-`raw` value cast (`raw T(addr)`) | no | - |
| Dereference (`*p`, implied by `p.field`) | yes | E10600 |
| Address-of a place (`&place`) | yes | E10601 |
| Pointer arithmetic or raw indexing (`p + n`, `p[i]`) | yes | E10602 |
| Pointer-to-pointer cast (`raw T(q)`) | yes | E10603 |

```clorraine
let addr: raw u8 = raw u8(0x00DFF000);   // integer-to-raw: safe
let spare: raw u8? = null;               // nullable raw pointer
if (addr != raw u8(0)) { }                // comparing raw: safe
unsafe {
    let first = *addr;                    // dereference: unsafe
}
```

`unsafe` is written as a block or as a prefix operator
([Ch 04](04-expressions.md)). Ownership never flows through a raw pointer
automatically; converting a handle to or from `raw` is explicit
([Ch 03](03-ownership.md), [Ch 12](12-ffi-contracts.md)).

## 8. Duplicability

Duplicability decides whether a value copies or moves (V8). It is **derived
from the type's fields and elements**, never from a marker. There is no `copy`
keyword and no `copy struct`.

| Type | Duplicable? |
|---|---|
| Scalars, `bool`, floats, enums, `usize`/`isize` | yes |
| Function types `func(...): R` | yes |
| Views (`str`, `cstring`, `static str`, `T[]`) | yes (a cheap copy) |
| `raw T` and `raw T?` | yes |
| Struct whose every field is duplicable | yes |
| Fixed array whose element type is duplicable | yes |
| Handle (`File`, `Buffer`, `String`, user handles) | no; move-only |
| Struct with any handle or resource field | no; move-only |
| Array with a move-only element type | no; move-only |
| Overlay (`volatile`) type | never a value at all |

```clorraine
struct Point { x: i32; y: i32; }          // duplicable
struct Label { name: String; n: u32; }    // move-only: String is a handle

let a = Point { x: 1, y: 2 };
let b = a;                                // copy
let l1 = Label { name: stringOf("hi"), n: 1 };
let l2 = l1;                              // move; l1 is no longer usable
```

The copy and move rules are in [Ch 03](03-ownership.md).

## 9. Conversions and casts

There is exactly one cast form: `T(expr)` (B8). C-style casts do not exist.
Some conversions happen implicitly; everything else is an explicit cast.

These implicit widenings are allowed and no others:

| From | To | Kind |
|---|---|---|
| `T` | `T?` | add the empty state |
| `static str` | `str` | view of static bytes |
| `String` | `str` | view of owned bytes |
| `u8` | `u16`, `u32` | same-signedness integer widening |
| `u16` | `u32` | same-signedness integer widening |
| `i8` | `i16`, `i32` | same-signedness integer widening |
| `i16` | `i32` | same-signedness integer widening |
| `f32` | `f64` | float widening |

Rules:

| Rule | Statement | Diagnostic |
|---|---|---|
| TY-11 | There is no implicit mixed-sign or narrowing conversion | E10200 |
| TY-12 | Integer-to-float and float-to-integer conversions are explicit | E10200 |
| TY-13 | An unlisted implicit conversion is an error | E10200 |
| TY-14 | A cast the language does not support is an error | E10201 |
| TY-15 | A literal must fit its context type | E10202 |

A `raw` pointer converts to an integer only through an explicit value cast
(`usize(p)`), which is allowed outside `unsafe` and reads the address as a
number without dereferencing (T6a).

```clorraine
let a: u8 = 200;
let b: u16 = a;        // implicit widening, same signedness
let c: u32 = b;        // implicit widening

// let e: i32 = c;     // E10200: u32 -> i32 is mixed-sign
// let f: u8 = c;      // E10200: u32 -> u8 narrows
let g: i32 = i32(c);   // explicit mixed-sign cast

let n: i32 = 3;
// let h: f64 = n;     // E10200: integer-to-float is explicit
let i: f64 = f64(n);   // explicit integer-to-float
let j: i32 = i32(i);   // explicit float-to-integer; out of range is E11212

let p: f32 = 1.5;      // context types the literal
let q: f64 = p;        // implicit f32 -> f64
```

Numeric literals carry no suffix (GF3). A decimal literal is typed by its
context and defaults to `i32`; a hex or binary literal may use the full
unsigned width of the target. The literal rules are in
[Ch 01](01-lexical.md) and [Ch 04](04-expressions.md).

## 10. Defined value semantics

Every arithmetic and shift operation is defined; there is no undefined behavior
(B7). The full operator table is in [Ch 04](04-expressions.md).

| Operation | Defined semantics | Diagnostic |
|---|---|---|
| Integer `+`, `-`, `*` | wrapping two's-complement arithmetic | - |
| Shift `<<`, `>>` | count masked to the operand width | - |
| Signed `>>` | sign-replicating (arithmetic) | - |
| Unsigned `>>` | zero-filling (logical) | - |
| Integer `/`, `%` | truncate toward zero; a zero divisor is a runtime fault | E11210 |
| Float arithmetic | IEEE 754; division by zero gives Inf or NaN | - |
| Float-to-integer cast | explicit; an out-of-range value is a runtime fault | E11212 |

For example, `250 + 10` in `u8` is `4` (wrapping, not an overflow trap), and a
shift count is masked to the operand width, so `u32(1) << 32` is `1`. Floats
follow IEEE 754: `1.0 / 0.0` is `Inf` and `0.0 / 0.0` is `NaN`, unlike
the integer rule. This difference is deliberate (T1, B7).

## 11. Diagnostics

| Code | Trigger |
|---|---|
| E10010 | a possibly-null value used without a narrowing check |
| E10011 | `T?` on a type that cannot be empty |
| E10012 | `null` where a non-nullable type is required |
| E10200 | an implicit conversion the language does not allow |
| E10201 | a cast the language does not allow |
| E10202 | a literal that does not fit the context type |
| E10205 | a view-typed struct field other than `static str` |
| E10207 | an overlay (volatile) type used as a value |
| E10208 | `volatile` on something that is neither a field nor a module-level `let` |
| E10210 | a function type mismatch |
| E10211 | an enum backing type that is not an integer type |
| E11210 | runtime fault: integer division or modulo by zero |
| E11212 | runtime fault: float-to-integer conversion out of range |

All message text lives in [Ch 14](14-diagnostics.md).
