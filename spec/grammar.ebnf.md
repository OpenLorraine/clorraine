# Clorraine - Master EBNF Grammar

> **Status:** draft (Phase B anchor - the grammar freeze)
> **Decision ledger:** [decisions.md](decisions.md) - the ledger wins on any conflict
> **Blend65 derivation:** [grammar-deltas.md](grammar-deltas.md) - every delta from the Blend65 v4 grammar with its reason

## 1. Notation

This grammar uses **ISO 14977 EBNF** with the same small extensions as the
Blend65 grammar:

| Notation | Meaning |
|---|---|
| `=` | Production rule definition |
| `,` | Concatenation (sequential) |
| `\|` | Alternation (choice) |
| `{ X }` | Repetition - zero or more of X |
| `[ X ]` | Optional - zero or one of X |
| `( X )` | Grouping |
| `"..."` | Terminal string literal |
| `(*...*)` | Comment |
| `;` | End of production |

Character ranges use the notation `"A"…"Z"`. The grammar is pure syntax:
ownership checking, name resolution, definite assignment, and constant
evaluation live in the semantics chapters. A production that the semantic
passes restrict further says so in a note.

## 2. Source structure

### 2.1 Program

```ebnf
program         = module_decl , { top_level_item } ;

top_level_item  = import_stmt
                | function_decl
                | comptime_decl
                | struct_decl
                | enum_decl
                | handle_decl
                | global_let_decl
                | global_const_decl
                | ffi_block
                | intrinsic_decl ;
```

One module declaration per file, first (M1). Imports may appear at any
top-level position. There are no global constructors (M5); module-level
initializers become generated init functions.

### 2.2 Module declaration

```ebnf
module_decl     = "module" , qualified_name , ";" ;
```

### 2.3 Imports

```ebnf
import_stmt     = "import" , "{" , import_list , "}" , "from" , qualified_name , ";" ;
import_list     = import_item , { "," , import_item } ;
import_item     = identifier , [ "as" , identifier ] ;
qualified_name  = identifier , { "." , identifier } ;
```

Named lists only, no wildcards (M4); qualified access (`A.B.f()`) is always
available. `as` is a contextual keyword (section 10.3).

## 3. Declarations

### 3.1 Functions

```ebnf
function_decl   = [ "export" ] , "func" , identifier , [ type_params ]
                , "(" , [ param_list ] , ")"
                , ":" , return_type
                , block ;

comptime_decl   = [ "export" ] , "comptime" , "func" , identifier , [ type_params ]
                , "(" , [ param_list ] , ")"
                , ":" , return_type
                , block ;

param_list      = param , { "," , param } ;
param           = [ param_mode ] , identifier , ":" , value_type ;
param_mode      = "const" | "borrow" | "take" ;

return_type     = "void" | value_type ;
```

The mode comes first: `borrow e: Entity`, `take f: File` (V5). Unmarked and
`const` are read-only lends; `borrow` is the writable lend; `take` transfers
ownership. `comptime` syntax is frozen now, semantics staged (R1; deltas
section 8). There is no `interrupt` declaration form; interrupts and hooks go
through the standard-library dispatcher (F7).

### 3.2 Structs

```ebnf
struct_decl     = [ "export" ] , "struct" , identifier , [ type_params ]
                , "{" , struct_field , { struct_field } , "}" ;

struct_field    = identifier , ":" , [ "volatile" ] , value_type , ";" ;
```

At least one field: C99 has no empty structs (P1). A `volatile` field makes
the struct an overlay type (T7, T11). Fields are semicolon-terminated and
declared in C-layout order (no reordering).

### 3.3 Enums

```ebnf
enum_decl       = [ "export" ] , "enum" , identifier , ":" , integer_type
                , "{" , enum_member , { "," , enum_member } , [ "," ] , "}" ;

enum_member     = identifier , [ "=" , const_expression ] ;
```

The backing integer type is required; members are scoped (`Direction.UP`) and
convert only through explicit casts. There is no `union` declaration: tagged
data is modeled with structs plus `switch` (T10).

### 3.4 Handles

```ebnf
handle_decl     = [ "export" ] , "handle" , identifier , "freedBy" , qualified_name , ";" ;
```

Declared once, in the library that owns the resource (X1). The deleter is an
FFI-declared function taking `take T` and returning `void`. Native structs
never use `freedBy` (X2).

### 3.5 Module-level values

