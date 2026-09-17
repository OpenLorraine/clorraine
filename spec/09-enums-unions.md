# Chapter 09 - Enums and Tagged Data

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 02 (types), Ch 04 (expressions), Ch 05 (statements), Ch 12 (FFI contracts)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.3, 5.3, 6.6
> **Ledger:** T10, E6, T8, B5, F8, G1-G8

## 1. Enums

An enum is a named set of integer constants with a distinct type. The backing
integer type is required.

```ebnf
enum_decl   = [ "export" ] , "enum" , identifier , ":" , scalar_type
            , "{" , enum_member , { "," , enum_member } , [ "," ] , "}" ;
enum_member = identifier , [ "=" , const_expression ] ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| EN-1 | A backing integer type is required: `i8`-`u32`, `usize`, or `isize` | E10211 |
| EN-2 | Members are scoped and reached with `.`: `Direction.UP` | E10212 |
| EN-3 | Member values are constant expressions | - |
| EN-4 | Conversions are explicit casts only; there is no implicit conversion | - |
| EN-5 | Enums are duplicable values | - |
| EN-6 | Enums compare with `==` and `!=` only; ordering an enum is E10222 | E10222 |

```clorraine
export enum Direction : u8 {
    UP,
    DOWN,
    LEFT = 10,
    RIGHT,
}

let dir: Direction = Direction.UP;    // scoped member access (Ch 04)
let n: u8 = u8(dir);                   // explicit cast out of the enum
let back: Direction = Direction(2);    // explicit cast into the enum
let bad = Direction.SIDEWAYS;          // E10212: no such member
```

A member with no `=` continues the C sequence: the first member is 0, and each
later member is the previous value plus one. A backing type outside the
integer set is E10211. Enums compare with `==` and `!=` only; ordering an enum
with `<`, `<=`, `>`, or `>=` is E10222 (T10a).

## 2. No tagged unions and no `match`

Clorraine has no tagged-union type and no `match` statement. This is permanent
(T10), not a staging decision. A `union` declaration is rejected with E10204,
whose help points at the supported pattern.

| Rule | Statement | Diagnostic |
|---|---|---|
| UN-1 | A `union` declaration is not part of Clorraine | E10204 |
| UN-2 | There is no `match` statement | - |

## 3. Tagged data as a struct plus `switch`

The supported model is a C-style tagged struct: one tag field, plus a field
for every variant payload. All fields are always present. The fields that do
not belong to the active variant hold their zero value, and zero is a valid
"nothing here" state for every field (O7). Read the active payload with a
C-exact `switch` ([Ch 05](05-statements.md)): fallthrough is the default and
there is no exhaustiveness requirement (B5).

| Rule | Statement | Diagnostic |
|---|---|---|
| TS-1 | One tag field plus every payload field, always present | - |
| TS-2 | Inactive payload fields stay zero-safe | - |
| TS-3 | Select behavior with `switch`; check the tag before reading a payload | - |

```clorraine
enum ShapeKind : u8 {
    CIRCLE,
    RECT,
}

struct Shape {
    kind: ShapeKind;
    radius: i32;    // CIRCLE only
    width: i32;     // RECT only
    height: i32;    // RECT only
}

func area(s: Shape): i32 {
    switch (s.kind) {
        case ShapeKind.CIRCLE:
            return 3 * s.radius * s.radius;
        case ShapeKind.RECT:
            return s.width * s.height;
        default:
            return 0;   // zero-safe: every field is always present
    }
}

let rect = Shape { kind: ShapeKind.RECT, radius: 0, width: 4, height: 3 };
```

A struct literal lists every field exactly once, so the unused payload fields
are written as zeros (F4). A payload may hold a handle; the struct then follows
the ordinary ownership rules for its fields ([Ch 03](03-ownership.md)).

## 4. Error detail: status structs

A recoverable failure with detail uses a plain status struct (E6). There is no
language-level result type and no optional type; such concepts are not part of
Clorraine. A nullable `T?` is detail-free and only says "empty", so use it
when that is the whole story (T8). A `borrow` out-parameter is the other
option.

| Rule | Statement | Diagnostic |
|---|---|---|
| ST-1 | Detail-carrying failure: a status struct or a `borrow` out-parameter | - |
| ST-2 | `T?` is detail-free; `null` is the only empty state (T8) | - |
| ST-3 | Details are fields or out-parameters, never a language result type | - |

```clorraine
struct ParseResult {
    ok: bool;
    value: i32;
    error: u32;
}

func parseInt(s: str): ParseResult {
    let out = ParseResult { ok: false, value: 0, error: 0 };
    if (len(s) == 0) {
        out.error = 1;              // empty input
        return out;
    }
    out.ok = true;
    out.value = i32(s[0]) - i32('0');
    return out;
}

let r = parseInt("42");
if (!r.ok) {
    // inspect r.error for the detail
}
```

A failure whose only useful fact is "it did not work" uses `T?`:

```clorraine
func openOrNull(path: str): File? {
    return openFile(path);   // null means failed; no detail travels
}
```

## 5. C unions at the FFI boundary

A C library `union` stays behind its FFI wrapper. Application code never
writes `ffi` blocks ([Ch 12](12-ffi-contracts.md)) and never touches a C union
directly. The wrapper reads the tag and copies the active field into a tagged
struct, so the rest of the program sees the patterns in section 3 (T10, X6).

## 6. Member access and diagnostics

Member access uses `.` for enum members and struct fields, and the same dot
qualifies module names ([Ch 04](04-expressions.md); F8, GF11). The `switch`
statement is documented in [Ch 05](05-statements.md).

| Code | Trigger |
|---|---|
| E10204 | `union` declaration |
| E10211 | enum backing type is not an integer type |
| E10212 | unknown member of a struct, enum, or module |
| E10222 | operator not defined for the operand type (ordering an enum) |

All message text lives in [Ch 14](14-diagnostics.md).
