# Chapter 17 - C Codegen Contract

> **Status:** draft (Phase C - rewritten against the ledger and the grammar)
> **Depends on:** Ch 03 (ownership), Ch 05 (statements), Ch 06 (functions), Ch 12 (FFI), Ch 14 (diagnostics), Ch 15 (profiles), Ch 16 (borrow rules)
> **Grammar:** [grammar.ebnf.md](grammar.ebnf.md) sections 3.1 (params), 3.4 (handles), 3.5 (module values), 4 (types), 5 (statements), 6.4 (member access), 7 (intrinsics and templates), 8 (lambdas)
> **Ledger:** [decisions.md](decisions.md) sections 1 (D4, D5), 5 (O1-O7), 6 (T2-T8, T12), 7 (F5, F8), 8 (G3), 9 (L1-L5), 10 (S1-S5), 11 (X1), 12 (M5, M8), 15 (P1-P15)

This chapter is the contract between the compiler front end and the emitted C.
It says what C the compiler writes, how ownership becomes explicit cleanup, and
which undefined behaviors the output avoids. The front end decides; codegen only
translates.

## 1. Target

The compiler emits a strict **C99 common subset** accepted by both
`m68k-amigaos-gcc` and `vbcc`. The emitted C is intentionally boring:
predictable, greppable, debuggable, and warning-free.

| Allowed | Forbidden |
|---|---|
| C99 declarations, `for`/`while`/`switch`, `static` functions | Compiler extensions, `__attribute__((cleanup))`, statement expressions |
| `//` and `/* */` comments (vbcc accepts `//` in C99 mode) | VLAs, compound literals, designated initializers |
| Fixed-width integer typedefs and the `Cl` types emitted in the prelude | Nested functions, `typeof`, `__builtin_*` beyond the shim |
| `goto` for cleanup ladders only (section 4) | `goto` for any other purpose; user labels do not exist |
| C `volatile` for volatile fields and globals | `setjmp`/`longjmp`, `_Generic`, atomics |

No language construct may depend on a compiler extension. The compile matrix
(section 13) is the acceptance test.

## 2. Typed control-flow IR

The checker and the emitter share one typed control-flow IR. It is also the
surface the borrow checker consumes ([Ch 16](16-borrow-rules.md) section 3), so
the two passes cannot disagree about ownership. The statements are:

| Statement | Meaning |
|---|---|
| `assign` | Copy a duplicable value, or store a fresh result; drops the old target value first (O3) |
| `move` | Transfer an owned value: copy plus zero the source (section 5) |
| `borrow` | Begin a loan over a place (checker only; no runtime code) |
| `call` | Invoke a func or an intrinsic |
| `drop` | Release an owned place |
| `return` | Leave the func with a value or a view |
| `branch` | Conditional control flow |
| `switch` | C-exact multi-way branch with fallthrough |
| `defer` | A registered cleanup entry that joins the ladder in the scope that registered it |

Every owned place carries an explicit `drop` on **every path that leaves its
scope**. The IR makes drops and defers explicit; the emitter translates them and
never re-discovers them, so an ownership decision cannot silently become a leak
or a double release.

## 3. Mapping

The table below is the whole surface mapping.

| Clorraine | Emitted C |
|---|---|
| `func f(a: T): R` (module-private) | `static R cl_<module>_f(T a)` |
| `export func f(a: T): R` | the external name `cl_<module>_f` (no `static`) |
| scalar binding `let x: i32 = 0;` | `i32 x = 0;` (the source name is kept when it is a valid unique C identifier) |
| unmarked or `const` parameter (aggregate) | `const T *` |
| `borrow` parameter | `T *` (writable lend) |
| `take` parameter | `T` by value; ownership transfers into the callee |
| scalar parameter | `T` by value in every mode (V5) |
| `handle File freedBy Close;` | an opaque pointer typedef `ClFile`; `NULL` is the zero empty state |
| `str` and `T[]` views | emitted pointer-and-length value structs (`ClStr`, `ClSlice_T`) |
| `cstring` | C `char *` |
| `raw T` | `T *` |
| `struct` | C `struct`, fields in declaration order, no reordering |
| `volatile` field | C `volatile` member; no folding, caching, or reordering (T7) |
| `enum E : u8` | `u8`-typed constants; `E.V` is an emitted constant; `E(x)` is a C cast |
| move of an owned value | `dst = src; cl_zero(&src);` (section 5) |
| drop of an owned handle | `if (value) { Deleter(value); }` (null-safe, exactly once) |
| field access `p.x` | `.` on a value |
| member access through `raw` (`unsafe p.x`) | `p->x` (the dereference is implied) |
| dot-call `x.f(y)` | `f(x, y)`; field first, then a free func; no methods, no vtables (F8) |
| temporary | compiler-named `__cl_tmp<N>`, dropped at statement end (D5) |
| `return e` | move/drop epilogue, then `return e;` |
| `defer { ... }` | the block is copied into the scope's ladder entry (section 4) |
| `f32` / `f64` | C `float` / `double` (P15) |
| template `$"Hi, {name}!"` | string-builder and text-conversion calls (section 9) |