```ebnf
global_let_decl    = [ "export" ] , [ "volatile" ] , "let"
                   , identifier , ":" , value_type , "=" , expression , ";" ;

global_const_decl  = [ "export" ] , "const"
                   , identifier , ":" , value_type , "=" , const_expression , ";" ;
```

Initializers are mandatory at module level (no uninitialized globals; M7).
Globals must be duplicable or `const` (O10). `volatile` is allowed only on
`let` and marks state shared with interrupt handlers (T7).

### 3.6 FFI and intrinsic declarations

```ebnf
ffi_block       = "ffi" , identifier , "{" , { ffi_decl } , "}" ;
ffi_decl        = "func" , identifier , "(" , [ param_list ] , ")" , ":" , return_type , ";" ;

intrinsic_decl  = "intrinsic" , "func" , identifier , "(" , [ param_list ] , ")" , ":" , return_type , ";" ;
```

FFI signatures use the ordinary vocabulary, including modes; nullability lives
in the type (`File?`), not in clauses (X3). FFI blocks are module-scoped and
never exported (M9); application code calls libraries through wrappers (X6).
`intrinsic` declarations are restricted to compiler-known names (deltas
section 8); they carry no body.

## 4. Types

```ebnf
type            = "void" | value_type ;

value_type      = type_atom , [ "?" ] ;

type_atom       = scalar_type
                | view_type
                | raw_type
                | array_type
                | function_type
                | named_type ;

scalar_type     = "i8" | "i16" | "i32" | "u8" | "u16" | "u32"
                | "f32" | "f64" | "bool" | "usize" | "isize" ;

view_type       = "str" | "cstring" | "static" , "str" ;

raw_type        = "raw" , raw_target ;
raw_target      = scalar_type | "str" | "cstring" | named_type ;

array_type      = array_element_type , extent , { extent } ;
array_element_type = scalar_type
                   | "str"
                   | "cstring"
                   | raw_type
                   | named_type
                   | "(" , function_type , ")" ;
extent          = "[" , [ const_expression ] , "]" ;

function_type   = "func" , "(" , [ function_type_params ] , ")" , ":" , return_type ;
function_type_params = function_type_param , { "," , function_type_param } ;
function_type_param  = [ param_mode ] , value_type ;

named_type      = qualified_name , [ type_args ] ;
```

Notes:

* `T?` is nullability for pointer-like atoms (handles, `raw`, `cstring`,
  views); scalars and structs cannot be nullable (T8). `null` is the only
  empty state.
* `T[]` is a slice view (one omitted outermost extent); `T[N]` is a fixed
  array. Multiple extents are syntactically allowed, but only the outermost
  may be omitted, and no dynamic or jagged forms exist (T9).
* `raw` binds to the immediately following type atom: `raw u8[4]` is an array
  of four raw bytes pointers, not a raw pointer to an array.
* `static str` is a string value with static origin; it widens to `str`
  implicitly (T3, T4).
* `func(...) : R` is an exact-signature function pointer (F3); only
  capture-less lambdas match it (L5).
* `named_type` covers structs, enums, handles, and `String` (a standard-library
  handle, T2), with optional explicit type arguments.

## 5. Statements

```ebnf
statement       = local_let_decl
                | local_const_decl
                | expression_stmt
                | if_stmt
                | while_stmt
                | do_while_stmt
                | for_stmt
                | switch_stmt
                | return_stmt
                | break_stmt
                | continue_stmt
                | defer_stmt
                | unsafe_stmt
                | block ;

block           = "{" , { statement } , "}" ;

local_let_decl  = "let" , identifier , [ ":" , value_type ]
                , [ "=" , expression ] , ";" ;

local_const_decl = "const" , identifier , [ ":" , value_type ]
                 , "=" , expression , ";" ;

expression_stmt = expression , ";" ;
```

Locals infer their type from the initializer (F1); `let x: T;` declares
without a value and definite assignment applies (F9). `const` requires an
initializer. There are no `own`, `move`, `var`, `loop`, or labeled statements.

### 5.1 If, while, do-while

```ebnf
if_stmt         = "if" , "(" , expression , ")" , block
                , [ "else" , ( if_stmt | block ) ] ;

while_stmt      = "while" , "(" , expression , ")" , block ;

do_while_stmt   = "do" , block , "while" , "(" , expression , ")" , ";" ;
```

Parentheses are C-style and braces are mandatory (B1); the dangling-else
question cannot arise.

### 5.2 For

