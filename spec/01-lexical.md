# Chapter 01 - Lexical Structure

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 02 (types), Ch 04 (expressions), Ch 10 (modules), Ch 12 (FFI contracts), Ch 14 (diagnostics)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 10.1-10.8
> **Ledger:** B9, GF3, F14, T3, S1 ([decisions.md](decisions.md))

This chapter defines the raw vocabulary of a Clorraine source file: the bytes a
file is made of, the words and symbols it may contain, and how those symbols
become the tokens the parser reads. Every later chapter builds on this one.

## 1. Source encoding

A source file is a sequence of UTF-8 bytes. The compiler reads it as bytes;
nothing depends on the machine's locale or text mode.

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-1 | Source files are UTF-8 | - |
| LEX-2 | Identifiers use ASCII letters, digits, and `_` only; they are case-sensitive | - |
| LEX-3 | String contents are Unicode scalar values encoded as UTF-8 bytes | - |
| LEX-4 | A character literal holds exactly one encoded byte | E20007 |

Because strings are byte-oriented, their length counts bytes, not characters.
A multi-byte character such as `€` is one scalar value but three bytes, so it
may appear inside a string but never as a whole character literal.

```clorraine
let ascii: u8 = 'A';        // one byte
let newline: u8 = '\n';     // one escaped byte
let euro: str = "€";        // three UTF-8 bytes inside the string
let bad: u8 = '€';          // E20007: more than one encoded byte
```

Whitespace separates tokens and is otherwise insignificant.

## 2. Identifiers and naming conventions

Grammar: `identifier = ( letter | "_" ) , { letter | digit | "_" }`, where
`letter` is `A`-`Z` or `a`-`z` and `digit` is `0`-`9`.

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-5 | An identifier starts with a letter or `_` and continues with letters, digits, or `_` | E20000 |
| LEX-6 | Identifiers are case-sensitive (`count` and `Count` differ) | - |
| LEX-7 | Non-ASCII characters are not allowed in identifiers | E20000 |

The conventions below are style, not grammar; the compiler accepts any legal
identifier, but the standard library and the examples follow these rules.

| Element | Convention | Example |
|---|---|---|
| Function, local, parameter | camelCase | `readFile`, `bufferSize` |
| Type (struct, enum, handle) | PascalCase | `Direction`, `File` |
| Module constant, enum member | UPPER_SNAKE | `MAX_WIDTH`, `Direction.UP` |
| Module name | PascalCase segments | `module Demo.Chips;` |

Reserved built-in names are spelled camelCase too (section 4).

## 3. Keywords

The grammar fixes exactly 50 keywords. They are reserved: a keyword used as a
declared name is E20005.

| Group | Keywords |
|---|---|
| Module and imports | `module`, `import`, `export`, `from` |
| Declarations | `func`, `comptime`, `struct`, `enum`, `handle`, `freedBy`, `ffi`, `intrinsic` |
| Bindings and safety | `let`, `const`, `borrow`, `take`, `defer`, `unsafe`, `volatile`, `raw`, `return` |
| Control flow | `if`, `else`, `while`, `do`, `for`, `in`, `switch`, `case`, `default`, `break`, `continue` |
| Type names | `i8`, `i16`, `i32`, `u8`, `u16`, `u32`, `f32`, `f64`, `bool`, `void`, `usize`, `isize`, `str`, `cstring`, `static` |
| Literals | `true`, `false`, `null` |

Type-name keywords are reserved so that `str`, `bool`, and the widths cannot be
shadowed. They are used in type positions; see [Ch 02](02-types.md).

### 3.1 Contextual keyword `as`

`as` is **contextual**: it is recognized only inside an import item, where it
renames the imported binding. Everywhere else it is an ordinary identifier.

```clorraine
import { readFile as loadFile } from std.io;   // 'as' renames here
```

`as` is never a cast operator. Casts use `T(expr)` only
([Ch 04](04-expressions.md)).

## 4. Reserved built-in names

Reserved built-in names are identifier-shaped tokens, not keywords. The
grammar lists them:

