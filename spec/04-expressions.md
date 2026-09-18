# Chapter 04 - Expressions and Operators

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 02 (types), Ch 03 (ownership), Ch 06 (functions), Ch 07 (structs), Ch 08 (arrays and strings), Ch 09 (enums), Ch 12 (FFI contracts), Ch 13 (data inclusion)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 6 and 7
> **Ledger:** B7, B8, GF2-GF5, GF11, GF13, GF14, F3, F8, F9, T1, T6, T8, T12, S1-S5, E4, E5

## 1. Strategy

Expressions use the C precedence table with the comma operator removed. The
parser is a recursive-descent chain for the precedence levels plus a Pratt
parser for the binary operators, so the binding powers are exactly C's (B7).
The EBNF shape of the chain is in [grammar.ebnf.md](grammar.ebnf.md)
section 6.

| Property | Rule |
|---|---|
| Precedence and associativity | Exactly C's, minus the comma operator (section 2) |
| Comma | Not an operator; it only separates call arguments, list elements, and `for` clauses |
| Assignment result | `void`; chaining and value use are E10217 (GF2) |
| Evaluation order | Strictly left-to-right and deterministic |
| `&&` and `||` | Short-circuit: the right operand runs only when needed |
| Parentheses | Always allowed, and required in the C places |

## 2. Precedence table

Highest binding first. Binary levels associate to the left except where noted.

| Level | Operators | Associativity |
|---|---|---|
| 1 | `f(...)`, `a[i]`, `x.f`, `x++`, `x--` | left |
| 2 | `!`, `~`, unary `-`, `&`, `*`, `++x`, `--x`, `unsafe` | right |
| 3 | `*`, `/`, `%` | left |
| 4 | `+`, `-` | left |
| 5 | `<<`, `>>` | left |
| 6 | `<`, `<=`, `>`, `>=` | left |
| 7 | `==`, `!=` | left |
| 8 | `&` (bitwise AND) | left |
| 9 | `^` | left |
| 10 | `|` | left |
| 11 | `&&` | left |
| 12 | `||` | left |
| 13 | `?:` | right |
| 14 | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `|=`, `^=`, `<<=`, `>>=` | right |

In prefix position `&` is address-of and `*` is raw dereference; in infix
position they are bitwise AND and multiplication (section 7). `?` is the
nullability suffix in type position and the conditional operator here
[grammar.ebnf.md](grammar.ebnf.md) section 11.

## 3. Evaluation order

Evaluation is strictly left-to-right and deterministic. There is no
implementation-defined order.

| Context | Order |
|---|---|
| Binary operator | Left operand, then right operand |
| Function call | Callee expression, then arguments left to right |
| Index | Base expression, then index |
| Conditional `?:` | Condition, then exactly one branch |
| Assignment | Target location, then right-hand side; the old value is dropped before the store (O3) |
| `&&`, `||` | Left operand first; the right operand only when the result is not yet fixed |
| Struct literal | Fields evaluate in source order (the names are order-independent) |
| Array literal | Elements evaluate in source order |
| Array fill form | The fill expression evaluates once; the value is copied to every element |

Arguments are also where view reborrows happen; passing a view is a reborrow
limited to the call (O12) [03-ownership.md](03-ownership.md).

Temporaries created anywhere in a statement — including inside nested calls and
aggregate literals — live until the end of that statement and drop there in
reverse creation order. The cleanup ladder is specified in
[Ch 05](05-statements.md) section 15 and [Ch 17](17-codegen.md).

## 4. Assignment and compound assignment

Assignment keeps the C expression grammar, but its result is `void` (GF2).
Using it as a value is E10217.

```clorraine
x = 1;             // statement: fine
x = y = 1;         // E10217: the inner assignment has no value
let z = (x = 1);   // E10217
```

The target must be a place: a local, a parameter, a struct field, an array
element, or a dereferenced raw pointer, otherwise E10221. Assignment replaces
the old value and drops it (O3). A compound assignment `x op= y` is equivalent to `x = x op y`
with the target evaluated once; the same operator rules apply.

## 5. Operator semantics

Every operator is defined; there is no undefined behavior (B7).