```ebnf
for_stmt        = "for" , "(" , ( c_for_header | element_for_header ) , ")" , block ;

c_for_header    = [ for_init ] , ";" , [ expression ] , ";" , [ expression_list ] ;
for_init        = for_let | for_const | expression_list ;
for_let         = "let" , identifier , [ ":" , value_type ] , [ "=" , expression ] ;
for_const       = "const" , identifier , [ ":" , value_type ] , "=" , expression ;

element_for_header = [ "borrow" ] , identifier , "in" , expression ;

expression_list = expression , { "," , expression } ;
```

`for (;;)` is the infinite loop (B2). The element form copies duplicable
elements; `borrow` binds an exclusive element view with per-element loan extent
and is required for non-duplicable elements (F1). There is no range syntax
(B4).

### 5.3 Switch

```ebnf
switch_stmt     = "switch" , "(" , expression , ")" , "{"
                , { case_clause | default_clause }
                , "}" ;

case_clause     = "case" , const_expression , ":" , { statement } ;
default_clause  = "default" , ":" , { statement } ;
```

C-exact: fallthrough by default, `break` to exit, no `fallthrough` keyword,
no exhaustiveness requirement (B5). `default` may appear anywhere among the
labels, as in C.

### 5.4 Jumps, defer, unsafe

```ebnf
return_stmt     = "return" , [ expression ] , ";" ;
break_stmt      = "break" , ";" ;
continue_stmt   = "continue" , ";" ;

defer_stmt      = "defer" , block ;
unsafe_stmt     = "unsafe" , block ;
```

Returns are by value or view type; there is no marker syntax (F2). `defer`
joins the cleanup ladder at scope exit and is checked at the exit context (F5).
There is no `goto` and there are no labels (B6).

## 6. Expressions

### 6.1 Strategy

Expressions use the C precedence table and are parsed by recursive descent
plus a Pratt parser. The grammar below is the EBNF shape of that table; the
binding powers and associativity are exactly C's, minus the comma operator
(B7). Assignment is syntactically an expression but its result is **void**:
chaining and value use are compile errors (grammar freeze decision GF2, deltas
section 8).

### 6.2 Precedence chain

```ebnf
expression      = assignment_expr ;

assignment_expr = conditional_expr , [ assign_op , assignment_expr ] ;
assign_op       = "=" | "+=" | "-=" | "*=" | "/=" | "%="
                | "&=" | "|=" | "^=" | "<<=" | ">>=" ;

conditional_expr = logical_or_expr , [ "?" , expression , ":" , conditional_expr ] ;

logical_or_expr  = logical_and_expr , { "||" , logical_and_expr } ;
logical_and_expr = bitwise_or_expr , { "&&" , bitwise_or_expr } ;
bitwise_or_expr  = bitwise_xor_expr , { "|" , bitwise_xor_expr } ;
bitwise_xor_expr = bitwise_and_expr , { "^" , bitwise_and_expr } ;
bitwise_and_expr = equality_expr , { "&" , equality_expr } ;
equality_expr    = relational_expr , { ( "==" | "!=" ) , relational_expr } ;
relational_expr  = shift_expr , { ( "<" | "<=" | ">" | ">=" ) , shift_expr } ;
shift_expr       = additive_expr , { ( "<<" | ">>" ) , additive_expr } ;
additive_expr    = multiplicative_expr , { ( "+" | "-" ) , multiplicative_expr } ;
multiplicative_expr = unary_expr , { ( "*" | "/" | "%" ) , unary_expr } ;
```

The conditional operator is right-associative. There is no comma operator.

### 6.3 Unary and unsafe expressions

```ebnf
unary_expr      = ( "!" | "~" | "-" | "&" | "*" | "++" | "--" ) , unary_expr
                | unsafe_expr
                | postfix_expr ;

unsafe_expr     = "unsafe" , unary_expr ;
```

`&` in prefix position is address-of (produces `raw T`, requires the unsafe
context, T6); `*` is raw dereference (requires the unsafe context); in infix
position they are bitwise AND and multiplication. `unsafe` as a **prefix
operator** wraps one unary expression (`let next = unsafe p.next;`, grammar
freeze decision GF4); the multi-statement form is `unsafe { ... }` (section
5.4). There is no block-expression value.

### 6.4 Postfix expressions