Module level values become file-scope `static` variables; `volatile` on a module
`let` becomes C `volatile`. A `volatile` overlay struct is emitted as a plain C
struct with `volatile` members, reached only through `raw`, so the C compiler
emits every access and preserves order.

Module initialization (P9): the compiler collects the module-level initializers
into generated `static void cl_init_<module>(void)` functions, emitted in
dependency order, and a generated entry stub that calls them before the user's
`main`. A dependency cycle that involves a runtime-initialized global is E11100
(M6). `main(): i32` is emitted as `int main(void)` in the stub (M8).

## 4. Cleanup ladder (normative) - P2

Cleanup uses a single-exit ladder: one label per scope, innermost first, with
drops and `defer` entries in reverse registration order (the Phase 0 decision).

| Rule | Statement |
|---|---|
| CL-1 | Each func has one cleanup ladder; the body falls through into it at the end |
| CL-2 | Each scope that owns a place or registers a `defer` gets one label, innermost first |
| CL-3 | Cleanup runs in one reverse registration order: locals and `defer` entries interleaved exactly as they were registered (O2, F5) |
| CL-4 | Every drop is null-safe: `if (value) { Deleter(value); }` (P5) |
| CL-5 | A `defer` block is code motion; it is copied into the ladder at its scope's label, and it runs only if its registration was reached. A conditionally registered `defer` carries a guard flag (or an equivalent per-registration label) that the ladder checks |
| CL-6 | Early exits jump to the label of the scope they leave: `return` to the outer label, `break`/`continue` to the loop scope label |
| CL-7 | Generated cleanup is attributed to the scope's closing line, so stepping never enters generated code (section 11) |

The example below emits `func copy(srcPath: str, dstPath: str): bool` from module
`Demo.Copy`. The `defer` is registered first, so it runs last at scope exit.

```c
typedef struct ClFileClosure *ClFile;  /* NULL is the empty state        */
typedef struct { const u8 *ptr; usize len; } ClStr;  /* the str view     */

bool tracing = true;                   /* duplicable global (O10)        */

static bool cl_Demo_Copy_copy(ClStr srcPath, ClStr dstPath)
{
    ClFile src = NULL;                 /* owned locals hoisted, empty    */
    ClFile dst = NULL;
    Buffer buf = NULL;
    bool   __cl_rc = false;

    src = cl_acquire_file(srcPath);    /* File? : NULL means null        */
    if (!src) { __cl_rc = false; goto __cl_c2; }

    dst = cl_acquire_file(dstPath);
    if (!dst) { __cl_rc = false; goto __cl_c2; }  /* src still drops    */

    buf = cl_acquire_buffer(4096);
    if (!buf) { __cl_rc = false; goto __cl_c1; }  /* dst and src drop   */

    /* ... copy bytes from src to dst ... */
    __cl_rc = true;

__cl_c1:                               /* scope 1: buf                   */
    if (buf) { FreeBuffer(buf); }
__cl_c2:                               /* scope 0: dst, src, then defer  */
    if (dst) { Close(dst); }
    if (src) { Close(src); }
    tracing = false;                   /* the deferred code runs last    */
    return __cl_rc;
}
```

Labels are per scope, not per variable, so the ladder stays short. A moved value
has been zeroed (section 5), so its drop is a no-op no matter which label runs.

## 5. Moves and zero = empty - P3, P4, O7

A move lowers to a **copy plus a zero of the source**. The zeroed source is the
drop flag: there are no hidden Rust-style drop flags (O7).

| Move or copy | Emitted C |
|---|---|
| duplicable scalar or struct | `dst = src;` (a copy, no zeroing) |
| owned handle | `dst = src; src = NULL;` |
| aggregate with owned fields | `dst = src; cl_zero(&src);` |
| partial move of a top-level field | `dst = s.field; cl_zero(&s.field);` (P3) |
| assignment to a live owner | drop the old target first, then store (O3) |

This is safe because every droppable type treats the all-zero bit pattern as its
empty state (O7), and drops check that state before releasing. The zeroed value
**is** the flag, so a drop of a moved value does nothing and a release can never
run twice. It is defensive, not a substitute for the checker: one zeroing per
aggregate move, removable only when the value is provably dead.