| Operator | Operands | Result | Semantics / diagnostic |
|---|---|---|---|
| `+`, `-`, `*` | integers | the unified integer type | Wrapping two's-complement arithmetic (B7, P12) |
| `+`, `-`, `*`, `/` | `f32`/`f64` | `f32` or `f64` | IEEE 754; mixed `f32`/`f64` unifies to `f64` |
| `/`, `%` | integers | the unified integer type | Truncates toward zero; a zero divisor, or the minimum signed value divided/modulo `-1`, is the runtime fault E11210 |
| `/` | `f32`/`f64` | float | IEEE 754; a zero divisor gives Inf or NaN |
| `<<`, `>>` | integers | left operand's type | Count masked to the operand width; `<<` fills with zero, `>>` is logical for unsigned and arithmetic for signed |
| `<`, `<=`, `>`, `>=` | integers, floats | `bool` | Ordering |
| `==`, `!=` | scalars, enums, `raw`, handles | `bool` | Handles compare by identity; enums compare with `==`/`!=` only (T10a); ordering handles or enums is E10222 |
| `&`, `|`, `^`, `~` | integers | the unified integer type | Bitwise, fixed width |
| `!` | `bool` | `bool` | Logical negation |
| `&&`, `||` | `bool` | `bool` | Short-circuit evaluation |
| `?:` | `bool` condition; same-typed branches | branch type | Right-associative; only the chosen branch evaluates |
| `++`, `--` | an integer or float place | old value (postfix), new value (prefix) | `x++` yields the old value; `++x` yields the new value |
| unary `-` | integer or float | same type | Wrapping for the minimum integer value |
| unary `&` | a place | `raw T` | Address-of; requires `unsafe`, E10601 |
| unary `*` | `raw T` | place of type `T` | Raw dereference; requires `unsafe`, E10600 |

No operator accepts `bool` as an integer. Binary arithmetic, bitwise, and
relational operators unify their operands: when the operand types are already
equal, the unified type is that type; otherwise the compiler applies the T1a
implicit widenings and widens one operand when only one direction applies. Only
a mixed-sign pair, or a non-equal pair that is a widening in neither direction,
is E10200 and needs an explicit cast. The result has the unified type, except
relational operators, which yield `bool`. A literal operand takes the other
operand's type when its value fits, otherwise E10202. `f32` and `f64` unify to
`f64`. The allowed widenings are listed in [Ch 02](02-types.md) section 9; any
other implicit conversion is E10200.

Unary `+` is not adopted; write the operand directly (`x`, not `+x`).

## 6. `bool` and conditions

`bool` holds `true` or `false`. It is not an integer and has no truthiness.
Wherever a condition is required — `if`, `while`, `do`/`while`, the `for`
condition, `?:`, `&&`, `||`, `!`, and `assert` — the operand must have type
`bool`, otherwise E10218. Integers and pointers are not converted, and `bool`
has no arithmetic or bitwise operators.

```clorraine
if (count) { tick(); }        // E10218: count is not bool
if (count != 0) { tick(); }   // correct
let ready = !empty && count > 0;
```

## 7. Pointers, `unsafe`, and `raw`

`raw T` is an unchecked copy pointer (T6). A raw pointer may be declared,
passed, stored, and compared outside `unsafe`. The operations that touch
memory require an `unsafe` context.

| Operation | Spelling | Requires `unsafe` | Diagnostic |
|---|---|---|---|
| Address of a place | `&place` | yes | E10601 |
| Dereference | `*p` | yes | E10600 |
| Member access through raw | `p.field` (dereference implied) | yes | E10600 |
| Pointer arithmetic or raw indexing | `p + n`, `p - n`, `p[i]` | yes | E10602 |
| Pointer-to-pointer cast | `raw T(q)` | yes | E10603 |
| Integer-to-`raw` value cast | `raw T(addr)` | no | - |
| Declare, pass, store, compare | - | no | - |

The `unsafe` prefix operator wraps exactly one unary expression: `let next =
unsafe p.next;` (GF4). The multi-statement form is the `unsafe { ... }`
statement [05-statements.md](05-statements.md). There is no block-expression
value.

```clorraine
struct Node { value: i32; next: raw Node?; }

let p = raw Node(allocMem(sizeof(Node)));   // integer-to-raw: fine outside unsafe
if (p != null) {
    unsafe { p.value = 1; }                 // dereference requires unsafe
    let next: raw Node? = unsafe p.next;     // prefix unsafe, one unary expression
}
```

## 8. Member access and dot-call

One dot serves every member-like access (GF5, GF11).

| Form | Meaning |
|---|---|
| `x.field` | struct field |
| `Type.MEMBER` | enum member, for example `Direction.UP` |
| `A.B.f` | module-qualified name or function |
| `p.field` | member access through `raw`; the dereference is implied and requires `unsafe` |
| `x.f(args)` | dot-call: resolves to the free function `f(x, args)` |

Dot-call resolution is static and decided at compile time. The candidate set
is the receiver's fields first, then free functions visible through the current
module scope (module declarations plus imported names). A field of the receiver
always wins, so a field and a function of the same name never collide. A
candidate function must accept the receiver as its first parameter and match
the argument list under the usual implicit widenings (section 5,
[Ch 02](02-types.md) section 9); no other conversions apply. Resolution is
unambiguous by construction: duplicate declarations and duplicate import binds
are already E11101 and E11103, so at most one function candidate remains. Zero
matches is an ordinary name or member error (E10212 for an unknown member,
E10219 for an undeclared name). There are no methods, no receivers, and no
vtables (F8). Standard-library APIs are written dot-call style, so `xs.len()`
means `len(xs)`.