```ebnf
postfix_expr    = primary_expr , { postfix_op } ;
postfix_op      = call_suffix | index_suffix | member_suffix | "++" | "--" ;

call_suffix     = "(" , [ arg_list ] , ")" ;
index_suffix    = "[" , expression , "]" ;
member_suffix   = "." , identifier ;

arg_list        = arg , { "," , arg } ;
arg             = [ call_marker ] , expression ;
call_marker     = "const" | "borrow" | "take" ;
```

Member access uses `.` for every type, including `raw` pointers (the
dereference is implied and checked inside `unsafe`) and module-qualified
names. Call-site markers are optional assertions that must match the
declaration (V6). Indexing is always bounds-checked (T12).

### 6.5 Primary expressions

```ebnf
primary_expr    = number_literal
                | float_literal
                | string_literal
                | template_string
                | char_literal
                | "true" | "false" | "null"
                | scalar_cast
                | raw_cast
                | struct_literal
                | array_literal
                | lambda_expr
                | intrinsic_call
                | generic_ref
                | identifier
                | "(" , expression , ")" ;

scalar_cast     = scalar_type , "(" , expression , ")" ;
raw_cast        = "raw" , raw_target , "(" , expression , ")" ;

generic_ref     = qualified_name , type_args ;
```

`T(expr)` is the only cast form (B8). Scalar and `raw` casts are syntactic;
`Direction(1)` and `File(0)` parse as `identifier` plus `call_suffix` and are
resolved semantically into enum casts or calls. A bare identifier names a
value, a function value (of `func(...)` type), or a zero-argument generic
reference; function values need no `&` (grammar freeze, deltas section 8).

### 6.6 Struct literals

```ebnf
struct_literal  = struct_type_ref , "{" , [ field_init_list ] , "}" ;
struct_type_ref = qualified_name , [ type_args ] ;
field_init_list = field_init , { "," , field_init } , [ "," ] ;
field_init      = identifier , ":" , expression ;
```

Fields are by name, order-independent; duplicates and missing fields are
errors; `Point {}` is the explicit zero value (F4). The type name precedes the
brace, so `Pair<i32> { a: 1, b: 2 }` works. In statement position, `identifier {`
is always this literal (see section 11).

### 6.7 Array literals

```ebnf
array_literal   = "[" , [ array_init_content ] , "]" ;
array_init_content = expression , { "," , expression }
                   | expression , { "," , expression } , ";" , expression
                   | ";" , expression ;
```

`[]` is the empty literal, `[1, 2, 3]` lists elements, and the semicolon form
fills (`[1, 2; 0]`, `[; 0]`) using the declared extent (Blend65 sugar kept).
String and `embed` results use the same shape.

### 6.8 Constant expressions

```ebnf
const_expression = expression ;
```

Syntactically identical to `expression`; the semantic pass requires
compile-time evaluability (literals, `const` values, `sizeof`, `offsetof`,
`len` of fixed arrays, and constant folds). Constant `raw` casts of integer
addresses are allowed so module-level register bases work (T7).

## 7. Intrinsic calls and templates

```ebnf
intrinsic_call  = query_intrinsic
                | volatile_intrinsic
                | runtime_intrinsic
                | embed_expr ;

query_intrinsic = "sizeof" , "(" , type , ")"
                | "offsetof" , "(" , named_type , "," , identifier , ")"
                | "len" , "(" , expression , ")" ;

volatile_intrinsic = ( "readVolatile8" | "readVolatile16" | "readVolatile32" )
                     , "(" , expression , ")"
                   | ( "writeVolatile8" | "writeVolatile16" | "writeVolatile32" )
                     , "(" , expression , "," , expression , ")" ;

runtime_intrinsic = "panic" , "(" , expression , ")"
                  | "assert" , "(" , expression , "," , expression , ")" ;

embed_expr      = "embed" , "(" , string_literal , [ "," , string_literal ] , ")" ;

template_string = "$" , '"' , { template_part } , '"' ;
template_part   = template_char | "{{" | "}}" | placeholder ;
placeholder     = "{" , expression , "}" ;
```

Intrinsic names are reserved built-ins (section 10.4) and callable as free
functions, so dot-call sugar works (`xs.len()` resolves to `len(xs)`, F8).
`sizeof`, `offsetof`, and `len` return `usize`; `len` is compile-time for
fixed arrays. `panic`/`assert` abort without running the cleanup ladder (E4).
The volatile intrinsics are the audited hardware primitives (T7). Template
placeholders may contain any expression; scanning tracks nested braces and
strings, and `{{`/`}}` are literal braces (S1, S4).

