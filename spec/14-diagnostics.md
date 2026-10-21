# Chapter 14 - Diagnostics Registry

> **Status:** draft (Phase C - normative for diagnostic codes and message templates)
> **Depends on:** every chapter; chapters reference codes defined here and do not define message text
> **Code blocks:** [decisions.md](decisions.md) section 14 (N1-N8)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) section 10 (lexical errors)

## 1. Registry rules

* Every diagnostic has a stable five-digit code: `Exxxxx` errors, `Wxxxxx`
  warnings. Codes are never reused after retirement; removed codes stay
  reserved.
* This chapter is the sole authority for message templates. Other chapters
  reference codes; they do not define wording.
* A diagnostic carries: severity and code, message, primary span, related
  spans, and optional help. The toolchain prints the code and message; the
  span formatting is a compiler concern.
* **Poisoning:** an error poisons the affected declaration; dependent errors
  are suppressed so one mistake does not produce a cascade. Independent
  causes are still reported.
* The compiler reports every recoverable error (it does not stop at the
  first) and exits nonzero when any error was reported (N5).
* Warnings are deniable by default and escalable with `--deny-warnings` (N6).
  `--max-errors=N` caps the output for scripted builds.

## 2. Code blocks

| Range | Area |
|---|---|
| `E100xx` | Ownership, moves, initialization, nullability, borrows |
| `E101xx` | FFI and the raw boundary |
| `E102xx` | Types, values, expressions |
| `E103xx` | Generics |
| `E104xx` | Lambdas |
| `E105xx` | Strings and templates |
| `E106xx` | Unsafe, raw, volatile |
| `E107xx`-`E110xx` | Reserved |
| `E111xx` | Modules and initialization |
| `E112xx` | Profiles, target limits, runtime faults |
| `E20xxx` | Lexical and syntax errors |
| `E90xxx` | Staged features ("not yet supported") |
| `W100xx` | Warnings |

## 3. Ownership, moves, initialization, nullability, borrows (`E100xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10010 | Possibly-null value used without a narrowing check | `value '{name}' may be null here` | check `!= null` before the use |
| E10011 | `T?` applied to a type that cannot be empty | `nullable type is not allowed on '{type}'` | `?` applies to handles, `raw`, `cstring`, and views |
| E10012 | `null` used where a non-nullable type is required | `'null' is not allowed here` | declare the type `T?` or avoid the empty state |
| E10020 | Read, move, borrow, or drop of a moved value | `'{name}' was moved and cannot be used` | re-initialize the binding or use a view |
| E10021 | Read before the binding is assigned on every path | `'{name}' may be used before it is initialized` | initialize the binding on every path; out-parameter slots are not exempt |
| E10022 | Move of an owner while a loan of it is live | `cannot move '{name}' while it is borrowed` | end the borrow first |
| E10023 | Assignment to a `const` binding or module `const` | `cannot assign to immutable '{name}'` | declare it with `let` (mutable) |
| E10024 | Write through a read-only parameter | `cannot write through read-only parameter '{name}'` | declare the parameter `borrow` |
| E10025 | Whole-value use of a partially moved value | `'{name}' is partially moved and cannot be used as a whole` | use the remaining fields or re-initialize the struct |
| E10026 | Partial move outside a top-level field of an owned local or `take` parameter | `partial moves are limited to top-level fields in v1` | move the whole value instead |
| E10030 | Loan outlives the owner's scope | `borrow of '{name}' does not live long enough` | finish the use before the owner's scope ends |
| E10031 | Mutation while a shared loan is live | `cannot modify '{name}' while it is shared-borrowed` | end the shared borrow first |
| E10032 | Conflicting loans at one point (second loan, or exclusive vs any) | `'{name}' is already borrowed` | end the first borrow first |
| E10033 | Loan stored in a field or global | `borrow of '{name}' escapes its scope` | derive the view from a parameter or take ownership |
| E10034 | Borrowed parameter released or stored by the callee | `cannot release or store borrowed parameter '{name}'` | declare the parameter `take` or by value; a `borrow T?` slot may hold only its own incoming value |
| E10035 | View returned from a local owner | `cannot return a view of local '{name}'` | return an owned value or a view of a parameter |
| E10036 | View return does not derive from exactly one lend parameter | `view return does not derive from exactly one parameter` | make exactly one parameter a view or handle lend |
| E10038 | Borrow of a global binding | `cannot borrow global '{name}'` | borrow from a local owner |
| E10039 | Call-site marker that contradicts the declaration | `call-site marker does not match the declaration of '{fn}'` | remove the marker or correct it |
| E10040 | Element loop over non-duplicable elements without `borrow` | `element loop requires 'borrow' for non-duplicable '{type}'` | write `for (borrow e in xs)` |
| E10041 | Writable view derived from a read-only borrow | `cannot write through a view of read-only borrow '{name}'` | take the owner by `borrow` |
| E10042 | Write to an element of an immutable view (`str`, `static str`, or `cstring`) | `cannot write through immutable view '{name}'` | copy the bytes into an owned `String` or a `borrow`-derived `u8[]` view |
| E10043 | Element loop over a non-iterable type | `type '{type}' cannot be iterated` | iterate a fixed array or a slice |

