# Chapter 17 - C Codegen Contract

> **Status:** draft
> **Depends on:** Ch 03 (ownership), Ch 12 (contracts)
> **Validated by:** Phase 0 (`docs/findings-phase0.md`)

## 1. Target

The compiler emits a strict **C99 common subset** accepted by both
`m68k-amigaos-gcc` and `vbcc`. The emitted C is intentionally boring:
predictable, greppable, debuggable, and warning-free.

| Allowed | Forbidden |
|---|---|
| C99 declarations, `for`/`while`/`switch`, `static` functions | GCC extensions, `__attribute__((cleanup))`, statement expressions |
| `/* */` and `//` comments (both compilers accept `//` in C99 mode) | VLAs, compound literals, designated initializers |
| Fixed-width integer typedefs emitted in the prelude | `long long` on m68k where avoidable |
| `goto` for cleanup ladders (normative pattern, §4) | `goto` for anything else |

No language construct may depend on a compiler extension; the compile matrix
is the acceptance test.

## 2. IR (informal)

The checker and emitter share a typed control-flow IR. Statements:
`assign`, `move`, `borrow`, `call`, `drop`, `return`, `branch`, `switch`.
Every owned place has an explicit `drop` on each path that leaves its scope;
the emitter only translates, it does not re-discover drops (§4).

## 3. Mapping

| Clorraine | Emitted C |
|---|---|
| `function f(a: T): R` | `static R cl_f(T a)` (export → `R f(T a)`, module-prefixed on collision) |
| scalar local | same-name local; source names preserved |
| `own T` handle | pointer/handle typedef; zero is the invalid sentinel |
| `borrow T` / `borrow mut T` | `const T *` / `T *`; no runtime distinction beyond C qualifiers |
| `raw T` | `T *`, only inside escape blocks |
| move of an owned value | `dst = src; cl_zero(&src);` (§5) |
| drop of an owned value | `if (ptr) { Deleter(ptr); }` (null-safe, exactly once) |
| field access | `.` / `->` unchanged |
| method-style call `x.f(y)` | `cl_Type_f(&x, y)` (v1 has no methods) |
| temporary | compiler-named local, dropped at statement end |
| `return e` | move/drop epilogue then `return e;` |

## 4. Cleanup ladder (normative)

Drops are emitted as a single-exit ladder with one label per scope, in reverse
declaration order. Early exits (`return`, `break`, `continue`) jump to the
appropriate label. This is the Phase 0 decision.

```c
static int cl_copyFile(void)
{
    File *src = NULL, *dst = NULL;
    Buffer *buf = NULL;
    int __cl_rc = 0;

    src = cl_acquire_file("RAM:a", MODE_OLDFILE);
    if (!src) { __cl_rc = -1; goto __cl_c2; }

    dst = cl_acquire_file("RAM:b", MODE_NEWFILE);
    if (!dst) { __cl_rc = -1; goto __cl_c2; }

    buf = cl_acquire_vec(4096, MEMF_ANY);
    if (!buf) { __cl_rc = -1; goto __cl_c1; }

    /* body ... */

__cl_c1:                /* drop scope 1 (buf) */
    cl_drop_vec(buf);
__cl_c2:                /* drop scope 0 (src and dst, reverse order) */
    cl_drop_file(dst);
    cl_drop_file(src);
    return __cl_rc;
}
```

* Labels are per scope, not per variable, so the ladder stays short.
* Generated cleanup is attributed to the scope's closing line for stepping
  (§8) and carries no user-visible statements.

## 5. Moves

* Scalar `copy` values: plain assignment.
* Owned handles: `dst = src; src = NULL;` - the nulled source is skipped by
  its later drop, so a checker bug cannot become a double release.
* Move-only aggregates: `dst = src; cl_zero(&src);` where `cl_zero` zeroes the
  whole struct. Correct because zero is the invalid sentinel for every owned
  handle and drops are null-safe; the cost is one zeroing per aggregate move,
  revisited in optimization.
* The emitted code never "moves out" of a field partially; field moves are
  lowered as whole-value moves or explicit copies by the IR.

## 6. Runtime shim

The shim is small, header-only where possible, and profile-specific:

| Piece | hosted-amiga | host (tests) | freestanding (later) |
|---|---|---|---|
| acquire/drop helpers | `Cl_acquire_*`/`Cl_drop_*` over exec/dos | same names over libc | LorraineOS entry points |
| null policy | zero handle | NULL | null |
| panic | print + `exit`/`Exit` | `abort` | halt/reset |
| debug output | `printf` (libnix) or serial | `fprintf(stderr, ...)` | serial/UART |

The shim contains no ownership logic; it only names the release functions and
the failure sentinel per handle type.

## 7. Naming

| Item | Scheme |
|---|---|
| User function | source name; module-prefixed only on collision |
| Exported function | exact source name (C ABI) |
| Local | source name if it is a valid C identifier and does not collide |
| Generated helper | `cl_` prefix (`cl_acquire_*`, `cl_drop_*`, `cl_zero`) |
| Compiled file symbols | module-qualified: `cl_<module>_<name>` |
| Typedefs | `<Module><Type>` to avoid C tag clashes |

## 8. Source mapping

* One source statement maps to one emitted statement; the emitter never splits
  or merges statements across mapped lines.
* `#line N "file.clor"` precedes each emitted statement.
* Inserted cleanup runs on the scope's closing line so stepping does not enter
  generated code.
* Result: the emitted C is fully debuggable and errors from the C compiler
  point at Clorraine sources.

## 9. No-undefined-behavior commitments

| C hazard | Emitted guarantee |
|---|---|
| Signed overflow | arithmetic emitted on unsigned or explicitly wrapped forms |
| Shift by width or negative count | checked statically; rejected (E11200) |
| Unaligned access | m68k profile default `-m68000`; no packed-struct tricks |
| Strict aliasing | no type-punning casts; all casts explicit and allowed by the IR's type rules |
| Uninitialized reads | definite-initialization checked (E10021); temporaries initialized |
| Null dereference | nullable owned use checked (E10010); raw dereference only in escape blocks |

## 10. Compile matrix (normative)

| Target | Command |
|---|---|
| GCC debug/development | `m68k-amigaos-gcc -mcrt=nix20 -m68000 -O0 -g -Wall -Wextra -Werror ... -lamiga` |
| GCC release | `... -O2 ...` |
| vbcc release | `vc +aos68k -c99 -O2 -cpu=68000 -I/opt/amiga/m68k-amigaos/ndk-include ... -lamiga` |

Phase 0 verified all four hosted legs (GCC 68000/68020, vbcc; `vamos` and both
FS-UAE boot media) with identical behavior. `-m68020` is opt-in per project.

## 11. Determinism

Same source, same profile, same compiler flags produce byte-identical C:
stable symbol names, stable temporary numbering, stable drop order, and no
timestamps or absolute paths in the output.