## 8. Lambdas

```ebnf
lambda_expr     = [ capture_list ] , "(" , [ lambda_params ] , ")" , "=>"
                , ( expression | block ) ;

capture_list    = "[" , [ capture , { "," , capture } ] , "]" ;
capture         = [ capture_mode ] , identifier ;
capture_mode    = "borrow" | "take" ;

lambda_params   = lambda_param , { "," , lambda_param } ;
lambda_param    = identifier , ":" , value_type ;
```

The full syntax is frozen now; semantics are staged (L1, L4). `take` captures
parse and are rejected (staged diagnostic, L2). Capture-less lambdas are plain
C function pointers and match `func(...)` types (L3, L5); capturing closures
are ephemeral and cannot be stored or returned.

## 9. Generics

```ebnf
type_params     = "<" , type_param , { "," , type_param } , ">" ;
type_param      = identifier , [ ":" , callable_bound ] ;
callable_bound  = "func" , "(" , [ type_list ] , ")" , ":" , return_type ;

type_args       = "<" , type_list , ">" ;
type_list       = value_type , { "," , value_type } ;
```

The only bound is callable (`F: func(T): R`, G6). Values of a callable-only
type parameter can be called or passed, never stored or returned (G7).
Explicit type arguments appear in type positions and, speculatively, in
expression position (`vecNew<i32>()`, G2); the disambiguation rule is in
section 11. Instantiation is monomorphized with hash-deduped symbols (G3);
generic declarations cannot be exported (G4).

## 10. Lexical grammar

### 10.1 Identifiers

```ebnf
identifier      = ( letter | "_" ) , { letter | digit | "_" } ;
letter          = "A"…"Z" | "a"…"z" ;
digit           = "0"…"9" ;
```

Keywords and reserved built-in names are rejected in declaration positions.

### 10.2 Keywords (48)

```ebnf
keyword         = "module" | "import" | "export" | "from"
                | "func" | "comptime" | "return"
                | "struct" | "enum" | "handle" | "freedBy"
                | "ffi" | "intrinsic" | "unsafe" | "volatile" | "raw"
                | "let" | "const" | "defer"
                | "if" | "else" | "while" | "do" | "for" | "in"
                | "switch" | "case" | "default" | "break" | "continue"
                | "i8" | "i16" | "i32" | "u8" | "u16" | "u32"
                | "f32" | "f64" | "bool" | "void" | "usize" | "isize"
                | "str" | "cstring" | "static"
                | "true" | "false" | "null" ;
```

### 10.3 Contextual keywords

```ebnf
contextual_keyword = "as" ;
```

`as` is recognized only inside an import item; it remains an ordinary
identifier elsewhere. It is never a cast operator.

### 10.4 Reserved built-in names

```ebnf
reserved_builtin = "sizeof" | "offsetof" | "len" | "embed"
                 | "panic" | "assert"
                 | "readVolatile8" | "readVolatile16" | "readVolatile32"
                 | "writeVolatile8" | "writeVolatile16" | "writeVolatile32" ;
```

Reserved built-ins are identifier-shaped tokens, not keywords. In primary
position, a reserved spelling followed by `(` is dispatched to
`intrinsic_call`; the spelling is rejected in declaration positions. This
keeps the tokenizer context-free (no symbol table) and gives deterministic
parsing.

### 10.5 Numeric literals

```ebnf
number_literal  = decimal_literal | hex_literal | bin_literal ;

decimal_literal = digit , { [ "_" ] , digit } ;

hex_digit       = "0"…"9" | "A"…"F" | "a"…"f" ;
hex_literal     = ( "0x" | "0X" ) , hex_digit , { [ "_" ] , hex_digit } ;

bin_digit       = "0" | "1" ;
bin_literal     = ( "0b" | "0B" ) , bin_digit , { [ "_" ] , bin_digit } ;

float_literal   = decimal_literal , "." , decimal_literal , [ exponent ]
                | decimal_literal , exponent ;
exponent        = ( "e" | "E" ) , [ "+" | "-" ] , decimal_literal ;
```

There are no literal suffixes (grammar freeze decision GF3). A decimal literal
must fit its context type and defaults to `i32`; hex and binary literals may
use the full unsigned width of the target (`0xFFFFFFFF` is a valid `u32`
value). `1.5` is an `f64`; a `float_literal` always has digits on both sides
of the dot. The legacy Blend65 `$FF` hex form is removed (`$` introduces
templates only).