```ebnf
reserved_builtin = "sizeof" | "offsetof" | "len" | "embed"
                 | "panic" | "assert"
                 | "readVolatile8" | "readVolatile16" | "readVolatile32"
                 | "writeVolatile8" | "writeVolatile16" | "writeVolatile32" ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-8 | In primary position, a reserved spelling followed by `(` is an intrinsic call | - |
| LEX-9 | Reserved built-in names may be bound only by an `intrinsic func` declaration; an ordinary declaration of one is E20006 | E20006 |

```clorraine
let n = len(xs);                 // intrinsic call
let m = xs.len();                // dot-call resolves to len(xs) (Ch 04)
intrinsic func readVolatile16(addr: raw u16): u16;   // allowed: binds a reserved name
func sizeof(x: i32): void { }    // E20006: an ordinary declaration of a reserved name
```

This rule keeps the tokenizer context-free: no symbol table is needed to decide
whether a name is an intrinsic. Only an `intrinsic func` declaration may bind a
reserved built-in name; an ordinary declaration of it is E20006
([Ch 06](06-functions.md)). The intrinsics are documented in
[Ch 04](04-expressions.md) and [grammar.ebnf.md](grammar.ebnf.md) section 7.

## 5. Comments

| Form | Meaning | Diagnostic |
|---|---|---|
| `// text` | Line comment to the end of the line | - |
| `/* text */` | Block comment; does **not** nest | E20001 |
| `/// text` | Documentation comment attached to the next declaration | - |

A line comment that begins with exactly three slashes is a **documentation
comment**. It attaches to the declaration that follows it. In v1 the compiler
emits nothing for a documentation comment; a future `--doc` tool reads them.
An unterminated block comment is E20001.

```clorraine
// A plain comment.
/* A block comment that may span lines. */

/// Returns the remaining capacity of the buffer.
func capacity(borrow b: Buffer): usize {
    return b.cap;
}
```

Block comments do not nest, so the first `*/` closes the comment. The text
inside a comment is never tokenized.

## 6. Statement termination

Semicolons terminate statements and declarations. There is **no automatic
semicolon insertion**: the compiler never guesses where a statement ends.

```ebnf
local_let_decl  = "let" , identifier , [ ":" , value_type ]
                , [ "=" , expression ] , ";" ;
local_const_decl = "const" , identifier , [ ":" , value_type ]
                 , "=" , expression , ";" ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-10 | A local, a module value, a struct field, and an `ffi`/`intrinsic` declaration end with `;` | E20000 |
| LEX-11 | A block `{ ... }` is a statement and needs no trailing `;` | - |
| LEX-12 | `do { ... } while (cond);` ends with `;` | E20000 |

Missing punctuation is reported as a syntax error. Statement grammar lives in
[Ch 05](05-statements.md).

## 7. Numeric literals

Integers are written in decimal, hexadecimal (`0x`/`0X`), or binary
(`0b`/`0B`). A single `_` may separate digits for readability; it may not lead
or trail.

| Form | Example | Notes |
|---|---|---|
| Decimal | `255`, `1_000` | Defaults to `i32` |
| Hex | `0xFF`, `0xFFFF_FFFF` | May use the full unsigned width of the target |
| Binary | `0b1010`, `0b1111_0000` | May use the full unsigned width |

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-13 | There are **no literal suffixes** | E20004 |
| LEX-14 | A literal is typed by its context; a decimal literal defaults to `i32` | - |
| LEX-15 | A literal that does not fit its context type is an error | E10202 |
| LEX-16 | The legacy `$FF` hex form is gone; `$` introduces templates only | E20000 |

```clorraine
let a = 255;                 // decimal, default i32
let b: u8 = 200;             // context makes it a u8
let c = 0xFF;                // hex
let d: u32 = 0xFFFF_FFFF;    // hex may use the full unsigned width
let e = 0b1010;              // binary
let f: u16 = 1_000;          // underscore separators
let g: u8 = 256;             // E10202: 256 does not fit u8
```

Because suffixes do not exist, the context decides the type. `1u8` is not a
literal; write `let x: u8 = 1;`. Context typing is defined in
[Ch 02](02-types.md) and [Ch 04](04-expressions.md).

## 8. Float literals

A float literal has digits on **both** sides of the dot, then an optional
exponent. A decimal integer followed directly by an exponent is also a float.

```ebnf
float_literal = decimal_literal , "." , decimal_literal , [ exponent ]
              | decimal_literal , exponent ;
```

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-17 | A float literal always has digits on both sides of the dot | E20004 |
| LEX-18 | A decimal float literal is `f64`; context may make it `f32` | - |
| LEX-19 | No float suffixes exist | E20004 |

```clorraine
let x = 1.5;         // f64
let y: f32 = 1.5;    // f32 by context
let z = 1e3;         // f64: exponent without a dot
let w = 6.02e23;     // digits on both sides, exponent
```

`1.` and `.5` are not float literals. Floats, their operators, and their
conversions are specified in [Ch 02](02-types.md) and
[Ch 04](04-expressions.md).

## 9. Character literals

A character literal is written in single quotes and has type `u8`: it is a
**byte**, not a text character.

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-20 | `'x'` is a `u8` byte literal | - |
| LEX-21 | The literal must encode exactly one byte | E20007 |
| LEX-22 | An invalid escape sequence is an error | E20003 |

```clorraine
let a: u8 = 'A';
let tab: u8 = '\t';
let zero: u8 = '\0';
let hex: u8 = '\x41';    // 'A' by hexadecimal escape
let quote: u8 = '\'';
let bad: u8 = '\q';      // E20003: unknown escape
```

## 10. String literals and templates

A double-quoted string has type `static str`: its bytes live in static storage
(T3). It widens implicitly to `str`, the immutable view type. An empty string
`""` is legal.

A template string begins with `$"` and interpolates expressions with `{...}`.
`{{` and `}}` are literal braces. Templates are summarized here and specified
in [Ch 04](04-expressions.md).

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-23 | `"..."` has type `static str`, which widens to `str` (T3) | - |
| LEX-24 | `$"..."` is a template (S1) | - |
| LEX-25 | A template with placeholders is an owned `String`; without placeholders it is a `str` | - |
| LEX-26 | `{{` and `}}` are literal braces ([Ch 04](04-expressions.md)) | - |

```clorraine
let greeting: static str = "hello";
let empty: static str = "";
let view: str = greeting;             // static str widens to str
let msg = $"Hi, {name}!";             // String: has a placeholder
let plain = $"hello";                 // str: no placeholder
let braces = $"literal {{ and }}";    // {{ and }} are literal braces
```

Interpolation rules, formattable types, and the placeholder grammar live in
[Ch 04](04-expressions.md).

## 11. No preprocessor

Clorraine has no preprocessor and no directives. `#`, `->`, `::`, `@`, `?.`,
and `??` are not syntax; each is rejected with E20008. Modules replace
`#include` ([Ch 10](10-modules.md)), and intrinsics and FFI wrappers replace
macros ([Ch 12](12-ffi-contracts.md)).

| Not Clorraine | Use instead |
|---|---|
| `#include`, `#define` | `import { ... } from A.B;` and named declarations |
| `->` | `:` for return types and `=>` for lambda bodies |
| `::` | `.` for all qualified and member access |
| `?.`, `??` | explicit `== null` / `!= null` checks (T8) |
| `@` | no attribute syntax exists |

## 12. Diagnostics

| Code | Trigger |
|---|---|
| E20000 | a token that cannot start a declaration or statement |
| E20001 | an unterminated block comment |
| E20002 | an unterminated string, template, or character literal |
| E20003 | an invalid escape sequence |
| E20004 | a malformed or out-of-range numeric literal |
| E20005 | a keyword used as a declared name |
| E20006 | an ordinary declaration of a reserved built-in name |
| E20007 | a character literal that is not exactly one byte |
| E20008 | syntax copied from another language |
| E10202 | a literal that does not fit the context type |

All message text lives in [Ch 14](14-diagnostics.md).

## 13. Rule summary

| Rule | Statement | Diagnostic |
|---|---|---|
| LEX-1 | Source is UTF-8; identifiers are ASCII and case-sensitive | - |
| LEX-4 | A character literal is exactly one encoded byte | E20007 |
| LEX-8 | A reserved built-in name in primary position is an intrinsic call | - |
| LEX-9 | Reserved built-in names are bound only by `intrinsic func` | E20006 |
| LEX-13 | No integer or float literal suffixes | E20004 |
| LEX-15 | A literal must fit its context type | E10202 |
| LEX-20 | `'x'` is a `u8` byte literal | - |
| LEX-23 | `"..."` is `static str`, widening to `str` | - |
| LEX-24 | `$"..."` is a template | - |
| LEX-25 | Placeholders give a `String`; otherwise a `str` | - |
| LEX-10 | No automatic semicolon insertion | E20000 |
| LEX-27 | Comments are `//`, `/* */` (non-nesting), and `///` | E20001 |