## 4. FFI and the raw boundary (`E101xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10100 | Call to an uncontracted extern without `unsafe` | `uncontracted call to '{fn}' requires 'unsafe'` | wrap the call, or add a contract declaration |
| E10101 | Owned or borrowed value passed to an uncontracted extern | `'{fn}' has no contract for parameter '{name}'` | pass a `raw` pointer derived inside `unsafe` |
| E10102 | `freedBy` deleter is not an FFI function taking `take T` and returning `void` | `'{fn}' is not a valid deleter for '{type}'` | declare the deleter in an `ffi` block with the right signature |
| E10103 | `ffi` block marked `export`, or FFI declarations leaking into app code | `'ffi' blocks are module-scoped and cannot be exported` | export a wrapper function instead |
| E10104 | Native struct declared with `freedBy` | `'freedBy' is only for handle types` | native structs drop structurally (X2) |

## 5. Types, values, expressions (`E102xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10200 | Implicit conversion the language does not allow (including a nullable module initializer) | `cannot convert '{from}' to '{to}' implicitly` | use an explicit cast `T(expr)` |
| E10201 | Cast that is not allowed (view, pointer, or aggregate mismatch) | `cannot cast '{from}' to '{to}'` | convert through a supported value or `raw` in `unsafe` |
| E10202 | Literal does not fit the context type | `literal does not fit '{type}'` | use a wider type or a hex/binary form |
| E10203 | Struct literal field problems (missing, unknown, or duplicate) | `struct literal for '{type}' does not match its fields` | list every field exactly once, by name |
| E10204 | `union` declaration | `tagged unions are not part of Clorraine` | model the data as a C-style tagged struct and use `switch` |
| E10205 | View-typed struct field other than `static str` | `view-typed field '{field}' is not allowed` | store an owner, or `static str` for literals |
| E10206 | Struct with no fields | `struct '{type}' declares no fields` | add a field; C99 has no empty structs |
| E10207 | Overlay (volatile) type used as a value | `overlay type '{type}' cannot be used by value` | access it through a `raw` pointer inside `unsafe` |
| E10208 | `volatile` on something that is neither a struct field nor a module-level `let` | `'volatile' is not allowed here` | declare it on a field or a module `let` |
| E10209 | Indexing a type that is not indexable | `type '{type}' cannot be indexed` | arrays, slices, and `str` are indexable |
| E10210 | Function type mismatch (signature or capture) | `function type mismatch: expected '{expected}', found '{found}'` | match the exact signature; capture-less lambdas only |
| E10211 | Enum backing type is not an integer type | `enum backing type must be an integer type` | use `i8`-`u32`, `usize`, or `isize` |
| E10212 | Unknown member of a struct, enum, module, or type parameter | `'{name}' is not a member of '{type}'` | check the declaration or the spelling |
| E10213 | `embed` outside a constant context | `'embed' is only allowed in a constant context` | use it in `const` or a module-level initializer |
| E10214 | Embedded file cannot be read at build time, or the path is absolute or escapes the project root | `cannot read embedded file '{path}'` | use a relative path inside the project root, and fix the build permissions |
| E10215 | Array fill literal without a fixed extent in context, or a fill value whose element type is not duplicable | `array fill requires a fixed extent` | annotate the array type; construct each element for a move-only fill |
| E10216 | Constant index into a fixed array is out of range | `index {index} is out of range for '{type}[{extent}]'` | fix the index or the extent |
| E10217 | Assignment result used as a value | `assignment result is 'void' and cannot be used as a value` | split the statement; do not chain assignments |
| E10218 | Condition expression that is not `bool` | `condition must be 'bool'` | compare explicitly, for example `x != 0` |
| E10219 | Name that is not declared | `'{name}' is not declared` | check the spelling, the import, or the module |
| E10220 | Inner extent omitted in a multi-dimensional type | `only the outermost extent may be omitted` | write every inner extent |
| E10221 | Assignment to something that is not a place | `left side of an assignment must be a place` | assign to a variable, field, or element |
| E10222 | Operator not defined for the operand type | `operator '{op}' is not defined for '{type}'` | convert explicitly or use a supported operator |
| E10223 | Constant expression division by zero or constant MIN/-1 overflow | `constant expression divides by zero` | remove the zero divisor or the MIN/-1 operation, or use a non-constant operand |
| E10224 | Expression used where a constant expression is required | `expression is not constant` | use literals, `const` bindings, enum members, and supported operators |
| E10226 | Zero-length array extent | `array extent must be greater than zero` | use a positive extent |