## 6. Drops, deleters, and String - P5, P6

A handle's deleter comes from its type declaration (`handle T freedBy F`, X1) and
is emitted null-safe, exactly once:

```c
if (value) { F(value); }
```

A native struct drops structurally: its fields in declaration order, array
elements first to last (O2). Parameters drop after the body's locals; temporaries
drop at the end of their statement (D5).

`String` is a handle with a fixed layout (P6):

| Part | Definition |
|---|---|
| Block | one `AllocVec` allocation `[len][cap][bytes][NUL]` |
| `len` | current byte length |
| `cap` | capacity in bytes |
| `bytes` | the payload bytes |
| `NUL` | a trailing zero byte, kept for FFI convenience (T5) |
| Deleter | `FreeString`; the drop is `if (s) { FreeString(s); }` |
| Ownership | move-only, mutable through `borrow` (T2) |

`str` is a value struct `{ ptr, len }` that copies freely and allocates nothing.
`static str` is a literal with static origin and needs no drop; it widens to `str`
implicitly. `cstring` is a raw zero-terminated pointer. `String` widens to `str`
implicitly (T2-T5).

## 7. Naming and mangling - P10, P11, G3

v1 compiles the whole program as **one translation unit** (P10). Internal symbols
are `static`; only `export` declarations and the entry stub are external, so the
linker never sees a name the program did not export.

| Item | Scheme |
|---|---|
| Private func | `static ... cl_<module>_<name>` |
| Exported func | `cl_<module>_<name>`, external |
| Generic instantiation | `cl_<name>__<typekey>`, hash-deduped (G3) |
| Local | source name if it is a valid, unique C identifier; otherwise a stable suffix |
| Generated helper | `cl_` prefix (`cl_zero`, `cl_acquire_*`, `cl_text_*`) |
| Temporary | `__cl_tmp<N>`, a stable counter |
| Cleanup label | `__cl_c<N>` |
| Type | `Cl<Module>_<Type>` to avoid C tag clashes |
| Entry | `int main(void)` in the generated stub |

`<module>` is the dotted module path with dots replaced by underscores, so
`Demo.Copy` mangles to `Demo_Copy`. Generics are **monomorphized** (G3): one
`static` func per distinct instantiation, keyed by a canonical `<typekey>` of the
type arguments (for example `i32`, `String`, or `Point_i32_i32`). Identical keys
dedupe to one symbol; an over-long key appends a stable hash. C macros and C
templates are not used, because they would be extensions.

## 8. Lambdas and closures - P7

A lambda is lifted to a file-scope `static` func with a generated name.

| Lambda | Emission |
|---|---|
| Capture-less `(x: i32) => x + 1` | a lifted `static` func; its address is a plain C function pointer |
| `func(...)` parameter | a plain C function pointer; only capture-less lambdas match (L5, E10400) |
| Capturing closure | a two-word callable: a func pointer plus a stack environment pointer; the lifted func takes the environment |
| Stored, returned, or captured closure | rejected; closures are ephemeral (L3, E10404) |
| Closure crossing a C hook | a generated trampoline; arrives in v1.1 (L5) |

A capturing closure's environment lives on the enclosing frame, valid only for
the synchronous call that receives it, which is why storing it is E10404.

## 9. Templates - P8

Interpolation desugars to string-builder and text-conversion calls. It never uses
`printf`-family functions, because their argument handling is not type-safe.

| Template | Desugaring |
|---|---|
| `$"plain"` (no placeholders) | a plain `str` literal; no allocation (S5) |
| `$"Hi, {name}!"` | create a builder, append the literal parts, append the converted placeholder, finish; the result is one owned `String` (S5) |
| `{x}` where `x: str` | append the bytes |
| `{n}` where `n` is an integer | append the integer's decimal text |
| `{f}` where `f: f32`/`f64` | append at the documented fixed precision (S2) |
| `{b}` where `b: bool` | append `true` or `false` |
| `{{` or `}}` | a literal brace |

There are no format specs in v1 (S3); a non-formattable placeholder type is
E10500. The builder is shim code, so the allocation strategy stays in one place.

## 10. Defined arithmetic and runtime traps - P12, B7

| Hazard | Emitted guarantee | Diagnostic |
|---|---|---|
| Signed overflow | wrapping, via unsigned-based forms or GCC `-fwrapv` (P12) | - |
| Shift count at or beyond the width, or negative | masked to the operand width: `a << (b & 31)`, `& 15`, or `& 7` (B7) | - |
| Signed shift into the sign bit | emitted through unsigned operands so both compilers agree | - |
| Integer `/` or `%` by zero | a runtime check before the operation | E11210 |
| Index out of range | a runtime check before every index | E11211 |
| Float-to-integer conversion out of range | a runtime check before the cast | E11212 |
| Float `/` by zero | IEEE 754 Inf or NaN; no check (T1) | - |

