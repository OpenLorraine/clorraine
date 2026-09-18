# Code Generation: Clorraine Compiler MVP

> **Document**: 03-06-codegen.md
> **Parent**: [Index](00-index.md)

## Overview

Codegen translates the typed control-flow IR into **one C99 translation unit**
per program (P10, Ch 17 §1): the profile C shim, the embedded standard-library
modules the program uses, and the program itself. Every ownership decision is
already explicit in the IR; the emitter translates it and never re-discovers
drops or defers (Ch 17 §2). `codegen.rs`, `abi.rs`, `profile.rs`, and the
`runtime/` shims implement this document; the stdlib symbols the shim provides
are owned by [03-07-stdlib.md](03-07-stdlib.md).

## Emitted File Structure

```
/* clorraine <profile> — generated; do not edit */
<shim prelude: runtime/shim_<profile>.c>            (AR-19)
<emitted typedefs, structs, enums, prototypes, globals>
<generated cl_init_<module> functions, topological order>   (P9)
<emitted function bodies, source order>                      (P13)
<int main(void) entry stub: run inits, call user main, return it>
<compiled stdlib slices the program references>
```

No timestamps, host paths, or environment data appear in the output (P13); the
`#line` directives carry the logical CLI path (AR-35).

## Naming (Ch 17 §7, P10/P11)

| Item | Scheme |
|---|---|
| Private function | `static cl_<module>_<name>` |
| Exported function | `cl_<module>_<name>` (external) |
| Generic instantiation | `cl_<name>__<typekey>`, hash-deduped |
| Handle typedef / built-in view struct | `Cl<Name>` (`ClFile`, `ClStr`, `ClSlice_u8`) |
| User aggregate | `Cl<Module>_<Type>` |
| Local | source name when it is a valid, unique C identifier; stable suffix otherwise |
| Temporary | `__cl_tmp<N>`, stable counter |
| Cleanup label | `__cl_c<N>` per scope |
| Generated helper | `cl_` prefix |

`<module>` is the dotted module path with dots replaced by underscores; C
keywords and reserved identifiers receive a stable suffix. Names are emitted
in a deterministic order (declaration order, then mangled-name order for
generics), so the same input always yields the same symbol order.

## Types and Values

- Scalars: fixed-width C typedefs from the shim (`i32`/`u32`/`u8`/`bool`/
  `usize`); `f32`/`f64` are C `float`/`double` (P15, AR-11).
- Views: pointer-and-length value structs (`ClStr`, `ClSlice_T`).
- Handles: opaque pointer typedefs; `NULL` is the zero empty state.
- `raw T` is `T *`, `cstring` is `char *`.
- Structs are C structs in declaration order, never reordered; `volatile`
  fields stay `volatile` (T7, STR-2).
- Enums are `u8`-typed constants and casts (EN-1).
- Fixed arrays are C arrays; zero extents never reach codegen (E10226).

## Statement and Expression Emission

| IR / construct | Emission |
|---|---|
| locals | hoisted declarations with zero initialization where the type has an empty state |
| move | `dst = src; cl_zero(&src);` (aggregates) or handle assignment plus `NULL` (P3, O7) |
| assignment to a live owner | drop the old value first (O3) |
| call | C call with the ABI prototypes; `take` arguments are moves |
| drop | `if (value) { Deleter(value); }` (P5, CL-4) |
| `defer` | copied into the scope's ladder entry; conditional registration carries a guard flag (CL-5) |
| `return` | move/drop epilogue then `return e;` (Ch 17 §3) |
| `branch`/`switch` | C `if`/`switch`, C-exact fallthrough (B5) |
| index | runtime bounds guard before the access (T12, E11211) |
| `/`, `%` | guard `d == 0 \|\| (a == MIN && d == -1)` first (E11210) |
| float→int cast | guard `!(f >= lo && f <= hi)` so NaN faults (E11212, PF-007) |
| `assert`/`panic` | shim `cl_trap`/`cl_panic` calls; no ladder (E4, E11213) |
| wrapping arithmetic | GCC: `-fwrapv`; vbcc: unsigned-idiom forms converted back (spike S3, PF-008) |
| shifts | counts masked to the operand width; signed `>>` sign-replicating (B7a, Ch 17 §10) |
| templates | builder and `cl_text_*` conversion calls, never `printf` (P8a, Ch 17 §9) |
| intrinsics | `sizeof`/`offsetof` fold or emit C `sizeof`; `len` from the view/array; volatile intrinsics emit C `volatile` accesses inside the shim |