## 6. Generics (`E103xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10300 | Wrong number of type arguments | `wrong number of type arguments for '{name}': expected {expected}, found {found}` | list all type arguments |
| E10301 | Type arguments cannot be inferred | `cannot infer type arguments for '{name}'` | write the type arguments explicitly |
| E10302 | `export` on a generic function | `generic '{name}' cannot be exported` | write a concrete exported wrapper |
| E10303 | Type argument does not satisfy a callable bound | `type argument '{type}' does not satisfy '{param}: func(...)'` | pass a matching function or lambda |
| E10304 | Value of a callable-only type parameter stored or returned | `value of type parameter '{param}' cannot be stored or returned` | call it or pass it on |
| E10305 | Instantiation recursion exceeds the cap | `instantiation of '{name}' is recursive beyond the limit` | break the generic recursion |
| E10306 | Generic aggregate instantiated with an ephemeral type argument | `'{name}' cannot be instantiated with an ephemeral type argument` | use a storable type |
| E10307 | Call of a type-parameter value without a callable bound | `type parameter '{param}' is not callable` | declare a callable bound: `T: func(...)` |
| E10308 | Wrong number of arguments | `wrong number of arguments for '{name}': expected {expected}, found {found}` | pass one argument per parameter |

## 7. Lambdas (`E104xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10400 | Capturing lambda used where a plain function pointer is required | `capturing lambda cannot be used as 'func(...)'` | use a capture-less lambda, or a C-hook trampoline (v1.1) |
| E10401 | Lambda moves a captured binding | `cannot move captured '{name}' in a lambda` | copy the value at capture time |
| E10402 | Lambda writes a captured binding without a `[borrow]` capture | `cannot write captured '{name}' without '[borrow {name}]'` | add the borrow capture |
| E10403 | `take` capture | `'take' captures are not supported` | copy or borrow the capture |
| E10404 | Closure stored, returned, or captured (closures are ephemeral) | `closures are ephemeral and cannot be stored, returned, or captured` | pass the lambda directly |
| E10405 | Lambda reads an enclosing name without listing it | `lambda reads '{name}' without listing it` | add '{name}' to the capture list |
| E10406 | Capture list names a module-level binding | `module-level '{name}' needs no capture` | use the name directly |

## 8. Strings and templates (`E105xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10500 | Non-formattable type in a template placeholder | `type '{type}' is not formattable` | convert with `text`, `toHex`, or `toDecimal` |
| E10501 | Format specifier (`:`) inside a template placeholder (parser detection) | `format specifiers are not supported in v1` | use a conversion function |
| E10503 | Unbalanced braces in a template | `unbalanced braces in template` | escape literal braces as `{{` and `}}` |

## 9. Unsafe, raw, volatile (`E106xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E10600 | Raw dereference outside `unsafe` | `dereference of a raw pointer requires 'unsafe'` | wrap it in `unsafe` or an `unsafe { ... }` block |
| E10601 | Address-of outside `unsafe` | `address-of requires 'unsafe'` | take a view parameter instead, or use `unsafe` |
| E10602 | Raw pointer arithmetic outside `unsafe` | `pointer arithmetic requires 'unsafe'` | wrap the computation in `unsafe` |
| E10603 | Raw pointer cast (pointer to pointer) outside `unsafe` | `raw pointer cast requires 'unsafe'` | integer-to-`raw` value casts are allowed; pointer casts are not |
| E10604 | Volatile intrinsic or overlay access outside `unsafe` | `volatile access requires 'unsafe'` | wrap hardware access in `unsafe` or use the stdlib wrappers |

## 10. Modules and initialization (`E111xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E11100 | Initializer dependency cycle (between modules or same-module globals) | `initializer cycle between modules: {modules}` | break the cycle with `const` values or an explicit `init()` |
| E11101 | Duplicate declaration in a merged module | `duplicate declaration of '{name}' in module '{module}'` | remove one declaration; declaration merging forbids duplicates |
| E11102 | Import name not found or not exported | `'{name}' is not exported from module '{module}'` | check the name or add `export` |
| E11103 | Import name bound twice | `duplicate import of '{name}'` | use `as` to bind a new name |
| E11104 | Entry-point problems (`main` missing, duplicated, or mistyped) | `executable builds need exactly one 'func main(): i32'` | declare or fix `main` |
| E11105 | Module declaration missing or not first | `'module' must be the first declaration in the file` | move the module declaration to the top |
| E11106 | Module-level `let` uses a non-duplicable initializer | `module-level 'let' must be duplicable or 'const'` | use `const`, or initialize inside an explicit `init()` |