### 10.6 String, template, and character literals

```ebnf
string_literal  = '"' , { string_char } , '"' ;
string_char     = escape_seq | ? any Unicode scalar value except U+0022, U+005C, U+000D, or U+000A ? ;

template_char   = escape_seq | ? any Unicode scalar value except U+0022, U+005C, U+000D, U+000A, U+007B, or U+007D ? ;

char_literal    = "'" , char_char , "'" ;
char_char       = escape_seq | ? any single Unicode scalar value except U+0027, U+005C, U+000D, or U+000A ? ;

escape_seq      = ? U+005C REVERSE SOLIDUS ?
                , ( "n" | "r" | "t" | "0" | ? U+005C REVERSE SOLIDUS ? | '"' | "'"
                  | "x" , hex_digit , hex_digit ) ;
```

Double quotes delimit strings (including `""`), single quotes delimit exactly
one encoded byte: `'x'` is a `u8` literal (F14). A string literal has type
`static str` (T3); `$"..."` produces a `String` when it contains placeholders
and a `str` when it does not (S5).

### 10.7 Comments

```ebnf
line_comment    = "//" , { ? any byte except LF ? } , ( ? LF byte ? | ? end of file ? ) ;
doc_comment     = "///" , { ? any byte except LF ? } , ( ? LF byte ? | ? end of file ? ) ;
block_comment   = "/*" , { ? any character except the byte pair "*/" ? } , "*/" ;
```

Block comments do not nest. `///` is a documentation comment attached to the
next declaration; v1 emits nothing and a future `--doc` tool reads it (B9).

### 10.8 Operators and punctuation

```
(* Single-character tokens *)
+ - * / % & | ^ ~ ! < > = . , : ; ? ( ) [ ] { }

(* Multi-character tokens *)
== != <= >= << >> && || += -= *= /= %= &= |= ^= <<= >>= => ++ --

(* Special prefixes *)
0x 0X 0b 0B $"
```

No `::`, `->`, `#`, `@`, `?.`, or `??`. `?` is the nullability suffix in type
position and the conditional operator in expression position.

## 11. Disambiguation and parsing notes

| Ambiguity | Resolution |
|---|---|
| `if ... else if` vs `else { if }` | Parsed as `else if_stmt`; identical semantics |
| `&` address-of vs bitwise AND | Prefix = address-of; infix = bitwise AND |
| `*` dereference vs multiplication | Prefix = raw dereference; infix = multiplication |
| `{` block vs struct literal | Statement position = block; after a struct type reference in expression position = struct literal. A statement that starts `identifier {` is the struct-literal expression statement |
| Generic type arguments vs `<` comparison | After a `qualified_name` in expression position, `<` starts `type_args` only when the token stream parses as a type list closed by `>` that is immediately followed by `(` or `{`; otherwise `<` is relational. Bounded speculative parse; no symbol table |
| `a<b>(c)` where `a` is a value | Always parsed as generic call; write `(a < b) > (c)` for the comparison |
| Lambda vs grouping / array literal | `(` starts a lambda when the content is `identifier :` typed parameters; `[` starts a capture list when the bracket content is captures and the closing `]` is followed by `(`. Otherwise grouping / array literal |
| `?` nullability vs conditional | Type position = nullability suffix; expression position = conditional operator |
| Assignment as a value | Parsed as an expression; the semantic pass types it `void`, so chaining and value use are errors |
| Element loop vs C-style for | After `(`, `borrow` or `identifier in` selects the element form; otherwise the three-clause form |
| `case` bodies and fallthrough | C-exact: labels do not end a body; `break` exits; no `fallthrough` keyword |
| `raw` binding | Binds to the immediately following type atom; extents bind to what precedes (`raw u8[4]` is an array) |
| `unsafe` block vs prefix operator | `unsafe {` is the statement block; `unsafe <expr>` is the prefix operator wrapping one unary expression |
| `identifier(...)` call vs cast | Parsed uniformly as call; the semantic pass resolves functions, enum casts, and handle casts |
| Statement start | LL(1): leading keyword or `{`; other starts are expressions |

## 12. Production index