The cleanup ladder follows CL-1..CL-7 (Ch 17 §4): one label per scope,
innermost first, reverse registration order, null-safe drops, early exits jump
to the leaving scope's label, and generated cleanup is attributed to the
scope's closing line so stepping never enters generated code.

## FFI ABI Mapping (`abi.rs`)

The Ch 17 §3 table is implemented literally: declared type ⇒ C ABI type
(`i8`-`u32` fixed-width typedefs, `usize`/`isize` `ULONG`/`LONG`, `bool`
`BOOL`, `cstring` `STRPTR`, `raw T` `APTR`/`T*`, handles their C typedef,
views `APTR` with a separate length parameter, native structs by value).
Parameter modes never shape the prototype (PF-002). Deleter and acquire
prototypes come from the handle/`ffi` declarations; the spike S2 fixture is
the acceptance reference.

## Module Initialization and Entry (P9, M5, M8)

Module-level initializers become `static void cl_init_<module>(void)` in the
topological order computed in [03-04-semantics.md](03-04-semantics.md), with a
byte-wise lexicographic tie-break (PF-009); `main(): i32` is emitted as
`int main(void)` in the generated stub, which runs the init functions before
the user `main` and returns its value. There are no C constructor attributes
(MOD-19); a library build is out of scope (AR-10).

## Profiles (`profile.rs`, Ch 15)

| Fact | `host` | `hosted-amiga` | `freestanding` |
|---|---|---|---|
| C compiler / flags | `cc -std=c99 -O0 -g -Wall -Wextra -Werror` (AR-37) | Ch 17 §13 GCC and vbcc commands | not implemented |
| Runtime shim | `runtime/shim_host.c` (libc) | `runtime/shim_hosted_amiga.c` (exec/dos, libnix) | — |
| Fault output | stderr, then `abort()` | stdout, then exit (Ch 17 §12) | — |
| Pointer size / endianness | host | 32-bit, big | — |
| Stack-frame warning (W10004) | 16384 bytes | 1024 bytes | — |

Selecting `freestanding` is E11200; an unknown name is E11201 (PR-5, PR-6).
Profile facts are compiled-in tables (AR-27, Ch 15 §6). The host profile's
runtime faults print to stderr and abort (AR-36), which the run-fault harness
captures.

## Warnings

| Warning | Produced by |
|---|---|
| W10000 unused local | resolve/ownership (scope end) |
| W10001 writable lend of a duplicable value | ownership (V9a, D9) |
| W10002 switch case falls through | sema/parser (B5) |
| W10003 unreachable statement | sema CFG |
| W10004 stack frame above the profile threshold | codegen (sum of local sizes; threshold from the profile) |
| W10006 unused import | resolve |

`--deny-warnings` makes any warning fatal for the exit code; the rendered
first line stays `warning CODE: message` (AR-33/AR-34).

## Driver (`driver.rs`)

The pipeline of [03-03-frontend.md](03-03-frontend.md) runs end to end; on
success the emitter's buffer is written to `-o OUT.c` (or stdout when omitted)
with a trailing newline. Errors always exit 1 (N5). The driver holds no
profile knowledge of its own; it reads the selected profile table and emits
the matching shim.

## Testing Requirements

- Spec tests (Phase 6): `#line` directives before each emitted statement;
  ladder shape for a program with nested scopes and `defer`; move zeroing;
  trap guards; ABI prototypes; module init order for two modules; byte-identical
  rebuilds (AR-35); E11200/E11201; host compile-and-run of small programs;
  the six run-fault cases on host.
- The host run-fault and corpus drivers are CLI-level tests (AR-24); codegen
  unit tests cover fragment emission and the naming table.
- The vbcc/GCC hosted-amiga legs run in `make verify`, not `cargo test`, so the
  fast loop stays container-cached.