## 11. Profiles, target limits, runtime faults (`E112xx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E11200 | Construct not lowerable on the selected profile | `'{feature}' is not available on profile '{profile}'` | switch profiles or avoid the feature |
| E11201 | Unknown or unsupported profile name | `unknown profile '{profile}'` | use `hosted-amiga`, `host`, or `freestanding` |
| E11210 | Runtime fault: integer division or modulo by zero, or by `-1` at the minimum signed value | `integer division by zero` | check the divisor before dividing |
| E11211 | Runtime fault: index out of range | `index out of range` | check the index against `len` |
| E11212 | Runtime fault: float-to-integer conversion out of range (including NaN) | `float-to-integer conversion out of range` | range-check the float before the cast |
| E11213 | Runtime fault: assertion failed | `assertion failed` | fix the invariant or the input |
| E11214 | Runtime fault: a feature the selected profile does not provide (a `std.io` file function under freestanding) | `file I/O is not available on the freestanding profile` | use a profile that provides the feature |

Runtime faults are not compile errors. The inserted checks print
`error CODE: message` to the diagnostic stream and abort without running the
cleanup ladder, exactly like `panic` (E4). `panic(msg)` prints
`panic: <msg>` and aborts. There is no profile in which the safety checks are
disabled (T12).

## 12. Lexical and syntax errors (`E20xxx`)

| Code | Trigger | First line (after the code) | Help |
|---|---|---|---|
| E20000 | Any token that cannot start a declaration or statement | `syntax error: unexpected {token}` | see the grammar for the expected form |
| E20001 | Block comment never closed | `unterminated block comment` | close the comment with `*/` |
| E20002 | String, template, or character literal never closed | `unterminated {kind} literal` | close the literal on the same line |
| E20003 | Invalid escape sequence | `invalid escape sequence '\{char}'` | use `\n`, `\r`, `\t`, `\0`, `\\`, `\"`, `\'`, or `\xNN` |
| E20004 | Malformed or out-of-range numeric literal | `invalid numeric literal '{text}'` | check digits and separators |
| E20005 | Keyword used as a declared name | `'{name}' is a keyword and cannot be used as a name` | choose another name |
| E20006 | Reserved built-in name redeclared | `'{name}' is a reserved built-in and cannot be redeclared` | rename the declaration |
| E20007 | Character literal that is not exactly one byte | `character literal must encode exactly one byte` | use one UTF-8 byte or `\xNN` |
| E20008 | Syntax copied from another language (`->`, `::`, `#`, `@`, `?.`, `??`) | `'{text}' is not Clorraine syntax` | see the grammar for the Clorraine form |

## 13. Staged features (`E90xxx`)

Staged diagnostics mark frozen syntax whose semantics arrive in a later phase
(R1). They are not syntax errors: the parser accepts the feature and the
semantic stage reports the code.

| Code | Feature | First line (after the code) |
|---|---|---|
| E90000 | Generic staged feature has no dedicated code | `'{feature}' is not yet supported` |
| E90001 | `comptime` bodies | `'comptime' semantics are not yet implemented` |
| E90003 | `stores` and `transfers` contracts | `'stores' and 'transfers' contracts are deferred to v2` |
| E90004 | Pools and arenas | `pools and arenas are deferred to v2` |
| E90005 | Nested templates and extended formatting | `nested templates are not yet supported` |

## 14. Warnings (`W100xx`)

| Code | Trigger | First line (after the code) |
|---|---|---|
| W10000 | Local declared and never used | `unused local '{name}'` |
| W10001 | Writable lend of a duplicable value; writes affect only the copy | `unnecessary lend of duplicable '{name}'` |
| W10002 | `switch` case body falls through | `case falls through to the next label` |
| W10003 | Statement is unreachable | `unreachable code` |
| W10004 | Stack frame exceeds the profile threshold | `stack frame of '{fn}' is {size} bytes` |
| W10005 | User-provided contract table is in use | `user-provided contract '{name}' is in use` |
| W10006 | Imported name is never used | `unused import '{name}'` |

## 15. Stability and verification

* Codes are stable forever (N2). Retiring a code leaves the number reserved;
  templates may be refined while a chapter is `draft`, and the first line
  format (`error CODE: message` / `warning CODE: message`) is fixed (N4).
* Golden compile-fail tests (Ch 16 section 9) assert the code and the first
  line; positive tests assert the absence of diagnostics.
* `check.sh` cross-validates codes referenced in chapters and in tests
  against this registry (N7).

### 15.1 Retired and reserved codes

A retired code is never reused (N2); it stays reserved and is listed here
instead of in the live tables above. The code column carries a backticked
value so a script that extracts the live registry with `^\| [EW][0-9]{5} \|`
does not treat these rows as active registrations.

| Retired code | Replacement |
|---|---|
| `E10037` retired | merged into E11106 |
| `E10225` retired | no reachable trigger |
| `E90002` retired | lambda capture semantics are implemented; no trigger remains |