```clorraine
let n = xs.len();                 // len(xs)
let d = Direction.UP;             // enum member
let v = math.vec.length(a);       // module-qualified free function
```

Call-site markers are optional assertions; a marker that contradicts the
declaration is E10039 [03-ownership.md](03-ownership.md).

## 9. Indexing

`T[]`, `str`, and fixed arrays `T[N]` are indexable. Indexing is always
bounds-checked (T12). There is no checked/unchecked mode and no profile that
disables the check.

| Case | Result |
|---|---|
| Index in range | The element or byte |
| Index out of range at run time | Runtime fault E11211 |
| Constant index known out of range | Compile error E10216 |
| Indexing a non-indexable type | E10209 |
| Indexing a `raw T` | Requires `unsafe`, E10602 |

`len(xs)` gives the element or byte count. Raw memory and the standard-library
`unchecked` accessors are the only unchecked paths, and they live in `unsafe`
(T12).

## 10. Function values

A function value is a bare identifier whose type is `func(...): R`; there is
no `&` (GF13). Function types are exact-signature C function pointers (F3):
no variadics, default arguments, named arguments, tuples, or multiple returns.
A signature mismatch is E10210. Only capture-less lambdas match a `func(...)`
type [grammar.ebnf.md](grammar.ebnf.md) section 8.

```clorraine
func onTick(): void { tick(); }
let handler: func(): void = onTick;   // bare identifier, no '&'
```

## 11. Intrinsics

Intrinsic names are reserved built-ins and are also callable as free
functions, so dot-call works (`xs.len()`). They are listed in
[grammar.ebnf.md](grammar.ebnf.md) section 7.

| Intrinsic | Result | Notes |
|---|---|---|
| `sizeof(type)` | `usize` | Constant expression |
| `offsetof(Type, field)` | `usize` | Constant expression |
| `len(expr)` | `usize` | Compile-time for fixed arrays (GF14) |
| `embed("path"[, "key"])` | byte array | Constant context only, E10213; unreadable file E10214 [13-data-inclusion.md](13-data-inclusion.md) |
| `panic(msg)` | never | Aborts without the cleanup ladder (E4) |
| `assert(cond, msg)` | `void` | A false condition is the runtime fault E11213; aborts without the cleanup ladder (E4); the condition is `bool` |
| `readVolatile8/16/32(addr)` | integer | Requires `unsafe`, E10604 (T7) |
| `writeVolatile8/16/32(addr, value)` | `void` | Requires `unsafe`, E10604 (T7) |

`panic` and `assert` abort immediately, so drops and `defer` blocks do not run
[05-statements.md](05-statements.md). The volatile intrinsics are the audited
hardware primitives (T7); the standard library wraps them.

## 12. Templates

A template string `$"..."` interpolates expressions. `{expr}` is a
placeholder, and `{{` and `}}` are literal braces (S1). A placeholder may hold
any expression, including a struct literal; scanning tracks nested braces and
strings (S4).

```clorraine
let msg = $"Hi, {name}!";                  // owned String
let point = $"at {Point { x: 1, y: 2 }}";
let braces = $"literal {{ and }}";         // str: no placeholders
```

| Rule | Diagnostic |
|---|---|
| Formattable types: `str`, `String`, integer types, `f32`, `f64`, `bool` | E10500 for any other type |
| No format specs in v1 (`{x:04x}`); use `toHex` or `toDecimal` | E10501 |
| No nested templates | E90005 |
| Braces must balance | E10503 |
| Placeholders present: the result is an owned `String`; absent: a plain `str` | - (S5) |

A `:` directly inside a placeholder is a format specifier (E10501). It is
recognized by the parser, which reports E10501 before semantic analysis, in the
same way the parser reports E10204 and E10208.

Float placeholders print with a fixed precision of six significant digits in
v1 (S2a); precision control is deferred (S6). A template
with placeholders allocates one `String`; a placeholder-free template is a
`str` and allocates nothing (S5).

## 13. Literal typing

There are no literal suffixes (GF3). A literal is typed by its context.

| Literal | Type |
|---|---|
| Decimal integer | The context type; default `i32` |
| Hex or binary integer | The context type; may use the full unsigned width of the target (`0xFFFFFFFF` is a valid `u32`) |
| Float | `f64`; `f32` in an `f32` context; mixed arithmetic promotes to `f64` |
| Character `'x'` | `u8` byte literal |
| String `"..."` | `static str`, which widens to `str` |
| Template `$"..."` | `String` with placeholders, `str` without |
| `true`, `false` | `bool` |
| `null` | The `T?` type of the context |

