# Clorraine — Decision Ledger

> **Status:** accepted (Phase A complete; amended by the pre-Phase-B ergonomics review and the Phase B grammar freezes) — supersedes any conflicting text in the draft chapters
> **Scope:** every language-surface and semantics decision made before the grammar freeze

This file is the source of truth for Phase A decisions. Chapters are rewritten
against it in Phase C; where a chapter disagrees with this ledger, the ledger
wins until the chapter is updated. The grammar harvest (Phase B) encodes the
surface decisions listed here.

## 0. How to read this ledger

* Decisions are grouped by area. Each table row is one locked decision.
* "D" numbers are lineage deviations; "E" numbers are error-handling
  decisions; other identifiers are local to this document.
* Superseded draft assumptions are listed in [section 17](#17-superseded-draft-assumptions)
  so nothing from the old draft survives silently.

## 1. Semantics base (Rust lineage)

| Ref | Decision |
|---|---|
| A1 | Rust is the **normative semantic reference**: ownership, moves, borrows (many readers XOR one writer), drop scopes and drop order, and lifetime-elision rules |
| A1-pin | Pinned sources: Rust Reference ("Destructors", "Lifetime elision"), Rust Book ch. 4 and 10, NLL RFC 2094. The exact revision is recorded in `lineage.md` when that file is created |
| D1 | Deleter comes from a **type declaration** (`freedBy`), not a `Drop` trait |
| D2 | No lifetime annotation syntax; Rust's elision rules apply, and cases that would need annotations are errors |
| D4 | No unwinding; `panic` aborts |
| D5 | Temporaries drop at statement end (simplified from Rust's temporary-scope table) |
| D6 | No interior mutability (`Cell`/`RefCell`/locks in the language) |
| D7 | No `Rc`/`Arc`/`Send`/`Sync`; the language is single-threaded |
| D8 | Surface spellings differ from Rust (`func`; `const`/`borrow`/`take`; no `fn`/`&`/`&mut`) |
| D9 | A writable lend of a duplicable type affects only a copy, so it is **not** an error but a warning (W10001). Read-only lends of duplicables copy silently. Generic code needs `borrow T` to compile for `T = u8` |
| A1-safety | Safety wording: memory safety is strong **above the hardware boundary**; below it (MMIO, interrupts, raw memory) access is audited `unsafe`. Standard-library wrappers shrink that surface over time |
| Local rule IDs | The draft `OW-*`/`BR-*` rule IDs become an index for diagnostics and tests, not the source of truth |

## 2. Concurrency and asynchronous work

| Ref | Decision |
|---|---|
| C1 | The language is single-threaded. Concurrency is the OS task model, exposed through FFI wrappers (`CreateTask`, message ports, semaphores) |
| C2 | v2 adds `transfers` contracts for ownership crossing task/message boundaries; shared mutable state across tasks stays `raw`/`unsafe` |
| C3 | **No async/await.** Asynchronous work is expressed with signals, message ports, and IO requests; the appendix documents the event-loop pattern |
| C4 | A library-level async runtime is possible later; it is not a language feature |

## 3. Error handling

| Ref | Decision |
|---|---|
| E1 | Three tiers: bugs/invariants/OOM → `panic` (abort); recoverable failure without detail → `T?`; recoverable with detail → status structs or `borrow` out-params |
| E2 | **No exceptions ever** — deterministic release and no unwinding make value-based errors the only model |
| E3 | v1 uses nullable values + explicit checks + `lastError()` in FFI wrappers |
| E4 | `panic(msg)` and `assert(cond, msg)` exist in v1; they abort without running the cleanup ladder |
| E5 | **No propagation sugar** (`?`, `else`, `try`). Explicit checks are the design, permanently; sugar can be added later if ever needed |
| E6 | Failure with detail uses plain structs (`struct ParseResult { ok: bool; value: i32; error: u32; }`) or `borrow` out-params. There is no `Maybe`/`Result` language concept |

Error paths are leak-free by construction: drops run on every `return` path
through the cleanup ladder.

## 4. Values, variables, and vocabulary

| Ref | Decision |
|---|---|
| V1 | `let` declares a **mutable** binding; `const` declares an **immutable** binding. There is no `var` |
| V2 | A `const` with a constant initializer is a compile-time value (usable for sizes/embed/case labels); a local `const` may have a runtime initializer. Module-level `const` must be compile-time |
| V3 | `const` bindings may be read, copied (if duplicable), and lent read-only; assignment, moving out, and writable lends are errors |
| V4 | Markers on copy types that have no effect (for example `const n: i32`) are warnings, not errors |
| V5 | Parameter conventions: **unmarked or `const`** = read-only lend; **`borrow`** = writable lend (exclusive); **`take`** = ownership transfer. Scalars are copied |
| V6 | Call-site markers (`const x`, `borrow x`, `take x`) are **optional assertions**. If written they must match the declaration; the declaration is always authoritative; markers are never required |
| V7 | A future `--require-explicit-markers` strict mode may enforce markers; it is a compiler feature, not a language rule |
| V8 | Duplicability is **derived from fields**: scalars and structs whose fields are all duplicable are duplicable; handles and structs with resource fields are move-only. There is no `copy` keyword |
| V9 | Generic bodies record **copy obligations**; they are discharged at instantiation (errors name the instantiation site). There are no bounds except the callable bound (section 8) |
| V10 | Retired words: `own`, `borrow mut`, `move`, `out`. `?` is reserved for nullability |
| V11 | Writable lends of owned temporaries are allowed; writable lends of `const` bindings are errors |

## 5. Ownership semantics

| Ref | Decision |
|---|---|
| O1 | Moves invalidate the source; drop of a moved value is a no-op; null-safe drops run exactly once |
| O2 | Drop order: locals in reverse declaration order; struct fields in declaration order; array elements first-to-last; parameters after body locals |
| O3 | Assignment drops the old value of the target; re-initializing a moved binding is allowed |
| O4 | **Partial moves** are allowed, restricted to top-level fields of owned values (locals, `take` parameters). Nested paths and element moves are deferred |
| O5 | After a partial move: whole-value use/move/lend is an error; remaining fields stay usable; field reassignment re-initializes the struct |
| O6 | Move state is flow-sensitive; at control-flow joins a field moved on any path counts as moved (conservative) |
| O7 | **Zero = empty**: the zero bit pattern is a valid "nothing to drop" state for every droppable type. Drop functions check empty before releasing. The zeroed field is the drop flag (no Rust-style drop flags) |
| O8 | **Field borrows are allowed** and tracked conservatively as whole-struct loans (read-only field lend = shared loan; `borrow` field lend = exclusive loan). Nested reads and lends are allowed; nested moves are deferred |
| O9 | **No local view expressions.** Bindings hold values; views arrive from parameters and view-typed returns (`let part = head(buf, 8);`) |
| O10 | Globals must be duplicable (`copy`) or `const`; `own` globals are banned (no exit drops). Mutable global state is allowed with constant or runtime initializers (section 13) |
| O11 | Borrow rules: many readers XOR one writer; borrows never outlive owners; borrows escape only when parameter-derived; v1 is lexical, non-lexical checking comes later |
| O12 | **Reborrowing views**: a read-only view derived from a borrow is a shared reborrow; a writable view derived from a writable borrow is an exclusive reborrow limited to the expression. This makes `borrow bufferBytes(buf)` valid when `buf` is itself a parameter |

## 6. Types

| Ref | Decision |
|---|---|
| T1 | Scalars: `i8 i16 i32 u8 u16 u32 bool void usize isize` plus `f32`/`f64`. Floating point covers arithmetic, comparison, casts, and template formatting in v1; math functions are stdlib. `'x'` is a `u8` byte literal. Decimal literals like `1.5` are `f64`; mixed `f32`/`f64` promotes to `f64`; integer↔float conversions are explicit. Out-of-range float→int conversion is a runtime fault; floating-point division by zero follows IEEE (Inf/NaN), a deliberate difference from the integer rule (B7) |
| T2 | Strings: **`String`** owned handle (`handle String freedBy FreeVec`; one block `[len][cap][bytes][NUL]`; move-only; mutable through `borrow`), **`str`** immutable view (pointer + length, copies freely, byte-oriented), **`cstring`** raw zero-terminated pointer for FFI |
| T3 | String literals have **static origin**; they are returnable, storable in `static str` fields, and usable as `const`. `static str` widens to `str` implicitly |
| T4 | Struct fields may hold `String`, `cstring`, and `static str`; plain `str` fields are banned in v1 (view-field ban) |
| T5 | String conversions are explicit (`text`, `bytes`, `clone`/`stringOf`, `toCString`); no hidden allocation; `String` keeps a trailing NUL for FFI convenience |
| T6 | `raw T` is an unchecked copy pointer (nullable as `raw T?`). Dereference, indexing, pointer arithmetic, pointer casts, and address-of require `unsafe`. Declaring/passing/storing/comparing is allowed outside `unsafe`. Integer → `raw` is a value cast allowed outside `unsafe`. The escape block is a greppable audit marker, not a provenanced scope |
| T7 | `volatile`: six intrinsics (`readVolatile8/16/32`, `writeVolatile8/16/32`), a **field-level qualifier** for memory-mapped overlay structs, and a `volatile` qualifier on module-level globals shared with interrupt handlers. Overlay types cannot be locals, copied, dropped, borrowed, or passed by value; access is only through `raw` pointers inside `unsafe`. Emission uses C `volatile` with no folding/caching/reordering. Module-level `const raw Regs = <address>` base constants are allowed so functions do not re-derive register bases. New keywords: `volatile`, `intrinsic` |
| T8 | Nullability: `T?` for pointer-like types (handles, `raw`, `cstring`, views). `T` widens into `T?`; narrowing requires `== null` / `!= null` checks; use before the check is an error. Nullable returns and nullable parameters are allowed; parameters follow the ordinary conventions (unmarked/`const` read-only lend, `borrow` writable lend, `take` transfer). `null` is the only empty state: `let f: File? = null; openInto(path, borrow f);` is the out-parameter pattern; there are no zero literals for handles. No `?.` or `??` |
| T9 | Views: slices `T[]` and `str` are view types; view returns derive from lends or static storage; view fields are banned except `static str` |
| T10 | **No tagged unions and no `match`.** Data modeling uses C-style tagged structs (all fields present, zero-safe) with C-exact `switch`. Error detail uses status structs. FFI C unions stay in `raw`/`unsafe` wrappers |
| T11 | A struct containing any `volatile` field is an overlay type (see T7). Raw fields are allowed in structs; using them for pointer operations still requires `unsafe` |
| T12 | **Indexing is always checked** on `T[]`, `str`, and arrays; out-of-range access traps. The only unchecked access is explicit raw/`unsafe` or stdlib `unchecked` accessors; no profile disables safety checks. Interrupt-context code uses the unsafe accessors |

## 7. Functions, returns, locals, and statements

| Ref | Decision |
|---|---|
| F1 | Locals infer type and mode from the initializer; explicit annotations are always allowed. Globals, signatures, and fields stay explicit |
| F2 | Returns are **by value** (copy for duplicable types, ownership transfer for move-only) or by **view type** (slices, `str`). There is no `const`/`borrow`/`ref` return marker; aggregate view returns are deferred |
| F3 | Function values are exact-signature C function pointers (`func(T): R`). No variadics, default arguments, named arguments, tuples, or multiple returns in v1 |
| F4 | Struct literals: `Point { x: 1, y: 2 }` — fields by name, order-independent, duplicates and missing fields are errors; `Point {}` is the explicit zero value; trailing commas allowed; update syntax deferred |
| F5 | `defer { ... }` runs at scope exit and joins the cleanup ladder (drops and defers run in reverse registration order). It is code motion, not a closure: the block is checked at the exit context; it runs on `return`/`break`/`continue`, not on `panic` |
| F6 | Control flow and operators are **C-exact** (section 14) |
| F7 | **Recursion is unrestricted**: direct, indirect, mutual, and generic recursion are allowed (the C activation stack replaces Blend65's SFA model; Blend65 codes `E10180`/`E10181` are not adopted). Stack depth is a runtime concern with documented guidance |
| F8 | **Dot-call** is part of the surface: `x.f(y)` resolves to the free function `f(x, y)` (field first, then functions; static resolution, no receiver semantics, no vtables). Standard-library APIs are written dot-call style |
| F9 | **Definite assignment**: reading a local before it is assigned on every path is a compile error (no zero-fill, no unspecified values). `borrow T?` out-parameters are exempt |

## 8. Generics

| Ref | Decision |
|---|---|
| G1 | Generics are **in v1**: TypeScript-like surface (`<T>` on functions and structs), explicit type arguments allowed |
| G2 | Inference is **arguments-only**; constructors without arguments need explicit type arguments (`vecNew<i32>()`) |
| G3 | Lowering is **monomorphization** with mangled symbols; identical instantiations are deduped by canonical type key; whole-program compilation |
| G4 | `export` on a generic declaration is an error; write a concrete exported wrapper |
| G5 | Instantiation recursion cap (error) and an opt-in instantiation size/count report |
| G6 | No bounds except **`F: func(T, ...): R`** for callable parameters. Copy requirements are obligations (V9), not bounds |
| G7 | Values of `func(...)`-bound type parameters are **callable-only**: call or pass, never store or return. Generic aggregates cannot be instantiated with ephemeral type arguments |
| G8 | Out of scope: const generics, variadic type parameters, parameter defaults, generic aliases, generic methods, variance annotations, type-level programming |

## 9. Lambdas

| Ref | Decision |
|---|---|
| L1 | Full lambda **syntax is frozen now**: `(params) => expr`, `(params) => { ... }` with typed parameters, optional capture list `[borrow x, ...]` (parse `take` but reject it) |
| L2 | Captures: duplicable types copy; non-duplicable types are read-only borrowed; parent mutation requires an explicit `[borrow x]` capture. No `take` captures in v1 |
| L3 | Closures are **ephemeral**: they cannot be returned, stored in fields/globals/arrays, or captured by another closure; they can be called and passed. No heap environments |
| L4 | Semantics are staged (syntax + AST from day one): stage 1 capture-less, stage 2 copy/borrow captures, stage 3 `[borrow]`, stage 4 `F: func(...)` generics; stage 5 C-hook trampolines (v1.1) |
| L5 | C boundary: `func(...)` is a plain C function pointer; only capture-less lambdas match it. Capturing closures cross only through synchronous C hooks via generated trampolines (contract `borrows`), never through `stores` APIs |

## 10. Strings and templates

| Ref | Decision |
|---|---|
| S1 | Interpolation is in v1: `$"Hi, {name}!"` with `{{`/`}}` escapes |
| S2 | Formattable types: `str`, `String`, integer types, `f32`/`f64`, `bool`. Float output uses a documented fixed precision in v1 (precision control later). Other types are compile errors with a conversion hint |
| S3 | No format specs in v1 (`{x:04x}`); use conversion functions (`toHex`, `toDecimal`). A future spec syntax must avoid the `:` ambiguity with the conditional operator |
| S4 | No nested templates in v1; struct literals inside placeholders are allowed (balanced braces) |
| S5 | A template with placeholders is an owned `String` (one allocation); placeholder-free templates are plain `str` literals |
| S6 | Future (additive): more formattable types, nested templates, raw/multi-line strings, codepoint iteration, small-string optimization |

## 11. FFI and contracts

| Ref | Decision |
|---|---|
| X1 | Opaque resources: `handle T freedBy F;` declared once, in the library. `F` must be an FFI-declared function taking `take T` and returning `void`; one deleter per type. v1.1 design item (F4): allow ordinary Clorraine functions as `freedBy` deleters, enabling a built-in `Box<T>` with generated drop glue |
| X2 | Native structs do not use `freedBy`; they drop structurally (fields in declaration order) |
| X3 | FFI declarations use ordinary signatures and the ordinary vocabulary. The draft clause words `consumes`, `value`, `borrows`, `borrows mut`, and `nullable` are removed; nullability lives in the type (`File?`) |
| X4 | Uncontracted extern calls may only appear inside `unsafe`; passing owned/borrowed values to them is an error (FFI-1) |
| X5 | `stores` and `transfers` stay deferred (v2) and live in audited `raw` wrappers |
| X6 | Application code never writes `ffi` blocks; the stdlib wraps them |

## 12. Modules and initialization

| Ref | Decision |
|---|---|
| M1 | `module A.B;` — one per file, first statement, dotted name |
| M2 | Declaration merging: several files may declare the same module; duplicate declarations are errors |
| M3 | `export` marks public declarations; default is module-private |
| M4 | Imports: `import { X, Y as Z } from A.B;` — named lists only, no wildcards. Qualified access (`A.B.f()`) is always available |
| M5 | **No global constructors, no C ctor attributes.** The compiler collects module-level initializers into generated `cl_init_<module>` functions and runs them before `main` in dependency order (imports plus modules referenced by initializers) |
| M6 | A dependency cycle that involves runtime-initialized globals is a compile error; cycles of `const`-only modules are fine |
| M7 | Globals stay duplicable; initializer expressions must be non-nullable; failures belong in explicit `init()` functions called from `main` |
| M8 | Entry: exactly one `main()` for executables; signature `main(): i32`. Library builds export a generated init entry that the host calls |
| M9 | `ffi` blocks are module-scoped and not exported; share wrappers through exported functions |
| M10 | Circular imports are allowed; declarations resolve globally |

## 13. Control flow, operators, and comments

| Ref | Decision |
|---|---|
| B1 | Parentheses are C-style: `if (cond)`, `while (cond)`, `switch (x)`, `for ( ... )`; braces are mandatory |
| B2 | `for (;;)` is the infinite loop (no `loop` keyword). C-style `for (init; cond; step)` and element loops `for (x in xs)` |
| B3 | `do { ... } while (cond);` is included |
| B4 | No range syntax; indices use the C-style `for` |
| B5 | `switch` is C-exact: fallthrough by default, `break` to exit, no `fallthrough` keyword, no exhaustiveness requirement. A `// fallthrough`-silenced warning is optional later |
| B6 | `break`/`continue` per C. No labels and no `goto` |
| B7 | Operators are the C set with **defined semantics** (no UB): arithmetic wraps, shifts mask counts to the width, integer division by zero is a runtime fault (floating point follows IEEE Inf/NaN instead, see T1). Includes compound assignment, `++`/`--` (prefix and postfix), and the conditional operator. The comma operator is excluded |
| B8 | Casts use exactly one form: `T(expr)`. No C-style casts |
| B9 | Comments: `//` and `/* */` (non-nesting). `///` is recognized as **documentation** attached to the next declaration; v1 emits nothing and a future `--doc` tool reads it |

## 14. Diagnostics

| Ref | Decision |
|---|---|
| N1 | 5-digit codes in area blocks: `E100xx` ownership/borrows/null use, `E101xx` FFI, `E102xx` types, `E103xx` generics, `E104xx` lambdas, `E105xx` strings/templates, `E106xx` unsafe/raw/volatile, `E107xx`–`E110xx` reserved, `E111xx` modules/init, `E112xx` profiles, `E20xxx` lexical/syntax, `E90xxx` staged "not yet supported" features, `W100xx` warnings |
| N2 | Codes are stable forever; removed codes stay reserved; new codes append |
| N3 | `spec/14-diagnostics.md` is the normative registry: message template, trigger, example, chapter reference for every code |
| N4 | Message format: the first line is always `error CODE: message` or `warning CODE: message`; compile-fail tests assert exactly that |
| N5 | The compiler reports all recoverable errors (not fail-fast) and exits nonzero |
| N6 | Warnings are deniable by default and escalable with `--deny-warnings`; warning codes are stable |
| N7 | `check.sh` cross-validates codes used in chapters against the registry |
| N8 | New codes to assign in Phase C: definite assignment (F9), float conversion rules (T1), and the indexing trap (T12) |

## 15. Codegen contract

| Ref | Decision |
|---|---|
| P1 | C99 common subset accepted by both GCC 16.2 (`m68k-amigaos-gcc`) and vbcc 0.9 as reported by the built SDK binaries; exact source pins live in `docker/versions.env`; no extensions, no VLAs, no compiler-specific tricks |
| P2 | Cleanup ladder: single-exit, one label per scope, reverse order, null-safe drops; `defer` entries join the same ladder |
| P3 | Moves lower to copy + zero; partial moves zero the field |
| P4 | Zero = empty layout guarantee (O7) |
| P5 | `freedBy` drops emit `Deleter(value)` calls, null-safe, in the ladder |
| P6 | `String` layout: one AllocVec block `[len][cap][bytes][NUL]`; `FreeString` deleter. `str` is a value struct (pointer + length) |
| P7 | Lambdas: lifted `static` functions; capture-less = plain function pointers; capturing = two-word callable (function + stack env). Trampolines for C hooks arrive in v1.1 |
| P8 | Interpolation desugars to string-builder/`toText` calls |
| P9 | Module init: generated `cl_init_<module>` functions in dependency order; a generated entry stub runs them before the user's `main` |
| P10 | Single translation unit in v1; internal symbols `static`, external only for `export` and the entry |
| P11 | Mangling: `cl_<module>_<name>`; instantiations `cl_<name>__<typekey>` with hash dedupe |
| P12 | Defined arithmetic: wrapping via emitter strategy (`-fwrapv` for GCC; verify vbcc or emit unsigned-based arithmetic), masked shifts, division by zero faults |
| P13 | `#line` mapping for every user statement location; deterministic output ordering |
| P14 | Build matrix stays: `-lamiga`, `-mcrt=nix20`, `-m68000` (opt-in `-m68020`); verified on vamos (GCC/vbcc) and FS-UAE (dir drive and ADF) |
| P15 | Floats lower to C `float`/`double`. The SDK verifies GCC and vbcc soft-float ABIs (argument passing and returns) as a new matrix leg, plus an opt-in FPU (`-m68881`) build leg |

## 16. Process decisions

| Ref | Decision |
|---|---|
| R1 | Syntax is frozen in the grammar before semantics are staged; later features are additive and must not reopen the front of the compiler chain |
| R2 | Every artifact references the ledger; chapters are rewritten in Phase C against the grammar and this ledger |
| R3 | `preflight` reviews the specification before the compiler MVP starts |

## 17. Superseded draft assumptions

| Draft said | Now |
|---|---|
| Generics deferred to v2 | Generics in v1 (monomorphized) |
| No closures | Staged lambdas in v1 (captures included by stage 4) |
| Placeholder keywords `own`, `borrow`, `borrow mut`, `move` | `const`/`borrow`/`take`; `let`/`const` for bindings |
| `copy struct` marker | Duplicability derived from fields |
| FFI clauses `owns`/`consumes`/`value`/`borrows`/`nullable` | `handle T freedBy F` + ordinary signatures + `T?` |
| `-> const T` / `-> borrow T` view returns | Dropped; views come from view types; aggregate views deferred |
| `loop {}` and range loops | C-exact `for` forms; no range syntax |
| `switch` auto-break and `fallthrough` | C-exact fallthrough |
| Doc comments `///` | `///` frozen as documentation comments; `--doc` tooling later |
| Module init: dependency schedule (Blend65) | Generated dependency-ordered init functions (portable C) |
| Module init: explicit-only (early Phase A proposal) | Superseded by the generated init decision above |
| Tagged unions + `match` | Cancelled; C-style tagged structs + `switch` |
| Owning text = `Buffer` | `String` / `str` / `cstring` |
| `var` for mutable locals | `let` is mutable; `const` immutable |
| Writable borrow of copy types banned | Warning (W10001: writes affect only the copy) |
| `out` parameters | Rejected |
| Local view expressions | None |
| `nullable` keyword in contracts | `T?` type suffix |
| `if cond` without parentheses | C-style `if (cond)` |
| No floating point in v1 | `f32`/`f64` added (arithmetic, comparison, casts, formatting) |
| Blend65 no-recursion (SFA/6502) | Recursion unrestricted (C activation stack) |
| `T{}` zero literals for handles (review draft) | Rejected; `null` is the only empty state |
| `for (e in xs)` binding unspecified (draft) | Copy by default; `for (borrow e in xs)` writes through with per-element loans |
| Ch 04 draft: assignment is a statement, not an expression | C expression grammar with a `void` result (GF2) |

## 18. Standard library scope (v1)

The library is where most ergonomics live. v1 ships, in priority order:

| Item | Purpose |
|---|---|
| `Buffer` (owned bytes) | Bulk IO, bitmaps, samples; safe slices into owned storage |
| `Array<T>` (growable) | Default dynamic container; most workloads need no other structure |
| One error/status carrier | Detail-carrying failures; `T?` stays detail-free; partial-IO conventions |
| Format helpers | `toHex32`, padding, and float formatting used by templates |
| Per-chip MMIO wrappers | Safe accessors over the volatile intrinsics; `const raw` register bases |
| Interrupt/hook dispatcher | C-side qualified handler plus token add/remove; handlers are allocation-free, drop-free, panic-free |
| `Span` | Offset-based parsing helper (view fields stay banned) |

Standard-library APIs are written dot-call style (F8). v1.1 candidate:
built-in `Box<T>` with compiler-generated drop glue, which needs the X1
amendment to allow ordinary Clorraine `freedBy` functions.

## 19. Phase B grammar freezes

The grammar harvest ([grammar.ebnf.md](grammar.ebnf.md)) froze four surface
questions the ledger left open. The complete freeze resolution table lives in
[grammar-deltas.md](grammar-deltas.md) section 8.

| Ref | Decision |
|---|---|
| GF1 | Generic explicit type arguments use angle brackets in type and expression positions; expression position uses a bounded speculative parse: `<` is comparison unless a type list closes immediately before `(` or `{` |
| GF2 | Assignment keeps the C expression grammar, but its result is `void`; chaining and value use are compile errors |
| GF3 | No integer literal suffixes; literals are context-typed (default `i32`; hex and binary may use the full unsigned width) |
| GF4 | `unsafe` is a prefix expression operator (`unsafe p.next`) and a statement block (`unsafe { ... }`); there is no block-expression value |

## 20. Phase C clarifications

The chapter rewrites surfaced gaps that the ledger did not cover. These
clarifications close them without changing any accepted decision; chapters and
the diagnostics registry follow them.

| Ref | Clarification |
|---|---|
| T1a | Same-signedness integer widening is implicit: `u8`→`u16`/`u32`, `u16`→`u32`, `i8`→`i16`/`i32`, `i16`→`i32`. Mixed-sign and narrowing integer conversions, `usize`/`isize` conversions, and integer↔float conversions are explicit. `f32`→`f64` is implicit. |
| B7a | `>>` on a signed integer is sign-replicating (arithmetic); on an unsigned integer it is zero-filling. Shift counts are masked to the operand width in both cases. |
| T6a | An explicit `raw`→integer value cast (for example `usize(p)`) is allowed outside `unsafe`; it reads the address as a number and performs no dereference. |
| T2a | Core handle types declare their own FFI deleters instead of reusing the deleter of a differently-typed handle: `handle String freedBy FreeString;` with `func FreeString(take s: String): void;`, and `handle Buffer freedBy FreeBuffer;`. C-facing types keep the C symbols (`Memory`/`FreeVec`, `Library`/`CloseLibrary`, `File`/`Close`, `MsgPort`/`DeleteMsgPort`). This makes T2's shorthand type-correct under X1. |
| T10a | Enums compare with `==` and `!=` only; ordering an enum is E10222. Members without an explicit value number sequentially from 0. |
| V9a | A writable lend parameter instantiated with a duplicable type binds a copy (V5). Writes affect that copy only, and W10001 marks the needless writable lend; read-only lends of duplicables copy silently. Generic copy obligations are recorded and discharged at instantiation. |
| X1a | The `freedBy` name is a qualified name resolved like any other declaration; the deleter is an FFI function taking `take T` and returning `void`. |
| E112xx | Runtime fault codes live in the E112xx block next to the profile limits (`E11210` division by zero, `E11211` index out of range, `E11212` float-to-integer range, `E11213` assertion failure). They print the standard `error CODE: message` first line and abort (N8). |
| N8a | New compile-time codes assigned in Phase C: definite assignment `E10021`, the index checks `E10216`/`E11211`, the float rules `E11212`, and the completeness codes `E10218`-`E10222`. `E10605` is retired before release; misplaced `volatile` is `E10208`. |
| S2a | Template float placeholders use a fixed v1 precision of six significant digits; precision control is deferred (S6). The public conversion is `text` (T5); `toText` in P8 names the internal desugaring helper. |
| S3a | The public format helpers are `toHex` and `toDecimal` (S3). Fixed-width variants such as `toHex32` are library-internal, not surface API. |
| P8a | Templates desugar to string-builder and `text` conversion calls; the exact helper names are codegen-internal. |
| M7a | A module-level initializer expression that is nullable is a type error (E10200): globals are non-nullable values. |
| W10004a | Stack-frame warning thresholds are profile facts with v1 defaults: `hosted-amiga` 1024 bytes, `host` 16384 bytes, `freestanding` 1024 bytes. |
| A5a | The `freestanding` profile is implemented provisionally: m68k, 32-bit, big-endian, no libc, a thin entry/exit/console surface, and a raw m68k hunk. LorraineOS may adopt or replace it. ARM64 and the 64-bit `usize`/`isize` width are deferred to the future port. |

## 21. Preflight rulings

The Phase E preflight gate ([preflight-report.md](preflight-report.md)) raised
49 findings. Every ruling below was accepted and applied; the chapters, the
grammar, and the diagnostics registry follow it. Status is `applied` for all.

| PF | Ruling | Status |
|---|---|---|
| PF-001 | `borrow T?` out-parameters are in-out ownership slots; E10034 does not apply | applied |
| PF-002 | FFI declarations lower with an explicit ABI table; modes are semantic only | applied |
| PF-003 | `borrow` and `take` are reserved keywords (50 total) | applied |
| PF-004 | Name resolution: scopes, declaration-before-use, shadowing, imports, merged modules | applied |
| PF-005 | Usual arithmetic unification with T1a widenings; mixed-sign is E10200 | applied |
| PF-006 | Integer `/` and `%` fault E11210 on a zero divisor and on MIN/-1 | applied |
| PF-007 | Float-to-integer range check using `!(f >= lo && f <= hi)`, so NaN faults E11212 | applied |
| PF-008 | `-fwrapv` for GCC; vbcc uses unsigned arithmetic converted back | applied |
| PF-009 | Module init order is a topological order with lexicographic tie-break | applied |
| PF-010 | Struct-literal fields and array-literal elements evaluate in source order | applied |
| PF-011 | Rule-ID renames: `LOOP-*`, `STMT-*`, `STR-*`, `ERR-*` | applied |
| PF-012 | Only `intrinsic func` declarations bind reserved built-in names | applied |
| PF-013 | `take` parameters are not elision candidates; view return is E10036 | applied |
| PF-014 | Standard-library appendix with normative signatures and ownership | applied |
| PF-015 | Constant-expression forms, folding semantics, and diagnostics | applied |
| PF-016 | Dot-call candidate set and deterministic ambiguity rule (E10225) | applied |
| PF-017 | E10037 retired into E11106 | applied |
| PF-018 | Uncontracted externs require `unsafe` (E10100) and accept scalars/`raw`/`cstring` | applied |
| PF-019 | Staged tests moved to an excluded directory | applied |
| PF-020 | Run-test suite for E11210-E11213; warning escalation documented | applied |
| PF-021 | Any zero extent is E10226 | applied |
| PF-022 | Implicit `String` -> `str` widening creates a shared loan | applied |
| PF-023 | Interrupt-handler constraints are an audited convention in v1 | applied |
| PF-024 | Chapter headers split into Ledger / Grammar freeze / Ergonomics | applied |
| PF-025 | Appendix API lists aligned with Ch 12; `Message` resolved | applied |
| PF-026 | Float matrix reworded to planned/not yet verified | applied |
| PF-027 | Build-matrix claim restated precisely | applied |
| PF-028 | P6 deleter corrected to `FreeString` | applied |
| PF-029 | Broken links fixed; `check.sh` validates nested markdown | applied |
| PF-030 | Six cross-references repointed | applied |
| PF-031 | `:`-inside-placeholder is a parser detection for E10501 | applied |
| PF-032 | E11100 trigger widened to same-module cycles | applied |
| PF-033 | README validation description refreshed | applied |
| PF-034 | T6a raw-to-integer cast rows added | applied |
| PF-035 | Placeholder-free `$"..."` unified to `str` | applied |
| PF-036 | E10033 and E10200 triggers corrected | applied |
| PF-037 | Bare `func ...;` declarations moved into `ffi` blocks | applied |
| PF-038 | P1 quotes the versions the SDK binaries report; pins live in `versions.env` | applied |
| PF-039 | NDK result types documented for contract declarations | applied |
| PF-040 | Nested temporaries drop at statement end in reverse creation order | applied |
| PF-041 | Reading order and Phase 2 stage list added; `Depends on` is informational | applied |
| PF-042 | Phase 2 `#line` spike under vbcc named | applied |
| PF-043 | Embed paths are relative and inside the project root (E10214) | applied |
| PF-044 | Raw laundering reworded as an audited convention | applied |
| PF-045 | Stack-depth guidance published; freestanding owns its policy | applied |
| PF-046 | Missing `Depends on` chapters added and marked informational | applied |
| PF-047 | Emitted-type naming: `Cl<Name>` for handles/views, `Cl<Module>_<Type>` for aggregates | applied |
| PF-048 | Unary `+` not adopted; write the operand directly | applied |
| PF-049 | `-std=c99` added to the GCC flags | applied |