Signed wrapping uses unsigned-based arithmetic, portable to both compilers; GCC
may use `-fwrapv` once the vbcc behavior is verified. Shifts are **masked, never
rejected**. Division by zero, failed bounds, and an out-of-range float-to-integer
conversion abort without the ladder, like `panic` (E4); no profile disables these
checks (T12). Codes: [Ch 14](14-diagnostics.md).

## 11. Source mapping and determinism - P13

* `#line N "file.clor"` precedes each emitted statement, mapping it to its
  Clorraine source line.
* One source statement maps to one emitted statement. The emitter never splits or
  merges statements across mapped lines.
* Inserted cleanup (drops, defers, trap checks) is attributed to the scope's
  closing line, or to the guarded statement for a trap check, so stepping does
  not enter generated code.
* Same source, same profile, same flags produce **byte-identical C**: stable
  symbol names, stable temporary numbering, stable drop order, no timestamps and
  no absolute paths (only the logical source path, in `#line`).

## 12. Runtime shim

The shim is small and profile-specific and contains **no ownership logic**. It
only names the acquire and release functions for each handle type, the failure
sentinel, the trap helper, the panic, and the debug output.

| Piece | hosted-amiga | host (tests) | freestanding (later) |
|---|---|---|---|
| acquire/drop helpers | `cl_acquire_*` and deleter wrappers over exec/dos | the same names over libc | LorraineOS entry points |
| null policy | zero handle / `NULL` | `NULL` | null |
| panic | print then `exit`/`Exit` | `abort` | halt or reset |
| runtime trap (E11210-E11212) | print `error CODE: message`, then abort | same | same |
| debug output | `printf` (libnix) or serial | `fprintf(stderr, ...)` | serial/UART |

Drops are emitted directly as `if (value) { Deleter(value); }` using the handle's
deleter; the `cl_drop_*` helpers are null-safe wrappers with identical behavior.
`cl_zero` (a `memset`) is the only shim helper the move lowering uses.

## 13. Compile matrix - P14, P15

Exact command lines (the input object is the generated single `.c` file):

| Target | Command |
|---|---|
| GCC debug/development | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O0 -g -Wall -Wextra -Werror -o demo demo.c -lamiga` |
| GCC release | `m68k-amigaos-gcc -std=c99 -mcrt=nix20 -m68000 -O2 -o demo demo.c -lamiga` |
| vbcc release | `vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include -o demo demo.c -lamiga` |

Float legs (P15), added to the matrix:

| Leg | Setting | Verifies |
|---|---|---|
| GCC soft-float 68000 | the GCC lines above (no FPU option) | `float`/`double` argument passing and returns |
| vbcc soft-float 68000 | the vbcc line above | the same soft-float ABI as GCC |
| GCC 68881 FPU (opt-in) | add `-m68881` | the FPU leg matches the soft-float legs bit-for-bit for the tested operations |

`-lamiga` is the stub archive, not `-ldos`; vbcc needs the NDK include path.
`-m68020` and `-m68881` are opt-in; `-m68000` stays the default. Phase 0 verified
the GCC 68000/68020 and vbcc legs on `vamos` and both FS-UAE media
([../docs/findings-phase0.md](../docs/findings-phase0.md)).

## 14. No-undefined-behavior commitments

| C hazard | Emitted guarantee | Diagnostic |
|---|---|---|
| Signed overflow | wrapped through unsigned-based forms or `-fwrapv` | - |
| Shift by width or a negative count | count masked to the operand width; never rejected | - |
| Unaligned access | `-m68000` baseline assumes none; no packed structs, fields keep natural alignment | - |
| Strict aliasing | no type-punning casts; the only aliasing surface is `raw` inside `unsafe` | - |
| Uninitialized reads | definite assignment rejects them at compile time | E10021 |
| Null dereference of an owned handle | use before a `== null` / `!= null` check is rejected; drops are null-safe | E10010 |
| Bounds | every indexed access is checked, always on | E11211 |
| Integer division or modulo by zero | runtime check | E11210 |
| Out-of-range float-to-integer conversion | runtime check | E11212 |

The front end rejects at compile time what it can prove; the inserted runtime
checks cover the rest. Memory safety is strong above the hardware boundary; below
it, `raw` inside `unsafe` is audited (A1-safety).

Fault codes and message text are in [Ch 14](14-diagnostics.md) section 11; the
checker that produces this IR is in [Ch 16](16-borrow-rules.md). Ownership and
drop order are the source contract in [Ch 03](03-ownership.md).