A literal that does not fit its context type is E10202. Integer-to-float and
float-to-integer conversions are explicit.

## 14. Conversions and casts

Casts use exactly one form, `T(expr)` (B8). There are no C-style casts.

| Cast | Form | Notes |
|---|---|---|
| Scalar cast | `u32(x)`, `i16(y)`, `f64(n)` | Numeric conversion; integer-to-float and float-to-integer are always explicit |
| Float-to-integer | `i32(f)` | Out-of-range value is the runtime fault E11212 |
| `raw` value cast | `raw T(expr)` | Integer-to-`raw` is allowed outside `unsafe` |
| `raw`-to-integer value cast | `usize(p)` | Reads the address as a number; allowed outside `unsafe` (T6a) |
| `raw` pointer cast | `raw T(p)` | Pointer-to-pointer; requires `unsafe`, E10603 |
| Enum cast | `Direction(1)` | Parses as a call, resolved semantically into an enum cast |
| Handle cast | `File(0)` | Parses as a call, resolved semantically into a handle cast |

The float-to-integer runtime check is `cl_isnan(f) || !(f >= lo && f <= hi)`, so
a NaN value faults E11212, as does a value outside the target range. The
`cl_isnan` test reads the IEEE bit pattern directly, so the check does not rely
on the platform math library's comparison behavior ([Ch 17](17-codegen.md) §10,
§12).

An implicit conversion the language does not allow is E10200. A cast it does
not allow is E10201.

## 15. Aggregate literals

Struct literals use named fields and are order-independent; `Point {}` is the
explicit zero value (F4) [07-structs.md](07-structs.md). Array literals are
`[]`, `[1, 2, 3]`, and the fill forms `[1, 2; 0]` and `[; 0]`; a fill without
a fixed extent in context is E10215 [08-arrays-strings.md](08-arrays-strings.md).

## 16. Constant expressions

A **constant expression** is an expression the compiler can evaluate at
compile time. It may use:

* literals;
* `const` bindings whose initializers are constant;
* enum members;
* `sizeof`, `offsetof`, and `len` of fixed things;
* arithmetic, comparison, and bitwise operators whose operands are constant;
* scalar casts and integer-to-`raw` casts (`raw T(addr)`, GF6);
* `embed` results.

```clorraine
const WIDTH: usize = 4;
const BYTES: usize = WIDTH * 2;            // 8
const MASK: u32 = (u32(1) << 8) - 1;       // 0xFF
```

Folding uses the runtime semantics of section 5: integer arithmetic wraps and
shift counts are masked. A constant expression that divides by zero, or that
overflows the minimum signed value divided or taken modulo `-1`, is E10223. A
constant float-to-integer conversion that overflows is E10202, not E10223. A
form in a constant context that cannot be folded is E10224. A constant value
that does not fit its context type is E10202.

Constant contexts are array extents, enum member values, `case` labels,
module-level `const` initializers, and `embed` arguments. A local `const` may
also have a runtime initializer (V2), and a module-level `let` initializer is
runtime too; neither is a constant context. Only the module-level `const`
initializer must be constant. The syntax is in
[grammar.ebnf.md](grammar.ebnf.md) section 6.8.

## 17. Rule summary

| Rule | Statement | Diagnostic |
|---|---|---|
| EX-1 | C precedence and associativity, minus the comma operator | - |
| EX-2 | The assignment result is `void` | E10217 |
| EX-3 | Left-to-right evaluation; `&&`/`||` short-circuit | - |
| EX-4 | Integer arithmetic wraps; shifts mask the count | - |
| EX-5 | Integer `/` or `%` by zero, or the minimum signed value over `-1`, faults | E11210 |
| EX-6 | A float-to-integer conversion out of range or of NaN faults | E11212 |
| EX-7 | `bool` is not an integer and has no truthiness; a non-`bool` condition is E10218 | E10218 |
| EX-8 | Casts are only `T(expr)` | E10200/E10201 |
| EX-9 | Raw operations require `unsafe` | E10600-E10603 |
| EX-10 | Indexing is always checked | E10209/E10216/E11211 |
| EX-11 | Function values are bare identifiers | E10210 |
| EX-12 | Volatile intrinsics require `unsafe` | E10604 |
| EX-13 | No format specs and no nested templates | E10501/E90005 |
| EX-14 | An assignment target must be a place | E10221 |
| EX-15 | A false `assert` aborts with a runtime fault | E11213 |
| EX-16 | Constant expressions fold with runtime semantics; a division by zero or MIN/-1 overflow is E10223, a non-constant form is E10224, and a float-to-int overflow is E10202 | E10223/E10224/E10202 |