| Production | Section | Description |
|---|---|---|
| `additive_expr` | 6.2 | `+`, `-` binary |
| `arg` | 6.4 | Argument with optional call marker |
| `arg_list` | 6.4 | Call argument list |
| `array_element_type` | 4 | Element type of an array |
| `array_init_content` | 6.7 | List or fill content |
| `array_literal` | 6.7 | `[]`, `[1, 2]`, `[1; 0]` |
| `array_type` | 4 | Fixed array or slice |
| `assign_op` | 6.2 | Assignment operators |
| `assignment_expr` | 6.2 | Assignment (void result) |
| `bin_digit` | 10.5 | `0` or `1` |
| `bin_literal` | 10.5 | `0b1010` |
| `bitwise_and_expr` | 6.2 | `&` binary |
| `bitwise_or_expr` | 6.2 | `\|` binary |
| `bitwise_xor_expr` | 6.2 | `^` binary |
| `block` | 5 | `{ ... }` statement block |
| `block_comment` | 10.7 | `/* ... */` |
| `break_stmt` | 5.4 | `break;` |
| `c_for_header` | 5.2 | Three-clause loop header |
| `call_marker` | 6.4 | `const`/`borrow`/`take` assertion |
| `call_suffix` | 6.4 | `( arguments )` |
| `callable_bound` | 9 | `func(T): R` bound |
| `capture` | 8 | One lambda capture |
| `capture_list` | 8 | `[borrow x, ...]` |
| `capture_mode` | 8 | `borrow` or `take` |
| `case_clause` | 5.3 | `case V:` |
| `char_char` | 10.6 | Character-literal content |
| `char_literal` | 10.6 | `'x'` |
| `comptime_decl` | 3.1 | `comptime func` declaration |
| `conditional_expr` | 6.2 | Right-associative `? :` |
| `const_expression` | 6.8 | Compile-time expression context |
| `contextual_keyword` | 10.3 | Import-alias `as` |
| `continue_stmt` | 5.4 | `continue;` |
| `decimal_literal` | 10.5 | `255`, `1_000` |
| `default_clause` | 5.3 | `default:` |
| `defer_stmt` | 5.4 | `defer { ... }` |
| `digit` | 10.1 | `0`…`9` |
| `do_while_stmt` | 5.1 | `do { } while (...);` |
| `doc_comment` | 10.7 | `/// ...` |
| `element_for_header` | 5.2 | `for (e in xs)` |
| `embed_expr` | 7 | `embed("path"[, "key"])` |
| `enum_decl` | 3.3 | Enum declaration |
| `enum_member` | 3.3 | Member and optional value |
| `equality_expr` | 6.2 | `==`, `!=` |
| `escape_seq` | 10.6 | Literal escape sequence |
| `exponent` | 10.5 | Float exponent part |
| `expression` | 6.2 | Entry point for expressions |
| `expression_list` | 5.2 | Comma-separated expressions |
| `expression_stmt` | 5 | Any expression followed by `;` |
| `extent` | 4 | One `[...]` dimension |
| `ffi_block` | 3.6 | `ffi name { ... }` |
| `ffi_decl` | 3.6 | Extern signature |
| `field_init` | 6.6 | Struct field initializer |
| `field_init_list` | 6.6 | Field initializer list |
| `float_literal` | 10.5 | `1.5`, `1e3` |
| `for_const` | 5.2 | `const` loop initializer |
| `for_init` | 5.2 | Loop initializer |
| `for_let` | 5.2 | `let` loop initializer |
| `for_stmt` | 5.2 | For loop |
| `function_decl` | 3.1 | Function declaration |
| `function_type` | 4 | `func(...): R` type |
| `function_type_param` | 4 | Unnamed parameter type |
| `function_type_params` | 4 | Type parameter list |
| `generic_ref` | 6.5 | `name<types>` in expressions |
| `global_const_decl` | 3.5 | Module-level constant |
| `global_let_decl` | 3.5 | Module-level mutable value |
| `handle_decl` | 3.4 | `handle T freedBy F;` |
| `hex_digit` | 10.5 | `0`…`F` |
| `hex_literal` | 10.5 | `0xFF` |
| `identifier` | 10.1 | User-defined name |
| `if_stmt` | 5.1 | Braced `if`/`else` |
| `import_item` | 2.3 | Imported name and alias |
| `import_list` | 2.3 | Import list |
| `import_stmt` | 2.3 | Module import |
| `index_suffix` | 6.4 | `[index]` |
| `intrinsic_call` | 7 | Reserved built-in call |
| `intrinsic_decl` | 3.6 | Compiler-provided signature |
| `keyword` | 10.2 | Reserved word |
| `lambda_expr` | 8 | Lambda expression |
| `lambda_param` | 8 | Typed lambda parameter |
| `lambda_params` | 8 | Lambda parameter list |
| `letter` | 10.1 | ASCII letter |
| `line_comment` | 10.7 | `// ...` |
| `local_const_decl` | 5 | Local `const` |
| `local_let_decl` | 5 | Local `let` |
| `logical_and_expr` | 6.2 | `&&` |
| `logical_or_expr` | 6.2 | `\|\|` |
| `member_suffix` | 6.4 | `.name` |
| `module_decl` | 2.2 | Required module header |
| `multiplicative_expr` | 6.2 | `*`, `/`, `%` |
| `named_type` | 4 | Named type with optional type args |
| `number_literal` | 10.5 | Decimal, hex, or binary |
| `param` | 3.1 | `[mode] name: type` |
| `param_list` | 3.1 | Parameter list |
| `param_mode` | 3.1 | `const`/`borrow`/`take` |
| `placeholder` | 7 | `{expression}` in a template |
| `postfix_expr` | 6.4 | Primary plus suffixes |
| `postfix_op` | 6.4 | Call, index, member, `++`/`--` |
| `primary_expr` | 6.5 | Literals, names, casts, aggregates |
| `program` | 2.1 | One compilation unit |
| `qualified_name` | 2.3 | Dot-separated name |
| `query_intrinsic` | 7 | `sizeof`, `offsetof`, `len` |
| `raw_cast` | 6.5 | `raw T(expr)` |
| `raw_target` | 4 | Type after `raw` |
| `raw_type` | 4 | `raw T` |
| `relational_expr` | 6.2 | `<`, `<=`, `>`, `>=` |
| `reserved_builtin` | 10.4 | Reserved built-in names |
| `return_stmt` | 5.4 | `return [expr];` |
| `return_type` | 3.1 | `void` or a value type |
| `runtime_intrinsic` | 7 | `panic`, `assert` |
| `scalar_cast` | 6.5 | `u32(expr)` and friends |
| `scalar_type` | 4 | Fixed-width scalar types |
| `shift_expr` | 6.2 | `<<`, `>>` |
| `statement` | 5 | Any statement |
| `string_char` | 10.6 | String-literal content |
| `string_literal` | 10.6 | Double-quoted literal |
| `struct_decl` | 3.2 | Struct declaration |
| `struct_field` | 3.2 | Struct field |
| `struct_literal` | 6.6 | `Point { x: 1 }` |
| `struct_type_ref` | 6.6 | Type name before a literal brace |
| `switch_stmt` | 5.3 | Switch statement |
| `template_char` | 10.6 | Template-literal content |
| `template_part` | 7 | Template segment |
| `template_string` | 7 | `$"..."` |
| `top_level_item` | 2.1 | Any declaration after the header |
| `type` | 4 | `void` or a value type |
| `type_args` | 9 | `<A, B>` |
| `type_atom` | 4 | One non-nullable type form |
| `type_list` | 9 | Type argument or parameter list |
| `type_param` | 9 | Generic parameter |
| `type_params` | 9 | `<T, F: bound>` |
| `unary_expr` | 6.3 | Prefix operators, `unsafe`, postfix |
| `unsafe_expr` | 6.3 | `unsafe <unary>` |
| `unsafe_stmt` | 5.4 | `unsafe { ... }` |
| `value_type` | 4 | Any non-void type with optional `?` |
| `view_type` | 4 | `str`, `cstring`, `static str` |
| `volatile_intrinsic` | 7 | Hardware read/write intrinsics |
| `while_stmt` | 5.1 | Braced `while` |

## 13. Freeze certification

| Criterion | Status |
|---|---|
| Every ledger construct has a production | Yes: sections 2-9 cover modules, ownership surface, types, statements, expressions, generics, lambdas, strings, FFI, and intrinsics |
| Parseable by recursive descent plus Pratt | Yes; one bounded speculative rule (generic type arguments, section 11) |
| Dangling `else` resolved | Yes: mandatory blocks (B1) |
| No tokenization ambiguities | Yes: quote kind, `$"`, `&`/`*` position, `?` position, `unsafe` lookahead |
| Grammar freeze checklist resolved | Yes: see [grammar-deltas.md](grammar-deltas.md) section 8 |
| Semantics staged without reopening syntax | Yes: `comptime`, lambda stages, and intrinsics stage behind stable syntax (R1) |
